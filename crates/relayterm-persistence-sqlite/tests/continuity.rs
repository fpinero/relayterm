use relayterm_application::{
    Clock, DurableReadStore, EventNotifier, EventPageRequest, IdGenerator, LaunchContext, Request,
    Service, Store, TaskHistoryItem, TaskHistoryPageRequest, Transaction,
};
use relayterm_domain::*;
use relayterm_persistence_sqlite::{
    Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore, SqliteTransaction,
    initialize_workspace,
};
use relayterm_platform::{LocationOptions, PrivateLocations};
use std::{
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone)]
struct TestClock(Arc<AtomicU64>);
impl Clock for TestClock {
    fn now(&self) -> Result<Timestamp> {
        Timestamp::try_from(i128::from(self.0.fetch_add(1, Ordering::SeqCst) + 1))
    }
}

struct TestIds(AtomicU64);
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

#[derive(Clone, Copy)]
struct NoopNotifier;
impl EventNotifier for NoopNotifier {
    async fn notify(&self, _: WorkspaceId, _: u64) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct SynchronizedStore {
    inner: SqliteStore,
    barrier: Arc<tokio::sync::Barrier>,
}

impl Store for SynchronizedStore {
    type Transaction = SqliteTransaction;

    async fn begin(&self, workspace_id: WorkspaceId) -> Result<Self::Transaction> {
        let transaction = self.inner.begin(workspace_id).await?;
        self.barrier.wait().await;
        Ok(transaction)
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

fn task_content(title: &str) -> TaskContent {
    TaskContent {
        title: title.into(),
        description: "Durable work".into(),
        priority: Priority::High,
        scope_paths: vec!["src/lib.rs".into()],
        acceptance_notes: "Verified".into(),
        dependency_ids: Vec::new(),
    }
}

#[test]
fn complete_handover_journey_survives_reopen() {
    runtime().block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let private = temporary.path().join("private");
        let project = temporary.path().join("project");
        std::fs::create_dir(&project).unwrap();
        relayterm_platform::create_private_dir(&private).unwrap();
        let path = private.join("workspace.sqlite3");
        let database = Database::open(
            &path,
            DatabaseKind::Workspace,
            OpenMode::ExplicitNew,
            PoolSettings::default(),
        )
        .await
        .unwrap();
        let store = SqliteStore::new(database.pool().clone());
        let workspace_id: WorkspaceId = "10000000-0000-4000-8000-000000000001".parse().unwrap();
        let service = Service::new(
            store.clone(),
            TestClock(Arc::new(AtomicU64::new(0))),
            TestIds(AtomicU64::new(100)),
            NoopNotifier,
        );
        service
            .create_workspace_reserved(workspace_id, "Durable workspace".into(), project.clone())
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::AddDefinition {
                    display_name: "Neutral agent".into(),
                    command: "agent".into(),
                    arguments: vec!["--mode".into()],
                    environment_allowlist: vec!["PATH".into()],
                    capabilities: vec!["terminal".into()],
                    enabled: true,
                },
            )
            .await
            .unwrap();
        let definition_id = service
            .snapshot(workspace_id)
            .await
            .unwrap()
            .state()
            .unwrap()
            .definitions()[0]
            .record()
            .id;
        service
            .register_instance(
                workspace_id,
                LaunchContext {
                    agent_definition_id: Some(definition_id),
                    task_id: None,
                    working_directory: project.clone(),
                    terminal_size: TerminalSize::new(24, 80).unwrap(),
                },
            )
            .await
            .unwrap();
        let first = service
            .snapshot(workspace_id)
            .await
            .unwrap()
            .state()
            .unwrap()
            .instances()[0]
            .record()
            .id;
        service
            .observe(
                workspace_id,
                Observation::Status {
                    id: first,
                    status: InstanceStatus::Running,
                    observed_at: Timestamp::try_from(4).unwrap(),
                    exit_code: None,
                },
            )
            .await
            .unwrap();
        let import_revision = service.snapshot(workspace_id).await.unwrap().revision();
        service
            .import_definitions(
                workspace_id,
                import_revision,
                vec![
                    AgentDefinition::restore(AgentDefinitionRecord {
                        id: definition_id,
                        workspace_id,
                        display_name: "Neutral agent v2".into(),
                        command: "agent-v2".into(),
                        arguments: vec!["--mode".into()],
                        environment_allowlist: vec!["PATH".into()],
                        capabilities: vec!["terminal".into()],
                        enabled: true,
                    })
                    .unwrap(),
                ],
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::CreateTask(task_content("Persist state")),
            )
            .await
            .unwrap();
        let task_id = service
            .snapshot(workspace_id)
            .await
            .unwrap()
            .state()
            .unwrap()
            .tasks()[0]
            .record()
            .id;
        service
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::Transition {
                    id: task_id,
                    to: TaskStatus::Ready,
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::Instance(first),
                Request::Claim {
                    task_id,
                    instance_id: first,
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::Instance(first),
                Request::Progress {
                    task_id,
                    summary: "Stored progress".into(),
                    verification: "cargo test".into(),
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::Instance(first),
                Request::Handover {
                    task_id,
                    content: HandoverContent {
                        summary: "Continue persistence".into(),
                        decisions: "Keep transactions atomic".into(),
                        changed_paths: vec!["src/lib.rs".into()],
                        verification_performed: "cargo test".into(),
                        open_questions: String::new(),
                        recommended_next_action: "Reopen and complete".into(),
                    },
                },
            )
            .await
            .unwrap();
        service
            .register_instance(
                workspace_id,
                LaunchContext {
                    agent_definition_id: Some(definition_id),
                    task_id: Some(task_id),
                    working_directory: project,
                    terminal_size: TerminalSize::new(30, 100).unwrap(),
                },
            )
            .await
            .unwrap();
        let second = service
            .snapshot(workspace_id)
            .await
            .unwrap()
            .state()
            .unwrap()
            .instances()[1]
            .record()
            .id;
        service
            .observe(
                workspace_id,
                Observation::Status {
                    id: second,
                    status: InstanceStatus::Running,
                    observed_at: Timestamp::try_from(12).unwrap(),
                    exit_code: None,
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::Instance(second),
                Request::Claim {
                    task_id,
                    instance_id: second,
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::Instance(second),
                Request::Transition {
                    id: task_id,
                    to: TaskStatus::Done,
                },
            )
            .await
            .unwrap();
        let ended = service
            .observe(
                workspace_id,
                Observation::Status {
                    id: second,
                    status: InstanceStatus::Exited,
                    observed_at: Timestamp::try_from(15).unwrap(),
                    exit_code: Some(0),
                },
            )
            .await
            .unwrap();
        let ended_revision = ended.committed.snapshot.revision();
        let repeated = service
            .observe(
                workspace_id,
                Observation::Status {
                    id: second,
                    status: InstanceStatus::Exited,
                    observed_at: Timestamp::try_from(15).unwrap(),
                    exit_code: Some(0),
                },
            )
            .await
            .unwrap();
        assert_eq!(repeated.committed.snapshot.revision(), ended_revision);
        assert!(repeated.committed.events.is_empty());
        let before = service.snapshot(workspace_id).await.unwrap();
        assert_eq!(before.state().unwrap().progress().len(), 1);
        assert_eq!(before.state().unwrap().handovers().len(), 1);
        let revision = before.revision();
        let watermarked = store.consistent_snapshot(workspace_id).await.unwrap();
        assert_eq!(watermarked.snapshot.revision(), revision);
        let history = store
            .task_history_page(
                workspace_id,
                task_id,
                TaskHistoryPageRequest::new(0, 2, revision).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(history.entries.len(), 2);
        assert!(matches!(history.entries[0].item, TaskHistoryItem::Claim(_)));
        let history_tail = store
            .task_history_page(
                workspace_id,
                task_id,
                TaskHistoryPageRequest::new(history.entries[1].sequence, 200, history.revision)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!history_tail.entries.is_empty());
        assert!(
            history_tail
                .entries
                .iter()
                .all(|entry| entry.sequence > history.entries[1].sequence)
        );
        assert_eq!(
            store
                .task_history_page(
                    workspace_id,
                    task_id,
                    TaskHistoryPageRequest::new(0, 50, revision - 1).unwrap(),
                )
                .await
                .err(),
            Some(Error::Conflict)
        );
        let first_page = store
            .event_page(workspace_id, EventPageRequest::new(0, 5).unwrap())
            .await
            .unwrap();
        assert_eq!(first_page.events.len(), 5);
        assert!(
            first_page
                .events
                .windows(2)
                .all(|pair| pair[0].record().sequence < pair[1].record().sequence)
        );
        assert_eq!(
            store
                .event_page(
                    workspace_id,
                    EventPageRequest::new(first_page.last_sequence + 1, 5).unwrap()
                )
                .await
                .err(),
            Some(Error::InvalidCursor)
        );
        sqlx::query("UPDATE workspace_meta SET retained_from_sequence=? WHERE workspace_id=?")
            .bind(relayterm_persistence_sqlite::encode_counter(2).to_vec())
            .bind(workspace_id.as_uuid().as_bytes().to_vec())
            .execute(store.pool())
            .await
            .unwrap();
        assert_eq!(
            store
                .event_page(workspace_id, EventPageRequest::new(0, 5).unwrap())
                .await
                .err(),
            Some(Error::ResnapshotRequired)
        );
        sqlx::query("UPDATE workspace_meta SET retained_from_sequence=? WHERE workspace_id=?")
            .bind(relayterm_persistence_sqlite::encode_counter(1).to_vec())
            .bind(workspace_id.as_uuid().as_bytes().to_vec())
            .execute(store.pool())
            .await
            .unwrap();
        drop(service);
        database.pool().close().await;

        let reopened = Database::open(
            &path,
            DatabaseKind::Workspace,
            OpenMode::Reopen,
            PoolSettings::default(),
        )
        .await
        .unwrap();
        let transaction = SqliteStore::new(reopened.pool().clone())
            .begin(workspace_id)
            .await
            .unwrap();
        assert_eq!(transaction.snapshot().revision(), revision);
        let state = transaction.snapshot().state().unwrap();
        assert_eq!(
            state.task(task_id).unwrap().record().status,
            TaskStatus::Done
        );
        assert_eq!(state.claims().len(), 2);
        assert!(
            state
                .claims()
                .iter()
                .all(|claim| claim.record().closed_at.is_some())
        );
        assert_eq!(state.progress()[0].record().summary, "Stored progress");
        assert_eq!(
            state.handovers()[0]
                .record()
                .content
                .recommended_next_action,
            "Reopen and complete"
        );
        assert_eq!(
            state.instances()[0]
                .record()
                .launch_definition
                .as_ref()
                .unwrap()
                .command,
            "agent"
        );
        assert_eq!(
            state.instances()[1]
                .record()
                .launch_definition
                .as_ref()
                .unwrap()
                .command,
            "agent-v2"
        );
    });
}

#[test]
fn repeated_initialization_and_aliases_converge_on_one_workspace() {
    runtime().block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let private_home = temporary.path().join("relayterm-home");
        let project = temporary.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let locations = PrivateLocations::resolve(LocationOptions {
            explicit_home: Some(private_home.clone()),
            relayterm_home: None,
        })
        .unwrap();
        let first = initialize_workspace(
            &locations,
            &project,
            "Registered workspace".into(),
            PoolSettings::default(),
            TestClock(Arc::new(AtomicU64::new(0))),
            TestIds(AtomicU64::new(400)),
            NoopNotifier,
        )
        .await
        .unwrap();
        let first_id = first.workspace_id;
        first.database.pool().close().await;
        let second = initialize_workspace(
            &locations,
            &project.join("."),
            "Ignored on reopen".into(),
            PoolSettings::default(),
            TestClock(Arc::new(AtomicU64::new(100))),
            TestIds(AtomicU64::new(900)),
            NoopNotifier,
        )
        .await
        .unwrap();
        assert_eq!(second.workspace_id, first_id);
        let event_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workspace_events WHERE event_type='workspace_created'",
        )
        .fetch_one(second.database.pool())
        .await
        .unwrap();
        assert_eq!(event_count, 1);
        assert!(
            sqlx::query("UPDATE workspace_events SET event_type='changed'")
                .execute(second.database.pool())
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM workspace_events")
                .execute(second.database.pool())
                .await
                .is_err()
        );
        assert!(!project.join(".relayterm").exists());
        assert!(private_home.join("data/registry.sqlite3").exists());
    });
}

#[test]
fn independent_clients_race_one_claim_without_partial_events() {
    runtime().block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let private = temporary.path().join("private");
        let project = temporary.path().join("project");
        std::fs::create_dir(&project).unwrap();
        relayterm_platform::create_private_dir(&private).unwrap();
        let path = private.join("workspace.sqlite3");
        let database = Database::open(
            &path,
            DatabaseKind::Workspace,
            OpenMode::ExplicitNew,
            PoolSettings::default(),
        )
        .await
        .unwrap();
        let store = SqliteStore::new(database.pool().clone());
        let workspace_id: WorkspaceId = "20000000-0000-4000-8000-000000000001".parse().unwrap();
        let shared_clock = Arc::new(AtomicU64::new(0));
        let setup = Service::new(
            store.clone(),
            TestClock(shared_clock.clone()),
            TestIds(AtomicU64::new(1_000)),
            NoopNotifier,
        );
        setup
            .create_workspace_reserved(workspace_id, "Race workspace".into(), project.clone())
            .await
            .unwrap();
        for _ in 0..2 {
            setup
                .register_instance(
                    workspace_id,
                    LaunchContext {
                        agent_definition_id: None,
                        task_id: None,
                        working_directory: project.clone(),
                        terminal_size: TerminalSize::new(24, 80).unwrap(),
                    },
                )
                .await
                .unwrap();
            let instance = setup
                .snapshot(workspace_id)
                .await
                .unwrap()
                .state()
                .unwrap()
                .instances()
                .last()
                .unwrap()
                .record()
                .id;
            let observed_at =
                Timestamp::try_from(i128::from(shared_clock.load(Ordering::SeqCst) + 1)).unwrap();
            setup
                .observe(
                    workspace_id,
                    Observation::Status {
                        id: instance,
                        status: InstanceStatus::Running,
                        observed_at,
                        exit_code: None,
                    },
                )
                .await
                .unwrap();
        }
        setup
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::CreateTask(task_content("Race claim")),
            )
            .await
            .unwrap();
        let snapshot = setup.snapshot(workspace_id).await.unwrap();
        let state = snapshot.state().unwrap();
        let task_id = state.tasks()[0].record().id;
        let first = state.instances()[0].record().id;
        let second = state.instances()[1].record().id;
        setup
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::Transition {
                    id: task_id,
                    to: TaskStatus::Ready,
                },
            )
            .await
            .unwrap();
        drop(setup);

        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let client_a = Service::new(
            SynchronizedStore {
                inner: store.clone(),
                barrier: barrier.clone(),
            },
            TestClock(shared_clock.clone()),
            TestIds(AtomicU64::new(2_000)),
            NoopNotifier,
        );
        let client_b = Service::new(
            SynchronizedStore {
                inner: store.clone(),
                barrier,
            },
            TestClock(shared_clock),
            TestIds(AtomicU64::new(3_000)),
            NoopNotifier,
        );
        let (left, right) = tokio::join!(
            client_a.execute(
                workspace_id,
                Actor::Instance(first),
                Request::Claim {
                    task_id,
                    instance_id: first
                }
            ),
            client_b.execute(
                workspace_id,
                Actor::Instance(second),
                Request::Claim {
                    task_id,
                    instance_id: second
                }
            ),
        );
        assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
        let rejected = if left.is_err() {
            left.err()
        } else {
            right.err()
        };
        assert_eq!(rejected, Some(Error::Conflict));
        let transaction = store.begin(workspace_id).await.unwrap();
        assert_eq!(transaction.snapshot().state().unwrap().claims().len(), 1);
        let claim_events: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workspace_events WHERE event_type='claim_opened'",
        )
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(claim_events, 1);
    });
}

#[test]
fn lost_instance_blocks_task_and_closes_claim_once_after_reopen() {
    runtime().block_on(async {
        let temporary = tempfile::tempdir().unwrap();
        let private = temporary.path().join("private");
        let project = temporary.path().join("project");
        std::fs::create_dir(&project).unwrap();
        relayterm_platform::create_private_dir(&private).unwrap();
        let database = Database::open(
            &private.join("workspace.sqlite3"),
            DatabaseKind::Workspace,
            OpenMode::ExplicitNew,
            PoolSettings::default(),
        )
        .await
        .unwrap();
        let store = SqliteStore::new(database.pool().clone());
        let workspace_id: WorkspaceId = "40000000-0000-4000-8000-000000000001".parse().unwrap();
        let service = Service::new(
            store.clone(),
            TestClock(Arc::new(AtomicU64::new(0))),
            TestIds(AtomicU64::new(4_000)),
            NoopNotifier,
        );
        service
            .create_workspace_reserved(workspace_id, "Recovery workspace".into(), project.clone())
            .await
            .unwrap();
        service
            .register_instance(
                workspace_id,
                LaunchContext {
                    agent_definition_id: None,
                    task_id: None,
                    working_directory: project,
                    terminal_size: TerminalSize::new(24, 80).unwrap(),
                },
            )
            .await
            .unwrap();
        let instance = service
            .snapshot(workspace_id)
            .await
            .unwrap()
            .state()
            .unwrap()
            .instances()[0]
            .record()
            .id;
        service
            .observe(
                workspace_id,
                Observation::Status {
                    id: instance,
                    status: InstanceStatus::Running,
                    observed_at: Timestamp::try_from(3).unwrap(),
                    exit_code: None,
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::CreateTask(task_content("Recover claim")),
            )
            .await
            .unwrap();
        let task = service
            .snapshot(workspace_id)
            .await
            .unwrap()
            .state()
            .unwrap()
            .tasks()[0]
            .record()
            .id;
        service
            .execute(
                workspace_id,
                Actor::LocalUser,
                Request::Transition {
                    id: task,
                    to: TaskStatus::Ready,
                },
            )
            .await
            .unwrap();
        service
            .execute(
                workspace_id,
                Actor::Instance(instance),
                Request::Claim {
                    task_id: task,
                    instance_id: instance,
                },
            )
            .await
            .unwrap();
        let lost = service
            .observe(
                workspace_id,
                Observation::Status {
                    id: instance,
                    status: InstanceStatus::Lost,
                    observed_at: Timestamp::try_from(7).unwrap(),
                    exit_code: None,
                },
            )
            .await
            .unwrap();
        let revision = lost.committed.snapshot.revision();
        let repeated = service
            .observe(
                workspace_id,
                Observation::Status {
                    id: instance,
                    status: InstanceStatus::Lost,
                    observed_at: Timestamp::try_from(7).unwrap(),
                    exit_code: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(repeated.committed.snapshot.revision(), revision);
        let transaction = store.begin(workspace_id).await.unwrap();
        let state = transaction.snapshot().state().unwrap();
        assert_eq!(
            state.task(task).unwrap().record().status,
            TaskStatus::Blocked
        );
        assert_eq!(
            state.claims()[0].record().close_reason,
            Some(CloseReason::InstanceEnd)
        );
        assert_eq!(state.instances()[0].record().status, InstanceStatus::Lost);
    });
}

#[test]
fn separate_processes_write_and_reopen_the_same_state() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().join("private home");
    let project = temporary.path().join("project unicode ñ");
    std::fs::create_dir(&project).unwrap();
    for mode in ["write", "read"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "process_worker", "--ignored"])
            .env("RELAYTERM_TEST_PROCESS_MODE", mode)
            .env("RELAYTERM_TEST_HOME", &home)
            .env("RELAYTERM_TEST_PROJECT", &project)
            .output()
            .unwrap();
        let diagnostics = [output.stdout, output.stderr].concat();
        assert!(output.status.success(), "worker failed");
        assert!(!String::from_utf8_lossy(&diagnostics).contains("SYNTHETIC_SECRET_MARKER"));
    }
}

#[test]
#[ignore = "invoked by separate_processes_write_and_reopen_the_same_state"]
fn process_worker() {
    let Some(mode) = std::env::var_os("RELAYTERM_TEST_PROCESS_MODE") else {
        return;
    };
    let home = std::path::PathBuf::from(std::env::var_os("RELAYTERM_TEST_HOME").unwrap());
    let project = std::path::PathBuf::from(std::env::var_os("RELAYTERM_TEST_PROJECT").unwrap());
    runtime().block_on(async {
        let locations = PrivateLocations::resolve(LocationOptions {
            explicit_home: Some(home),
            relayterm_home: None,
        })
        .unwrap();
        let initialized = initialize_workspace(
            &locations,
            &project,
            "Process workspace".into(),
            PoolSettings::default(),
            TestClock(Arc::new(AtomicU64::new(100))),
            TestIds(AtomicU64::new(5_000)),
            NoopNotifier,
        )
        .await
        .unwrap();
        let store = SqliteStore::new(initialized.database.pool().clone());
        if mode == "write" {
            let service = Service::new(
                store,
                TestClock(Arc::new(AtomicU64::new(200))),
                TestIds(AtomicU64::new(6_000)),
                NoopNotifier,
            );
            service
                .execute(
                    initialized.workspace_id,
                    Actor::LocalUser,
                    Request::AddDefinition {
                        display_name: "Process agent".into(),
                        command: "synthetic-agent".into(),
                        arguments: Vec::new(),
                        environment_allowlist: Vec::new(),
                        capabilities: vec!["terminal".into()],
                        enabled: true,
                    },
                )
                .await
                .unwrap();
            let definition_id = service
                .snapshot(initialized.workspace_id)
                .await
                .unwrap()
                .state()
                .unwrap()
                .definitions()[0]
                .record()
                .id;
            service
                .register_instance(
                    initialized.workspace_id,
                    LaunchContext {
                        agent_definition_id: Some(definition_id),
                        task_id: None,
                        working_directory: project.clone(),
                        terminal_size: TerminalSize::new(24, 80).unwrap(),
                    },
                )
                .await
                .unwrap();
            let first = service
                .snapshot(initialized.workspace_id)
                .await
                .unwrap()
                .state()
                .unwrap()
                .instances()[0]
                .record()
                .id;
            service
                .observe(
                    initialized.workspace_id,
                    Observation::Status {
                        id: first,
                        status: InstanceStatus::Running,
                        observed_at: Timestamp::try_from(203).unwrap(),
                        exit_code: None,
                    },
                )
                .await
                .unwrap();
            service
                .execute(
                    initialized.workspace_id,
                    Actor::LocalUser,
                    Request::CreateTask(task_content("Process durable task")),
                )
                .await
                .unwrap();
            let task_id = service
                .snapshot(initialized.workspace_id)
                .await
                .unwrap()
                .state()
                .unwrap()
                .tasks()[0]
                .record()
                .id;
            service
                .execute(
                    initialized.workspace_id,
                    Actor::LocalUser,
                    Request::Transition {
                        id: task_id,
                        to: TaskStatus::Ready,
                    },
                )
                .await
                .unwrap();
            service
                .execute(
                    initialized.workspace_id,
                    Actor::Instance(first),
                    Request::Claim {
                        task_id,
                        instance_id: first,
                    },
                )
                .await
                .unwrap();
            service
                .execute(
                    initialized.workspace_id,
                    Actor::Instance(first),
                    Request::Progress {
                        task_id,
                        summary: "Process progress".into(),
                        verification: "Process verification".into(),
                    },
                )
                .await
                .unwrap();
            service
                .execute(
                    initialized.workspace_id,
                    Actor::Instance(first),
                    Request::Handover {
                        task_id,
                        content: HandoverContent {
                            summary: "Process handover".into(),
                            decisions: "Use durable context".into(),
                            changed_paths: vec!["src/lib.rs".into()],
                            verification_performed: "Process verification".into(),
                            open_questions: String::new(),
                            recommended_next_action: "Resume in another process".into(),
                        },
                    },
                )
                .await
                .unwrap();
        } else {
            let service = Service::new(
                store,
                TestClock(Arc::new(AtomicU64::new(300))),
                TestIds(AtomicU64::new(7_000)),
                NoopNotifier,
            );
            let snapshot = service.snapshot(initialized.workspace_id).await.unwrap();
            let state = snapshot.state().unwrap();
            let task_id = state.tasks()[0].record().id;
            let definition_id = state.definitions()[0].record().id;
            assert_eq!(
                state.tasks()[0].record().content.title,
                "Process durable task"
            );
            assert_eq!(state.tasks()[0].record().status, TaskStatus::HandoverReady);
            assert_eq!(
                state.handovers()[0]
                    .record()
                    .content
                    .recommended_next_action,
                "Resume in another process"
            );
            service
                .register_instance(
                    initialized.workspace_id,
                    LaunchContext {
                        agent_definition_id: Some(definition_id),
                        task_id: Some(task_id),
                        working_directory: project,
                        terminal_size: TerminalSize::new(30, 100).unwrap(),
                    },
                )
                .await
                .unwrap();
            let second = service
                .snapshot(initialized.workspace_id)
                .await
                .unwrap()
                .state()
                .unwrap()
                .instances()[1]
                .record()
                .id;
            service
                .observe(
                    initialized.workspace_id,
                    Observation::Status {
                        id: second,
                        status: InstanceStatus::Running,
                        observed_at: Timestamp::try_from(302).unwrap(),
                        exit_code: None,
                    },
                )
                .await
                .unwrap();
            service
                .execute(
                    initialized.workspace_id,
                    Actor::Instance(second),
                    Request::Claim {
                        task_id,
                        instance_id: second,
                    },
                )
                .await
                .unwrap();
            service
                .execute(
                    initialized.workspace_id,
                    Actor::Instance(second),
                    Request::Transition {
                        id: task_id,
                        to: TaskStatus::Done,
                    },
                )
                .await
                .unwrap();
            let final_snapshot = service.snapshot(initialized.workspace_id).await.unwrap();
            assert_eq!(
                final_snapshot
                    .state()
                    .unwrap()
                    .task(task_id)
                    .unwrap()
                    .record()
                    .status,
                TaskStatus::Done
            );
        }
    });
}
