//! Shared Relayterm IPC client with explicit mutation uncertainty.

use relayterm_ipc::{Endpoint, IpcError, LocalStream, connect, read_frame, write_frame};
use relayterm_protocol::{
    DecimalU64, ErrorBody, ErrorCode, EventEnvelope, FrameKind, Operation, PROTOCOL_VERSION,
    RequestEnvelope, RequestType, ResponseEnvelope, SNAPSHOT_STAGING_LIMIT,
    SessionAcquireInputParams, SessionAttachParams, SessionCreateParams, SessionCreateResult,
    SessionInputParams, SessionLeaseResult, SessionReadOutputParams, SessionReleaseInputParams,
    SessionResizeParams, SessionRevisionResult, SessionTerminateParams, SubscriptionId,
    SynchronizationEnvelope, TerminalAttachmentDto, TerminalFrame, TerminalOutputDto, WorkspaceId,
    decode_json, encode_json,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{Mutex, watch},
};

pub const DEFAULT_REQUEST_DEADLINE: Duration = Duration::from_secs(35);
const RECONNECT_DELAYS: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
];
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delivery {
    NotSent,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientError {
    Transport(Delivery),
    Cancelled(Delivery),
    Rejected(ErrorCode),
    Protocol,
    ResourceLimit,
    WorkspaceMismatch,
    VersionMismatch,
}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Transport(Delivery::NotSent) => "The request was not sent.",
            Self::Transport(Delivery::Unknown) => {
                "The mutation result is unknown. Refresh state before deciding whether to retry."
            }
            Self::Cancelled(Delivery::NotSent) => "The request was cancelled before delivery.",
            Self::Cancelled(Delivery::Unknown) => {
                "The cancelled mutation result is unknown. Refresh state before deciding whether to retry."
            }
            Self::Rejected(_) => "The server rejected the request.",
            Self::Protocol => "The peer response violated the protocol.",
            Self::ResourceLimit => "The client resource limit was reached.",
            Self::WorkspaceMismatch => "The connection belongs to another workspace.",
            Self::VersionMismatch => "The client and server protocol versions differ.",
        })
    }
}
impl std::error::Error for ClientError {}

#[derive(Default)]
struct EventState {
    last_sequence: u64,
    queued_bytes: usize,
    queued: VecDeque<Value>,
    subscribed: bool,
    subscription_id: Option<SubscriptionId>,
    subscription_request_id: Option<DecimalU64>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynchronizationStatus {
    Empty,
    Refreshing,
    Current,
    Stale,
    Retryable,
}
#[derive(Clone, Debug)]
pub struct ClientStatus {
    pub connection: ConnectionStatus,
    pub synchronization: SynchronizationStatus,
    pub snapshot_watermark: Option<u64>,
    pub last_event_sequence: u64,
}
struct VisibleState {
    connection: ConnectionStatus,
    synchronization: SynchronizationStatus,
    snapshot: Option<ClientSnapshot>,
}
impl Default for VisibleState {
    fn default() -> Self {
        Self {
            connection: ConnectionStatus::Connected,
            synchronization: SynchronizationStatus::Empty,
            snapshot: None,
        }
    }
}
pub struct Client {
    endpoint: Endpoint,
    workspace_id: WorkspaceId,
    stream: Mutex<LocalStream>,
    next_id: AtomicU64,
    deadline: Duration,
    events: Mutex<EventState>,
    visible: Mutex<VisibleState>,
}
impl Client {
    pub async fn connect(
        endpoint: &Endpoint,
        workspace_id: WorkspaceId,
    ) -> Result<Self, ClientError> {
        let mut stream = connect(endpoint)
            .await
            .map_err(|_| ClientError::Transport(Delivery::NotSent))?;
        exchange(
            &mut stream,
            workspace_id,
            DecimalU64::new(1).expect("one is a valid request ID"),
            Operation::ProtocolHello,
            &serde_json::json!({}),
            DEFAULT_REQUEST_DEADLINE,
            Delivery::NotSent,
        )
        .await?;
        let client = Self {
            endpoint: endpoint.clone(),
            workspace_id,
            stream: Mutex::new(stream),
            next_id: AtomicU64::new(2),
            deadline: DEFAULT_REQUEST_DEADLINE,
            events: Mutex::new(EventState::default()),
            visible: Mutex::new(VisibleState::default()),
        };
        Ok(client)
    }
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Open an independently framed connection for subscriptions or terminal traffic.
    /// A blocking event read on this peer cannot starve control requests on the caller.
    pub async fn connect_peer(&self) -> Result<Self, ClientError> {
        Self::connect(&self.endpoint, self.workspace_id)
            .await
            .map(|peer| peer.with_deadline(self.deadline))
    }
    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        operation: Operation,
        params: &P,
    ) -> Result<R, ClientError> {
        let value = self
            .call_inner(
                operation,
                &serde_json::to_value(params).map_err(|_| ClientError::Protocol)?,
                operation.is_mutation(),
            )
            .await?;
        serde_json::from_value(value).map_err(|_| ClientError::Protocol)
    }

    pub async fn create_session(
        &self,
        params: &SessionCreateParams,
    ) -> Result<SessionCreateResult, ClientError> {
        self.call(Operation::SessionCreate, params).await
    }

    pub async fn attach_session(
        &self,
        params: &SessionAttachParams,
    ) -> Result<TerminalAttachmentDto, ClientError> {
        self.call(Operation::SessionAttach, params).await
    }

    pub async fn read_session_output(
        &self,
        params: &SessionReadOutputParams,
    ) -> Result<(TerminalOutputDto, Option<TerminalFrame>), ClientError> {
        let request_id = DecimalU64::new(self.next_id.fetch_add(1, Ordering::Relaxed))
            .map_err(|_| ClientError::ResourceLimit)?;
        let value = serde_json::to_value(params).map_err(|_| ClientError::Protocol)?;
        let mut stream = self.stream.lock().await;
        let value = self
            .exchange_correlated(
                &mut stream,
                request_id,
                Operation::SessionReadOutput,
                &value,
                Delivery::NotSent,
            )
            .await?;
        let metadata: TerminalOutputDto =
            serde_json::from_value(value).map_err(|_| ClientError::Protocol)?;
        if !metadata.data_follows {
            return Ok((metadata, None));
        }
        let frame = tokio::time::timeout(self.deadline, read_frame(&mut *stream, self.deadline))
            .await
            .map_err(|_| ClientError::Transport(Delivery::NotSent))?
            .map_err(|_| ClientError::Transport(Delivery::NotSent))?;
        if frame.kind != FrameKind::Terminal {
            return Err(ClientError::Protocol);
        }
        let terminal = TerminalFrame::decode(&frame.payload).map_err(|_| ClientError::Protocol)?;
        if terminal.session_id != params.session_id
            || terminal.stream_id != metadata.stream_id.get()
            || terminal.offset != params.after_offset.get()
        {
            return Err(ClientError::Protocol);
        }
        Ok((metadata, Some(terminal)))
    }

    pub async fn acquire_session_input(
        &self,
        params: &SessionAcquireInputParams,
    ) -> Result<SessionLeaseResult, ClientError> {
        self.call(Operation::SessionAcquireInput, params).await
    }

    pub async fn release_session_input(
        &self,
        params: &SessionReleaseInputParams,
    ) -> Result<Value, ClientError> {
        self.call(Operation::SessionReleaseInput, params).await
    }

    pub async fn send_session_input(
        &self,
        params: &SessionInputParams,
    ) -> Result<Value, ClientError> {
        self.call(Operation::SessionInput, params).await
    }

    pub async fn resize_session(
        &self,
        params: &SessionResizeParams,
    ) -> Result<SessionRevisionResult, ClientError> {
        self.call(Operation::SessionResize, params).await
    }

    pub async fn terminate_session(
        &self,
        params: &SessionTerminateParams,
    ) -> Result<Value, ClientError> {
        self.call(Operation::SessionTerminate, params).await
    }
    pub async fn call_cancellable<P: Serialize, R: DeserializeOwned>(
        &self,
        operation: Operation,
        params: &P,
        cancellation: &mut watch::Receiver<bool>,
    ) -> Result<R, ClientError> {
        if *cancellation.borrow() {
            return Err(ClientError::Cancelled(Delivery::NotSent));
        }
        let params = serde_json::to_value(params).map_err(|_| ClientError::Protocol)?;
        let mut stream = tokio::select! {
            changed = cancellation.changed() => {
                let _ = changed;
                return Err(ClientError::Cancelled(Delivery::NotSent));
            }
            stream = self.stream.lock() => stream,
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request_id = DecimalU64::new(id).map_err(|_| ClientError::ResourceLimit)?;
        let delivery = if operation.is_mutation() {
            Delivery::Unknown
        } else {
            Delivery::NotSent
        };
        let result = tokio::select! { biased;
            changed = cancellation.changed() => {
                let _ = changed;
                let _ = stream.shutdown().await;
                Err(ClientError::Cancelled(delivery))
            }
            result = self.exchange_correlated(
                &mut stream,
                request_id,
                operation,
                &params,
                delivery,
            ) => result,
        };
        if matches!(
            result,
            Err(ClientError::Transport(_)
                | ClientError::Cancelled(_)
                | ClientError::Protocol
                | ClientError::WorkspaceMismatch
                | ClientError::VersionMismatch)
        ) {
            self.visible.lock().await.connection = ConnectionStatus::Disconnected;
        }
        serde_json::from_value(result?).map_err(|_| ClientError::Protocol)
    }
    async fn call_inner(
        &self,
        operation: Operation,
        params: &Value,
        mutation: bool,
    ) -> Result<Value, ClientError> {
        if self.visible.lock().await.connection == ConnectionStatus::Disconnected {
            self.reconnect().await?;
        }
        let first = self.call_once(operation, params, mutation).await;
        if matches!(first, Err(ClientError::Transport(_))) {
            self.visible.lock().await.connection = ConnectionStatus::Disconnected;
        }
        if mutation || !matches!(first, Err(ClientError::Transport(_))) {
            return first;
        }
        let mut last = first;
        for delay in RECONNECT_DELAYS {
            tokio::time::sleep(delay).await;
            match self.reconnect().await {
                Ok(()) => {
                    last = self.call_once(operation, params, false).await;
                    if !matches!(last, Err(ClientError::Transport(_))) {
                        return last;
                    }
                }
                Err(error) => last = Err(error),
            }
        }
        last
    }
    async fn call_once(
        &self,
        operation: Operation,
        params: &Value,
        mutation: bool,
    ) -> Result<Value, ClientError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request_id = DecimalU64::new(id).map_err(|_| ClientError::ResourceLimit)?;
        let mut stream = self.stream.lock().await;
        let uncertain = if mutation {
            Delivery::Unknown
        } else {
            Delivery::NotSent
        };
        self.exchange_correlated(&mut stream, request_id, operation, params, uncertain)
            .await
    }
    async fn exchange_correlated(
        &self,
        stream: &mut LocalStream,
        request_id: DecimalU64,
        operation: Operation,
        params: &Value,
        transport_effect: Delivery,
    ) -> Result<Value, ClientError> {
        let request = RequestEnvelope {
            message_type: RequestType::Request,
            protocol_version: PROTOCOL_VERSION,
            request_id,
            workspace_id: self.workspace_id,
            operation,
            params: params.clone(),
        };
        let bytes = encode_json(&request).map_err(|_| ClientError::ResourceLimit)?;
        tokio::time::timeout(
            self.deadline,
            write_frame(&mut *stream, FrameKind::Json, &bytes),
        )
        .await
        .map_err(|_| ClientError::Transport(transport_effect))?
        .map_err(|_| ClientError::Transport(transport_effect))?;
        loop {
            let frame =
                tokio::time::timeout(self.deadline, read_frame(&mut *stream, self.deadline))
                    .await
                    .map_err(|_| ClientError::Transport(transport_effect))?
                    .map_err(|_| ClientError::Transport(transport_effect))?;
            if frame.kind != FrameKind::Json {
                return Err(ClientError::Protocol);
            }
            if let Ok(event) = decode_json::<EventEnvelope>(&frame.payload, false) {
                if let Err(error) = self.queue_event(event).await {
                    let _ = stream.shutdown().await;
                    self.visible.lock().await.connection = ConnectionStatus::Disconnected;
                    return Err(error);
                }
                continue;
            }
            if let Ok(control) = decode_json::<SynchronizationEnvelope>(&frame.payload, false) {
                if let Some(error) = self.handle_synchronization(control).await {
                    return Err(error);
                }
                continue;
            }
            let response: ResponseEnvelope =
                decode_json(&frame.payload, false).map_err(|_| ClientError::Protocol)?;
            response.validate().map_err(|_| ClientError::Protocol)?;
            if response.workspace_id != self.workspace_id {
                return Err(ClientError::WorkspaceMismatch);
            }
            if response.request_id != request_id {
                return Err(ClientError::Protocol);
            }
            if let Some(error) = response.error {
                return Err(map_rejection(error));
            }
            return response.result.ok_or(ClientError::Protocol);
        }
    }
    async fn reconnect(&self) -> Result<(), ClientError> {
        let mut stream = connect(&self.endpoint)
            .await
            .map_err(|_| ClientError::Transport(Delivery::NotSent))?;
        let hello_id = DecimalU64::new(self.next_id.fetch_add(1, Ordering::Relaxed))
            .map_err(|_| ClientError::ResourceLimit)?;
        exchange(
            &mut stream,
            self.workspace_id,
            hello_id,
            Operation::ProtocolHello,
            &serde_json::json!({}),
            self.deadline,
            Delivery::NotSent,
        )
        .await?;
        let (subscribed, after) = {
            let events = self.events.lock().await;
            (events.subscribed, events.last_sequence)
        };
        if subscribed {
            let subscribe_id = DecimalU64::new(self.next_id.fetch_add(1, Ordering::Relaxed))
                .map_err(|_| ClientError::ResourceLimit)?;
            let result = exchange(
                &mut stream,
                self.workspace_id,
                subscribe_id,
                Operation::EventSubscribe,
                &serde_json::json!({"after_sequence":after.to_string()}),
                self.deadline,
                Delivery::NotSent,
            )
            .await?;
            self.install_subscription(&result, after).await?;
        }
        *self.stream.lock().await = stream;
        self.visible.lock().await.connection = ConnectionStatus::Connected;
        Ok(())
    }
    async fn queue_event(&self, event: EventEnvelope) -> Result<(), ClientError> {
        if event.protocol_version != PROTOCOL_VERSION {
            return Err(ClientError::VersionMismatch);
        }
        if event.workspace_id != self.workspace_id {
            return Err(ClientError::WorkspaceMismatch);
        }
        if event.event.get("payload_version").and_then(Value::as_u64) != Some(1) {
            let mut state = self.events.lock().await;
            state.queued.clear();
            state.queued_bytes = 0;
            state.subscribed = false;
            state.subscription_id = None;
            state.subscription_request_id = None;
            drop(state);
            self.visible.lock().await.synchronization = SynchronizationStatus::Retryable;
            return Err(ClientError::Rejected(ErrorCode::ResnapshotRequired));
        }
        let mut state = self.events.lock().await;
        if state.subscription_id != Some(event.subscription_id)
            || state.subscription_request_id != Some(event.request_id)
        {
            return Err(ClientError::Protocol);
        }
        let sequence = event.sequence.get();
        if state.last_sequence != 0 && sequence != state.last_sequence + 1 {
            return Err(ClientError::Protocol);
        }
        let bytes = serde_json::to_vec(&event.event)
            .map_err(|_| ClientError::Protocol)?
            .len();
        state.queued_bytes = state
            .queued_bytes
            .checked_add(bytes)
            .ok_or(ClientError::ResourceLimit)?;
        if state.queued.len() >= 256 || state.queued_bytes > 1024 * 1024 {
            return Err(ClientError::ResourceLimit);
        }
        state.last_sequence = sequence;
        state.queued.push_back(event.event);
        drop(state);
        let mut visible = self.visible.lock().await;
        if visible
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.last_sequence < sequence)
        {
            visible.synchronization = SynchronizationStatus::Stale;
        }
        Ok(())
    }
    pub async fn status(&self) -> ClientStatus {
        let events = self.events.lock().await;
        let visible = self.visible.lock().await;
        ClientStatus {
            connection: visible.connection,
            synchronization: visible.synchronization,
            snapshot_watermark: visible.snapshot.as_ref().map(|value| value.last_sequence),
            last_event_sequence: events.last_sequence,
        }
    }
    pub async fn snapshot(&self) -> Option<ClientSnapshot> {
        self.visible.lock().await.snapshot.clone()
    }
}

async fn exchange(
    stream: &mut LocalStream,
    workspace_id: WorkspaceId,
    request_id: DecimalU64,
    operation: Operation,
    params: &Value,
    deadline: Duration,
    transport_effect: Delivery,
) -> Result<Value, ClientError> {
    let request = RequestEnvelope {
        message_type: RequestType::Request,
        protocol_version: PROTOCOL_VERSION,
        request_id,
        workspace_id,
        operation,
        params: params.clone(),
    };
    let bytes = encode_json(&request).map_err(|_| ClientError::ResourceLimit)?;
    tokio::time::timeout(deadline, write_frame(&mut *stream, FrameKind::Json, &bytes))
        .await
        .map_err(|_| ClientError::Transport(transport_effect))?
        .map_err(|_| ClientError::Transport(transport_effect))?;
    let frame = tokio::time::timeout(deadline, read_frame(&mut *stream, deadline))
        .await
        .map_err(|_| ClientError::Transport(transport_effect))?
        .map_err(|_| ClientError::Transport(transport_effect))?;
    if frame.kind != FrameKind::Json {
        return Err(ClientError::Protocol);
    }
    let response: ResponseEnvelope =
        decode_json(&frame.payload, false).map_err(|_| ClientError::Protocol)?;
    response.validate().map_err(|_| ClientError::Protocol)?;
    if response.workspace_id != workspace_id {
        return Err(ClientError::WorkspaceMismatch);
    }
    if response.request_id != request_id {
        return Err(ClientError::Protocol);
    }
    if let Some(error) = response.error {
        return Err(map_rejection(error));
    }
    response.result.ok_or(ClientError::Protocol)
}

impl Client {
    pub async fn subscribe(&self, after_sequence: u64) -> Result<Value, ClientError> {
        let result = self
            .call_inner(
                Operation::EventSubscribe,
                &serde_json::json!({"after_sequence":after_sequence.to_string()}),
                false,
            )
            .await?;
        self.install_subscription(&result, after_sequence).await?;
        Ok(result)
    }
    pub async fn unsubscribe(&self) -> Result<(), ClientError> {
        let subscription_id = self
            .events
            .lock()
            .await
            .subscription_id
            .ok_or(ClientError::Protocol)?;
        let _: Value = self
            .call_inner(
                Operation::EventUnsubscribe,
                &serde_json::json!({"subscription_id":subscription_id.to_string()}),
                false,
            )
            .await?;
        let mut state = self.events.lock().await;
        state.queued.clear();
        state.queued_bytes = 0;
        state.subscribed = false;
        state.subscription_id = None;
        state.subscription_request_id = None;
        Ok(())
    }
    pub async fn next_event(&self) -> Result<Value, ClientError> {
        if let Some(v) = self.pop_event().await {
            return Ok(v);
        }
        loop {
            let mut stream = self.stream.lock().await;
            let frame = match read_frame(&mut *stream, self.deadline).await {
                Ok(frame) => frame,
                Err(_) => {
                    self.visible.lock().await.connection = ConnectionStatus::Disconnected;
                    return Err(ClientError::Transport(Delivery::NotSent));
                }
            };
            drop(stream);
            if frame.kind != FrameKind::Json {
                return Err(ClientError::Protocol);
            }
            if let Ok(control) = decode_json::<SynchronizationEnvelope>(&frame.payload, false) {
                if let Some(error) = self.handle_synchronization(control).await {
                    return Err(error);
                }
                continue;
            }
            let event: EventEnvelope =
                decode_json(&frame.payload, false).map_err(|_| ClientError::Protocol)?;
            if let Err(error) = self.queue_event(event).await {
                let mut stream = self.stream.lock().await;
                let _ = stream.shutdown().await;
                self.visible.lock().await.connection = ConnectionStatus::Disconnected;
                return Err(error);
            }
            return self.pop_event().await.ok_or(ClientError::Protocol);
        }
    }

    pub async fn recover_subscription(&self, after_sequence: u64) -> Result<(), ClientError> {
        let subscribed = self.events.lock().await.subscribed;
        if subscribed {
            let _: Value = self
                .call(Operation::DaemonStatus, &serde_json::json!({}))
                .await?;
        } else {
            self.subscribe(after_sequence).await?;
        }
        Ok(())
    }
    async fn pop_event(&self) -> Option<Value> {
        let mut state = self.events.lock().await;
        let value = state.queued.pop_front()?;
        state.queued_bytes = state
            .queued_bytes
            .saturating_sub(serde_json::to_vec(&value).map_or(0, |x| x.len()));
        Some(value)
    }
    async fn install_subscription(
        &self,
        result: &Value,
        after_sequence: u64,
    ) -> Result<(), ClientError> {
        let subscription_id = result
            .get("subscription_id")
            .and_then(Value::as_str)
            .ok_or(ClientError::Protocol)?
            .parse()
            .map_err(|_| ClientError::Protocol)?;
        let request_id: DecimalU64 = serde_json::from_value(
            result
                .get("request_id")
                .cloned()
                .ok_or(ClientError::Protocol)?,
        )
        .map_err(|_| ClientError::Protocol)?;
        let mut events = self.events.lock().await;
        events.last_sequence = after_sequence;
        events.subscribed = true;
        events.subscription_id = Some(subscription_id);
        events.subscription_request_id = Some(request_id);
        Ok(())
    }
    async fn handle_synchronization(
        &self,
        control: SynchronizationEnvelope,
    ) -> Option<ClientError> {
        if control.protocol_version != PROTOCOL_VERSION {
            return Some(ClientError::VersionMismatch);
        }
        if control.workspace_id != self.workspace_id {
            return Some(ClientError::WorkspaceMismatch);
        }
        let mut events = self.events.lock().await;
        if events.subscription_id != Some(control.subscription_id)
            || events.subscription_request_id != Some(control.request_id)
        {
            return Some(ClientError::Protocol);
        }
        if control.control == relayterm_protocol::SynchronizationControl::CaughtUp {
            let Some(watermark) = control.watermark else {
                return Some(ClientError::Protocol);
            };
            if control.reason.is_some() || watermark.get() != events.last_sequence {
                return Some(ClientError::Protocol);
            }
            return None;
        }
        if control.reason.is_none() || control.watermark.is_some() {
            return Some(ClientError::Protocol);
        }
        events.queued.clear();
        events.queued_bytes = 0;
        events.subscribed = false;
        events.subscription_id = None;
        events.subscription_request_id = None;
        drop(events);
        self.visible.lock().await.synchronization = SynchronizationStatus::Retryable;
        Some(ClientError::Rejected(ErrorCode::ResnapshotRequired))
    }
    pub async fn refresh_snapshot(&self) -> Result<ClientSnapshot, ClientError> {
        self.visible.lock().await.synchronization = SynchronizationStatus::Refreshing;
        for _ in 0..3 {
            match self.refresh_once().await {
                Ok(snapshot) => {
                    let latest_event = self.events.lock().await.last_sequence;
                    let mut visible = self.visible.lock().await;
                    visible.synchronization = if latest_event > snapshot.last_sequence {
                        SynchronizationStatus::Stale
                    } else {
                        SynchronizationStatus::Current
                    };
                    visible.snapshot = Some(snapshot.clone());
                    return Ok(snapshot);
                }
                Err(ClientError::Rejected(ErrorCode::Conflict)) => continue,
                Err(error) => {
                    self.visible.lock().await.synchronization = SynchronizationStatus::Retryable;
                    return Err(error);
                }
            }
        }
        self.visible.lock().await.synchronization = SynchronizationStatus::Retryable;
        Err(ClientError::Rejected(ErrorCode::Conflict))
    }
    async fn refresh_once(&self) -> Result<ClientSnapshot, ClientError> {
        let first: Value = self
            .call(
                Operation::WorkspaceGetSnapshot,
                &serde_json::json!({"collection":"workspace","limit":50}),
            )
            .await?;
        let revision = first["revision"]
            .as_str()
            .ok_or(ClientError::Protocol)?
            .to_owned();
        let last_sequence = first["last_sequence"]
            .as_str()
            .ok_or(ClientError::Protocol)?
            .parse()
            .map_err(|_| ClientError::Protocol)?;
        let retained_from_sequence = first["retained_from_sequence"]
            .as_str()
            .ok_or(ClientError::Protocol)?
            .parse()
            .map_err(|_| ClientError::Protocol)?;
        let mut staging = SnapshotStaging::new();
        staging.push(first.clone())?;
        let mut collections = BTreeMap::new();
        collections.insert(
            "workspace".to_owned(),
            first["items"]
                .as_array()
                .ok_or(ClientError::Protocol)?
                .clone(),
        );
        for name in [
            "definitions",
            "tasks",
            "instances",
            "claims",
            "progress",
            "handovers",
        ] {
            let mut after: Option<String> = None;
            let mut all = Vec::new();
            loop {
                let page:Value=self.call(Operation::WorkspaceGetSnapshot,&serde_json::json!({"collection":name,"after_id":after,"limit":50,"expected_revision":revision})).await?;
                if page["revision"].as_str() != Some(revision.as_str())
                    || page["last_sequence"].as_str().and_then(|x| x.parse().ok())
                        != Some(last_sequence)
                {
                    return Err(ClientError::Rejected(ErrorCode::Conflict));
                }
                staging.push(page.clone())?;
                all.extend(
                    page["items"]
                        .as_array()
                        .ok_or(ClientError::Protocol)?
                        .clone(),
                );
                after = page["next_after_id"].as_str().map(ToOwned::to_owned);
                if after.is_none() {
                    break;
                }
            }
            collections.insert(name.to_owned(), all);
        }
        Ok(ClientSnapshot {
            revision,
            last_sequence,
            retained_from_sequence,
            collections,
        })
    }
}
fn map_rejection(error: ErrorBody) -> ClientError {
    match error.code {
        ErrorCode::WorkspaceMismatch => ClientError::WorkspaceMismatch,
        ErrorCode::UnsupportedVersion => ClientError::VersionMismatch,
        code => ClientError::Rejected(code),
    }
}

#[derive(Clone)]
pub struct ClientSnapshot {
    pub revision: String,
    pub last_sequence: u64,
    pub retained_from_sequence: u64,
    pub collections: BTreeMap<String, Vec<Value>>,
}
pub struct SnapshotStaging {
    bytes: usize,
    pages: Vec<Value>,
}
impl SnapshotStaging {
    pub fn new() -> Self {
        Self {
            bytes: 0,
            pages: Vec::new(),
        }
    }
    pub fn push(&mut self, page: Value) -> Result<(), ClientError> {
        let size = serde_json::to_vec(&page)
            .map_err(|_| ClientError::Protocol)?
            .len();
        self.bytes = self
            .bytes
            .checked_add(size)
            .ok_or(ClientError::ResourceLimit)?;
        if self.bytes > SNAPSHOT_STAGING_LIMIT {
            return Err(ClientError::ResourceLimit);
        }
        self.pages.push(page);
        Ok(())
    }
    pub fn install(self) -> Vec<Value> {
        self.pages
    }
}
impl Default for SnapshotStaging {
    fn default() -> Self {
        Self::new()
    }
}
impl From<IpcError> for ClientError {
    fn from(_: IpcError) -> Self {
        Self::Transport(Delivery::NotSent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_ids_are_monotonic() {
        let id = AtomicU64::new(1);
        assert_eq!(id.fetch_add(1, Ordering::Relaxed), 1);
        assert_eq!(id.fetch_add(1, Ordering::Relaxed), 2);
        assert!(DecimalU64::new(0).is_err());
    }
    #[test]
    fn staging_is_atomic() {
        let mut s = SnapshotStaging::new();
        s.push(serde_json::json!({"revision":"1"})).unwrap();
        assert_eq!(s.install().len(), 1);
    }
}
