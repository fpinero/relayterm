use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskField {
    Title,
    Description,
    Priority,
    ScopePaths,
    AcceptanceNotes,
    DependencyIds,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityId {
    Workspace(WorkspaceId),
    Definition(AgentDefinitionId),
    Task(TaskId),
    Instance(AgentInstanceId),
    Claim(ClaimId),
    Progress(ProgressEntryId),
    Handover(HandoverId),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    WorkspaceCreated,
    DefinitionCreated,
    TaskCreated,
    TaskEdited,
    TaskTransitioned,
    InstanceRegistered,
    InstanceObserved,
    ClaimOpened,
    ClaimClosed,
    ProgressAdded,
    HandoverPrepared,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventPayload {
    WorkspaceCreated {
        id: WorkspaceId,
    },
    DefinitionCreated {
        id: AgentDefinitionId,
    },
    TaskCreated {
        id: TaskId,
    },
    TaskEdited {
        id: TaskId,
        fields: Vec<TaskField>,
    },
    TaskTransitioned {
        id: TaskId,
        from: TaskStatus,
        to: TaskStatus,
    },
    InstanceRegistered {
        id: AgentInstanceId,
        session_id: TerminalSessionId,
    },
    InstanceObserved {
        id: AgentInstanceId,
        from: InstanceStatus,
        to: InstanceStatus,
    },
    ClaimOpened {
        id: ClaimId,
        task_id: TaskId,
        instance_id: AgentInstanceId,
    },
    ClaimClosed {
        id: ClaimId,
        task_id: TaskId,
        instance_id: AgentInstanceId,
        reason: CloseReason,
    },
    ProgressAdded {
        id: ProgressEntryId,
        task_id: TaskId,
    },
    HandoverPrepared {
        id: HandoverId,
        task_id: TaskId,
    },
}
impl EventPayload {
    pub fn event_type(&self) -> EventType {
        match self {
            Self::WorkspaceCreated { .. } => EventType::WorkspaceCreated,
            Self::DefinitionCreated { .. } => EventType::DefinitionCreated,
            Self::TaskCreated { .. } => EventType::TaskCreated,
            Self::TaskEdited { .. } => EventType::TaskEdited,
            Self::TaskTransitioned { .. } => EventType::TaskTransitioned,
            Self::InstanceRegistered { .. } => EventType::InstanceRegistered,
            Self::InstanceObserved { .. } => EventType::InstanceObserved,
            Self::ClaimOpened { .. } => EventType::ClaimOpened,
            Self::ClaimClosed { .. } => EventType::ClaimClosed,
            Self::ProgressAdded { .. } => EventType::ProgressAdded,
            Self::HandoverPrepared { .. } => EventType::HandoverPrepared,
        }
    }
    pub fn entity_id(&self) -> EntityId {
        match self {
            Self::WorkspaceCreated { id } => EntityId::Workspace(*id),
            Self::DefinitionCreated { id } => EntityId::Definition(*id),
            Self::TaskCreated { id }
            | Self::TaskEdited { id, .. }
            | Self::TaskTransitioned { id, .. } => EntityId::Task(*id),
            Self::InstanceRegistered { id, .. } | Self::InstanceObserved { id, .. } => {
                EntityId::Instance(*id)
            }
            Self::ClaimOpened { id, .. } | Self::ClaimClosed { id, .. } => EntityId::Claim(*id),
            Self::ProgressAdded { id, .. } => EntityId::Progress(*id),
            Self::HandoverPrepared { id, .. } => EntityId::Handover(*id),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingEvent {
    pub event_id: EventId,
    pub workspace_id: WorkspaceId,
    pub timestamp: Timestamp,
    pub actor: Actor,
    pub payload: EventPayload,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "EventRecord")]
pub struct WorkspaceEvent(EventRecord);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventRecord {
    pub sequence: u64,
    pub event_id: EventId,
    pub workspace_id: WorkspaceId,
    pub event_type: EventType,
    pub entity_id: EntityId,
    pub timestamp: Timestamp,
    pub actor: Actor,
    pub payload_version: u32,
    pub payload: EventPayload,
}
impl WorkspaceEvent {
    pub fn confirm(event: PendingEvent, sequence: u64) -> Result<Self> {
        Self::try_from(EventRecord {
            sequence,
            event_id: event.event_id,
            workspace_id: event.workspace_id,
            event_type: event.payload.event_type(),
            entity_id: event.payload.entity_id(),
            timestamp: event.timestamp,
            actor: event.actor,
            payload_version: 1,
            payload: event.payload,
        })
    }
    pub fn record(&self) -> &EventRecord {
        &self.0
    }
}
impl TryFrom<EventRecord> for WorkspaceEvent {
    type Error = Error;
    fn try_from(record: EventRecord) -> Result<Self> {
        if record.payload_version != 1 {
            return Err(Error::Version);
        }
        if record.sequence == 0
            || record.event_type != record.payload.event_type()
            || record.entity_id != record.payload.entity_id()
        {
            return Err(Error::State);
        }
        if let EventPayload::TaskEdited { fields, .. } = &record.payload {
            crate::validation::set(fields, "fields")?;
        }
        Ok(Self(record))
    }
}
impl From<WorkspaceEvent> for EventRecord {
    fn from(event: WorkspaceEvent) -> Self {
        event.0
    }
}

impl<'de> Deserialize<'de> for WorkspaceEvent {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let record = EventRecord::deserialize(deserializer)
            .map_err(|_| serde::de::Error::custom("invalid workspace event"))?;
        Self::try_from(record).map_err(serde::de::Error::custom)
    }
}
