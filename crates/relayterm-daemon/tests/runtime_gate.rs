use relayterm_application::{
    Clock, EventNotifier, LaunchContext, Request, Service, Store, Transaction,
};
use relayterm_client::Client;
use relayterm_daemon::{
    DaemonControl, RuntimeError, WorkspaceServer, initialize, prepare_workspace, serve_workspace,
};
use relayterm_domain::{
    Actor, InstanceStatus, Observation, Priority, TaskContent, TaskStatus, TerminalSize,
    WorkspaceId,
};
use relayterm_ipc::Endpoint;
use relayterm_persistence_sqlite::{Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore};
use relayterm_platform::{RandomIdGenerator, SystemClock};
use relayterm_protocol::{Operation, WorkspaceId as WireWorkspaceId};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone, Copy)]
struct Noop;
impl EventNotifier for Noop {
    async fn notify(&self, _: WorkspaceId, _: u64) -> relayterm_domain::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
fn short_temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("rt5-")
        .tempdir_in(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        })
        .unwrap()
}

#[tokio::test]
async fn runtime_ownership_precedes_recovery_and_rejects_a_contender() {
    let temporary = short_temp();
    let root = temporary.path().join("project");
    let home = temporary.path().join("private");
    std::fs::create_dir(&root).unwrap();
    let (route, locations) = initialize(&root, Some(home), "Ownership fixture".into())
        .await
        .unwrap();
    let workspace: WorkspaceId = route.workspace_id.parse().unwrap();
    let prepared = prepare_workspace(&root, Some(temporary.path().join("private")), workspace)
        .await
        .unwrap();
    let service = prepared.service();
    service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::CreateTask(TaskContent {
                title: "Owned task".into(),
                description: String::new(),
                priority: Priority::Normal,
                scope_paths: vec![],
                acceptance_notes: String::new(),
                dependency_ids: vec![],
            }),
        )
        .await
        .unwrap();
    let transaction = SqliteStore::new(
        Database::open(
            &locations
                .data()
                .join("workspaces")
                .join(workspace.to_string())
                .join("workspace.sqlite3"),
            DatabaseKind::Workspace,
            OpenMode::Reopen,
            PoolSettings::default(),
        )
        .await
        .unwrap()
        .pool()
        .clone(),
    )
    .begin(workspace)
    .await
    .unwrap();
    let task = transaction.snapshot().state().unwrap().tasks()[0]
        .record()
        .id;
    drop(transaction);
    service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::Transition {
                id: task,
                to: TaskStatus::Ready,
            },
        )
        .await
        .unwrap();
    let registered = service
        .register_instance(
            workspace,
            LaunchContext {
                agent_definition_id: None,
                task_id: Some(task),
                working_directory: root.clone(),
                terminal_size: TerminalSize::new(24, 80).unwrap(),
            },
        )
        .await
        .unwrap();
    let owner = registered.committed.snapshot.state().unwrap().instances()[0]
        .record()
        .id;
    service
        .register_instance(
            workspace,
            LaunchContext {
                agent_definition_id: None,
                task_id: None,
                working_directory: root.clone(),
                terminal_size: TerminalSize::new(24, 80).unwrap(),
            },
        )
        .await
        .unwrap();
    service
        .observe(
            workspace,
            Observation::Status {
                id: owner,
                status: InstanceStatus::Running,
                observed_at: SystemClock.now().unwrap(),
                exit_code: None,
            },
        )
        .await
        .unwrap();
    service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::Claim {
                task_id: task,
                instance_id: owner,
            },
        )
        .await
        .unwrap();
    let before = service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::Progress {
                task_id: task,
                summary: "Ownership established".into(),
                verification: "Deterministic contender gate".into(),
            },
        )
        .await
        .unwrap()
        .committed
        .snapshot
        .revision();
    assert!(matches!(
        prepare_workspace(&root, Some(temporary.path().join("private")), workspace).await,
        Err(RuntimeError::Busy)
    ));
    let endpoint = Endpoint::derive(
        locations.runtime(),
        WireWorkspaceId::from_uuid(workspace.as_uuid()),
    )
    .unwrap();
    let server = tokio::spawn(prepared.run());
    let client = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let status: Value = client
        .call(Operation::DaemonStatus, &json!({}))
        .await
        .unwrap();
    assert_eq!(status["revision"], before.to_string());
    let task_state: Value = client
        .call(Operation::TaskGet, &json!({"task_id":task.to_string()}))
        .await
        .unwrap();
    assert_eq!(task_state["status"], "active");
    let generation = status["generation"].as_str().unwrap();
    let _: Value = client
        .call(Operation::DaemonShutdown, &json!({"generation":generation}))
        .await
        .unwrap();
    server.await.unwrap().unwrap();
}

#[cfg(not(unix))]
fn short_temp() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("rt5-").tempdir().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_reconciles_active_claim_once_before_readiness() {
    let temporary = short_temp();
    let root = temporary.path().join("project");
    let home = temporary.path().join("private");
    std::fs::create_dir(&root).unwrap();
    let (route, locations) = initialize(&root, Some(home.clone()), "Recovery fixture".into())
        .await
        .unwrap();
    let workspace: WorkspaceId = route.workspace_id.parse().unwrap();
    let database_path = locations
        .data()
        .join("workspaces")
        .join(workspace.to_string())
        .join("workspace.sqlite3");
    let database = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(database.pool().clone());
    let service = Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Noop,
    );
    service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::CreateTask(TaskContent {
                title: "Recovery task".into(),
                description: String::new(),
                priority: Priority::Normal,
                scope_paths: vec![],
                acceptance_notes: String::new(),
                dependency_ids: vec![],
            }),
        )
        .await
        .unwrap();
    let snapshot = store.begin(workspace).await.unwrap();
    let task = snapshot.snapshot().state().unwrap().tasks()[0].record().id;
    service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::Transition {
                id: task,
                to: TaskStatus::Ready,
            },
        )
        .await
        .unwrap();
    service
        .register_instance(
            workspace,
            LaunchContext {
                agent_definition_id: None,
                task_id: Some(task),
                working_directory: root.clone(),
                terminal_size: TerminalSize::new(24, 80).unwrap(),
            },
        )
        .await
        .unwrap();
    let snapshot = store.begin(workspace).await.unwrap();
    let instance = snapshot.snapshot().state().unwrap().instances()[0]
        .record()
        .id;
    service
        .observe(
            workspace,
            Observation::Status {
                id: instance,
                status: InstanceStatus::Running,
                observed_at: SystemClock.now().unwrap(),
                exit_code: None,
            },
        )
        .await
        .unwrap();
    service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::Claim {
                task_id: task,
                instance_id: instance,
            },
        )
        .await
        .unwrap();
    drop(service);
    drop(store);
    drop(database);

    let serving_root = root.clone();
    let serving_home = home.clone();
    let server =
        tokio::spawn(
            async move { serve_workspace(&serving_root, Some(serving_home), workspace).await },
        );
    let endpoint = Endpoint::derive(
        locations.runtime(),
        WireWorkspaceId::from_uuid(workspace.as_uuid()),
    )
    .unwrap();
    let client = loop {
        if let Ok(client) =
            Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid())).await
        {
            break client;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    let status: Value = client
        .call(Operation::DaemonStatus, &json!({}))
        .await
        .unwrap();
    let wrong_generation: Result<Value, _> = client
        .call(
            Operation::DaemonShutdown,
            &json!({"generation":"00000000-0000-4000-8000-000000000599"}),
        )
        .await;
    assert!(matches!(
        wrong_generation,
        Err(relayterm_client::ClientError::Rejected(_))
    ));
    let revision = status["revision"].as_str().unwrap();
    let imported: Value = client
        .call(
            Operation::AgentImportDefinitions,
            &json!({
                "expected_revision": revision,
                "document": "format_version = 1\n[[definitions]]\nid = \"00000000-0000-4000-8000-000000000503\"\ndisplay_name = \"Synthetic agent\"\ncommand = \"synthetic-agent\"\narguments = []\nenvironment_allowlist = [\"PATH\"]\ncapabilities = [\"terminal\"]\nenabled = false\n"
            }),
        )
        .await
        .unwrap();
    assert_eq!(imported["warnings"], json!([]));
    let stale: Result<Value, _> = client
        .call(
            Operation::AgentImportDefinitions,
            &json!({
                "expected_revision": revision,
                "document": "format_version = 1\n[[definitions]]\nid = \"00000000-0000-4000-8000-000000000503\"\ndisplay_name = \"Synthetic agent\"\ncommand = \"synthetic-agent\"\narguments = []\nenvironment_allowlist = [\"PATH\"]\ncapabilities = [\"terminal\"]\nenabled = false\n"
            }),
        )
        .await;
    assert!(matches!(
        stale,
        Err(relayterm_client::ClientError::Rejected(_))
    ));
    let generation = status["generation"].as_str().unwrap();
    let _: Value = client
        .call(Operation::DaemonShutdown, &json!({"generation":generation}))
        .await
        .unwrap();
    server.await.unwrap().unwrap();

    let reopened = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(reopened.pool().clone());
    let snapshot = store.begin(workspace).await.unwrap();
    let state = snapshot.snapshot().state().unwrap();
    assert_eq!(state.instances()[0].record().status, InstanceStatus::Lost);
    assert_eq!(state.tasks()[0].record().status, TaskStatus::Blocked);
    assert!(state.claims()[0].record().closed_at.is_some());
    let revision = snapshot.snapshot().revision();
    assert_eq!(state.definitions().len(), 1);
    drop(snapshot);
    drop(store);
    drop(reopened);

    let second_root = root.clone();
    let second_home = home.clone();
    let second =
        tokio::spawn(
            async move { serve_workspace(&second_root, Some(second_home), workspace).await },
        );
    let client = loop {
        if let Ok(client) =
            Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid())).await
        {
            break client;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    let status: Value = client
        .call(Operation::DaemonStatus, &json!({}))
        .await
        .unwrap();
    let generation = status["generation"].as_str().unwrap();
    let _: Value = client
        .call(Operation::DaemonShutdown, &json!({"generation":generation}))
        .await
        .unwrap();
    second.await.unwrap().unwrap();
    let reopened = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let snapshot = SqliteStore::new(reopened.pool().clone())
        .begin(workspace)
        .await
        .unwrap();
    assert_eq!(snapshot.snapshot().revision(), revision);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_waits_for_an_admitted_mutation_and_preserves_its_result() {
    let temporary = short_temp();
    let root = temporary.path().join("project");
    let home = temporary.path().join("private");
    std::fs::create_dir(&root).unwrap();
    let (route, locations) = initialize(&root, Some(home), "Drain fixture".into())
        .await
        .unwrap();
    let workspace: WorkspaceId = route.workspace_id.parse().unwrap();
    let database_path = locations
        .data()
        .join("workspaces")
        .join(workspace.to_string())
        .join("workspace.sqlite3");
    let database = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(database.pool().clone());
    let service = std::sync::Arc::new(Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Noop,
    ));
    let endpoint = Endpoint::derive(
        locations.runtime(),
        WireWorkspaceId::from_uuid(workspace.as_uuid()),
    )
    .unwrap();
    let (control, shutdown) = DaemonControl::new("00000000-0000-4000-8000-000000000504".into());
    let server = WorkspaceServer::bind(workspace, &endpoint, service, store.clone())
        .await
        .unwrap()
        .with_lifecycle(control);
    let faults = server.fault_injector();
    faults.pause_next_mutation_response();
    let mut server_task = tokio::spawn(server.run(shutdown));
    let writer = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let stopper = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let mutation = tokio::spawn(async move {
        writer.call::<_, Value>(Operation::TaskCreate, &json!({
            "expected_revision":"1","title":"Drained task","description":"","priority":"normal",
            "scope_paths":[],"acceptance_notes":"","dependency_ids":[]
        })).await
    });
    faults.wait_until_mutation_response_paused().await;
    let stop = tokio::spawn(async move {
        stopper
            .call::<_, Value>(
                Operation::DaemonShutdown,
                &json!({
                    "generation":"00000000-0000-4000-8000-000000000504"
                }),
            )
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut server_task)
            .await
            .is_err()
    );
    faults.release_paused_mutation_response();
    mutation.await.unwrap().unwrap();
    stop.await.unwrap().unwrap();
    server_task.await.unwrap().unwrap();
    drop(store);
    drop(database);

    let reopened = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let transaction = SqliteStore::new(reopened.pool().clone())
        .begin(workspace)
        .await
        .unwrap();
    assert_eq!(transaction.snapshot().state().unwrap().tasks().len(), 1);
    assert_eq!(transaction.snapshot().revision(), 2);
}
