use domain::*;
use relayterm_application::*;
use std::{
    collections::HashMap,
    future::{Future, poll_fn},
    pin::pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
    thread,
};

pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::noop();
    loop {
        match future.as_mut().poll(&mut Context::from_waker(waker)) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::yield_now(),
        }
    }
}
#[derive(Clone, Default)]
pub struct Memory {
    pub inner: Arc<Mutex<Database>>,
}
#[derive(Default)]
pub struct Database {
    pub snapshots: HashMap<WorkspaceId, Snapshot>,
    pub events: HashMap<WorkspaceId, Vec<WorkspaceEvent>>,
    pub fail_before: bool,
    pub fail_validation: bool,
}
pub struct MemoryTransaction {
    memory: Memory,
    id: WorkspaceId,
    snapshot: Snapshot,
}
impl Store for Memory {
    type Transaction = MemoryTransaction;
    async fn begin(&self, id: WorkspaceId) -> Result<MemoryTransaction> {
        let db = self.inner.lock().map_err(|_| Error::Storage)?;
        if db.fail_before {
            return Err(Error::Storage);
        }
        let snapshot = db
            .snapshots
            .get(&id)
            .cloned()
            .unwrap_or(Snapshot::restore(0, None)?);
        Ok(MemoryTransaction {
            memory: self.clone(),
            id,
            snapshot,
        })
    }
}
impl Transaction for MemoryTransaction {
    fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    async fn commit(self, batch: WriteBatch) -> Result<Committed> {
        // Yield exactly once so tests can open two transactions before either commits.
        let mut yielded = false;
        poll_fn(|cx| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
        let mut db = self.memory.inner.lock().map_err(|_| Error::Storage)?;
        let current = db
            .snapshots
            .get(&self.id)
            .cloned()
            .unwrap_or(Snapshot::restore(0, None)?);
        batch.validate(&current)?;
        if db.fail_validation {
            return Err(Error::Storage);
        }
        if batch.state().workspace().record().id != self.id {
            return Err(Error::Reference);
        }
        let existing = db.events.get(&self.id).cloned().unwrap_or_default();
        let mut all = existing;
        let mut confirmed = vec![];
        for pending in batch.events() {
            if db
                .events
                .values()
                .flatten()
                .any(|e| e.record().event_id == pending.event_id)
                || confirmed
                    .iter()
                    .any(|e: &WorkspaceEvent| e.record().event_id == pending.event_id)
            {
                return Err(Error::Conflict);
            }
            let sequence = u64::try_from(all.len())
                .map_err(|_| Error::Storage)?
                .checked_add(1)
                .ok_or(Error::Storage)?;
            let event = WorkspaceEvent::confirm(pending.clone(), sequence)?;
            all.push(event.clone());
            confirmed.push(event);
        }
        let revision = if confirmed.is_empty() {
            current.revision()
        } else {
            current.revision().checked_add(1).ok_or(Error::Storage)?
        };
        let snapshot = Snapshot::restore(revision, Some(batch.state().clone()))?;
        // All fallible work precedes either write.
        db.events.insert(self.id, all);
        db.snapshots.insert(self.id, snapshot.clone());
        Ok(Committed {
            snapshot,
            events: confirmed,
        })
    }
}
#[derive(Clone)]
pub struct TestClock(pub Arc<AtomicU64>);
impl Clock for TestClock {
    fn now(&self) -> Result<Timestamp> {
        Timestamp::try_from(i128::from(self.0.load(Ordering::SeqCst)))
    }
}
#[derive(Default)]
pub struct TestIds(AtomicU64);
impl IdGenerator for TestIds {
    fn next(&self) -> Result<EventId> {
        format!(
            "00000000-0000-4000-8000-{:012x}",
            self.0.fetch_add(1, Ordering::SeqCst) + 1
        )
        .parse()
        .map_err(|_| Error::Storage)
    }
}
#[derive(Clone, Default)]
pub struct Notifier {
    pub fail: Arc<AtomicBool>,
    pub calls: Arc<Mutex<Vec<(WorkspaceId, u64)>>>,
}
impl EventNotifier for Notifier {
    async fn notify(&self, id: WorkspaceId, revision: u64) -> Result<()> {
        self.calls.lock().unwrap().push((id, revision));
        if self.fail.load(Ordering::SeqCst) {
            Err(Error::Unavailable)
        } else {
            Ok(())
        }
    }
}
pub type TestService = Service<Memory, TestClock, TestIds, Notifier>;
pub struct Fixture {
    pub service: TestService,
    pub memory: Memory,
    pub clock: TestClock,
    pub notifier: Notifier,
    pub workspace: WorkspaceId,
}
impl Fixture {
    pub fn new() -> Self {
        let memory = Memory::default();
        let clock = TestClock(Arc::new(AtomicU64::new(100)));
        let notifier = Notifier::default();
        let service = Service::new(
            memory.clone(),
            clock.clone(),
            TestIds::default(),
            notifier.clone(),
        );
        let outcome =
            block_on(service.create_workspace("Synthetic workspace".into(), "project".into()))
                .unwrap();
        let workspace = outcome
            .committed
            .snapshot
            .state()
            .unwrap()
            .workspace()
            .record()
            .id;
        Self {
            service,
            memory,
            clock,
            notifier,
            workspace,
        }
    }
    pub fn run(&self, actor: Actor, request: Request) -> Result<Outcome> {
        block_on(self.service.execute(self.workspace, actor, request))
    }
    pub fn state(&self) -> WorkspaceState {
        block_on(self.service.snapshot(self.workspace))
            .unwrap()
            .state()
            .unwrap()
            .clone()
    }
    pub fn events(&self) -> Vec<WorkspaceEvent> {
        self.memory.inner.lock().unwrap().events[&self.workspace].clone()
    }
    pub fn task(&self) -> TaskId {
        let result = self
            .run(Actor::LocalUser, Request::CreateTask(content()))
            .unwrap();
        result
            .committed
            .snapshot
            .state()
            .unwrap()
            .tasks()
            .last()
            .unwrap()
            .record()
            .id
    }
    pub fn ready(&self, id: TaskId) {
        self.run(
            Actor::LocalUser,
            Request::Transition {
                id,
                to: TaskStatus::Ready,
            },
        )
        .unwrap();
    }
    pub fn instance(&self, definition: Option<AgentDefinitionId>) -> AgentInstanceId {
        let result = block_on(self.service.register_instance(
            self.workspace,
            LaunchContext {
                agent_definition_id: definition,
                task_id: None,
                working_directory: "project".into(),
                terminal_size: TerminalSize::new(24, 80).unwrap(),
            },
        ))
        .unwrap();
        let id = result
            .committed
            .snapshot
            .state()
            .unwrap()
            .instances()
            .last()
            .unwrap()
            .record()
            .id;
        self.observe(id, InstanceStatus::Running, None).unwrap();
        id
    }
    pub fn observe(
        &self,
        id: AgentInstanceId,
        status: InstanceStatus,
        exit_code: Option<i32>,
    ) -> Result<Outcome> {
        block_on(self.service.observe(
            self.workspace,
            Observation::Status {
                id,
                status,
                observed_at: self.clock.now()?,
                exit_code,
            },
        ))
    }
    pub fn claim(&self, task_id: TaskId, instance_id: AgentInstanceId) -> Result<Outcome> {
        self.run(
            Actor::Instance(instance_id),
            Request::Claim {
                task_id,
                instance_id,
            },
        )
    }
}
pub fn content() -> TaskContent {
    TaskContent {
        title: "Synthetic task".into(),
        description: "Private synthetic narrative".into(),
        priority: Priority::Normal,
        scope_paths: vec!["src/lib.rs".into()],
        acceptance_notes: "Tests pass".into(),
        dependency_ids: vec![],
    }
}
pub fn handover() -> HandoverContent {
    HandoverContent {
        summary: "Continue implementation".into(),
        decisions: "Keep the core neutral".into(),
        changed_paths: vec!["src/lib.rs".into()],
        verification_performed: "Unit tests passed".into(),
        open_questions: "".into(),
        recommended_next_action: "Review the tests".into(),
    }
}
