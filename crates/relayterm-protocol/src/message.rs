use crate::{
    DEFAULT_PAGE_SIZE, HELLO_FRAME_LIMIT, MAX_JSON_DEPTH, MAX_PAGE_SIZE, PROTOCOL_VERSION,
    TERMINAL_DATA_LIMIT, TERMINAL_METADATA_SIZE,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeSeed},
};
use serde_json::Value;
use std::{collections::HashSet, fmt, str::FromStr};
use uuid::Uuid;

macro_rules! wire_uuid {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(Uuid);
        impl $name {
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }
        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.parse().map(Self)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.to_string())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                String::deserialize(d)?
                    .parse()
                    .map_err(|_| de::Error::custom("invalid identifier"))
            }
        }
    };
}
wire_uuid!(WorkspaceId);
wire_uuid!(ConnectionId);
wire_uuid!(EntityId);
wire_uuid!(SessionId);
wire_uuid!(TaskId);
wire_uuid!(AgentDefinitionId);
wire_uuid!(AgentInstanceId);
wire_uuid!(ClaimId);
wire_uuid!(ProgressId);
wire_uuid!(HandoverId);
wire_uuid!(EventId);
wire_uuid!(WorktreeId);
wire_uuid!(SubscriptionId);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DecimalU64(u64);
impl DecimalU64 {
    pub fn new(value: u64) -> Result<Self, ScalarError> {
        (value != 0).then_some(Self(value)).ok_or(ScalarError)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}
impl Serialize for DecimalU64 {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        if v.is_empty() || v.starts_with('0') || !v.bytes().all(|x| x.is_ascii_digit()) {
            return Err(de::Error::custom("invalid decimal counter"));
        }
        v.parse()
            .ok()
            .and_then(|x| Self::new(x).ok())
            .ok_or_else(|| de::Error::custom("invalid decimal counter"))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScalarError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    ProtocolHello,
    ProtocolPing,
    DaemonStatus,
    DaemonShutdown,
    WorkspaceGetSnapshot,
    AgentListDefinitions,
    AgentRegisterDefinition,
    AgentUpdateDefinition,
    AgentImportDefinitions,
    TaskList,
    TaskGet,
    TaskCreate,
    TaskUpdate,
    TaskTransition,
    TaskClaim,
    TaskRelease,
    ProgressAppend,
    HandoverCreate,
    HandoverGet,
    TaskGetHistory,
    TaskGetClaimHistory,
    SessionList,
    EventList,
    EventSubscribe,
    EventUnsubscribe,
    SessionCreate,
    SessionAttach,
    SessionInput,
    SessionResize,
    SessionTerminate,
    WorktreeCreate,
    WorktreeList,
    Unknown,
}
impl Operation {
    pub const ALL: [Self; 32] = [
        Self::ProtocolHello,
        Self::ProtocolPing,
        Self::DaemonStatus,
        Self::DaemonShutdown,
        Self::WorkspaceGetSnapshot,
        Self::AgentListDefinitions,
        Self::AgentRegisterDefinition,
        Self::AgentUpdateDefinition,
        Self::AgentImportDefinitions,
        Self::TaskList,
        Self::TaskGet,
        Self::TaskCreate,
        Self::TaskUpdate,
        Self::TaskTransition,
        Self::TaskClaim,
        Self::TaskRelease,
        Self::ProgressAppend,
        Self::HandoverCreate,
        Self::HandoverGet,
        Self::TaskGetHistory,
        Self::TaskGetClaimHistory,
        Self::SessionList,
        Self::EventList,
        Self::EventSubscribe,
        Self::EventUnsubscribe,
        Self::SessionCreate,
        Self::SessionAttach,
        Self::SessionInput,
        Self::SessionResize,
        Self::SessionTerminate,
        Self::WorktreeCreate,
        Self::WorktreeList,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProtocolHello => "protocol.hello",
            Self::ProtocolPing => "protocol.ping",
            Self::DaemonStatus => "daemon.status",
            Self::DaemonShutdown => "daemon.shutdown",
            Self::WorkspaceGetSnapshot => "workspace.get_snapshot",
            Self::AgentListDefinitions => "agent.list_definitions",
            Self::AgentRegisterDefinition => "agent.register_definition",
            Self::AgentUpdateDefinition => "agent.update_definition",
            Self::AgentImportDefinitions => "agent.import_definitions",
            Self::TaskList => "task.list",
            Self::TaskGet => "task.get",
            Self::TaskCreate => "task.create",
            Self::TaskUpdate => "task.update",
            Self::TaskTransition => "task.transition",
            Self::TaskClaim => "task.claim",
            Self::TaskRelease => "task.release",
            Self::ProgressAppend => "progress.append",
            Self::HandoverCreate => "handover.create",
            Self::HandoverGet => "handover.get",
            Self::TaskGetHistory => "task.get_history",
            Self::TaskGetClaimHistory => "task.get_claim_history",
            Self::SessionList => "session.list",
            Self::EventList => "event.list",
            Self::EventSubscribe => "event.subscribe",
            Self::EventUnsubscribe => "event.unsubscribe",
            Self::SessionCreate => "session.create",
            Self::SessionAttach => "session.attach",
            Self::SessionInput => "session.input",
            Self::SessionResize => "session.resize",
            Self::SessionTerminate => "session.terminate",
            Self::WorktreeCreate => "worktree.create",
            Self::WorktreeList => "worktree.list",
            Self::Unknown => "unknown",
        }
    }
    pub const fn is_reserved(self) -> bool {
        matches!(
            self,
            Self::SessionCreate
                | Self::SessionAttach
                | Self::SessionInput
                | Self::SessionResize
                | Self::SessionTerminate
                | Self::WorktreeCreate
                | Self::WorktreeList
        )
    }
    pub const fn is_mutation(self) -> bool {
        matches!(
            self,
            Self::DaemonShutdown
                | Self::AgentRegisterDefinition
                | Self::AgentUpdateDefinition
                | Self::AgentImportDefinitions
                | Self::TaskCreate
                | Self::TaskUpdate
                | Self::TaskTransition
                | Self::TaskClaim
                | Self::TaskRelease
                | Self::ProgressAppend
                | Self::HandoverCreate
        )
    }
}
impl Serialize for Operation {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;
        if value.is_empty() || value.len() > 64 || !value.is_ascii() {
            return Err(de::Error::custom("invalid operation"));
        }
        Ok(Self::ALL
            .into_iter()
            .find(|x| x.as_str() == value)
            .unwrap_or(Self::Unknown))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    #[serde(rename = "type")]
    pub message_type: RequestType,
    pub protocol_version: u16,
    pub request_id: DecimalU64,
    pub workspace_id: WorkspaceId,
    pub operation: Operation,
    pub params: Value,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    Request,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    #[serde(rename = "type")]
    pub message_type: ResponseType,
    pub protocol_version: u16,
    pub request_id: DecimalU64,
    pub workspace_id: WorkspaceId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Response,
}
impl ResponseEnvelope {
    pub fn success(request_id: DecimalU64, workspace_id: WorkspaceId, result: Value) -> Self {
        Self {
            message_type: ResponseType::Response,
            protocol_version: PROTOCOL_VERSION,
            request_id,
            workspace_id,
            result: Some(result),
            error: None,
        }
    }
    pub fn failure(request_id: DecimalU64, workspace_id: WorkspaceId, error: ErrorBody) -> Self {
        Self {
            message_type: ResponseType::Response,
            protocol_version: PROTOCOL_VERSION,
            request_id,
            workspace_id,
            result: None,
            error: Some(error),
        }
    }
    pub fn validate(&self) -> Result<(), MessageError> {
        if self.protocol_version != PROTOCOL_VERSION
            || self.result.is_some() == self.error.is_some()
        {
            Err(MessageError::InvalidEnvelope)
        } else {
            Ok(())
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    #[serde(rename = "type")]
    pub message_type: EventMessageType,
    pub protocol_version: u16,
    pub workspace_id: WorkspaceId,
    pub subscription_id: SubscriptionId,
    pub request_id: DecimalU64,
    pub sequence: DecimalU64,
    pub event: Value,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventMessageType {
    Event,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynchronizationEnvelope {
    #[serde(rename = "type")]
    pub message_type: SynchronizationMessageType,
    pub protocol_version: u16,
    pub workspace_id: WorkspaceId,
    pub subscription_id: SubscriptionId,
    pub request_id: DecimalU64,
    pub control: SynchronizationControl,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SynchronizationReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark: Option<DecimalU64>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynchronizationMessageType {
    Event,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynchronizationControl {
    CaughtUp,
    ResnapshotRequired,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynchronizationReason {
    SlowSubscriber,
    CursorExpired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidParams,
    UnknownOperation,
    UnsupportedVersion,
    Unauthorized,
    WorkspaceMismatch,
    InvalidState,
    InvalidReference,
    Conflict,
    InvalidTime,
    StorageBusy,
    ReadOnly,
    StorageError,
    IntegrityError,
    InvalidCursor,
    ResnapshotRequired,
    OperationUnavailable,
    ResourceLimit,
    ResultUnknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorEffect {
    NotApplied,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    None,
    Refresh,
    Reconnect,
    Resnapshot,
    InspectState,
    UseMatchingVersion,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorField {
    ProtocolVersion,
    RequestId,
    WorkspaceId,
    Operation,
    Params,
    PageLimit,
    ExpectedRevision,
    Cursor,
    Frame,
    TerminalData,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<ErrorField>,
    pub effect: ErrorEffect,
    pub recovery: Recovery,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
impl ErrorBody {
    pub const fn not_applied(code: ErrorCode, recovery: Recovery) -> Self {
        Self {
            code,
            field: None,
            effect: ErrorEffect::NotApplied,
            recovery,
            message: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelloResult {
    pub protocol_version: u16,
    pub connection_id: ConnectionId,
    pub workspace_id: WorkspaceId,
    pub operations: Vec<Operation>,
    pub json_frame_limit: usize,
    pub terminal_frame_limit: usize,
    pub default_page_size: u16,
    pub maximum_page_size: u16,
    pub terminal_available: bool,
}
impl HelloResult {
    pub fn new(
        connection_id: ConnectionId,
        workspace_id: WorkspaceId,
        operations: Vec<Operation>,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            connection_id,
            workspace_id,
            operations,
            json_frame_limit: crate::JSON_FRAME_LIMIT,
            terminal_frame_limit: crate::TERMINAL_FRAME_LIMIT,
            default_page_size: DEFAULT_PAGE_SIZE,
            maximum_page_size: MAX_PAGE_SIZE,
            terminal_available: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    #[serde(default)]
    pub after_id: Option<String>,
    #[serde(default = "default_page_size")]
    pub limit: u16,
    pub expected_revision: DecimalU64,
}
const fn default_page_size() -> u16 {
    DEFAULT_PAGE_SIZE
}
impl PageRequest {
    pub fn validate(&self) -> Result<(), MessageError> {
        if self.limit == 0 || self.limit > MAX_PAGE_SIZE {
            Err(MessageError::InvalidPage)
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationReceipt {
    pub entity_ids: Vec<EntityId>,
    pub revision: DecimalU64,
    pub changed: bool,
    pub first_sequence: Option<DecimalU64>,
    pub last_sequence: Option<DecimalU64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonStatusParams {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonStatusResult {
    pub workspace_id: WorkspaceId,
    pub generation: String,
    pub lifecycle: String,
    pub protocol_version: u16,
    pub schema_version: u32,
    pub revision: DecimalU64,
    pub definitions: usize,
    pub tasks: usize,
    pub instances: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonShutdownParams {
    pub generation: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonShutdownResult {
    pub generation: String,
    pub lifecycle: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentImportDefinitionsParams {
    pub document: String,
    #[serde(default)]
    pub expected_revision: Option<DecimalU64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePathDto {
    pub encoding: String,
    pub data: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimestampDto {
    pub unix_seconds: String,
    pub nanoseconds: u32,
}
impl TimestampDto {
    pub fn validate(&self) -> Result<(), MessageError> {
        let seconds = self
            .unix_seconds
            .parse::<i64>()
            .map_err(|_| MessageError::InvalidScalar)?;
        if self.unix_seconds != seconds.to_string() || self.nanoseconds >= 1_000_000_000 {
            Err(MessageError::InvalidScalar)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Backlog,
    Ready,
    Active,
    Blocked,
    HandoverReady,
    Done,
    Cancelled,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Starting,
    Running,
    Exited,
    Failed,
    Terminated,
    Lost,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    ExplicitRelease,
    Blocking,
    Handover,
    Completion,
    Cancellation,
    InstanceEnd,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorDto {
    LocalUser,
    Instance { instance_id: AgentInstanceId },
    System,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDto {
    pub id: WorkspaceId,
    pub display_name: String,
    pub project_root: NativePathDto,
    pub created_at: TimestampDto,
    pub updated_at: TimestampDto,
    pub schema_version: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDefinitionDto {
    pub id: AgentDefinitionId,
    pub workspace_id: WorkspaceId,
    pub display_name: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub environment_allowlist: Vec<String>,
    pub capabilities: Vec<String>,
    pub enabled: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskContentDto {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub scope_paths: Vec<String>,
    pub acceptance_notes: String,
    pub dependency_ids: Vec<TaskId>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDto {
    pub id: TaskId,
    pub workspace_id: WorkspaceId,
    pub content: TaskContentDto,
    pub status: TaskStatus,
    pub claimed_by_instance_id: Option<AgentInstanceId>,
    pub worktree_id: Option<WorktreeId>,
    pub created_at: TimestampDto,
    pub updated_at: TimestampDto,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSizeDto {
    pub rows: u16,
    pub columns: u16,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentInstanceDto {
    pub id: AgentInstanceId,
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub agent_definition_id: Option<AgentDefinitionId>,
    pub task_id: Option<TaskId>,
    pub working_directory: NativePathDto,
    pub status: InstanceStatus,
    pub started_at: TimestampDto,
    pub last_observed_at: TimestampDto,
    pub ended_at: Option<TimestampDto>,
    pub exit_code: Option<i32>,
    pub terminal_size: TerminalSizeDto,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimDto {
    pub id: ClaimId,
    pub workspace_id: WorkspaceId,
    pub task_id: TaskId,
    pub instance_id: AgentInstanceId,
    pub requested_by: ActorDto,
    pub opened_at: TimestampDto,
    pub closed_at: Option<TimestampDto>,
    pub close_reason: Option<CloseReason>,
    pub closed_by: Option<ActorDto>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressDto {
    pub id: ProgressId,
    pub workspace_id: WorkspaceId,
    pub task_id: TaskId,
    pub agent_instance_id: Option<AgentInstanceId>,
    pub summary: String,
    pub verification: String,
    pub created_at: TimestampDto,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoverContentDto {
    pub summary: String,
    pub decisions: String,
    pub changed_paths: Vec<String>,
    pub verification_performed: String,
    pub open_questions: String,
    pub recommended_next_action: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoverDto {
    pub id: HandoverId,
    pub workspace_id: WorkspaceId,
    pub task_id: TaskId,
    pub from_agent_instance_id: Option<AgentInstanceId>,
    pub content: HandoverContentDto,
    pub created_at: TimestampDto,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCreateParams {
    pub launch_kind: SessionLaunchKind,
    pub definition_id: Option<AgentDefinitionId>,
    pub task_id: Option<TaskId>,
    pub working_directory: Option<NativePathDto>,
    pub rows: u16,
    pub columns: u16,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLaunchKind {
    DefaultShell,
    Definition,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionAttachParams {
    pub session_id: SessionId,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionInputParams {
    pub session_id: SessionId,
    pub stream_id: DecimalU64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionResizeParams {
    pub session_id: SessionId,
    pub rows: u16,
    pub columns: u16,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTerminateParams {
    pub session_id: SessionId,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeCreateParams {
    pub task_id: TaskId,
    pub base_ref: String,
    pub branch_name: String,
    pub relative_destination: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeListParams {
    #[serde(default)]
    pub after_id: Option<WorktreeId>,
    #[serde(default = "default_page_size")]
    pub limit: u16,
}
impl NativePathDto {
    pub fn from_bytes(encoding: &str, bytes: &[u8]) -> Result<Self, MessageError> {
        if bytes.len() > 8192 {
            return Err(MessageError::ResourceLimit);
        }
        Ok(Self {
            encoding: encoding.to_owned(),
            data: STANDARD.encode(bytes),
        })
    }
    pub fn decode(&self) -> Result<Vec<u8>, MessageError> {
        let value = STANDARD
            .decode(&self.data)
            .map_err(|_| MessageError::InvalidScalar)?;
        if value.len() > 8192 {
            Err(MessageError::ResourceLimit)
        } else {
            Ok(value)
        }
    }
}

pub struct TerminalFrame {
    pub session_id: SessionId,
    pub stream_id: u64,
    pub offset: u64,
    pub data: Vec<u8>,
}
impl TerminalFrame {
    pub fn encode(&self) -> Result<Vec<u8>, MessageError> {
        if self.data.is_empty() || self.data.len() > TERMINAL_DATA_LIMIT {
            return Err(MessageError::ResourceLimit);
        }
        let mut b = Vec::with_capacity(TERMINAL_METADATA_SIZE + self.data.len());
        b.extend_from_slice(self.session_id.as_uuid().as_bytes());
        b.extend_from_slice(&self.stream_id.to_be_bytes());
        b.extend_from_slice(&self.offset.to_be_bytes());
        b.extend_from_slice(&self.data);
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self, MessageError> {
        if !(TERMINAL_METADATA_SIZE + 1..=TERMINAL_METADATA_SIZE + TERMINAL_DATA_LIMIT)
            .contains(&b.len())
        {
            return Err(MessageError::ResourceLimit);
        }
        Ok(Self {
            session_id: SessionId::from_uuid(Uuid::from_bytes(
                b[..16].try_into().expect("fixed identifier"),
            )),
            stream_id: u64::from_be_bytes(b[16..24].try_into().expect("fixed stream")),
            offset: u64::from_be_bytes(b[24..32].try_into().expect("fixed offset")),
            data: b[32..].to_vec(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageError {
    TooLarge,
    InvalidJson,
    DuplicateKey,
    TooDeep,
    InvalidEnvelope,
    InvalidScalar,
    InvalidPage,
    ResourceLimit,
}
impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The protocol message is invalid.")
    }
}
impl std::error::Error for MessageError {}
pub fn decode_json<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    hello: bool,
) -> Result<T, MessageError> {
    let limit = if hello {
        HELLO_FRAME_LIMIT
    } else {
        crate::JSON_FRAME_LIMIT
    };
    if bytes.is_empty() || bytes.len() > limit {
        return Err(MessageError::TooLarge);
    }
    let value = strict_value(bytes)?;
    serde_json::from_value(value).map_err(|_| MessageError::InvalidEnvelope)
}
pub fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>, MessageError> {
    let bytes = serde_json::to_vec(value).map_err(|_| MessageError::InvalidJson)?;
    if bytes.len() > crate::JSON_FRAME_LIMIT {
        Err(MessageError::TooLarge)
    } else {
        Ok(bytes)
    }
}

fn strict_value(bytes: &[u8]) -> Result<Value, MessageError> {
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    let value = StrictSeed { depth: 0 }
        .deserialize(&mut parser)
        .map_err(|e| {
            let s = e.to_string();
            if s.contains("duplicate key") {
                MessageError::DuplicateKey
            } else if s.contains("maximum nesting") {
                MessageError::TooDeep
            } else {
                MessageError::InvalidJson
            }
        })?;
    parser.end().map_err(|_| MessageError::InvalidJson)?;
    Ok(value)
}
struct StrictSeed {
    depth: usize,
}
impl<'de> de::DeserializeSeed<'de> for StrictSeed {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        if self.depth > MAX_JSON_DEPTH {
            return Err(de::Error::custom("maximum nesting exceeded"));
        }
        d.deserialize_any(StrictVisitor { depth: self.depth })
    }
}
struct StrictVisitor {
    depth: usize,
}
impl<'de> de::Visitor<'de> for StrictVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid number"))
    }
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        StrictSeed { depth: self.depth }.deserialize(d)
    }
    fn visit_seq<A: de::SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut v = Vec::new();
        while let Some(x) = a.next_element_seed(StrictSeed {
            depth: self.depth + 1,
        })? {
            v.push(x)
        }
        Ok(Value::Array(v))
    }
    fn visit_map<A: de::MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut v = serde_json::Map::new();
        let mut keys = HashSet::new();
        while let Some(k) = a.next_key::<String>()? {
            if !keys.insert(k.clone()) {
                return Err(de::Error::custom("duplicate key"));
            }
            v.insert(
                k,
                a.next_value_seed(StrictSeed {
                    depth: self.depth + 1,
                })?,
            );
        }
        Ok(Value::Object(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn workspace() -> WorkspaceId {
        "00000000-0000-4000-8000-000000000001".parse().unwrap()
    }
    #[test]
    fn golden_hello_round_trip() {
        let b=br#"{"type":"request","protocol_version":1,"request_id":"1","workspace_id":"00000000-0000-4000-8000-000000000001","operation":"protocol.hello","params":{}}"#;
        let r: RequestEnvelope = decode_json(b, true).unwrap();
        assert_eq!(r.request_id.get(), 1);
        assert_eq!(r.workspace_id, workspace());
        assert_eq!(encode_json(&r).unwrap(), b);
    }
    #[test]
    fn strict_json_rejects_bad_inputs() {
        assert_eq!(
            strict_value(br#"{"a":1,"a":2}"#),
            Err(MessageError::DuplicateKey)
        );
        assert_eq!(
            strict_value(br#"{"outer":{"marker":1,"marker":2}}"#),
            Err(MessageError::DuplicateKey)
        );
        assert_eq!(
            strict_value(br#"{"value":1} {"value":2}"#),
            Err(MessageError::InvalidJson)
        );
        let u=br#"{"type":"request","protocol_version":1,"request_id":"1","workspace_id":"00000000-0000-4000-8000-000000000001","operation":"protocol.ping","params":{},"marker":"private"}"#;
        assert_eq!(
            decode_json::<RequestEnvelope>(u, true).err(),
            Some(MessageError::InvalidEnvelope)
        );
        let d = format!(
            "{}0{}",
            "[".repeat(MAX_JSON_DEPTH + 2),
            "]".repeat(MAX_JSON_DEPTH + 2)
        );
        assert_eq!(strict_value(d.as_bytes()), Err(MessageError::TooDeep));
    }
    #[test]
    fn response_requires_one_outcome() {
        let r = ResponseEnvelope {
            message_type: ResponseType::Response,
            protocol_version: 1,
            request_id: DecimalU64::new(1).unwrap(),
            workspace_id: workspace(),
            result: Some(json!({})),
            error: Some(ErrorBody::not_applied(
                ErrorCode::Conflict,
                Recovery::Refresh,
            )),
        };
        assert_eq!(r.validate(), Err(MessageError::InvalidEnvelope));
    }
    #[test]
    fn subscription_messages_are_correlated_and_strict() {
        let subscription: SubscriptionId = "00000000-0000-4000-8000-000000000099".parse().unwrap();
        let event = EventEnvelope {
            message_type: EventMessageType::Event,
            protocol_version: PROTOCOL_VERSION,
            workspace_id: workspace(),
            subscription_id: subscription,
            request_id: DecimalU64::new(7).unwrap(),
            sequence: DecimalU64::new(9).unwrap(),
            event: json!({"kind":"task_changed"}),
        };
        let bytes = encode_json(&event).unwrap();
        let decoded: EventEnvelope = decode_json(&bytes, false).unwrap();
        assert_eq!(decoded.subscription_id, subscription);
        assert_eq!(decoded.request_id.get(), 7);
        assert_eq!(decoded.sequence.get(), 9);

        let control = SynchronizationEnvelope {
            message_type: SynchronizationMessageType::Event,
            protocol_version: PROTOCOL_VERSION,
            workspace_id: workspace(),
            subscription_id: subscription,
            request_id: DecimalU64::new(7).unwrap(),
            control: SynchronizationControl::ResnapshotRequired,
            reason: Some(SynchronizationReason::SlowSubscriber),
            watermark: None,
        };
        let bytes = encode_json(&control).unwrap();
        let decoded: SynchronizationEnvelope = decode_json(&bytes, false).unwrap();
        assert_eq!(decoded.control, SynchronizationControl::ResnapshotRequired);
        assert_eq!(decoded.reason, Some(SynchronizationReason::SlowSubscriber));

        let with_unknown = br#"{"type":"event","protocol_version":1,"workspace_id":"00000000-0000-4000-8000-000000000001","subscription_id":"00000000-0000-4000-8000-000000000099","request_id":"7","control":"resnapshot_required","reason":"slow_subscriber","private_marker":"must-not-pass"}"#;
        assert_eq!(
            decode_json::<SynchronizationEnvelope>(with_unknown, false).err(),
            Some(MessageError::InvalidEnvelope)
        );
    }
    #[test]
    fn terminal_is_opaque_and_bounded() {
        let f = TerminalFrame {
            session_id: SessionId::from_uuid(Uuid::nil()),
            stream_id: 7,
            offset: 9,
            data: vec![0, 255, 3],
        };
        let d = TerminalFrame::decode(&f.encode().unwrap()).unwrap();
        assert_eq!(d.data, f.data);
        assert_eq!(d.stream_id, 7);
        assert!(TerminalFrame { data: vec![], ..f }.encode().is_err());
    }
    #[test]
    fn paths_and_pages_are_bounded() {
        let d = NativePathDto::from_bytes("unix_bytes_v1", &[255, 0]).unwrap();
        assert_eq!(d.decode().unwrap(), vec![255, 0]);
        assert!(NativePathDto::from_bytes("unix_bytes_v1", &vec![0; 8193]).is_err());
        assert!(
            PageRequest {
                after_id: None,
                limit: 0,
                expected_revision: DecimalU64::new(1).unwrap()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn scalars_and_unknown_operations_are_explicit() {
        for value in [r#""0""#, r#""01""#, "1", r#""18446744073709551616""#] {
            assert!(serde_json::from_str::<DecimalU64>(value).is_err());
        }
        assert_eq!(
            serde_json::from_str::<DecimalU64>(r#""18446744073709551615""#)
                .unwrap()
                .get(),
            u64::MAX
        );
        assert_eq!(
            serde_json::from_str::<Operation>(r#""future.operation""#).unwrap(),
            Operation::Unknown
        );
        assert!(serde_json::from_str::<Operation>(&format!("\"{}\"", "x".repeat(65))).is_err());
        assert!(
            TimestampDto {
                unix_seconds: "01".into(),
                nanoseconds: 0
            }
            .validate()
            .is_err()
        );
        assert!(
            TimestampDto {
                unix_seconds: "0".into(),
                nanoseconds: 999_999_999
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn payload_limits_and_utf8_are_checked_before_deserialization() {
        let oversized = vec![b' '; crate::JSON_FRAME_LIMIT + 1];
        assert_eq!(
            decode_json::<Value>(&oversized, false).err(),
            Some(MessageError::TooLarge)
        );
        assert_eq!(
            decode_json::<Value>(&[0xff], false).err(),
            Some(MessageError::InvalidJson)
        );
    }
}
