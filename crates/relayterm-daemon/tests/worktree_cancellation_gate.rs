#![cfg(feature = "test-hooks")]

use relayterm_application::{EventNotifier, Service};
use relayterm_client::Client;
use relayterm_daemon::WorkspaceServer;
use relayterm_domain::WorkspaceId;
use relayterm_ipc::Endpoint;
use relayterm_persistence_sqlite::{Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore};
use relayterm_platform::{RandomIdGenerator, SystemClock, create_private_dir};
use relayterm_protocol::{Operation, WorkspaceId as WireWorkspaceId};
use serde_json::{Value, json};
use std::{path::Path, process::Command, sync::Arc, time::Duration};
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
async fn lost_client_during_git_add_can_query_the_stable_receipt_without_repeating_git() {
    #[cfg(unix)]
    let temporary = tempfile::Builder::new()
        .prefix("rt11g-")
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
    let add_ready = temporary.path().join("add.ready");
    let add_release = temporary.path().join("add.release");
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

    let database = Database::open(
        &private.join("workspace.db"),
        DatabaseKind::Workspace,
        OpenMode::ExplicitNew,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(database.pool().clone());
    let workspace: WorkspaceId = "00000000-0000-4000-8000-000000000811".parse().unwrap();
    let service = Arc::new(Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        Notify,
    ));
    service
        .create_workspace_reserved(workspace, "Cancellation fixture".into(), project.clone())
        .await
        .unwrap();
    let wire_workspace = WireWorkspaceId::from_uuid(workspace.as_uuid());
    let endpoint = Endpoint::derive(&runtime, wire_workspace).unwrap();
    let git_adapter =
        relayterm_git::Git::default().with_add_barrier(add_ready.clone(), add_release.clone());
    let server = WorkspaceServer::bind(workspace, &endpoint, service, store)
        .await
        .unwrap()
        .with_worktrees(worktrees.clone())
        .with_git(git_adapter);
    let faults = server.fault_injector();
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
            &json!({"expected_revision":snapshot["revision"],"title":"Cancelled caller","description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]}),
        )
        .await
        .unwrap();
    let task_id = created["entity_ids"][0].as_str().unwrap().to_owned();
    let ready: Value = client
        .call(
            Operation::TaskTransition,
            &json!({"task_id":task_id,"expected_revision":created["revision"],"status":"ready"}),
        )
        .await
        .unwrap();
    let operation_id = "00000000-0000-4000-8000-000000001812";
    let request = json!({"payload_version":1,"operation_id":operation_id,"task_id":task_id,"expected_revision":ready["revision"],"base_ref":"HEAD","branch_name":"rt/cancelled-caller","destination_leaf":"cancelled-caller","parent":null});
    let mutation = tokio::spawn({
        let request = request.clone();
        async move {
            client
                .call::<_, Value>(Operation::WorktreeCreate, &request)
                .await
        }
    });
    wait_for_file(&add_ready).await;
    mutation.abort();
    assert!(mutation.await.unwrap_err().is_cancelled());
    std::fs::write(&add_release, b"release").unwrap();

    let observer = Client::connect(&endpoint, wire_workspace).await.unwrap();
    let receipt = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Ok(receipt) = observer
                .call::<_, Value>(
                    Operation::WorktreeGetOperation,
                    &json!({"operation_id":operation_id}),
                )
                .await
                && receipt["phase"] == "ready"
            {
                break receipt;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the admitted Git operation did not finish after client loss");
    assert_eq!(receipt["operation_id"], operation_id);
    assert_eq!(receipt["selected"], true);
    assert!(
        worktrees
            .join("cancelled-caller")
            .join("fixture.txt")
            .exists()
    );
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        2
    );

    let repeated: Value = observer
        .call(Operation::WorktreeCreate, &request)
        .await
        .unwrap();
    assert_eq!(repeated, receipt);
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        2,
        "querying and resubmitting the stable operation ID must not repeat git worktree add"
    );

    let snapshot: Value = observer
        .call(
            Operation::WorkspaceGetSnapshot,
            &json!({"collection":"tasks","limit":50}),
        )
        .await
        .unwrap();
    let created: Value = observer
        .call(
            Operation::TaskCreate,
            &json!({"expected_revision":snapshot["revision"],"title":"Cancelled finalization","description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]}),
        )
        .await
        .unwrap();
    let second_task = created["entity_ids"][0].as_str().unwrap().to_owned();
    let ready: Value = observer
        .call(
            Operation::TaskTransition,
            &json!({"task_id":second_task,"expected_revision":created["revision"],"status":"ready"}),
        )
        .await
        .unwrap();
    let second_operation = "00000000-0000-4000-8000-000000001813";
    let second_request = json!({"payload_version":1,"operation_id":second_operation,"task_id":second_task,"expected_revision":ready["revision"],"base_ref":"HEAD","branch_name":"rt/cancelled-finalization","destination_leaf":"cancelled-finalization","parent":null});
    faults.pause_next_worktree_after_git();
    let caller = Client::connect(&endpoint, wire_workspace).await.unwrap();
    let mutation = tokio::spawn({
        let request = second_request.clone();
        async move {
            caller
                .call::<_, Value>(Operation::WorktreeCreate, &request)
                .await
        }
    });
    tokio::time::timeout(
        Duration::from_secs(30),
        faults.wait_until_worktree_after_git_paused(),
    )
    .await
    .expect("timed out waiting for the post-Git barrier");
    assert!(
        worktrees
            .join("cancelled-finalization")
            .join("fixture.txt")
            .exists(),
        "the post-Git barrier must acknowledge the completed external effect"
    );
    let applying: Value = observer
        .call(
            Operation::WorktreeGetOperation,
            &json!({"operation_id":second_operation}),
        )
        .await
        .unwrap();
    assert_eq!(applying["phase"], "applying");
    assert_eq!(applying["selected"], false);
    mutation.abort();
    assert!(mutation.await.unwrap_err().is_cancelled());
    faults.release_worktree_after_git();

    let finalized = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Ok(receipt) = observer
                .call::<_, Value>(
                    Operation::WorktreeGetOperation,
                    &json!({"operation_id":second_operation}),
                )
                .await
                && receipt["phase"] == "ready"
            {
                break receipt;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the post-Git operation did not finalize after client loss");
    assert_eq!(finalized["selected"], true);
    let repeated: Value = observer
        .call(Operation::WorktreeCreate, &second_request)
        .await
        .unwrap();
    assert_eq!(repeated, finalized);
    assert_eq!(
        relayterm_git::Git::default().list(&project).unwrap().len(),
        3,
        "post-Git recovery must not repeat either worktree creation"
    );

    drop(observer);
    shutdown.send(true).unwrap();
    running.await.unwrap().unwrap();
    database.pool().close().await;
}

async fn wait_for_file(path: &Path) {
    tokio::time::timeout(Duration::from_secs(30), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("timed out waiting for the pre-add barrier");
}
