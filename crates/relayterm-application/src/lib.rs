//! Deterministic coordination services over asynchronous storage ports.
use domain::*;
pub use relayterm_domain as domain;
use std::{future::Future, path::PathBuf};

pub trait Clock: Sync {
    fn now(&self) -> Result<Timestamp>;
}
pub trait IdGenerator: Sync {
    /// Supply a fresh opaque identity, without requiring random generation.
    fn next(&self) -> Result<EventId>;
}
pub trait Store: Sync {
    type Transaction: Transaction;
    fn begin(
        &self,
        workspace_id: WorkspaceId,
    ) -> impl Future<Output = Result<Self::Transaction>> + Send;
}
pub trait Transaction: Send {
    fn snapshot(&self) -> &Snapshot;
    /// Compare the captured revision and validate the batch before any writes.
    /// Entity changes and confirmed events must commit together or not at all.
    fn commit(self, batch: WriteBatch) -> impl Future<Output = Result<Committed>> + Send;
}
pub trait EventNotifier: Sync {
    fn notify(
        &self,
        workspace_id: WorkspaceId,
        revision: u64,
    ) -> impl Future<Output = Result<()>> + Send;
}
#[derive(Clone)]
pub struct Snapshot {
    revision: u64,
    state: Option<WorkspaceState>,
}
impl Snapshot {
    pub fn restore(revision: u64, state: Option<WorkspaceState>) -> Result<Self> {
        if (revision == 0) != state.is_none() {
            return Err(Error::State);
        }
        if let Some(s) = &state {
            s.validate()?;
        }
        Ok(Self { revision, state })
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn state(&self) -> Result<&WorkspaceState> {
        self.state.as_ref().ok_or(Error::Reference)
    }
}
enum Write {
    Create(Box<WorkspaceState>),
    Change(Box<Changes>),
}
/// Only services can construct batches, coupling entity mutations to event intent.
pub struct WriteBatch {
    expected_revision: u64,
    write: Write,
    events: Vec<PendingEvent>,
}
impl WriteBatch {
    pub fn expected_revision(&self) -> u64 {
        self.expected_revision
    }
    pub fn state(&self) -> &WorkspaceState {
        match &self.write {
            Write::Create(s) => s,
            Write::Change(c) => c.after(),
        }
    }
    pub fn events(&self) -> &[PendingEvent] {
        &self.events
    }
    /// Adapters call this against their current snapshot inside the commit lock.
    pub fn validate(&self, current: &Snapshot) -> Result<()> {
        if current.revision != self.expected_revision {
            return Err(Error::Conflict);
        }
        match &self.write {
            Write::Create(_) if current.state.is_some() => return Err(Error::Conflict),
            Write::Change(changes) if current.state.as_ref() != Some(changes.before()) => {
                return Err(Error::Conflict);
            }
            _ => {}
        }
        self.state().validate()?;
        if self.events.iter().enumerate().any(|(i, e)| {
            e.workspace_id != self.state().workspace().record().id
                || self.events[..i].iter().any(|x| x.event_id == e.event_id)
        }) {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}
pub struct Committed {
    pub snapshot: Snapshot,
    pub events: Vec<WorkspaceEvent>,
}
pub struct Outcome {
    pub committed: Committed,
    /// Notification failure does not undo or retry the committed mutation.
    pub notification_delivered: bool,
}
pub enum Request {
    AddDefinition {
        display_name: String,
        command: String,
        arguments: Vec<String>,
        environment_allowlist: Vec<String>,
        capabilities: Vec<String>,
        enabled: bool,
    },
    CreateTask(TaskContent),
    EditTask {
        id: TaskId,
        content: TaskContent,
    },
    Transition {
        id: TaskId,
        to: TaskStatus,
    },
    Claim {
        task_id: TaskId,
        instance_id: AgentInstanceId,
    },
    Release {
        task_id: TaskId,
    },
    Progress {
        task_id: TaskId,
        summary: String,
        verification: String,
    },
    Handover {
        task_id: TaskId,
        content: HandoverContent,
    },
}
/// Metadata received by the future supervisor before attempting launch.
pub struct LaunchContext {
    pub agent_definition_id: Option<AgentDefinitionId>,
    pub task_id: Option<TaskId>,
    pub working_directory: PathBuf,
    pub terminal_size: TerminalSize,
}
pub struct Service<S, C, I, N> {
    store: S,
    clock: C,
    ids: I,
    notifier: N,
}
impl<S: Store, C: Clock, I: IdGenerator, N: EventNotifier> Service<S, C, I, N> {
    pub fn new(store: S, clock: C, ids: I, notifier: N) -> Self {
        Self {
            store,
            clock,
            ids,
            notifier,
        }
    }
    pub async fn snapshot(&self, id: WorkspaceId) -> Result<Snapshot> {
        Ok(self.store.begin(id).await?.snapshot().clone())
    }
    pub async fn create_workspace(&self, name: String, root: PathBuf) -> Result<Outcome> {
        let at = self.clock.now()?;
        let id = WorkspaceId::from_uuid(self.ids.next()?.as_uuid());
        let workspace = Workspace::restore(WorkspaceRecord {
            id,
            display_name: name,
            project_root: root,
            created_at: at,
            updated_at: at,
            schema_version: 1,
        })?;
        let transaction = self.store.begin(id).await?;
        let event = PendingEvent {
            event_id: self.ids.next()?,
            workspace_id: id,
            timestamp: at,
            actor: Actor::LocalUser,
            payload: EventPayload::WorkspaceCreated { id },
        };
        let batch = WriteBatch {
            expected_revision: transaction.snapshot().revision,
            write: Write::Create(Box::new(WorkspaceState::new(workspace))),
            events: vec![event],
        };
        self.commit(transaction, batch).await
    }
    pub async fn execute(
        &self,
        workspace_id: WorkspaceId,
        actor: Actor,
        request: Request,
    ) -> Result<Outcome> {
        let command = match request {
            Request::AddDefinition {
                display_name,
                command,
                arguments,
                environment_allowlist,
                capabilities,
                enabled,
            } => Command::AddDefinition(AgentDefinition::restore(AgentDefinitionRecord {
                id: AgentDefinitionId::from_uuid(self.ids.next()?.as_uuid()),
                workspace_id,
                display_name,
                command,
                arguments,
                environment_allowlist,
                capabilities,
                enabled,
            })?),
            Request::CreateTask(content) => Command::CreateTask {
                id: TaskId::from_uuid(self.ids.next()?.as_uuid()),
                content,
            },
            Request::EditTask { id, content } => Command::EditTask { id, content },
            Request::Transition { id, to } => Command::Transition { id, to },
            Request::Claim {
                task_id,
                instance_id,
            } => Command::Claim {
                id: ClaimId::from_uuid(self.ids.next()?.as_uuid()),
                task_id,
                instance_id,
            },
            Request::Release { task_id } => Command::Release { task_id },
            Request::Progress {
                task_id,
                summary,
                verification,
            } => Command::Progress {
                id: ProgressEntryId::from_uuid(self.ids.next()?.as_uuid()),
                task_id,
                summary,
                verification,
            },
            Request::Handover { task_id, content } => Command::Handover {
                id: HandoverId::from_uuid(self.ids.next()?.as_uuid()),
                task_id,
                content,
            },
        };
        let transaction = self.store.begin(workspace_id).await?;
        let changes = transaction
            .snapshot()
            .state()?
            .execute(actor, command, self.clock.now()?)?;
        self.commit_changes(transaction, changes).await
    }
    /// Internal supervisor entry point, never exposed as a user-selected actor.
    pub async fn register_instance(
        &self,
        workspace_id: WorkspaceId,
        context: LaunchContext,
    ) -> Result<Outcome> {
        let at = self.clock.now()?;
        let instance = AgentInstance::restore(AgentInstanceRecord {
            id: AgentInstanceId::from_uuid(self.ids.next()?.as_uuid()),
            session_id: TerminalSessionId::from_uuid(self.ids.next()?.as_uuid()),
            workspace_id,
            agent_definition_id: context.agent_definition_id,
            task_id: context.task_id,
            working_directory: context.working_directory,
            status: InstanceStatus::Starting,
            started_at: at,
            last_observed_at: at,
            ended_at: None,
            exit_code: None,
            terminal_size: context.terminal_size,
        })?;
        self.observe_at(workspace_id, Observation::Register(instance), at)
            .await
    }
    pub async fn observe(
        &self,
        workspace_id: WorkspaceId,
        observation: Observation,
    ) -> Result<Outcome> {
        self.observe_at(workspace_id, observation, self.clock.now()?)
            .await
    }
    async fn observe_at(
        &self,
        workspace_id: WorkspaceId,
        observation: Observation,
        at: Timestamp,
    ) -> Result<Outcome> {
        let transaction = self.store.begin(workspace_id).await?;
        let changes = transaction.snapshot().state()?.observe(observation, at)?;
        self.commit_changes(transaction, changes).await
    }
    async fn commit_changes(
        &self,
        transaction: S::Transaction,
        changes: Changes,
    ) -> Result<Outcome> {
        let events = changes
            .payloads()
            .iter()
            .map(|payload| {
                Ok(PendingEvent {
                    event_id: self.ids.next()?,
                    workspace_id: changes.after().workspace().record().id,
                    timestamp: changes.timestamp(),
                    actor: changes.actor(),
                    payload: payload.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let batch = WriteBatch {
            expected_revision: transaction.snapshot().revision,
            write: Write::Change(Box::new(changes)),
            events,
        };
        self.commit(transaction, batch).await
    }
    async fn commit(&self, transaction: S::Transaction, batch: WriteBatch) -> Result<Outcome> {
        batch.validate(transaction.snapshot())?;
        let id = batch.state().workspace().record().id;
        let committed = transaction.commit(batch).await?;
        let delivered = committed.events.is_empty()
            || self
                .notifier
                .notify(id, committed.snapshot.revision)
                .await
                .is_ok();
        Ok(Outcome {
            committed,
            notification_delivered: delivered,
        })
    }
    pub async fn history(
        &self,
        id: WorkspaceId,
        task_id: TaskId,
        page: Page,
    ) -> Result<HistoryPage> {
        let snapshot = self.snapshot(id).await?;
        let state = snapshot.state()?;
        state.task(task_id)?;
        Ok(HistoryPage {
            revision: snapshot.revision,
            claims: page.select(
                state
                    .claims()
                    .iter()
                    .filter(|x| x.record().task_id == task_id),
            ),
            progress: page.select(
                state
                    .progress()
                    .iter()
                    .filter(|x| x.record().task_id == task_id),
            ),
            handovers: page.select(
                state
                    .handovers()
                    .iter()
                    .filter(|x| x.record().task_id == task_id),
            ),
        })
    }
}
/// Append-order offset pages. Keep a snapshot revision when comparing pages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Page {
    offset: usize,
    limit: usize,
}
impl Default for Page {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}
impl Page {
    pub fn new(offset: usize, limit: usize) -> Result<Self> {
        if limit == 0 || limit > 200 {
            return Err(Error::Validation("page_limit"));
        }
        Ok(Self { offset, limit })
    }
    fn select<'a, T: Clone + 'a>(self, values: impl Iterator<Item = &'a T>) -> Vec<T> {
        values.skip(self.offset).take(self.limit).cloned().collect()
    }
}
pub struct HistoryPage {
    pub revision: u64,
    pub claims: Vec<Claim>,
    pub progress: Vec<ProgressEntry>,
    pub handovers: Vec<Handover>,
}
