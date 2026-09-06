//! Shared Relayterm IPC client with explicit mutation uncertainty.

use relayterm_ipc::{Endpoint, IpcError, LocalStream, connect, read_frame, write_frame};
use relayterm_protocol::{
    DecimalU64, ErrorBody, ErrorCode, EventEnvelope, FrameKind, Operation, PROTOCOL_VERSION,
    RequestEnvelope, RequestType, ResponseEnvelope, SNAPSHOT_STAGING_LIMIT, WorkspaceId,
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
use tokio::sync::Mutex;

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
    async fn call_inner(
        &self,
        operation: Operation,
        params: &Value,
        mutation: bool,
    ) -> Result<Value, ClientError> {
        let first = self.call_once(operation, params, mutation).await;
        if mutation || !matches!(first, Err(ClientError::Transport(_))) {
            return first;
        }
        self.visible.lock().await.connection = ConnectionStatus::Disconnected;
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
        .map_err(|_| ClientError::Transport(uncertain))?
        .map_err(|_| ClientError::Transport(uncertain))?;
        loop {
            let frame =
                tokio::time::timeout(self.deadline, read_frame(&mut *stream, self.deadline))
                    .await
                    .map_err(|_| ClientError::Transport(uncertain))?
                    .map_err(|_| ClientError::Transport(uncertain))?;
            if frame.kind != FrameKind::Json {
                return Err(ClientError::Protocol);
            }
            if let Ok(event) = decode_json::<EventEnvelope>(&frame.payload, false) {
                self.queue_event(event).await?;
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
            exchange(
                &mut stream,
                self.workspace_id,
                subscribe_id,
                Operation::EventSubscribe,
                &serde_json::json!({"after_sequence":after.to_string()}),
                self.deadline,
                Delivery::NotSent,
            )
            .await?;
        }
        *self.stream.lock().await = stream;
        self.visible.lock().await.connection = ConnectionStatus::Connected;
        Ok(())
    }
    async fn queue_event(&self, event: EventEnvelope) -> Result<(), ClientError> {
        if event.workspace_id != self.workspace_id {
            return Err(ClientError::WorkspaceMismatch);
        }
        let mut state = self.events.lock().await;
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
        let mut events = self.events.lock().await;
        events.last_sequence = after_sequence;
        events.subscribed = true;
        Ok(result)
    }
    pub async fn unsubscribe(&self) -> Result<(), ClientError> {
        let _: Value = self
            .call_inner(Operation::EventUnsubscribe, &serde_json::json!({}), false)
            .await?;
        let mut state = self.events.lock().await;
        state.queued.clear();
        state.queued_bytes = 0;
        state.subscribed = false;
        Ok(())
    }
    pub async fn next_event(&self) -> Result<Value, ClientError> {
        if let Some(v) = self.pop_event().await {
            return Ok(v);
        }
        let mut stream = self.stream.lock().await;
        let frame = read_frame(&mut *stream, self.deadline)
            .await
            .map_err(|_| ClientError::Transport(Delivery::NotSent))?;
        if frame.kind != FrameKind::Json {
            return Err(ClientError::Protocol);
        }
        let event: EventEnvelope =
            decode_json(&frame.payload, false).map_err(|_| ClientError::Protocol)?;
        self.queue_event(event).await?;
        self.pop_event().await.ok_or(ClientError::Protocol)
    }
    async fn pop_event(&self) -> Option<Value> {
        let mut state = self.events.lock().await;
        let value = state.queued.pop_front()?;
        state.queued_bytes = state
            .queued_bytes
            .saturating_sub(serde_json::to_vec(&value).map_or(0, |x| x.len()));
        Some(value)
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
