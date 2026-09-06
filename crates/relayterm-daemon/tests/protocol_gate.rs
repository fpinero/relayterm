use relayterm_application::{Clock, EventNotifier, LaunchContext, Service};
use relayterm_client::{Client, ClientError, Delivery};
use relayterm_daemon::WorkspaceServer;
use relayterm_domain::{
    AgentInstanceId, EntityId, InstanceStatus, Observation, TaskStatus, TerminalSize, WorkspaceId,
};
use relayterm_ipc::{Endpoint, connect};
use relayterm_persistence_sqlite::{Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore};
use relayterm_platform::{RandomIdGenerator, SystemClock, create_private_dir};
use relayterm_protocol::{ErrorCode, Operation, WorkspaceId as WireWorkspaceId};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;

#[cfg(unix)]
fn private_temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("rt-m04-")
        .tempdir_in(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        })
        .unwrap()
}

#[test]
#[ignore]
fn process_helper() {
    let Ok(role) = std::env::var("RELAYTERM_M04_ROLE") else {
        return;
    };
    let database_path = PathBuf::from(std::env::var_os("RELAYTERM_M04_DB").unwrap());
    let runtime_root = PathBuf::from(std::env::var_os("RELAYTERM_M04_RUNTIME").unwrap());
    let workspace: WorkspaceId = std::env::var("RELAYTERM_M04_WORKSPACE")
        .unwrap()
        .parse()
        .unwrap();
    let wire_workspace = WireWorkspaceId::from_uuid(workspace.as_uuid());
    let endpoint = Endpoint::derive(&runtime_root, wire_workspace).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        if role == "server" {
            let database = Database::open(&database_path, DatabaseKind::Workspace, OpenMode::Reopen, PoolSettings::default()).await.unwrap();
            let store = SqliteStore::new(database.pool().clone());
            let service = Arc::new(Service::new(store.clone(), SystemClock, RandomIdGenerator::default(), Notify::default()));
            let server = WorkspaceServer::bind(workspace, &endpoint, service, store).await.unwrap();
            let (_sender, receiver) = watch::channel(false);
            server.run(receiver).await.unwrap();
            return;
        }
        let client = Client::connect(&endpoint, wire_workspace).await.unwrap();
        if role == "client_a" {
            let instance = std::env::var("RELAYTERM_M04_INSTANCE").unwrap();
            let snapshot: Value = client.call(Operation::WorkspaceGetSnapshot, &json!({"collection":"tasks","limit":50})).await.unwrap();
            let mut revision = snapshot["revision"].as_str().unwrap().to_owned();
            let created: Value = client.call(Operation::TaskCreate, &json!({"expected_revision":revision,"title":"Process task","description":"Cross-process continuity","priority":"normal","scope_paths":[],"acceptance_notes":"Complete in another process","dependency_ids":[]})).await.unwrap();
            let task_id = entity_id(&created);
            revision = created["revision"].as_str().unwrap().to_owned();
            let _: Value = client.call(Operation::TaskTransition, &json!({"task_id":task_id,"expected_revision":revision,"status":"ready"})).await.unwrap();
            let _: Value = client.call(Operation::TaskClaim, &json!({"task_id":task_id,"instance_id":instance})).await.unwrap();
            let progress: Value = client.call(Operation::ProgressAppend, &json!({"task_id":task_id,"summary":"Created in client A","verification":"Process gate"})).await.unwrap();
            revision = progress["revision"].as_str().unwrap().to_owned();
            let _: Value = client.call(Operation::HandoverCreate, &json!({"task_id":task_id,"expected_revision":revision,"summary":"Continue in client B","decisions":"Use durable state","changed_paths":[],"verification_performed":"Process gate","open_questions":"","recommended_next_action":"Complete"})).await.unwrap();
            let result = PathBuf::from(std::env::var_os("RELAYTERM_M04_RESULT").unwrap());
            let mut file = relayterm_platform::create_private_file(&result).unwrap();
            file.write_all(task_id.as_bytes()).unwrap();
            file.sync_all().unwrap();
            return;
        }
        if role == "client_b" {
            let instance = std::env::var("RELAYTERM_M04_INSTANCE").unwrap();
            let task_id = std::fs::read_to_string(std::env::var_os("RELAYTERM_M04_RESULT").unwrap()).unwrap();
            let claimed: Value = client.call(Operation::TaskClaim, &json!({"task_id":task_id,"instance_id":instance})).await.unwrap();
            let revision = claimed["revision"].as_str().unwrap();
            let _: Value = client.call(Operation::TaskTransition, &json!({"task_id":task_id,"expected_revision":revision,"status":"done"})).await.unwrap();
        }
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn separate_server_and_client_processes_share_durable_state() {
    let temp = private_temp();
    let private = temp.path().join("private");
    create_private_dir(&private).unwrap();
    let database_path = private.join("workspace.db");
    let result_path = private.join("task.id");
    let database = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::ExplicitNew,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(database.pool().clone());
    let workspace: WorkspaceId = "00000000-0000-4000-8000-000000000202".parse().unwrap();
    let service = Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Notify::default(),
    );
    service
        .create_workspace_reserved(
            workspace,
            "Process fixture".into(),
            temp.path().join("project"),
        )
        .await
        .unwrap();
    let first = running_instance(&service, workspace, temp.path().join("project")).await;
    let second = running_instance(&service, workspace, temp.path().join("project")).await;
    drop(service);
    drop(store);
    drop(database);
    let runtime_root = temp.path().join("runtime");
    let environment = [
        (
            "RELAYTERM_M04_DB",
            database_path.to_string_lossy().into_owned(),
        ),
        (
            "RELAYTERM_M04_RUNTIME",
            runtime_root.to_string_lossy().into_owned(),
        ),
        ("RELAYTERM_M04_WORKSPACE", workspace.to_string()),
        (
            "RELAYTERM_M04_RESULT",
            result_path.to_string_lossy().into_owned(),
        ),
    ];
    let mut server = ChildGuard(helper("server", &environment));
    let endpoint = Endpoint::derive(
        &runtime_root,
        WireWorkspaceId::from_uuid(workspace.as_uuid()),
    )
    .unwrap();
    let mut ready = false;
    for _ in 0..100 {
        if Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
            .await
            .is_ok()
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(ready, "server helper did not become ready");
    let mut first_environment = environment.to_vec();
    first_environment.push(("RELAYTERM_M04_INSTANCE", first.to_string()));
    assert!(
        helper("client_a", &first_environment)
            .wait()
            .unwrap()
            .success()
    );
    let mut second_environment = environment.to_vec();
    second_environment.push(("RELAYTERM_M04_INSTANCE", second.to_string()));
    assert!(
        helper("client_b", &second_environment)
            .wait()
            .unwrap()
            .success()
    );
    server.0.kill().unwrap();
    server.0.wait().unwrap();
    let reopened = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let state = relayterm_application::DurableReadStore::consistent_snapshot(
        &SqliteStore::new(reopened.pool().clone()),
        workspace,
    )
    .await
    .unwrap();
    let task_id: relayterm_domain::TaskId = std::fs::read_to_string(result_path)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        state
            .snapshot
            .state()
            .unwrap()
            .task(task_id)
            .unwrap()
            .record()
            .status,
        TaskStatus::Done
    );
    assert_eq!(state.snapshot.state().unwrap().claims().len(), 2);
    assert_eq!(state.snapshot.state().unwrap().progress().len(), 1);
    assert_eq!(state.snapshot.state().unwrap().handovers().len(), 1);
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn helper(role: &str, environment: &[(&str, String)]) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", "process_helper", "--nocapture"])
        .env("RELAYTERM_M04_ROLE", role)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (name, value) in environment {
        command.env(name, value);
    }
    command.spawn().unwrap()
}

#[cfg(windows)]
fn private_temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("rt-m04-")
        .tempdir()
        .unwrap()
}

#[derive(Clone, Default)]
struct Notify(Option<watch::Sender<u64>>);
impl EventNotifier for Notify {
    async fn notify(&self, _: WorkspaceId, revision: u64) -> relayterm_domain::Result<()> {
        if let Some(sender) = &self.0 {
            sender.send_replace(revision);
        }
        Ok(())
    }
}

fn entity_id(value: &Value) -> String {
    value["entity_ids"][0].as_str().unwrap().to_owned()
}
async fn running_instance(
    service: &Service<SqliteStore, SystemClock, RandomIdGenerator, Notify>,
    workspace: WorkspaceId,
    root: PathBuf,
) -> AgentInstanceId {
    let outcome = service
        .register_instance(
            workspace,
            LaunchContext {
                agent_definition_id: None,
                task_id: None,
                working_directory: root,
                terminal_size: TerminalSize::new(24, 80).unwrap(),
            },
        )
        .await
        .unwrap();
    let id = match outcome.committed.events[0].record().entity_id {
        EntityId::Instance(id) => id,
        _ => panic!("expected instance"),
    };
    let observed = SystemClock.now().unwrap();
    service
        .observe(
            workspace,
            Observation::Status {
                id,
                status: InstanceStatus::Running,
                observed_at: observed,
                exit_code: None,
            },
        )
        .await
        .unwrap();
    id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_clients_complete_a_durable_handover_journey() {
    let temp = private_temp();
    let private = temp.path().join("private");
    create_private_dir(&private).unwrap();
    let database_path = private.join("workspace.db");
    let database = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::ExplicitNew,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(database.pool().clone());
    let workspace: WorkspaceId = "00000000-0000-4000-8000-000000000101".parse().unwrap();
    let (event_wakeup_tx, event_wakeup_rx) = watch::channel(0);
    let service = Arc::new(Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Notify(Some(event_wakeup_tx)),
    ));
    service
        .create_workspace_reserved(
            workspace,
            "Protocol fixture".into(),
            temp.path().join("project"),
        )
        .await
        .unwrap();
    let first = running_instance(&service, workspace, temp.path().join("project")).await;
    let second = running_instance(&service, workspace, temp.path().join("project")).await;
    let endpoint = Endpoint::derive(
        &temp.path().join("runtime"),
        WireWorkspaceId::from_uuid(workspace.as_uuid()),
    )
    .unwrap();
    let server = WorkspaceServer::bind(workspace, &endpoint, service.clone(), store.clone())
        .await
        .unwrap()
        .with_event_wakeups(event_wakeup_rx);
    let faults = server.fault_injector();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(server.run(shutdown_rx));
    let a = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let b = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let (_cancel_tx, mut already_cancelled) = watch::channel(true);
    let cancelled_before_delivery = b
        .call_cancellable::<_, Value>(Operation::ProtocolPing, &json!({}), &mut already_cancelled)
        .await;
    assert!(matches!(
        cancelled_before_delivery,
        Err(ClientError::Cancelled(Delivery::NotSent))
    ));
    let snapshot: Value = a
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    assert_eq!(snapshot["items"].as_array().unwrap().len(), 0);
    let mut revision = snapshot["revision"].as_str().unwrap().to_owned();
    let subscription_start = snapshot["last_sequence"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let definition: Value = a
        .call(
            Operation::AgentRegisterDefinition,
            &json!({"expected_revision":revision,"display_name":"Replay fixture","command":"agent","arguments":[],"environment_allowlist":[],"capabilities":[],"enabled":true}),
        )
        .await
        .unwrap();
    revision = definition["revision"].as_str().unwrap().to_owned();
    b.subscribe(subscription_start).await.unwrap();
    let replayed = b.next_event().await.unwrap();
    assert_eq!(
        replayed["sequence"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        subscription_start + 1
    );
    let event_wakeups_before_live_commit = faults.event_wakeup_count();
    let created:Value=a.call(Operation::TaskCreate,&json!({"expected_revision":revision,"title":"IPC task","description":"Shared state","priority":"normal","scope_paths":[],"acceptance_notes":"Complete the flow","dependency_ids":[]})).await.unwrap();
    let task_id = entity_id(&created);
    faults
        .wait_for_event_wakeup_after(event_wakeups_before_live_commit)
        .await;
    let first_live_event = b.next_event().await.unwrap();
    assert_eq!(
        first_live_event["sequence"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        subscription_start + 2
    );
    revision = created["revision"].as_str().unwrap().to_owned();
    let ready: Value = a
        .call(
            Operation::TaskTransition,
            &json!({"task_id":task_id,"expected_revision":revision,"status":"ready"}),
        )
        .await
        .unwrap();
    let _ = ready;
    let claimed: Value = a
        .call(
            Operation::TaskClaim,
            &json!({"task_id":task_id,"instance_id":first.to_string()}),
        )
        .await
        .unwrap();
    let _ = claimed;
    let competing = b
        .call::<_, Value>(
            Operation::TaskClaim,
            &json!({"task_id":task_id,"instance_id":second.to_string()}),
        )
        .await;
    assert!(matches!(
        competing,
        Err(ClientError::Rejected(ErrorCode::Conflict))
    ));
    let progress:Value=a.call(Operation::ProgressAppend,&json!({"task_id":task_id,"summary":"Implementation advanced","verification":"cargo test"})).await.unwrap();
    revision = progress["revision"].as_str().unwrap().to_owned();
    let handover:Value=a.call(Operation::HandoverCreate,&json!({"task_id":task_id,"expected_revision":revision,"summary":"Ready for continuation","decisions":"Keep protocol neutral","changed_paths":["src/lib.rs"],"verification_performed":"cargo test","open_questions":"","recommended_next_action":"Complete the task"})).await.unwrap();
    let _ = handover;
    let resumed: Value = b
        .call(
            Operation::TaskClaim,
            &json!({"task_id":task_id,"instance_id":second.to_string()}),
        )
        .await
        .unwrap();
    revision = resumed["revision"].as_str().unwrap().to_owned();
    let done: Value = b
        .call(
            Operation::TaskTransition,
            &json!({"task_id":task_id,"expected_revision":revision,"status":"done"}),
        )
        .await
        .unwrap();
    revision = done["revision"].as_str().unwrap().to_owned();
    let history:Value=b.call(Operation::TaskGetHistory,&json!({"task_id":task_id,"after_sequence":"0","expected_revision":revision,"limit":50})).await.unwrap();
    assert!(history["entries"].as_array().unwrap().len() >= 4);
    let events: Value = a
        .call(
            Operation::EventList,
            &json!({"after_sequence":"0","limit":200}),
        )
        .await
        .unwrap();
    let sequences = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["sequence"].as_str().unwrap().parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    assert!(sequences.windows(2).all(|x| x[1] == x[0] + 1));
    let state: Value = b
        .call(Operation::TaskGet, &json!({"task_id":task_id}))
        .await
        .unwrap();
    assert_eq!(state["status"], json!(TaskStatus::Done));
    faults.drop_next_response();
    let reconnected_pong: Value = b.call(Operation::ProtocolPing, &json!({})).await.unwrap();
    assert_eq!(reconnected_pong["ok"], true);
    b.unsubscribe().await.unwrap();
    let synchronized = b.refresh_snapshot().await.unwrap();
    assert_eq!(synchronized.revision, revision);
    assert_eq!(synchronized.collections["tasks"].len(), 1);
    assert_eq!(
        b.status().await.synchronization,
        relayterm_client::SynchronizationStatus::Current
    );
    let reserved = b
        .call::<_, Value>(
            Operation::SessionAttach,
            &json!({"session_id":"00000000-0000-4000-8000-000000000303"}),
        )
        .await;
    assert!(matches!(
        reserved,
        Err(ClientError::Rejected(ErrorCode::OperationUnavailable))
    ));
    let invalid_reserved = b
        .call::<_, Value>(Operation::SessionCreate, &json!({}))
        .await;
    assert!(matches!(
        invalid_reserved,
        Err(ClientError::Rejected(ErrorCode::InvalidParams))
    ));
    let unchanged: Value = b
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"workspace","limit":50}),
        )
        .await
        .unwrap();
    assert_eq!(unchanged["revision"], revision);
    let unknown_operation = b.call::<_, Value>(Operation::Unknown, &json!({})).await;
    assert!(matches!(
        unknown_operation,
        Err(ClientError::Rejected(ErrorCode::UnknownOperation))
    ));
    faults.pause_next_mutation_response();
    let (cancel_tx, mut cancellation) = watch::channel(false);
    let cancelled_progress = json!({"task_id":task_id,"summary":"Cancelled local wait","verification":"Recovered from durable state"});
    let (cancelled_after_delivery, ()) = tokio::join!(
        a.call_cancellable::<_, Value>(
            Operation::ProgressAppend,
            &cancelled_progress,
            &mut cancellation,
        ),
        async {
            faults.wait_until_mutation_response_paused().await;
            cancel_tx.send(true).unwrap();
            faults.release_paused_mutation_response();
        }
    );
    assert!(matches!(
        cancelled_after_delivery,
        Err(ClientError::Cancelled(Delivery::Unknown))
    ));
    let pong_after_cancellation: Value = a.call(Operation::ProtocolPing, &json!({})).await.unwrap();
    assert_eq!(pong_after_cancellation["ok"], true);
    let uncertainty_client =
        Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
            .await
            .unwrap();
    faults.drop_next_mutation_response();
    let uncertain=uncertainty_client.call::<_,Value>(Operation::ProgressAppend,&json!({"task_id":task_id,"summary":"Historical correction","verification":"Reviewed after completion"})).await;
    assert!(matches!(
        uncertain,
        Err(ClientError::Transport(Delivery::Unknown))
    ));
    let recovered = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap()
        .refresh_snapshot()
        .await
        .unwrap();
    assert_eq!(recovered.collections["progress"].len(), 3);
    let expired = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    expired.subscribe(recovered.last_sequence).await.unwrap();
    sqlx::query("UPDATE workspace_meta SET retained_from_sequence=? WHERE workspace_id=?")
        .bind(relayterm_persistence_sqlite::encode_counter(recovered.last_sequence + 2).to_vec())
        .bind(workspace.as_uuid().as_bytes().to_vec())
        .execute(store.pool())
        .await
        .unwrap();
    let _: Value=a.call(Operation::ProgressAppend,&json!({"task_id":task_id,"summary":"Force cursor expiry","verification":"Controlled retention watermark"})).await.unwrap();
    assert!(matches!(
        expired.next_event().await,
        Err(ClientError::Rejected(ErrorCode::ResnapshotRequired))
    ));
    assert_eq!(
        expired.status().await.synchronization,
        relayterm_client::SynchronizationStatus::Retryable
    );
    sqlx::query("UPDATE workspace_meta SET retained_from_sequence=? WHERE workspace_id=?")
        .bind(relayterm_persistence_sqlite::encode_counter(1).to_vec())
        .bind(workspace.as_uuid().as_bytes().to_vec())
        .execute(store.pool())
        .await
        .unwrap();
    let mut malformed = connect(&endpoint).await.unwrap();
    malformed.write_all(&[0, 0, 0, 1, 0, 2, 1]).await.unwrap();
    drop(malformed);
    let survivor = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let invalid_subscription = survivor
        .call::<_, Value>(Operation::EventSubscribe, &json!({}))
        .await;
    assert!(matches!(
        invalid_subscription,
        Err(ClientError::Rejected(ErrorCode::InvalidParams))
    ));
    let pong: Value = survivor
        .call(Operation::ProtocolPing, &json!({}))
        .await
        .unwrap();
    assert_eq!(pong["ok"], true);
    shutdown_tx.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    drop(database);
    let reopened = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let reopened_store = SqliteStore::new(reopened.pool().clone());
    let durable =
        relayterm_application::DurableReadStore::consistent_snapshot(&reopened_store, workspace)
            .await
            .unwrap();
    assert_eq!(
        durable
            .snapshot
            .state()
            .unwrap()
            .task(task_id.parse().unwrap())
            .unwrap()
            .record()
            .status,
        TaskStatus::Done
    );
}
