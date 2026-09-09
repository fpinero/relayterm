use crate::validation::{native_path, paths, set, text};
use crate::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

macro_rules! record {
    ($entity:ident, $record:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        /// Boundary record; reconstruction must pass the entity validator.
        #[derive(Clone, PartialEq, Eq)]
        pub struct $record { $(pub $field: $ty),* }
        #[derive(Clone, PartialEq, Eq)]
        pub struct $entity(pub(crate) $record);
        impl $entity {
            pub fn record(&self) -> &$record { &self.0 }
            pub fn restore(record: $record) -> Result<Self> {
                let entity = Self(record); entity.validate()?; Ok(entity)
            }
        }
    };
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    LocalUser,
    Instance(AgentInstanceId),
    System,
}
impl Actor {
    pub fn instance(self) -> Option<AgentInstanceId> {
        if let Self::Instance(id) = self {
            Some(id)
        } else {
            None
        }
    }
    pub(crate) fn user(self) -> Result<()> {
        if self == Self::LocalUser {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
impl TaskStatus {
    pub const ALL: [Self; 7] = [
        Self::Backlog,
        Self::Ready,
        Self::Active,
        Self::Blocked,
        Self::HandoverReady,
        Self::Done,
        Self::Cancelled,
    ];
    pub fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
    pub fn allows(self, next: Self) -> bool {
        use TaskStatus::*;
        matches!(
            (self, next),
            (Backlog, Ready | Cancelled)
                | (Ready, Active | Cancelled)
                | (Active, Blocked | HandoverReady | Done | Cancelled)
                | (Blocked, Ready | Cancelled)
                | (HandoverReady, Active | Cancelled)
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Starting,
    Running,
    Exited,
    Failed,
    Terminated,
    Lost,
}
impl InstanceStatus {
    pub const ALL: [Self; 6] = [
        Self::Starting,
        Self::Running,
        Self::Exited,
        Self::Failed,
        Self::Terminated,
        Self::Lost,
    ];
    pub fn is_final(self) -> bool {
        !matches!(self, Self::Starting | Self::Running)
    }
    pub fn allows(self, next: Self) -> bool {
        use InstanceStatus::*;
        matches!(
            (self, next),
            (Starting, Running | Failed | Terminated | Lost)
                | (Running, Exited | Failed | Terminated | Lost)
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    ExplicitRelease,
    Blocking,
    Handover,
    Completion,
    Cancellation,
    InstanceEnd,
}

record!(
    Workspace,
    WorkspaceRecord {
        id: WorkspaceId,
        display_name: String,
        project_root: PathBuf,
        created_at: Timestamp,
        updated_at: Timestamp,
        schema_version: u32,
    }
);
impl Workspace {
    fn validate(&self) -> Result<()> {
        text(&self.0.display_name, "display_name", 256, true, false)?;
        native_path(&self.0.project_root, "project_root")?;
        if self.0.schema_version != 1 {
            return Err(Error::Version);
        }
        self.0.updated_at.not_before(self.0.created_at)
    }
}
record!(AgentDefinition, AgentDefinitionRecord {
    id: AgentDefinitionId, workspace_id: WorkspaceId, display_name: String,
    command: String, arguments: Vec<String>, environment_allowlist: Vec<String>,
    capabilities: Vec<String>, enabled: bool,
});
impl AgentDefinition {
    fn validate(&self) -> Result<()> {
        let r = &self.0;
        text(&r.display_name, "display_name", 256, true, false)?;
        text(&r.command, "command", 4096, true, false)?;
        if r.arguments.len() > 128 || r.arguments.iter().map(String::len).sum::<usize>() > 32768 {
            return Err(Error::Validation("arguments"));
        }
        for arg in &r.arguments {
            text(arg, "arguments", 4096, false, false)?;
        }
        set(&r.environment_allowlist, "environment_allowlist")?;
        if cfg!(windows) {
            let names: Vec<_> = r
                .environment_allowlist
                .iter()
                .map(|name| name.to_ascii_uppercase())
                .collect();
            set(&names, "environment_allowlist")?;
        }
        for name in &r.environment_allowlist {
            text(name, "environment_allowlist", 256, true, false)?;
            if !name
                .bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
            {
                return Err(Error::Validation("environment_allowlist"));
            }
        }
        set(&r.capabilities, "capabilities")?;
        for name in &r.capabilities {
            text(name, "capabilities", 256, true, false)?;
        }
        Ok(())
    }
}
#[derive(Clone, Eq, PartialEq)]
pub struct LaunchDefinitionSnapshot {
    pub definition_id: AgentDefinitionId,
    pub display_name: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub environment_allowlist: Vec<String>,
    pub capabilities: Vec<String>,
    pub enabled: bool,
}
impl LaunchDefinitionSnapshot {
    pub fn from_definition(definition: &AgentDefinition) -> Self {
        let record = definition.record();
        Self {
            definition_id: record.id,
            display_name: record.display_name.clone(),
            command: record.command.clone(),
            arguments: record.arguments.clone(),
            environment_allowlist: record.environment_allowlist.clone(),
            capabilities: record.capabilities.clone(),
            enabled: record.enabled,
        }
    }
    pub fn validate(&self, workspace_id: WorkspaceId) -> Result<()> {
        AgentDefinition::restore(AgentDefinitionRecord {
            id: self.definition_id,
            workspace_id,
            display_name: self.display_name.clone(),
            command: self.command.clone(),
            arguments: self.arguments.clone(),
            environment_allowlist: self.environment_allowlist.clone(),
            capabilities: self.capabilities.clone(),
            enabled: self.enabled,
        })
        .map(|_| ())
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct TaskContent {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub scope_paths: Vec<String>,
    pub acceptance_notes: String,
    pub dependency_ids: Vec<TaskId>,
}
impl TaskContent {
    pub fn validate(&self) -> Result<()> {
        text(&self.title, "title", 256, true, false)?;
        text(&self.description, "description", 16384, false, true)?;
        text(
            &self.acceptance_notes,
            "acceptance_notes",
            16384,
            false,
            true,
        )?;
        paths(&self.scope_paths, "scope_paths")?;
        set(&self.dependency_ids, "dependency_ids")
    }
}
record!(Task, TaskRecord {
    id: TaskId, workspace_id: WorkspaceId, content: TaskContent, status: TaskStatus,
    claimed_by_instance_id: Option<AgentInstanceId>, worktree_id: Option<WorktreeId>,
    created_at: Timestamp, updated_at: Timestamp,
});
impl Task {
    fn validate(&self) -> Result<()> {
        let r = &self.0;
        r.content.validate()?;
        if r.content.dependency_ids.contains(&r.id) {
            return Err(Error::Reference);
        }
        if (r.status == TaskStatus::Active) != r.claimed_by_instance_id.is_some() {
            return Err(Error::State);
        }
        r.updated_at.not_before(r.created_at)
    }
    pub fn select_worktree(&self, worktree_id: Option<WorktreeId>, at: Timestamp) -> Result<Self> {
        if self.0.status.is_final() || self.0.status == TaskStatus::Active {
            return Err(Error::State);
        }
        at.not_before(self.0.updated_at)?;
        let mut record = self.0.clone();
        record.worktree_id = worktree_id;
        record.updated_at = at;
        Self::restore(record)
    }
    pub(crate) fn transition(
        &mut self,
        next: TaskStatus,
        owner: Option<AgentInstanceId>,
        at: Timestamp,
    ) -> Result<()> {
        if !self.0.status.allows(next) {
            return Err(Error::State);
        }
        at.not_before(self.0.updated_at)?;
        let mut record = self.0.clone();
        record.status = next;
        record.claimed_by_instance_id = owner;
        record.updated_at = at;
        *self = Self::restore(record)?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSize {
    rows: u16,
    columns: u16,
}
impl TerminalSize {
    pub fn new(rows: u16, columns: u16) -> Result<Self> {
        if !(1..=1000).contains(&rows) || !(1..=1000).contains(&columns) {
            return Err(Error::Validation("terminal_size"));
        }
        Ok(Self { rows, columns })
    }
    pub fn rows(self) -> u16 {
        self.rows
    }
    pub fn columns(self) -> u16 {
        self.columns
    }
}
record!(AgentInstance, AgentInstanceRecord {
    id: AgentInstanceId, session_id: TerminalSessionId, workspace_id: WorkspaceId,
    agent_definition_id: Option<AgentDefinitionId>, task_id: Option<TaskId>,
    launch_definition: Option<LaunchDefinitionSnapshot>, worktree_id: Option<WorktreeId>,
    working_directory: PathBuf, status: InstanceStatus, started_at: Timestamp, last_observed_at: Timestamp,
    ended_at: Option<Timestamp>, exit_code: Option<i32>, terminal_size: TerminalSize,
});
impl AgentInstance {
    fn validate(&self) -> Result<()> {
        let r = &self.0;
        native_path(&r.working_directory, "working_directory")?;
        if r.agent_definition_id != r.launch_definition.as_ref().map(|x| x.definition_id) {
            return Err(Error::State);
        }
        if let Some(snapshot) = &r.launch_definition {
            snapshot.validate(r.workspace_id)?;
        }
        if r.status.is_final() != r.ended_at.is_some()
            || (!r.status.is_final() && r.exit_code.is_some())
            || (r.status == InstanceStatus::Lost && r.exit_code.is_some())
        {
            return Err(Error::State);
        }
        r.last_observed_at.not_before(r.started_at)?;
        if r.status == InstanceStatus::Starting && r.last_observed_at != r.started_at {
            return Err(Error::Time);
        }
        if let Some(end) = r.ended_at
            && end != r.last_observed_at
        {
            return Err(Error::Time);
        }
        Ok(())
    }
    pub(crate) fn observe(
        &mut self,
        status: InstanceStatus,
        at: Timestamp,
        exit_code: Option<i32>,
    ) -> Result<bool> {
        if self.0.status.is_final() {
            return if self.0.status == status
                && self.0.ended_at == Some(at)
                && self.0.exit_code == exit_code
            {
                Ok(false)
            } else {
                Err(Error::State)
            };
        }
        if !self.0.status.allows(status)
            || (self.0.status == InstanceStatus::Starting && exit_code.is_some())
        {
            return Err(Error::State);
        }
        at.not_before(self.0.last_observed_at)?;
        let mut r = self.0.clone();
        r.status = status;
        r.last_observed_at = at;
        r.ended_at = status.is_final().then_some(at);
        r.exit_code = exit_code;
        *self = Self::restore(r)?;
        Ok(true)
    }
}
record!(Claim, ClaimRecord {
    id: ClaimId, workspace_id: WorkspaceId, task_id: TaskId, instance_id: AgentInstanceId,
    requested_by: Actor, opened_at: Timestamp, closed_at: Option<Timestamp>,
    close_reason: Option<CloseReason>, closed_by: Option<Actor>,
});
impl Claim {
    fn validate(&self) -> Result<()> {
        let r = &self.0;
        if !matches!(r.requested_by, Actor::LocalUser)
            && r.requested_by != Actor::Instance(r.instance_id)
        {
            return Err(Error::Unauthorized);
        }
        if r.closed_at.is_some() != r.close_reason.is_some()
            || r.closed_at.is_some() != r.closed_by.is_some()
        {
            return Err(Error::State);
        }
        if let Some(at) = r.closed_at {
            at.not_before(r.opened_at)?;
            match (r.close_reason, r.closed_by) {
                (Some(CloseReason::InstanceEnd), Some(Actor::System)) => {}
                (Some(CloseReason::InstanceEnd), _) | (_, Some(Actor::System)) => {
                    return Err(Error::Unauthorized);
                }
                (_, Some(Actor::LocalUser)) => {}
                (_, Some(Actor::Instance(id))) if id == r.instance_id => {}
                _ => return Err(Error::Unauthorized),
            }
        }
        Ok(())
    }
    pub(crate) fn close(&mut self, reason: CloseReason, actor: Actor, at: Timestamp) -> Result<()> {
        if self.0.closed_at.is_some() {
            return Err(Error::State);
        }
        at.not_before(self.0.opened_at)?;
        self.0.closed_at = Some(at);
        self.0.close_reason = Some(reason);
        self.0.closed_by = Some(actor);
        Ok(())
    }
}
record!(ProgressEntry, ProgressEntryRecord {
    id: ProgressEntryId, workspace_id: WorkspaceId, task_id: TaskId,
    agent_instance_id: Option<AgentInstanceId>, summary: String, verification: String, created_at: Timestamp,
});
impl ProgressEntry {
    fn validate(&self) -> Result<()> {
        text(&self.0.summary, "summary", 8192, true, true)?;
        text(&self.0.verification, "verification", 8192, false, true)
    }
}
#[derive(Clone, Eq, PartialEq)]
pub struct HandoverContent {
    pub summary: String,
    pub decisions: String,
    pub changed_paths: Vec<String>,
    pub verification_performed: String,
    pub open_questions: String,
    pub recommended_next_action: String,
}
impl HandoverContent {
    pub fn validate(&self) -> Result<()> {
        text(&self.summary, "summary", 8192, true, true)?;
        text(&self.decisions, "decisions", 16384, false, true)?;
        paths(&self.changed_paths, "changed_paths")?;
        text(
            &self.verification_performed,
            "verification_performed",
            8192,
            true,
            true,
        )?;
        text(&self.open_questions, "open_questions", 16384, false, true)?;
        text(
            &self.recommended_next_action,
            "recommended_next_action",
            8192,
            true,
            true,
        )?;
        let size = self.summary.len()
            + self.decisions.len()
            + self.changed_paths.iter().map(String::len).sum::<usize>()
            + self.verification_performed.len()
            + self.open_questions.len()
            + self.recommended_next_action.len();
        if size > 65536 {
            return Err(Error::Validation("handover"));
        }
        Ok(())
    }
}
record!(Handover, HandoverRecord {
    id: HandoverId, workspace_id: WorkspaceId, task_id: TaskId, from_agent_instance_id: Option<AgentInstanceId>,
    content: HandoverContent, created_at: Timestamp,
});
impl Handover {
    fn validate(&self) -> Result<()> {
        self.0.content.validate()
    }
}
