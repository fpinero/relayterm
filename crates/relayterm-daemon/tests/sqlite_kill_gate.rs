#![cfg(feature = "test-hooks")]

use relayterm_application::{EventNotifier, Request, Service, Store, Transaction};
use relayterm_client::Client;
use relayterm_daemon::WorkspaceServer;
use relayterm_domain::{Actor, TaskContent, WorkspaceId};
use relayterm_ipc::Endpoint;
use relayterm_persistence_sqlite::{Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore};
use relayterm_platform::{RandomIdGenerator, SystemClock};
use relayterm_protocol::{Operation, WorkspaceId as WireWorkspaceId};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tokio::sync::watch;

const DEADLINE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
struct Notify;

impl EventNotifier for Notify {
    async fn notify(&self, _: WorkspaceId, _: u64) -> relayterm_domain::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn killing_the_server_before_sqlite_commit_rolls_back_the_whole_mutation() {
    #[cfg(unix)]
    let temporary = tempfile::Builder::new()
        .prefix("rt11k-")
        .tempdir_in(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        })
        .unwrap();
    #[cfg(not(unix))]
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let home = temporary.path().join("private");
    std::fs::create_dir(&root).unwrap();
    let (route, locations) =
        relayterm_daemon::initialize(&root, Some(home), "Abrupt transaction fixture".into())
            .await
            .unwrap();
    let workspace: WorkspaceId = route.workspace_id.parse().unwrap();
    let database_path = locations
        .data()
        .join("workspaces")
        .join(workspace.to_string())
        .join("workspace.sqlite3");
    let before = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let before_store = SqliteStore::new(before.pool().clone());
    let before_snapshot = before_store.begin(workspace).await.unwrap();
    let revision_before = before_snapshot.snapshot().revision();
    let events_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workspace_events WHERE workspace_id=?")
            .bind(workspace.as_uuid().as_bytes().as_slice())
            .fetch_one(before.pool())
            .await
            .unwrap();
    drop(before_snapshot);
    before.pool().close().await;

    let runtime = locations.runtime().to_owned();
    let server_ready = temporary.path().join("server.ready");
    let commit_ready = temporary.path().join("commit.ready");
    let mut server = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "sqlite_kill_helper",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RELAYTERM_KILL_DATABASE", &database_path)
        .env("RELAYTERM_KILL_RUNTIME", &runtime)
        .env("RELAYTERM_KILL_WORKSPACE", workspace.to_string())
        .env("RELAYTERM_KILL_SERVER_READY", &server_ready)
        .env("RELAYTERM_KILL_COMMIT_READY", &commit_ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    wait_for_file(&server_ready, "server readiness");

    let endpoint =
        Endpoint::derive(&runtime, WireWorkspaceId::from_uuid(workspace.as_uuid())).unwrap();
    let client = Client::connect(&endpoint, WireWorkspaceId::from_uuid(workspace.as_uuid()))
        .await
        .unwrap();
    let mutation = tokio::spawn(async move {
        client
            .call::<_, Value>(
                Operation::TaskCreate,
                &json!({
                    "expected_revision":revision_before.to_string(),
                    "title":"Must roll back",
                    "description":"",
                    "priority":"normal",
                    "scope_paths":[],
                    "acceptance_notes":"",
                    "dependency_ids":[]
                }),
            )
            .await
    });
    wait_for_file(&commit_ready, "pre-commit barrier");
    stop_child(&mut server);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), mutation)
            .await
            .unwrap()
            .unwrap()
            .is_err(),
        "a killed pre-commit request cannot report success"
    );

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
    assert_eq!(snapshot.snapshot().revision(), revision_before);
    assert!(snapshot.snapshot().state().unwrap().tasks().is_empty());
    drop(snapshot);
    let events_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workspace_events WHERE workspace_id=?")
            .bind(workspace.as_uuid().as_bytes().as_slice())
            .fetch_one(reopened.pool())
            .await
            .unwrap();
    assert_eq!(events_after, events_before);

    let service = Service::new(store, SystemClock, RandomIdGenerator::default(), Notify);
    let healthy = service
        .execute(
            workspace,
            Actor::LocalUser,
            Request::CreateTask(TaskContent {
                title: "Healthy mutation".into(),
                description: String::new(),
                priority: relayterm_domain::Priority::Normal,
                scope_paths: Vec::new(),
                acceptance_notes: String::new(),
                dependency_ids: Vec::new(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(healthy.committed.snapshot.revision(), revision_before + 1);
    assert_eq!(healthy.committed.events.len(), 1);
    reopened.pool().close().await;
}

#[test]
#[ignore]
fn sqlite_kill_helper() {
    let Some(database_path) = std::env::var_os("RELAYTERM_KILL_DATABASE") else {
        return;
    };
    let database_path = PathBuf::from(database_path);
    let runtime = PathBuf::from(std::env::var_os("RELAYTERM_KILL_RUNTIME").unwrap());
    let workspace: WorkspaceId = std::env::var("RELAYTERM_KILL_WORKSPACE")
        .unwrap()
        .parse()
        .unwrap();
    let server_ready = PathBuf::from(std::env::var_os("RELAYTERM_KILL_SERVER_READY").unwrap());
    let commit_ready = PathBuf::from(std::env::var_os("RELAYTERM_KILL_COMMIT_READY").unwrap());
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let database = Database::open(
                &database_path,
                DatabaseKind::Workspace,
                OpenMode::Reopen,
                PoolSettings::default(),
            )
            .await
            .unwrap();
            let hook = Arc::new(move || {
                std::fs::write(&commit_ready, b"ready").unwrap();
                loop {
                    thread::sleep(Duration::from_secs(1));
                }
            });
            let store = SqliteStore::new(database.pool().clone()).with_before_commit_hook(hook);
            let service = Arc::new(Service::new(
                store.clone(),
                SystemClock,
                RandomIdGenerator::default(),
                Notify,
            ));
            let endpoint =
                Endpoint::derive(&runtime, WireWorkspaceId::from_uuid(workspace.as_uuid()))
                    .unwrap();
            let server = WorkspaceServer::bind(workspace, &endpoint, service, store)
                .await
                .unwrap();
            std::fs::write(server_ready, b"ready").unwrap();
            let (_shutdown, receiver) = watch::channel(false);
            server.run(receiver).await.unwrap();
        });
}

fn wait_for_file(path: &Path, label: &str) {
    let deadline = Instant::now() + DEADLINE;
    while !path.exists() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
