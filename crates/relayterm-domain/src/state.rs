use crate::*;

/// A workspace snapshot is private, validated state, never a wire DTO.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkspaceState {
    workspace: Workspace,
    definitions: Vec<AgentDefinition>,
    tasks: Vec<Task>,
    instances: Vec<AgentInstance>,
    claims: Vec<Claim>,
    progress: Vec<ProgressEntry>,
    handovers: Vec<Handover>,
}
/// Rows loaded by an adapter; every reference is checked on reconstruction.
#[derive(Default)]
pub struct WorkspaceRows {
    pub definitions: Vec<AgentDefinition>,
    pub tasks: Vec<Task>,
    pub instances: Vec<AgentInstance>,
    pub claims: Vec<Claim>,
    pub progress: Vec<ProgressEntry>,
    pub handovers: Vec<Handover>,
}
/// User-facing intent cannot carry the internal System actor.
pub enum Command {
    AddDefinition(AgentDefinition),
    UpdateDefinition(AgentDefinition),
    ImportDefinitions(Vec<AgentDefinition>),
    CreateTask {
        id: TaskId,
        content: TaskContent,
    },
    EditTask {
        id: TaskId,
        content: TaskContent,
    },
    Transition {
        id: TaskId,
        to: TaskStatus,
    },
    Claim {
        id: ClaimId,
        task_id: TaskId,
        instance_id: AgentInstanceId,
    },
    Release {
        task_id: TaskId,
    },
    Progress {
        id: ProgressEntryId,
        task_id: TaskId,
        summary: String,
        verification: String,
    },
    Handover {
        id: HandoverId,
        task_id: TaskId,
        content: HandoverContent,
    },
}
/// Supervisor-only observations, routed separately from client commands.
pub enum Observation {
    Register(Box<AgentInstance>),
    Status {
        id: AgentInstanceId,
        status: InstanceStatus,
        observed_at: Timestamp,
        exit_code: Option<i32>,
    },
    ReconcileLost,
}
/// Opaque result of a successful pure operation, suitable for atomic storage.
pub struct Changes {
    before: WorkspaceState,
    after: WorkspaceState,
    payloads: Vec<EventPayload>,
    actor: Actor,
    timestamp: Timestamp,
}
impl Changes {
    pub fn before(&self) -> &WorkspaceState {
        &self.before
    }
    pub fn after(&self) -> &WorkspaceState {
        &self.after
    }
    pub fn payloads(&self) -> &[EventPayload] {
        &self.payloads
    }
    pub fn actor(&self) -> Actor {
        self.actor
    }
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}
impl WorkspaceState {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace,
            definitions: vec![],
            tasks: vec![],
            instances: vec![],
            claims: vec![],
            progress: vec![],
            handovers: vec![],
        }
    }
    pub fn restore(workspace: Workspace, rows: WorkspaceRows) -> Result<Self> {
        let state = Self {
            workspace,
            definitions: rows.definitions,
            tasks: rows.tasks,
            instances: rows.instances,
            claims: rows.claims,
            progress: rows.progress,
            handovers: rows.handovers,
        };
        state.validate()?;
        Ok(state)
    }
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }
    pub fn definitions(&self) -> &[AgentDefinition] {
        &self.definitions
    }
    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }
    pub fn instances(&self) -> &[AgentInstance] {
        &self.instances
    }
    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }
    pub fn progress(&self) -> &[ProgressEntry] {
        &self.progress
    }
    pub fn handovers(&self) -> &[Handover] {
        &self.handovers
    }
    pub fn task(&self, id: TaskId) -> Result<&Task> {
        self.tasks
            .iter()
            .find(|x| x.0.id == id)
            .ok_or(Error::Reference)
    }
    pub fn instance(&self, id: AgentInstanceId) -> Result<&AgentInstance> {
        self.instances
            .iter()
            .find(|x| x.0.id == id)
            .ok_or(Error::Reference)
    }
    pub fn current_claim(&self, id: TaskId) -> Option<&Claim> {
        self.claims
            .iter()
            .find(|x| x.0.task_id == id && x.0.closed_at.is_none())
    }
    fn task_mut(&mut self, id: TaskId) -> Result<&mut Task> {
        self.tasks
            .iter_mut()
            .find(|x| x.0.id == id)
            .ok_or(Error::Reference)
    }
    fn owner(&self, id: TaskId, actor: Actor) -> Result<()> {
        let claim = self.current_claim(id).ok_or(Error::State)?;
        if actor == Actor::LocalUser || actor == Actor::Instance(claim.0.instance_id) {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
    fn dependencies(&self, id: TaskId, content: &TaskContent) -> Result<()> {
        content.validate()?;
        for dep in &content.dependency_ids {
            if *dep == id {
                return Err(Error::Reference);
            }
            self.task(*dep)?;
        }
        Ok(())
    }
    fn close(
        &mut self,
        task_id: TaskId,
        reason: CloseReason,
        actor: Actor,
        at: Timestamp,
        events: &mut Vec<EventPayload>,
    ) -> Result<()> {
        let claim = self
            .claims
            .iter_mut()
            .find(|x| x.0.task_id == task_id && x.0.closed_at.is_none())
            .ok_or(Error::State)?;
        claim.close(reason, actor, at)?;
        events.push(EventPayload::ClaimClosed {
            id: claim.0.id,
            task_id,
            instance_id: claim.0.instance_id,
            reason,
        });
        Ok(())
    }
    fn transition(
        &mut self,
        id: TaskId,
        to: TaskStatus,
        owner: Option<AgentInstanceId>,
        at: Timestamp,
        events: &mut Vec<EventPayload>,
    ) -> Result<()> {
        let task = self.task_mut(id)?;
        let from = task.0.status;
        task.transition(to, owner, at)?;
        events.push(EventPayload::TaskTransitioned { id, from, to });
        Ok(())
    }
    pub fn execute(&self, actor: Actor, command: Command, at: Timestamp) -> Result<Changes> {
        if actor == Actor::System {
            return Err(Error::Unauthorized);
        }
        at.not_before(self.workspace.0.updated_at)?;
        let mut after = self.clone();
        let mut events = vec![];
        after.apply(actor, command, at, &mut events)?;
        if !events.is_empty() {
            after.workspace.0.updated_at = at;
        }
        after.validate()?;
        Ok(Changes {
            before: self.clone(),
            after,
            payloads: events,
            actor,
            timestamp: at,
        })
    }
    fn apply(
        &mut self,
        actor: Actor,
        command: Command,
        at: Timestamp,
        events: &mut Vec<EventPayload>,
    ) -> Result<()> {
        let workspace_id = self.workspace.0.id;
        match command {
            Command::AddDefinition(definition) => {
                actor.user()?;
                if definition.0.workspace_id != workspace_id {
                    return Err(Error::Reference);
                }
                if self.definitions.iter().any(|d| d.0.id == definition.0.id) {
                    return Err(Error::Conflict);
                }
                events.push(EventPayload::DefinitionCreated {
                    id: definition.0.id,
                });
                self.definitions.push(definition);
            }
            Command::UpdateDefinition(definition) => {
                actor.user()?;
                if definition.0.workspace_id != workspace_id {
                    return Err(Error::Reference);
                }
                let current = self
                    .definitions
                    .iter_mut()
                    .find(|current| current.0.id == definition.0.id)
                    .ok_or(Error::Reference)?;
                let old = current.record();
                let new = definition.record();
                let mut fields = Vec::new();
                if old.display_name != new.display_name {
                    fields.push(DefinitionField::DisplayName);
                }
                if old.command != new.command {
                    fields.push(DefinitionField::Command);
                }
                if old.arguments != new.arguments {
                    fields.push(DefinitionField::Arguments);
                }
                if old.environment_allowlist != new.environment_allowlist {
                    fields.push(DefinitionField::EnvironmentAllowlist);
                }
                if old.capabilities != new.capabilities {
                    fields.push(DefinitionField::Capabilities);
                }
                if old.enabled != new.enabled {
                    fields.push(DefinitionField::Enabled);
                }
                if !fields.is_empty() {
                    let id = new.id;
                    *current = definition;
                    events.push(EventPayload::DefinitionUpdated { id, fields });
                }
            }
            Command::ImportDefinitions(definitions) => {
                actor.user()?;
                for (index, definition) in definitions.iter().enumerate() {
                    if definition.0.workspace_id != workspace_id {
                        return Err(Error::Reference);
                    }
                    if definitions[..index]
                        .iter()
                        .any(|candidate| candidate.0.id == definition.0.id)
                    {
                        return Err(Error::Conflict);
                    }
                }
                for definition in definitions {
                    if let Some(current) = self
                        .definitions
                        .iter_mut()
                        .find(|current| current.0.id == definition.0.id)
                    {
                        let old = current.record();
                        let new = definition.record();
                        let mut fields = Vec::new();
                        if old.display_name != new.display_name {
                            fields.push(DefinitionField::DisplayName);
                        }
                        if old.command != new.command {
                            fields.push(DefinitionField::Command);
                        }
                        if old.arguments != new.arguments {
                            fields.push(DefinitionField::Arguments);
                        }
                        if old.environment_allowlist != new.environment_allowlist {
                            fields.push(DefinitionField::EnvironmentAllowlist);
                        }
                        if old.capabilities != new.capabilities {
                            fields.push(DefinitionField::Capabilities);
                        }
                        if old.enabled != new.enabled {
                            fields.push(DefinitionField::Enabled);
                        }
                        if !fields.is_empty() {
                            let id = new.id;
                            *current = definition;
                            events.push(EventPayload::DefinitionUpdated { id, fields });
                        }
                    } else {
                        events.push(EventPayload::DefinitionCreated {
                            id: definition.0.id,
                        });
                        self.definitions.push(definition);
                    }
                }
            }
            Command::CreateTask { id, content } => {
                actor.user()?;
                if self.task(id).is_ok() {
                    return Err(Error::Conflict);
                }
                self.dependencies(id, &content)?;
                self.tasks.push(Task::restore(TaskRecord {
                    id,
                    workspace_id,
                    content,
                    status: TaskStatus::Backlog,
                    claimed_by_instance_id: None,
                    worktree_id: None,
                    created_at: at,
                    updated_at: at,
                })?);
                events.push(EventPayload::TaskCreated { id });
            }
            Command::EditTask { id, content } => {
                if actor != Actor::LocalUser {
                    self.owner(id, actor)?;
                }
                self.dependencies(id, &content)?;
                let task = self.task_mut(id)?;
                if task.0.status.is_final() {
                    return Err(Error::State);
                }
                let old = &task.0.content;
                let mut fields = vec![];
                if old.title != content.title {
                    fields.push(TaskField::Title);
                }
                if old.description != content.description {
                    fields.push(TaskField::Description);
                }
                if old.priority != content.priority {
                    fields.push(TaskField::Priority);
                }
                if old.scope_paths != content.scope_paths {
                    fields.push(TaskField::ScopePaths);
                }
                if old.acceptance_notes != content.acceptance_notes {
                    fields.push(TaskField::AcceptanceNotes);
                }
                if old.dependency_ids != content.dependency_ids {
                    fields.push(TaskField::DependencyIds);
                }
                task.0.content = content;
                task.0.updated_at = at;
                events.push(EventPayload::TaskEdited { id, fields });
            }
            Command::Transition { id, to } => {
                if matches!(to, TaskStatus::Active | TaskStatus::HandoverReady) {
                    return Err(Error::State);
                }
                let active = self.task(id)?.0.status == TaskStatus::Active;
                if active {
                    self.owner(id, actor)?;
                } else {
                    actor.user()?;
                }
                self.transition(id, to, None, at, events)?;
                if active {
                    let reason = match to {
                        TaskStatus::Blocked => CloseReason::Blocking,
                        TaskStatus::Done => CloseReason::Completion,
                        TaskStatus::Cancelled => CloseReason::Cancellation,
                        _ => return Err(Error::State),
                    };
                    self.close(id, reason, actor, at, events)?;
                }
            }
            Command::Claim {
                id,
                task_id,
                instance_id,
            } => {
                if actor != Actor::LocalUser && actor != Actor::Instance(instance_id) {
                    return Err(Error::Unauthorized);
                }
                if self.instance(instance_id)?.0.status != InstanceStatus::Running {
                    return Err(Error::State);
                }
                if self.claims.iter().any(|c| {
                    c.0.id == id
                        || (c.0.closed_at.is_none()
                            && (c.0.task_id == task_id || c.0.instance_id == instance_id))
                }) {
                    return Err(Error::Conflict);
                }
                self.transition(task_id, TaskStatus::Active, Some(instance_id), at, events)?;
                self.claims.push(Claim::restore(ClaimRecord {
                    id,
                    workspace_id,
                    task_id,
                    instance_id,
                    requested_by: actor,
                    opened_at: at,
                    closed_at: None,
                    close_reason: None,
                    closed_by: None,
                })?);
                events.push(EventPayload::ClaimOpened {
                    id,
                    task_id,
                    instance_id,
                });
            }
            Command::Release { task_id } => {
                self.owner(task_id, actor)?;
                self.transition(task_id, TaskStatus::Blocked, None, at, events)?;
                self.close(task_id, CloseReason::ExplicitRelease, actor, at, events)?;
            }
            Command::Progress {
                id,
                task_id,
                summary,
                verification,
            } => {
                self.task(task_id)?;
                if actor != Actor::LocalUser {
                    self.owner(task_id, actor)?;
                }
                if self.progress.iter().any(|p| p.0.id == id) {
                    return Err(Error::Conflict);
                }
                self.progress
                    .push(ProgressEntry::restore(ProgressEntryRecord {
                        id,
                        workspace_id,
                        task_id,
                        agent_instance_id: actor.instance(),
                        summary,
                        verification,
                        created_at: at,
                    })?);
                events.push(EventPayload::ProgressAdded { id, task_id });
            }
            Command::Handover {
                id,
                task_id,
                content,
            } => {
                self.owner(task_id, actor)?;
                if self.handovers.iter().any(|h| h.0.id == id) {
                    return Err(Error::Conflict);
                }
                let handover = Handover::restore(HandoverRecord {
                    id,
                    workspace_id,
                    task_id,
                    from_agent_instance_id: actor.instance(),
                    content,
                    created_at: at,
                })?;
                self.transition(task_id, TaskStatus::HandoverReady, None, at, events)?;
                self.close(task_id, CloseReason::Handover, actor, at, events)?;
                self.handovers.push(handover);
                events.push(EventPayload::HandoverPrepared { id, task_id });
            }
        }
        Ok(())
    }
    pub fn observe(&self, observation: Observation, at: Timestamp) -> Result<Changes> {
        let mut after = self.clone();
        let mut events = vec![];
        match observation {
            Observation::Register(instance) => {
                let instance = *instance;
                if instance.0.workspace_id != self.workspace.0.id {
                    return Err(Error::Reference);
                }
                if instance.0.status != InstanceStatus::Starting || instance.0.started_at != at {
                    return Err(Error::State);
                }
                if self
                    .instances
                    .iter()
                    .any(|i| i.0.id == instance.0.id || i.0.session_id == instance.0.session_id)
                {
                    return Err(Error::Conflict);
                }
                if let Some(definition_id) = instance.0.agent_definition_id {
                    let definition = self
                        .definitions
                        .iter()
                        .find(|d| d.0.id == definition_id)
                        .ok_or(Error::Reference)?;
                    if !definition.0.enabled {
                        return Err(Error::Unavailable);
                    }
                    if instance.0.launch_definition.as_ref()
                        != Some(&LaunchDefinitionSnapshot::from_definition(definition))
                    {
                        return Err(Error::State);
                    }
                }
                events.push(EventPayload::InstanceRegistered {
                    id: instance.0.id,
                    session_id: instance.0.session_id,
                });
                after.instances.push(instance);
            }
            Observation::Status {
                id,
                status,
                observed_at,
                exit_code,
            } => {
                after.observe_one(id, status, observed_at, exit_code, at, &mut events)?;
            }
            Observation::ReconcileLost => {
                for instance in &self.instances {
                    if !instance.0.status.is_final() {
                        after.observe_one(
                            instance.0.id,
                            InstanceStatus::Lost,
                            at,
                            None,
                            at,
                            &mut events,
                        )?;
                    }
                }
            }
        }
        if !events.is_empty() {
            at.not_before(self.workspace.0.updated_at)?;
            after.workspace.0.updated_at = at;
        }
        after.validate()?;
        Ok(Changes {
            before: self.clone(),
            after,
            payloads: events,
            actor: Actor::System,
            timestamp: at,
        })
    }
    fn observe_one(
        &mut self,
        id: AgentInstanceId,
        status: InstanceStatus,
        observed_at: Timestamp,
        exit_code: Option<i32>,
        at: Timestamp,
        events: &mut Vec<EventPayload>,
    ) -> Result<()> {
        let instance = self
            .instances
            .iter_mut()
            .find(|i| i.0.id == id)
            .ok_or(Error::Reference)?;
        let from = instance.0.status;
        if !instance.observe(status, observed_at, exit_code)? {
            return Ok(());
        }
        if observed_at > at {
            return Err(Error::Time);
        }
        events.push(EventPayload::InstanceObserved {
            id,
            from,
            to: status,
        });
        if status.is_final()
            && let Some(claim) = self
                .claims
                .iter()
                .find(|c| c.0.instance_id == id && c.0.closed_at.is_none())
        {
            observed_at.not_before(claim.0.opened_at)?;
            let task_id = claim.0.task_id;
            self.transition(task_id, TaskStatus::Blocked, None, at, events)?;
            self.close(
                task_id,
                CloseReason::InstanceEnd,
                Actor::System,
                observed_at,
                events,
            )?;
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        let wid = self.workspace.0.id;
        macro_rules! unique {
            ($list:expr) => {
                for (n, row) in $list.iter().enumerate() {
                    if row.0.workspace_id != wid {
                        return Err(Error::Reference);
                    }
                    if $list[..n].iter().any(|x| x.0.id == row.0.id) {
                        return Err(Error::Conflict);
                    }
                }
            };
        }
        unique!(self.definitions);
        unique!(self.tasks);
        unique!(self.instances);
        unique!(self.claims);
        unique!(self.progress);
        unique!(self.handovers);
        for (n, i) in self.instances.iter().enumerate() {
            if self.instances[..n]
                .iter()
                .any(|x| x.0.session_id == i.0.session_id)
            {
                return Err(Error::Conflict);
            }
            if let Some(id) = i.0.agent_definition_id
                && !self.definitions.iter().any(|d| d.0.id == id)
            {
                return Err(Error::Reference);
            }
            if let Some(id) = i.0.task_id {
                self.task(id)?;
            }
            i.0.started_at.not_before(self.workspace.0.created_at)?;
            self.workspace
                .0
                .updated_at
                .not_before(i.0.last_observed_at)?;
        }
        for t in &self.tasks {
            self.dependencies(t.0.id, &t.0.content)?;
            t.0.created_at.not_before(self.workspace.0.created_at)?;
            self.workspace.0.updated_at.not_before(t.0.updated_at)?;
            let open: Vec<_> = self
                .claims
                .iter()
                .filter(|c| c.0.task_id == t.0.id && c.0.closed_at.is_none())
                .collect();
            if t.0.status == TaskStatus::Active {
                if open.len() != 1 || Some(open[0].0.instance_id) != t.0.claimed_by_instance_id {
                    return Err(Error::State);
                }
            } else if !open.is_empty() {
                return Err(Error::State);
            }
            let history: Vec<_> = self
                .claims
                .iter()
                .filter(|c| c.0.task_id == t.0.id)
                .collect();
            let last_reason = history.last().and_then(|c| c.0.close_reason);
            let valid_history = match t.0.status {
                TaskStatus::Backlog => history.is_empty(),
                TaskStatus::Done => last_reason == Some(CloseReason::Completion),
                TaskStatus::Blocked => matches!(
                    last_reason,
                    Some(
                        CloseReason::ExplicitRelease
                            | CloseReason::Blocking
                            | CloseReason::InstanceEnd
                    )
                ),
                TaskStatus::HandoverReady => last_reason == Some(CloseReason::Handover),
                TaskStatus::Ready => !matches!(
                    last_reason,
                    Some(CloseReason::Completion | CloseReason::Cancellation)
                ),
                TaskStatus::Active | TaskStatus::Cancelled => true,
            };
            if !valid_history {
                return Err(Error::State);
            }
            if let Some(last) = history.last() {
                t.0.updated_at
                    .not_before(last.0.closed_at.unwrap_or(last.0.opened_at))?;
            }
        }
        for pair in self.claims.windows(2) {
            pair[1].0.opened_at.not_before(pair[0].0.opened_at)?;
        }
        for pair in self.progress.windows(2) {
            pair[1].0.created_at.not_before(pair[0].0.created_at)?;
        }
        for pair in self.handovers.windows(2) {
            pair[1].0.created_at.not_before(pair[0].0.created_at)?;
        }
        for (n, c) in self.claims.iter().enumerate() {
            for earlier in &self.claims[..n] {
                if (earlier.0.task_id == c.0.task_id || earlier.0.instance_id == c.0.instance_id)
                    && earlier.0.closed_at.is_none_or(|end| end > c.0.opened_at)
                {
                    return Err(Error::Conflict);
                }
            }
            let t = self.task(c.0.task_id)?;
            let i = self.instance(c.0.instance_id)?;
            c.0.opened_at.not_before(t.0.created_at)?;
            c.0.opened_at.not_before(i.0.started_at)?;
            if c.0.closed_at.is_none() {
                c.0.opened_at.not_before(i.0.last_observed_at)?;
            }
            self.workspace
                .0
                .updated_at
                .not_before(c.0.closed_at.unwrap_or(c.0.opened_at))?;
            if c.0.close_reason == Some(CloseReason::Handover)
                && !self.handovers.iter().any(|h| {
                    h.0.task_id == c.0.task_id
                        && Some(h.0.created_at) == c.0.closed_at
                        && h.0.from_agent_instance_id == c.0.closed_by.and_then(Actor::instance)
                })
            {
                return Err(Error::State);
            }
            if c.0.closed_at.is_none()
                && (i.0.status != InstanceStatus::Running
                    || self.claims[..n]
                        .iter()
                        .any(|x| x.0.instance_id == c.0.instance_id && x.0.closed_at.is_none()))
            {
                return Err(Error::Conflict);
            }
        }
        for p in &self.progress {
            self.annotation(p.0.task_id, p.0.agent_instance_id, p.0.created_at)?;
        }
        for h in &self.handovers {
            self.annotation(h.0.task_id, h.0.from_agent_instance_id, h.0.created_at)?;
            if !self.claims.iter().any(|c| {
                c.0.task_id == h.0.task_id
                    && c.0.closed_at == Some(h.0.created_at)
                    && c.0.close_reason == Some(CloseReason::Handover)
            }) {
                return Err(Error::State);
            }
        }
        Ok(())
    }
    fn annotation(
        &self,
        id: TaskId,
        instance: Option<AgentInstanceId>,
        at: Timestamp,
    ) -> Result<()> {
        at.not_before(self.task(id)?.0.created_at)?;
        self.workspace.0.updated_at.not_before(at)?;
        if let Some(instance) = instance {
            self.instance(instance)?;
            if !self.claims.iter().any(|c| {
                c.0.task_id == id
                    && c.0.instance_id == instance
                    && c.0.opened_at <= at
                    && c.0.closed_at.is_none_or(|end| at <= end)
            }) {
                return Err(Error::Unauthorized);
            }
        }
        Ok(())
    }
}
