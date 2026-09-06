use relayterm_pty::{NativeControl, NativeSession, PtyError, SpawnRequest, write_input};
use relayterm_terminal::{
    DEFAULT_SCROLLBACK_BYTES, TerminalError, TerminalSnapshot, TerminalState,
};
use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
    io::{Read, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
    time::Duration,
};
use uuid::Uuid;

pub const MAX_LIVE_SESSIONS: usize = 8;
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
pub const MAX_INPUT_FRAME: usize = 64 * 1024;
pub const MAX_ATTACHMENTS_PER_SESSION: usize = 8;
const MAX_LAUNCH_RECEIPTS: usize = 128;
static NEXT_ATTACHMENT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisorError {
    Reference,
    Conflict,
    ResourceLimit,
    Final,
    Io,
    ResnapshotRequired(u64),
}

struct InputChunk(Vec<u8>);

#[derive(Eq, PartialEq)]
pub struct LaunchKey {
    pub definition_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub program: OsString,
    pub arguments: Vec<OsString>,
    pub working_directory: std::path::PathBuf,
    pub rows: u16,
    pub columns: u16,
}

#[derive(Clone, Copy)]
pub struct LaunchOutcome {
    pub instance_id: Uuid,
    pub session_id: Uuid,
    pub succeeded: bool,
}

pub enum LaunchAdmission {
    New,
    Existing(LaunchOutcome),
}

struct Receipt {
    key: LaunchKey,
    outcome: Option<LaunchOutcome>,
}

struct InputOwner {
    connection: Uuid,
    lease: u64,
    next_sequence: u64,
}

#[derive(Clone, Copy)]
struct Attachment {
    id: Uuid,
    stream_id: u64,
}

struct Session {
    instance_id: Uuid,
    terminal: Arc<Mutex<TerminalState>>,
    control: Mutex<Option<NativeControl>>,
    input: SyncSender<InputChunk>,
    queued_input: Arc<AtomicU64>,
    input_owner: Mutex<Option<InputOwner>>,
    attachments: Mutex<HashMap<Uuid, Attachment>>,
    next_lease: AtomicU64,
    final_state: AtomicBool,
    terminate_requested: AtomicBool,
}

pub struct ExitObservation {
    pub instance_id: Uuid,
    pub code: i32,
    pub terminated: bool,
}

#[derive(Default)]
pub struct SessionSupervisor {
    sessions: Mutex<HashMap<Uuid, Arc<Session>>>,
    receipts: Mutex<(HashMap<Uuid, Receipt>, VecDeque<Uuid>)>,
}

impl SessionSupervisor {
    pub fn launch(
        &self,
        session_id: Uuid,
        instance_id: Uuid,
        request: SpawnRequest,
    ) -> Result<(), SupervisorError> {
        let mut sessions = self.sessions.lock().map_err(|_| SupervisorError::Io)?;
        if sessions.contains_key(&session_id) {
            return Err(SupervisorError::Conflict);
        }
        if sessions
            .values()
            .filter(|session| !session.final_state.load(Ordering::Acquire))
            .count()
            >= MAX_LIVE_SESSIONS
        {
            return Err(SupervisorError::ResourceLimit);
        }
        let rows = request.rows;
        let columns = request.columns;
        let native = NativeSession::spawn(request).map_err(map_pty)?;
        let (control, writer, reader) = native.into_parts();
        let terminal = Arc::new(Mutex::new(
            TerminalState::new(rows, columns, DEFAULT_SCROLLBACK_BYTES)
                .map_err(|_| SupervisorError::ResourceLimit)?,
        ));
        let (input, receiver) = mpsc::sync_channel(16);
        let queued_input = Arc::new(AtomicU64::new(0));
        spawn_writer(writer, receiver, queued_input.clone());
        spawn_reader(reader, terminal.clone());
        sessions.insert(
            session_id,
            Arc::new(Session {
                instance_id,
                terminal,
                control: Mutex::new(Some(control)),
                input,
                queued_input,
                input_owner: Mutex::new(None),
                attachments: Mutex::new(HashMap::new()),
                next_lease: AtomicU64::new(1),
                final_state: AtomicBool::new(false),
                terminate_requested: AtomicBool::new(false),
            }),
        );
        Ok(())
    }

    pub fn admit_launch(
        &self,
        receipt_id: Uuid,
        key: LaunchKey,
    ) -> Result<LaunchAdmission, SupervisorError> {
        let mut receipts = self.receipts.lock().map_err(|_| SupervisorError::Io)?;
        if let Some(receipt) = receipts.0.get(&receipt_id) {
            if receipt.key != key {
                return Err(SupervisorError::Conflict);
            }
            return receipt
                .outcome
                .map(LaunchAdmission::Existing)
                .ok_or(SupervisorError::Conflict);
        }
        while receipts.0.len() >= MAX_LAUNCH_RECEIPTS {
            if let Some(expired) = receipts.1.pop_front() {
                receipts.0.remove(&expired);
            }
        }
        receipts
            .0
            .insert(receipt_id, Receipt { key, outcome: None });
        receipts.1.push_back(receipt_id);
        Ok(LaunchAdmission::New)
    }

    pub fn complete_launch(
        &self,
        receipt_id: Uuid,
        outcome: LaunchOutcome,
    ) -> Result<(), SupervisorError> {
        let mut receipts = self.receipts.lock().map_err(|_| SupervisorError::Io)?;
        let receipt = receipts
            .0
            .get_mut(&receipt_id)
            .ok_or(SupervisorError::Reference)?;
        receipt.outcome = Some(outcome);
        Ok(())
    }

    pub fn cancel_launch(&self, receipt_id: Uuid) {
        let Ok(mut receipts) = self.receipts.lock() else {
            return;
        };
        if receipts
            .0
            .get(&receipt_id)
            .is_some_and(|receipt| receipt.outcome.is_none())
        {
            receipts.0.remove(&receipt_id);
            receipts.1.retain(|current| *current != receipt_id);
        }
    }

    pub fn acquire_input(
        &self,
        session_id: Uuid,
        connection: Uuid,
    ) -> Result<u64, SupervisorError> {
        let session = self.session(session_id)?;
        if session.final_state.load(Ordering::Acquire) {
            return Err(SupervisorError::Final);
        }
        let mut owner = session
            .input_owner
            .lock()
            .map_err(|_| SupervisorError::Io)?;
        if owner.is_some() {
            return Err(SupervisorError::Conflict);
        }
        let lease = session.next_lease.fetch_add(1, Ordering::AcqRel);
        *owner = Some(InputOwner {
            connection,
            lease,
            next_sequence: 1,
        });
        Ok(lease)
    }

    pub fn release_input(
        &self,
        session_id: Uuid,
        connection: Uuid,
        lease: u64,
    ) -> Result<(), SupervisorError> {
        let session = self.session(session_id)?;
        let mut owner = session
            .input_owner
            .lock()
            .map_err(|_| SupervisorError::Io)?;
        if !matches!(&*owner, Some(current) if current.connection == connection && current.lease == lease)
        {
            return Err(SupervisorError::Conflict);
        }
        *owner = None;
        Ok(())
    }

    pub fn release_connection(&self, connection: Uuid) {
        let Ok(sessions) = self.sessions.lock() else {
            return;
        };
        for session in sessions.values() {
            if let Ok(mut owner) = session.input_owner.lock()
                && owner
                    .as_ref()
                    .is_some_and(|current| current.connection == connection)
            {
                *owner = None;
            }
            if let Ok(mut attachments) = session.attachments.lock() {
                attachments.remove(&connection);
            }
        }
    }

    pub fn attach(
        &self,
        session_id: Uuid,
        connection: Uuid,
    ) -> Result<(Uuid, u64, TerminalSnapshot), SupervisorError> {
        let session = self.session(session_id)?;
        let mut attachments = session
            .attachments
            .lock()
            .map_err(|_| SupervisorError::Io)?;
        let attachment = if let Some(existing) = attachments.get(&connection) {
            *existing
        } else {
            if attachments.len() >= MAX_ATTACHMENTS_PER_SESSION {
                return Err(SupervisorError::ResourceLimit);
            }
            let ordinal = NEXT_ATTACHMENT.fetch_add(1, Ordering::AcqRel);
            let value = Attachment {
                id: Uuid::from_u128((0xa771_u128 << 64) | u128::from(ordinal)),
                stream_id: ordinal,
            };
            attachments.insert(connection, value);
            value
        };
        let snapshot = session
            .terminal
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .snapshot()
            .map_err(|_| SupervisorError::ResourceLimit)?;
        Ok((attachment.id, attachment.stream_id, snapshot))
    }

    pub fn detach(&self, session_id: Uuid, connection: Uuid) -> Result<(), SupervisorError> {
        let session = self.session(session_id)?;
        session
            .attachments
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .remove(&connection);
        Ok(())
    }

    pub fn read_output(
        &self,
        session_id: Uuid,
        connection: Uuid,
        attachment_id: Uuid,
        after_offset: u64,
    ) -> Result<(u64, u64, Vec<u8>), SupervisorError> {
        let session = self.session(session_id)?;
        let attachments = session
            .attachments
            .lock()
            .map_err(|_| SupervisorError::Io)?;
        let attachment = attachments
            .get(&connection)
            .filter(|attachment| attachment.id == attachment_id)
            .ok_or(SupervisorError::Conflict)?;
        let stream_id = attachment.stream_id;
        let result = session
            .terminal
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .output_since(after_offset, relayterm_protocol::TERMINAL_DATA_LIMIT);
        match result {
            Ok((next, data)) => Ok((stream_id, next, data)),
            Err(TerminalError::ResnapshotRequired) => {
                Err(SupervisorError::ResnapshotRequired(stream_id))
            }
            Err(_) => Err(SupervisorError::ResourceLimit),
        }
    }

    pub fn input(
        &self,
        session_id: Uuid,
        connection: Uuid,
        lease: u64,
        sequence: u64,
        bytes: Vec<u8>,
    ) -> Result<(), SupervisorError> {
        if bytes.is_empty() || bytes.len() > MAX_INPUT_FRAME {
            return Err(SupervisorError::ResourceLimit);
        }
        let session = self.session(session_id)?;
        if session.final_state.load(Ordering::Acquire) {
            return Err(SupervisorError::Final);
        }
        let mut owner = session
            .input_owner
            .lock()
            .map_err(|_| SupervisorError::Io)?;
        let Some(current) = owner.as_mut() else {
            return Err(SupervisorError::Conflict);
        };
        if current.connection != connection
            || current.lease != lease
            || current.next_sequence != sequence
        {
            return Err(SupervisorError::Conflict);
        }
        let amount = u64::try_from(bytes.len()).map_err(|_| SupervisorError::ResourceLimit)?;
        let previous = session.queued_input.fetch_add(amount, Ordering::AcqRel);
        if previous.saturating_add(amount) > MAX_INPUT_BYTES as u64 {
            session.queued_input.fetch_sub(amount, Ordering::AcqRel);
            return Err(SupervisorError::ResourceLimit);
        }
        match session.input.try_send(InputChunk(bytes)) {
            Ok(()) => {
                current.next_sequence = current
                    .next_sequence
                    .checked_add(1)
                    .ok_or(SupervisorError::ResourceLimit)?;
                Ok(())
            }
            Err(TrySendError::Full(chunk) | TrySendError::Disconnected(chunk)) => {
                session
                    .queued_input
                    .fetch_sub(chunk.0.len() as u64, Ordering::AcqRel);
                Err(SupervisorError::ResourceLimit)
            }
        }
    }

    pub fn resize(
        &self,
        session_id: Uuid,
        connection: Uuid,
        lease: u64,
        rows: u16,
        columns: u16,
    ) -> Result<u64, SupervisorError> {
        let session = self.session(session_id)?;
        let owner = session
            .input_owner
            .lock()
            .map_err(|_| SupervisorError::Io)?;
        if !matches!(&*owner, Some(current) if current.connection == connection && current.lease == lease)
        {
            return Err(SupervisorError::Conflict);
        }
        session
            .control
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .as_ref()
            .ok_or(SupervisorError::Final)?
            .resize(rows, columns)
            .map_err(map_pty)?;
        session
            .terminal
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .resize(rows, columns)
            .map_err(|_| SupervisorError::ResourceLimit)
    }

    pub fn terminate(&self, session_id: Uuid) -> Result<(), SupervisorError> {
        let session = self.session(session_id)?;
        if session.final_state.load(Ordering::Acquire) {
            return Err(SupervisorError::Final);
        }
        session.terminate_requested.store(true, Ordering::Release);
        session
            .control
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .as_mut()
            .ok_or(SupervisorError::Final)?
            .terminate()
            .map_err(map_pty)
    }

    pub fn terminate_all(&self) {
        let Ok(sessions) = self.sessions.lock() else {
            return;
        };
        for session in sessions.values() {
            if !session.final_state.load(Ordering::Acquire) {
                session.terminate_requested.store(true, Ordering::Release);
                if let Ok(mut control) = session.control.lock()
                    && let Some(control) = control.as_mut()
                {
                    let _ = control.terminate();
                }
            }
        }
    }

    pub fn poll_exits(&self) -> Vec<ExitObservation> {
        let Ok(mut sessions) = self.sessions.lock() else {
            return Vec::new();
        };
        let observations: Vec<_> = sessions
            .values()
            .filter_map(|session| {
                if session.final_state.load(Ordering::Acquire) {
                    return None;
                }
                let mut control = session.control.lock().ok()?;
                let status = control.as_mut()?.try_wait().ok()??;
                *control = None;
                session.final_state.store(true, Ordering::Release);
                if let Ok(mut owner) = session.input_owner.lock() {
                    *owner = None;
                }
                Some(ExitObservation {
                    instance_id: session.instance_id,
                    code: status.code,
                    terminated: session.terminate_requested.load(Ordering::Acquire),
                })
            })
            .collect();
        let finalized: Vec<_> = sessions
            .iter()
            .filter(|(_, session)| session.final_state.load(Ordering::Acquire))
            .map(|(id, _)| *id)
            .collect();
        for id in finalized.iter().take(finalized.len().saturating_sub(8)) {
            sessions.remove(id);
        }
        observations
    }

    pub fn live_count(&self) -> usize {
        self.sessions
            .lock()
            .map(|sessions| {
                sessions
                    .values()
                    .filter(|session| !session.final_state.load(Ordering::Acquire))
                    .count()
            })
            .unwrap_or(0)
    }

    pub fn is_live(&self, session_id: Uuid) -> bool {
        self.session(session_id)
            .is_ok_and(|session| !session.final_state.load(Ordering::Acquire))
    }

    fn session(&self, id: Uuid) -> Result<Arc<Session>, SupervisorError> {
        self.sessions
            .lock()
            .map_err(|_| SupervisorError::Io)?
            .get(&id)
            .cloned()
            .ok_or(SupervisorError::Reference)
    }
}

fn spawn_reader(mut reader: Box<dyn Read + Send>, terminal: Arc<Mutex<TerminalState>>) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(amount) => {
                    let Ok(mut state) = terminal.lock() else {
                        return;
                    };
                    if state.process(&buffer[..amount]).is_err() {
                        return;
                    }
                }
            }
        }
    });
}

fn spawn_writer(
    mut writer: Box<dyn Write + Send>,
    receiver: mpsc::Receiver<InputChunk>,
    queued: Arc<AtomicU64>,
) {
    thread::spawn(move || {
        while let Ok(chunk) = receiver.recv() {
            let amount = chunk.0.len() as u64;
            let result = write_input(&mut *writer, &chunk.0);
            queued.fetch_sub(amount, Ordering::AcqRel);
            if result.is_err() {
                return;
            }
        }
    });
}

fn map_pty(error: PtyError) -> SupervisorError {
    match error {
        PtyError::InvalidRequest => SupervisorError::ResourceLimit,
        PtyError::Spawn | PtyError::Io => SupervisorError::Io,
    }
}

pub async fn wait_for_output() {
    tokio::time::sleep(Duration::from_millis(20)).await;
}
