use relayterm_application::{EventNotifier, Service};
use relayterm_client::Client;
use relayterm_daemon::WorkspaceServer;
use relayterm_domain::WorkspaceId;
use relayterm_ipc::Endpoint;
use relayterm_persistence_sqlite::{Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore};
use relayterm_platform::{RandomIdGenerator, SystemClock, create_private_dir};
use relayterm_protocol::{Operation, WorkspaceId as WireWorkspaceId};
use serde_json::{Value, json};
use std::{path::Path, process::Command, sync::Arc};
use tokio::sync::watch;

#[derive(Clone, Copy)]
struct Notify;

impl EventNotifier for Notify {
    async fn notify(&self, _: WorkspaceId, _: u64) -> relayterm_domain::Result<()> {
        Ok(())
    }
}

fn git(root: &Path, arguments: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(arguments)
            .status()
            .unwrap()
            .success()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_reconciliation_finishes_a_confirmed_add_without_repeating_git() {
    #[cfg(unix)]
    let temporary = tempfile::Builder::new()
        .prefix("rt10r-")
        .tempdir_in(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        })
        .unwrap();
    #[cfg(not(unix))]
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    let private = temporary.path().join("private");
    let worktrees = temporary.path().join("worktrees");
    let runtime = temporary.path().join("runtime");
    std::fs::create_dir(&project).unwrap();
    create_private_dir(&private).unwrap();
    create_private_dir(&worktrees).unwrap();
    create_private_dir(&runtime).unwrap();
    git(&project, &["init", "-q"]);
    git(&project, &["config", "user.name", "Fixture"]);
    git(
        &project,
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(project.join("fixture.txt"), "fixture\n").unwrap();
    git(&project, &["add", "fixture.txt"]);
    git(&project, &["commit", "-qm", "fixture"]);

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
    let workspace: WorkspaceId = "00000000-0000-4000-8000-000000000810".parse().unwrap();
    let service = Arc::new(Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Notify,
    ));
    service
        .create_workspace_reserved(workspace, "Recovery fixture".into(), project.clone())
        .await
        .unwrap();
    let wire_workspace = WireWorkspaceId::from_uuid(workspace.as_uuid());
    let endpoint = Endpoint::derive(&runtime, wire_workspace).unwrap();
    let server = WorkspaceServer::bind(workspace, &endpoint, service, store)
        .await
        .unwrap()
        .with_worktrees(worktrees.clone());
    let faults = server.fault_injector();
    faults.fail_next_worktree_after_git();
    let (shutdown, receiver) = watch::channel(false);
    let running = tokio::spawn(server.run(receiver));
    let client = Client::connect(&endpoint, wire_workspace).await.unwrap();

    let snapshot: Value = client
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    let created: Value = client
        .call(
            Operation::TaskCreate,
            &json!({"expected_revision":snapshot["revision"],"title":"Recover add","description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]}),
        )
        .await
        .unwrap();
    let task_id = created["entity_ids"][0].as_str().unwrap();
    let ready: Value = client
        .call(
            Operation::TaskTransition,
            &json!({"task_id":task_id,"expected_revision":created["revision"],"status":"ready"}),
        )
        .await
        .unwrap();
    let operation_id = "00000000-0000-4000-8000-000000001810";
    let failed = client
        .call::<_, Value>(
            Operation::WorktreeCreate,
            &json!({"payload_version":1,"operation_id":operation_id,"task_id":task_id,"expected_revision":ready["revision"],"base_ref":"HEAD","branch_name":"rt/recovery","destination_leaf":"recovery","parent":null}),
        )
        .await;
    assert!(failed.is_err());
    let applying: Value = client
        .call(
            Operation::WorktreeGetOperation,
            &json!({"operation_id":operation_id}),
        )
        .await
        .unwrap();
    assert_eq!(applying["phase"], "applying");
    assert!(worktrees.join("recovery").join("fixture.txt").exists());
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        2
    );
    let recovered_checkout = worktrees.join("recovery");
    git(&recovered_checkout, &["config", "user.name", "Fixture"]);
    git(
        &recovered_checkout,
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(recovered_checkout.join("continued.txt"), "continued\n").unwrap();
    git(&recovered_checkout, &["add", "continued.txt"]);
    git(&recovered_checkout, &["commit", "-qm", "continued work"]);

    drop(client);
    shutdown.send(true).unwrap();
    running.await.unwrap().unwrap();
    database.pool().close().await;

    let reopened = Database::open(
        &database_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(reopened.pool().clone());
    let service = Arc::new(Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Notify,
    ));
    let server = WorkspaceServer::bind(workspace, &endpoint, service, store)
        .await
        .unwrap()
        .with_worktrees(worktrees.clone());
    let (shutdown, receiver) = watch::channel(false);
    let running = tokio::spawn(server.run(receiver));
    let client = Client::connect(&endpoint, wire_workspace).await.unwrap();
    let current: Value = client
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    let reconciled: Value = client
        .call(
            Operation::WorktreeReconcile,
            &json!({"operation_id":operation_id,"expected_revision":current["revision"]}),
        )
        .await
        .unwrap();
    assert_eq!(reconciled["phase"], "ready");
    assert_eq!(reconciled["selected"], true);
    assert!(recovered_checkout.join("continued.txt").exists());
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        2,
        "reconciliation must not invoke worktree add again"
    );

    let current: Value = client
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    let created: Value = client
        .call(
            Operation::TaskCreate,
            &json!({"expected_revision":current["revision"],"title":"Recover final commit","description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]}),
        )
        .await
        .unwrap();
    let second_task_id = created["entity_ids"][0].as_str().unwrap();
    let ready: Value = client
        .call(
            Operation::TaskTransition,
            &json!({"task_id":second_task_id,"expected_revision":created["revision"],"status":"ready"}),
        )
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER inject_worktree_finalize_failure BEFORE INSERT ON worktrees BEGIN SELECT RAISE(ABORT, 'synthetic_worktree_finalize_failure'); END",
    )
    .execute(reopened.pool())
    .await
    .unwrap();
    let second_operation_id = "00000000-0000-4000-8000-000000001811";
    let failed = client
        .call::<_, Value>(
            Operation::WorktreeCreate,
            &json!({"payload_version":1,"operation_id":second_operation_id,"task_id":second_task_id,"expected_revision":ready["revision"],"base_ref":"HEAD","branch_name":"rt/final-commit-recovery","destination_leaf":"final-commit-recovery","parent":null}),
        )
        .await;
    assert!(failed.is_err());
    let applying: Value = client
        .call(
            Operation::WorktreeGetOperation,
            &json!({"operation_id":second_operation_id}),
        )
        .await
        .unwrap();
    assert_eq!(applying["phase"], "applying");
    assert_eq!(applying["selected"], false);
    assert!(
        worktrees
            .join("final-commit-recovery")
            .join("fixture.txt")
            .exists()
    );
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        3
    );
    sqlx::query("DROP TRIGGER inject_worktree_finalize_failure")
        .execute(reopened.pool())
        .await
        .unwrap();
    let current: Value = client
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    let reconciled: Value = client
        .call(
            Operation::WorktreeReconcile,
            &json!({"operation_id":second_operation_id,"expected_revision":current["revision"]}),
        )
        .await
        .unwrap();
    assert_eq!(reconciled["phase"], "ready");
    assert_eq!(reconciled["selected"], true);
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        3,
        "final-commit recovery must not invoke worktree add again"
    );

    drop(client);
    shutdown.send(true).unwrap();
    running.await.unwrap().unwrap();
    reopened.pool().close().await;
}
