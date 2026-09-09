#[path = "support/m09_native.rs"]
#[allow(dead_code)]
mod native;

use native::{OuterTerminal, admin_output, wait_until};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn temporary() -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        builder
            .prefix("rt10-")
            .tempdir_in(if cfg!(target_os = "macos") {
                "/private/tmp"
            } else {
                "/tmp"
            })
            .unwrap()
    }
    #[cfg(not(unix))]
    {
        builder.prefix("rt10-").tempdir().unwrap()
    }
}

fn command(root: &Path, home: &Path, args: &[&str]) -> Value {
    let output = admin_output(root, home, args);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "rt returned invalid JSON with status {:?}",
            output.status.code()
        )
    });
    assert!(output.status.success(), "rt failed for {args:?}: {value}");
    assert_eq!(value.get("ok").and_then(Value::as_bool), Some(true));
    value.get("result").cloned().unwrap_or(Value::Null)
}

fn rejected(root: &Path, home: &Path, args: &[&str]) {
    let output = admin_output(root, home, args);
    assert!(
        !output.status.success(),
        "rt unexpectedly succeeded for {args:?}"
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value.get("ok").and_then(Value::as_bool), Some(false));
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}

fn create_task(root: &Path, home: &Path, temporary: &Path, suffix: &str) -> (String, String) {
    let snapshot = command(root, home, &["task", "list"]);
    let revision = snapshot["revision"].as_str().unwrap();
    let input = temporary.join(format!("task-{suffix}.json"));
    std::fs::write(&input, json!({"title":format!("Task {suffix}"),"description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]}).to_string()).unwrap();
    let result = command(
        root,
        home,
        &[
            "task",
            "create",
            "--expected-revision",
            revision,
            "--file",
            input.to_str().unwrap(),
        ],
    );
    let task = result["entity_ids"][0].as_str().unwrap().to_owned();
    let revision = result["revision"].as_str().unwrap();
    let result = command(
        root,
        home,
        &[
            "task",
            "transition",
            &task,
            "ready",
            "--expected-revision",
            revision,
        ],
    );
    (task, result["revision"].as_str().unwrap().to_owned())
}

fn create_worktree(
    root: &Path,
    home: &Path,
    task: &str,
    revision: &str,
    suffix: &str,
    operation: &str,
) -> Value {
    let branch = format!("rt/gate-{suffix}");
    let leaf = format!("gate-{suffix}");
    let arguments = [
        "worktree",
        "create",
        task,
        "--expected-revision",
        revision,
        "--operation-id",
        operation,
        "--branch",
        branch.as_str(),
        "--leaf",
        leaf.as_str(),
    ];
    let output = admin_output(root, home, &arguments);
    if !output.status.success() {
        let receipt = admin_output(root, home, &["worktree", "operation", operation]);
        panic!(
            "worktree create failed: response={} receipt={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&receipt.stdout)
        );
    }
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value.get("ok").and_then(Value::as_bool), Some(true));
    value.get("result").cloned().unwrap_or(Value::Null)
}

fn task_items(root: &Path, home: &Path) -> Vec<Value> {
    command(root, home, &["task", "list"])["items"]
        .as_array()
        .unwrap()
        .clone()
}

fn worktree_items(root: &Path, home: &Path) -> Vec<Value> {
    command(root, home, &["worktree", "list"])["items"]
        .as_array()
        .unwrap()
        .clone()
}

fn session_items(root: &Path, home: &Path) -> Vec<Value> {
    command(root, home, &["session", "list"])["items"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn real_tui_creates_and_launches_two_task_worktrees() {
    let started = Instant::now();
    let temporary = temporary();
    let repository = temporary.path().join("tui-repository");
    let home = temporary.path().join("tui-private");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, &["init", "-q"]);
    git(&repository, &["config", "user.name", "Fixture"]);
    git(
        &repository,
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(repository.join("fixture.txt"), "source\n").unwrap();
    git(&repository, &["add", "fixture.txt"]);
    git(&repository, &["commit", "-qm", "fixture"]);

    let mut tui = OuterTerminal::spawn(&repository, &home);
    tui.wait_for("Initialize it?");
    tui.send(b"y\r");
    tui.finish_startup();
    tui.send(b"2");
    tui.wait_for("No tasks. Press n to create one.");
    tui.send(b"nTask one\tFirst isolated task\tnormal\t\tNative worktree gate\t\x13");
    wait_until(
        || task_items(&repository, &home).len() == 1,
        "first TUI task creation",
    );
    tui.send(b"w");
    tui.wait_for("Create worktree form");
    tui.send(b"\x13");
    wait_until(
        || worktree_items(&repository, &home).len() == 1,
        "first TUI worktree creation",
    );

    tui.send(b"nTask two\tSecond isolated task\tnormal\t\tNative worktree gate\t\x13");
    wait_until(
        || task_items(&repository, &home).len() == 2,
        "second TUI task creation",
    );
    tui.send(b"j");
    tui.wait_for("Second isolated task");
    tui.send(b"w");
    tui.wait_for("Create worktree form");
    tui.send(b"\x13");
    wait_until(
        || worktree_items(&repository, &home).len() == 2,
        "second TUI worktree creation",
    );
    let tasks = task_items(&repository, &home);
    let worktrees = worktree_items(&repository, &home);
    for task in &tasks {
        assert!(worktrees.iter().any(|worktree| {
            worktree["task_id"] == task["id"] && task["worktree_id"] == worktree["id"]
        }));
    }

    tui.send(b"a");
    wait_until(
        || session_items(&repository, &home).len() == 1,
        "second task launch through TUI",
    );
    tui.send(b"2k");
    tui.wait_for("First isolated task");
    tui.send(b"a");
    wait_until(
        || session_items(&repository, &home).len() == 2,
        "first task launch through TUI",
    );
    let sessions = session_items(&repository, &home);
    for task in &tasks {
        let instance = sessions
            .iter()
            .find(|instance| instance["task_id"] == task["id"])
            .unwrap();
        assert_eq!(instance["worktree_id"], task["worktree_id"]);
    }
    tui.send(b"\x03");
    tui.wait_exit();
    command(
        &repository,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    );
    eprintln!(
        "M10 TUI gate os={} arch={} elapsed_ms={} tasks=2 worktrees=2 sessions=2",
        std::env::consts::OS,
        std::env::consts::ARCH,
        started.elapsed().as_millis()
    );
}

#[test]
fn two_real_worktrees_are_isolated_and_survive_restart() {
    let started = Instant::now();
    let git_version = Command::new("git").arg("--version").output().unwrap();
    let temporary = temporary();
    let repository = temporary.path().join("repository");
    let home = temporary.path().join("private");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, &["init", "-q"]);
    git(&repository, &["config", "user.name", "Fixture"]);
    git(
        &repository,
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(repository.join("fixture.txt"), "source\n").unwrap();
    git(&repository, &["add", "fixture.txt"]);
    git(&repository, &["commit", "-qm", "fixture"]);
    let source_head = Command::new("git")
        .current_dir(&repository)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap()
        .stdout;
    std::fs::write(repository.join("dirty.txt"), "preserve\n").unwrap();

    command(
        &repository,
        &home,
        &["workspace", "init", "--name", "Worktree gate"],
    );
    let (first_task, first_revision) = create_task(&repository, &home, temporary.path(), "one");
    let first_operation = "00000000-0000-4000-8000-000000001001";
    let first = create_worktree(
        &repository,
        &home,
        &first_task,
        &first_revision,
        "one",
        first_operation,
    );
    assert_eq!(first["phase"], "ready");
    assert_eq!(first["selected"], true);
    let first_id = first["worktree_id"].as_str().unwrap().to_owned();

    let (second_task, second_revision) = create_task(&repository, &home, temporary.path(), "two");
    let second_operation = "00000000-0000-4000-8000-000000001002";
    let second = create_worktree(
        &repository,
        &home,
        &second_task,
        &second_revision,
        "two",
        second_operation,
    );
    let second_id = second["worktree_id"].as_str().unwrap().to_owned();
    assert_ne!(first_id, second_id);

    let page = command(&repository, &home, &["worktree", "list"]);
    let items = page["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let first_path = PathBuf::from(
        items.iter().find(|item| item["id"] == first_id).unwrap()["checkout_display"]
            .as_str()
            .unwrap(),
    );
    let second_path = PathBuf::from(
        items.iter().find(|item| item["id"] == second_id).unwrap()["checkout_display"]
            .as_str()
            .unwrap(),
    );
    std::fs::write(first_path.join("isolated.txt"), "first\n").unwrap();
    std::fs::write(second_path.join("isolated.txt"), "second\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(first_path.join("isolated.txt")).unwrap(),
        "first\n"
    );
    assert_eq!(
        std::fs::read_to_string(second_path.join("isolated.txt")).unwrap(),
        "second\n"
    );
    assert!(!repository.join("isolated.txt").exists());
    assert_eq!(
        std::fs::read_to_string(repository.join("dirty.txt")).unwrap(),
        "preserve\n"
    );
    let current_head = Command::new("git")
        .current_dir(&repository)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap()
        .stdout;
    assert_eq!(source_head, current_head);

    let revision = page["revision"].as_str().unwrap();
    rejected(
        &repository,
        &home,
        &[
            "worktree",
            "select",
            &first_task,
            &second_id,
            "--expected-revision",
            revision,
        ],
    );
    let launch = command(
        &repository,
        &home,
        &["session", "create", "--task-id", &first_task],
    );
    let instance_id = launch["instance_id"].as_str().unwrap();
    let sessions = command(&repository, &home, &["session", "list"]);
    let instance = sessions["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == instance_id)
        .unwrap();
    assert_eq!(instance["worktree_id"], first_id);
    rejected(
        &repository,
        &home,
        &[
            "worktree",
            "clear",
            &first_task,
            "--expected-revision",
            sessions["revision"].as_str().unwrap(),
        ],
    );

    command(
        &repository,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    );
    command(&repository, &home, &["workspace", "open"]);
    let receipt = command(
        &repository,
        &home,
        &["worktree", "operation", first_operation],
    );
    assert_eq!(receipt["phase"], "ready");
    let duplicate = create_worktree(
        &repository,
        &home,
        &first_task,
        &first_revision,
        "one",
        first_operation,
    );
    assert_eq!(duplicate["worktree_id"], first_id);
    assert_eq!(
        command(&repository, &home, &["worktree", "list"])["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    command(
        &repository,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    );

    let nongit = temporary.path().join("nongit");
    let nongit_home = temporary.path().join("nongit-private");
    std::fs::create_dir(&nongit).unwrap();
    command(
        &nongit,
        &nongit_home,
        &["workspace", "init", "--name", "Non Git gate"],
    );
    let inspect = command(&nongit, &nongit_home, &["worktree", "inspect"]);
    assert_eq!(inspect["status"], "not_repository");
    assert_eq!(
        command(&nongit, &nongit_home, &["session", "create"])["status"],
        "running"
    );
    command(
        &nongit,
        &nongit_home,
        &["daemon", "stop", "--terminate-sessions"],
    );
    eprintln!(
        "M10 administrative gate os={} arch={} git={} elapsed_ms={} worktrees=2 restarts=1 nongit=passed",
        std::env::consts::OS,
        std::env::consts::ARCH,
        String::from_utf8_lossy(&git_version.stdout).trim(),
        started.elapsed().as_millis()
    );
}
