use relayterm_application::{Clock, EventNotifier, LaunchContext, Service};
use relayterm_client::{Client, ClientError, Delivery};
use relayterm_daemon::WorkspaceServer;
use relayterm_domain::{
    AgentInstanceId, EntityId, InstanceStatus, Observation, TaskStatus, TerminalSize, WorkspaceId,
};
use relayterm_ipc::{Endpoint, connect, read_frame, write_frame};
use relayterm_persistence_sqlite::{Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore};
use relayterm_platform::{RandomIdGenerator, SystemClock, create_private_dir};
use relayterm_protocol::{
    DecimalU64, ErrorCode, FrameKind, JSON_FRAME_LIMIT, Operation, RequestEnvelope, RequestType,
    WorkspaceId as WireWorkspaceId, encode_frame, encode_json,
};
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
    let workspace = test_workspace_id(2);
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

fn test_workspace_id(discriminator: u16) -> WorkspaceId {
    let suffix = (u64::from(std::process::id()) << 16) | u64::from(discriminator);
    format!("00000000-0000-4000-8000-{suffix:012x}")
        .parse()
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

async fn send_fault_and_wait(
    endpoint: &Endpoint,
    faults: &relayterm_daemon::ServerFaults,
    label: &str,
    bytes: &[u8],
) {
    let previous = faults.connection_accept_count();
    let mut stream = connect(endpoint).await.unwrap();
    let connection = tokio::time::timeout(
        Duration::from_secs(2),
        faults.wait_for_connection_accepted_after(previous),
    )
    .await
    .unwrap();
    stream.write_all(bytes).await.unwrap();
    stream.shutdown().await.unwrap();
    // Windows named pipes do not expose the truncated peer as closed until the
    // client handle itself is released. Unix sockets observe the half-close.
    drop(stream);
    tokio::time::timeout(
        Duration::from_secs(5),
        faults.wait_for_connection_completed(connection),
    )
    .await
    .unwrap_or_else(|_| panic!("server did not reject {label} before the test deadline"));
}

fn request_bytes(
    workspace: WireWorkspaceId,
    request_id: u64,
    operation: Operation,
    protocol_version: u16,
) -> Vec<u8> {
    let request = RequestEnvelope {
        message_type: RequestType::Request,
        protocol_version,
        request_id: DecimalU64::new(request_id).unwrap(),
        workspace_id: workspace,
        operation,
        params: json!({}),
    };
    encode_frame(FrameKind::Json, &encode_json(&request).unwrap()).unwrap()
}

async fn duplicate_request_id_and_wait(
    endpoint: &Endpoint,
    workspace: WireWorkspaceId,
    faults: &relayterm_daemon::ServerFaults,
) {
    let previous = faults.connection_accept_count();
    let mut stream = connect(endpoint).await.unwrap();
    let connection = tokio::time::timeout(
        Duration::from_secs(2),
        faults.wait_for_connection_accepted_after(previous),
    )
    .await
    .unwrap();
    write_frame(
        &mut stream,
        FrameKind::Json,
        &encode_json(&RequestEnvelope {
            message_type: RequestType::Request,
            protocol_version: relayterm_protocol::PROTOCOL_VERSION,
            request_id: DecimalU64::new(1).unwrap(),
            workspace_id: workspace,
            operation: Operation::ProtocolHello,
            params: json!({}),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    read_frame(&mut stream, Duration::from_secs(2))
        .await
        .unwrap();
    write_frame(
        &mut stream,
        FrameKind::Json,
        &encode_json(&RequestEnvelope {
            message_type: RequestType::Request,
            protocol_version: relayterm_protocol::PROTOCOL_VERSION,
            request_id: DecimalU64::new(1).unwrap(),
            workspace_id: workspace,
            operation: Operation::ProtocolPing,
            params: json!({}),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        faults.wait_for_connection_completed(connection),
    )
    .await
    .unwrap();
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
    let workspace = test_workspace_id(1);
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
    let catalog: Value = a
        .call(Operation::AgentListTemplates, &json!({}))
        .await
        .unwrap();
    assert_eq!(catalog["catalog_version"], 1);
    assert_eq!(catalog["templates"].as_array().unwrap().len(), 3);
    assert!(
        catalog["templates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|template| {
                template["enabled"] == false
                    && template["arguments"].as_array().is_some_and(Vec::is_empty)
                    && template["environment_allowlist"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
            })
    );
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
    let definition_id = entity_id(&definition);
    let unavailable: Value = a
        .call(
            Operation::AgentCheckDefinition,
            &json!({"definition_id":definition_id,"expected_revision":revision}),
        )
        .await
        .unwrap();
    assert_eq!(unavailable["status"], "not_found");
    assert_eq!(unavailable["guidance_code"], "check_daemon_path_or_command");
    let missing_update = a
        .call::<_, Value>(
            Operation::AgentUpdateDefinition,
            &json!({
                "definition_id":"00000000-0000-4000-8000-000000009999",
                "expected_revision":revision,"display_name":"Missing","command":"missing",
                "arguments":[],"environment_allowlist":[],"capabilities":[],"enabled":false
            }),
        )
        .await;
    assert!(matches!(
        missing_update,
        Err(ClientError::Rejected(ErrorCode::InvalidReference))
    ));
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
    let survivor = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let before_faults: Value = survivor
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"workspace","limit":50}),
        )
        .await
        .unwrap();
    let wire_workspace = WireWorkspaceId::from_uuid(workspace.as_uuid());
    let mut oversized = [0_u8; relayterm_protocol::HEADER_SIZE];
    oversized[..4].copy_from_slice(&((JSON_FRAME_LIMIT + 1) as u32).to_be_bytes());
    oversized[5] = relayterm_protocol::PROTOCOL_VERSION as u8;
    oversized[6] = FrameKind::Json as u8;
    let mut partial_body = request_bytes(
        wire_workspace,
        1,
        Operation::ProtocolHello,
        relayterm_protocol::PROTOCOL_VERSION,
    );
    partial_body.pop();
    let unknown_field = encode_frame(
        FrameKind::Json,
        format!(
            "{{\"type\":\"request\",\"protocol_version\":1,\"request_id\":\"1\",\"workspace_id\":\"{wire_workspace}\",\"operation\":\"protocol.hello\",\"params\":{{}},\"private_marker\":\"must-not-pass\"}}"
        )
        .as_bytes(),
    )
    .unwrap();
    let malformed_cases = [
        ("partial header", vec![0, 0, 0]),
        ("partial body", partial_body),
        ("oversized frame", oversized.to_vec()),
        (
            "invalid UTF-8",
            encode_frame(FrameKind::Json, &[0xff]).unwrap(),
        ),
        ("invalid JSON", encode_frame(FrameKind::Json, b"{").unwrap()),
        ("unknown envelope field", unknown_field),
        (
            "unsupported version",
            request_bytes(wire_workspace, 1, Operation::ProtocolHello, 2),
        ),
    ];
    for (label, bytes) in malformed_cases {
        send_fault_and_wait(&endpoint, &faults, label, &bytes).await;
        let healthy: Value = survivor
            .call(Operation::ProtocolPing, &json!({}))
            .await
            .unwrap();
        assert_eq!(healthy["ok"], true);
    }
    duplicate_request_id_and_wait(&endpoint, wire_workspace, &faults).await;
    let after_faults: Value = survivor
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"workspace","limit":50}),
        )
        .await
        .unwrap();
    assert_eq!(after_faults["revision"], before_faults["revision"]);
    assert_eq!(
        after_faults["last_sequence"],
        before_faults["last_sequence"]
    );
    let mut slow = connect(&endpoint).await.unwrap();
    write_frame(
        &mut slow,
        FrameKind::Json,
        &encode_json(&RequestEnvelope {
            message_type: RequestType::Request,
            protocol_version: relayterm_protocol::PROTOCOL_VERSION,
            request_id: DecimalU64::new(1).unwrap(),
            workspace_id: wire_workspace,
            operation: Operation::ProtocolHello,
            params: json!({}),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    read_frame(&mut slow, Duration::from_secs(2)).await.unwrap();
    write_frame(
        &mut slow,
        FrameKind::Json,
        &encode_json(&RequestEnvelope {
            message_type: RequestType::Request,
            protocol_version: relayterm_protocol::PROTOCOL_VERSION,
            request_id: DecimalU64::new(2).unwrap(),
            workspace_id: wire_workspace,
            operation: Operation::EventSubscribe,
            params: json!({"after_sequence":before_faults["last_sequence"]}),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    read_frame(&mut slow, Duration::from_secs(2)).await.unwrap();
    let slow_before = faults.slow_subscriber_count();
    faults.pause_next_event_write();
    let _: Value = survivor
        .call(
            Operation::ProgressAppend,
            &json!({"task_id":task_id,"summary":"Slow subscriber seed","verification":"Deterministic writer barrier"}),
        )
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        faults.wait_until_event_writer_paused(),
    )
    .await
    .unwrap();
    for index in 0..300 {
        let _: Value = survivor
            .call(
                Operation::ProgressAppend,
                &json!({"task_id":task_id,"summary":format!("Bounded event {index}"),"verification":"Nonreading subscriber overflow"}),
            )
            .await
            .unwrap();
    }
    tokio::time::timeout(
        Duration::from_secs(2),
        faults.wait_for_slow_subscriber_after(slow_before),
    )
    .await
    .unwrap();
    let healthy_during_overflow: Value = survivor
        .call(Operation::ProtocolPing, &json!({}))
        .await
        .unwrap();
    assert_eq!(healthy_during_overflow["ok"], true);
    faults.release_paused_event_writer();
    let mut observed_resnapshot = false;
    for _ in 0..3 {
        let frame = read_frame(&mut slow, Duration::from_secs(2)).await.unwrap();
        let value: Value = serde_json::from_slice(&frame.payload).unwrap();
        if value["control"] == "resnapshot_required" {
            assert_eq!(value["reason"], "slow_subscriber");
            observed_resnapshot = true;
            break;
        }
    }
    assert!(observed_resnapshot);
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_subscription_delivers_event_without_reconnecting() {
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
    let workspace = test_workspace_id(3);
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

    let snapshot: Value = a
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    let sequence = snapshot["last_sequence"].as_str().unwrap().parse().unwrap();
    let revision = snapshot["revision"].as_str().unwrap();
    let b = b.with_deadline(Duration::from_millis(200));
    b.subscribe(sequence).await.unwrap();
    let connections = faults.connection_accept_count();
    let (event, created) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(15), b.next_event()),
        async {
            tokio::time::sleep(relayterm_ipc::PARTIAL_FRAME_TIMEOUT + Duration::from_secs(1)).await;
            a.call::<_, Value>(Operation::TaskCreate, &json!({"expected_revision":revision,"title":"After idle","description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]})).await
        }
    );
    created.unwrap();
    let event = event
        .expect("event wait must finish")
        .expect("idle must not disconnect");
    assert_eq!(
        event["sequence"].as_str().unwrap().parse::<u64>().unwrap(),
        sequence + 1
    );
    assert_eq!(
        faults.connection_accept_count(),
        connections,
        "idle request and event connections must remain open"
    );
    drop(a);
    drop(b);
    shutdown_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
}
