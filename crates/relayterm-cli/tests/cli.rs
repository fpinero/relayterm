use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
        #[cfg(unix)]
        let base = PathBuf::from(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        });
        #[cfg(not(unix))]
        let base = std::env::temp_dir();
        let path = base.join(format!("rt5-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn command_contracts_have_no_runtime_side_effects() {
    let cases: &[(&[&str], i32, &str)] = &[
        (&["--help"], 0, "Usage: rt"),
        (&["--version"], 0, concat!("rt ", env!("CARGO_PKG_VERSION"))),
        (&["daemon", "--help"], 0, "Usage: rt daemon"),
        (&[], 1, "requires an interactive input and output terminal"),
        (&["daemon"], 2, "command arguments are invalid"),
        (&["--unknown-option"], 2, "command arguments are invalid"),
    ];
    for (args, code, expected) in cases {
        let scratch = Scratch::new();
        let private = scratch.0.join("private");
        fs::create_dir(&private).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_rt"))
            .args(*args)
            .current_dir(&scratch.0)
            .env("RELAYTERM_TEST_MARKER", "test-token-not-secret")
            .env("XDG_DATA_HOME", &private)
            .env("XDG_CONFIG_HOME", &private)
            .env("XDG_RUNTIME_DIR", &private)
            .env("LOCALAPPDATA", &private)
            .env("APPDATA", &private)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("CLI did not terminate promptly for {args:?}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(*code), "{args:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let combined = format!("{stdout}{stderr}");
        assert!(combined.contains(expected), "{args:?}: {combined}");
        if *code == 0 {
            assert!(stderr.is_empty());
        } else {
            assert!(stdout.is_empty());
        }
        assert!(!combined.contains("test-token-not-secret"));
        assert!(!combined.contains(scratch.0.to_str().unwrap()));
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
        assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
    }
}

#[test]
fn status_of_unknown_workspace_is_non_creating() {
    let scratch = Scratch::new();
    let root = scratch.0.join("project");
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    let output = invoke(
        env!("CARGO_BIN_EXE_rt"),
        &root,
        &private,
        &["daemon", "status"],
    );
    assert_eq!(output.status.code(), Some(3));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "workspace_not_initialized");
    assert!(!private.exists());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn private_backup_restores_into_a_fresh_home_without_overwriting() {
    let scratch = Scratch::new();
    let root = scratch.0.join("project");
    let private = scratch.0.join("private");
    let restored = private.join("data").join("r");
    fs::create_dir(&root).unwrap();

    let initialized = invoke(
        env!("CARGO_BIN_EXE_rt"),
        &root,
        &private,
        &["workspace", "init", "--name", "Synthetic workspace"],
    );
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stdout)
    );
    let initialized: Value = serde_json::from_slice(&initialized.stdout).unwrap();
    let workspace_id = initialized["result"]["workspace_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let busy_backup = private.join("data").join("busy-backup");
    let busy = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .arg("--home")
        .arg(&private)
        .args([
            "--format",
            "json",
            "--timeout",
            "1",
            "backup",
            "create",
            "--destination",
        ])
        .arg(&busy_backup)
        .output()
        .unwrap();
    assert_eq!(busy.status.code(), Some(1));
    let busy: Value = serde_json::from_slice(&busy.stdout).unwrap();
    assert_eq!(busy["error"]["code"], "workspace_busy");
    assert!(!busy_backup.exists());
    assert!(
        invoke(
            env!("CARGO_BIN_EXE_rt"),
            &root,
            &private,
            &["daemon", "stop", "--terminate-sessions"],
        )
        .status
        .success()
    );

    let backup = private.join("data").join("backup-one");
    let created = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .arg("--home")
        .arg(&private)
        .args(["--format", "json", "backup", "create", "--destination"])
        .arg(&backup)
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stdout)
    );
    let value: Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(value["result"]["workspace_id"], workspace_id);
    assert!(backup.join("manifest.json").is_file());
    assert!(backup.join("workspace.sqlite3").is_file());

    let second = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .arg("--home")
        .arg(&private)
        .args(["--format", "json", "backup", "create", "--destination"])
        .arg(&backup)
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(1));
    let second: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second["error"]["code"], "invalid_location");

    let restored_output = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .args(["--format", "json", "backup", "restore", "--source"])
        .arg(&backup)
        .arg("--destination")
        .arg(&restored)
        .output()
        .unwrap();
    assert!(
        restored_output.status.success(),
        "{}",
        String::from_utf8_lossy(&restored_output.stdout)
    );
    let value: Value = serde_json::from_slice(&restored_output.stdout).unwrap();
    assert_eq!(value["result"]["workspace_id"], workspace_id);

    let opened = invoke(
        env!("CARGO_BIN_EXE_rt"),
        &root,
        &restored,
        &["workspace", "open"],
    );
    assert!(opened.status.success());
    let value: Value = serde_json::from_slice(&opened.stdout).unwrap();
    assert_eq!(value["result"]["workspace_id"], workspace_id);
    assert!(
        invoke(
            env!("CARGO_BIN_EXE_rt"),
            &root,
            &restored,
            &["daemon", "stop", "--terminate-sessions"],
        )
        .status
        .success()
    );

    let mut damaged = fs::OpenOptions::new()
        .append(true)
        .open(backup.join("workspace.sqlite3"))
        .unwrap();
    damaged.write_all(b"synthetic-damage").unwrap();
    damaged.sync_all().unwrap();
    let rejected_home = private.join("data").join("x");
    let rejected = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .args(["--format", "json", "backup", "restore", "--source"])
        .arg(&backup)
        .arg("--destination")
        .arg(&rejected_home)
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(6));
    let rejected: Value = serde_json::from_slice(&rejected.stdout).unwrap();
    assert_eq!(rejected["error"]["code"], "recovery_required");
    assert!(!rejected_home.exists());
}

#[test]
fn bootstrap_rejects_invalid_input_without_side_effects() {
    for input in [
        b"{".to_vec(),
        br#"{"schema_version":1,"unknown":true}"#.to_vec(),
        vec![b'x'; 64 * 1024 + 1],
    ] {
        let scratch = Scratch::new();
        let output = invoke_raw_stdin(
            env!("CARGO_BIN_EXE_rt"),
            &["__bootstrap", "--format", "json"],
            &input,
            &scratch.0,
        );
        assert_eq!(output.status.code(), Some(2));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], "invalid_input");
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
    }
}

fn invoke(binary: &str, root: &PathBuf, private: &PathBuf, args: &[&str]) -> std::process::Output {
    let child = Command::new(binary)
        .args(["--workspace"])
        .arg(root)
        .args(["--home"])
        .arg(private)
        .args(["--format", "json"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    finish_line_after_exit(child, "CLI command")
}

fn invoke_raw_stdin(
    binary: &str,
    args: &[&str],
    input: &[u8],
    current_dir: &PathBuf,
) -> std::process::Output {
    Command::new(binary)
        .args(args)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(input)?;
            child.wait_with_output()
        })
        .unwrap()
}

#[cfg(unix)]
fn initialize_from_disposable_shell(
    binary: &str,
    root: &PathBuf,
    private: &PathBuf,
) -> std::process::Output {
    let child = Command::new("sh")
        .args([
            "-c",
            "exec \"$RT_TEST_BIN\" --workspace \"$RT_TEST_ROOT\" --home \"$RT_TEST_HOME\" --format json workspace init --name \"Synthetic workspace\"",
        ])
        .env("RT_TEST_BIN", binary)
        .env("RT_TEST_ROOT", root)
        .env("RT_TEST_HOME", private)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    finish_line_after_exit(child, "Unix launcher shell")
}

#[cfg(windows)]
fn initialize_from_disposable_shell(
    binary: &str,
    root: &PathBuf,
    private: &PathBuf,
) -> std::process::Output {
    let child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "& $env:RT_TEST_BIN --workspace $env:RT_TEST_ROOT --home $env:RT_TEST_HOME --format json workspace init --name 'Synthetic workspace'",
        ])
        .env("RT_TEST_BIN", binary)
        .env("RT_TEST_ROOT", root)
        .env("RT_TEST_HOME", private)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    finish_line_after_exit(child, "PowerShell launcher")
}

fn finish_line_after_exit(mut child: std::process::Child, stage: &str) -> std::process::Output {
    let deadline = Instant::now() + Duration::from_secs(30);
    let stdout = child.stdout.take().unwrap();
    let (output_tx, output_rx) = std::sync::mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut output = Vec::new();
        let result = BufReader::new(stdout)
            .take((64 * 1024 + 1) as u64)
            .read_until(b'\n', &mut output)
            .map(|_| output);
        let _ = output_tx.send(result);
    });
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{stage} did not exit independently");
        }
        thread::sleep(Duration::from_millis(10));
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    let stdout = output_rx
        .recv_timeout(remaining)
        .expect("CLI output did not close before its deadline")
        .unwrap();
    assert!(
        stdout.len() <= 64 * 1024,
        "{stage} output exceeded its bound"
    );
    std::process::Output {
        status,
        stdout,
        stderr: Vec::new(),
    }
}

fn result(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

#[test]
fn detached_daemon_survives_starters_and_reopens_durable_state() {
    let scratch = Scratch::new();
    let root = scratch.0.join("project with spaces \u{03a9}");
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    let binary = env!("CARGO_BIN_EXE_rt");

    eprintln!("stage: initialize from disposable shell");
    let initialized = result(initialize_from_disposable_shell(binary, &root, &private));
    eprintln!("stage: initialized");
    let workspace_id = initialized["workspace_id"].as_str().unwrap().to_owned();
    assert_eq!(initialized["started"], true);
    assert_eq!(initialized["already_initialized"], false);

    let first_status = result(invoke(binary, &root, &private, &["daemon", "status"]));
    eprintln!("stage: first status");
    assert_eq!(first_status["lifecycle"], "ready");
    let first_generation = first_status["generation"].as_str().unwrap().to_owned();

    let task = Command::new(binary)
        .args(["--workspace"]).arg(&root)
        .args(["--home"]).arg(&private)
        .args([
            "--format",
            "json",
            "task",
            "create",
            "--expected-revision",
            "1",
            "--stdin",
        ])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().and_then(|mut child| {
            child.stdin.take().unwrap().write_all(br#"{"title":"Synthetic task","description":"Disposable","priority":"normal","scope_paths":[],"acceptance_notes":"Persist","dependency_ids":[]}"#)?;
            child.wait_with_output()
        }).unwrap();
    let task = result(task);
    eprintln!("stage: task created");
    assert_eq!(task["revision"], "2");

    let duplicate = Command::new(binary)
        .args(["--workspace"])
        .arg(&root)
        .args(["--home"])
        .arg(&private)
        .args([
            "--format",
            "json",
            "task",
            "create",
            "--expected-revision",
            "2",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(br#"{"title":"first","title":"second","description":"","priority":"normal","scope_paths":[],"acceptance_notes":"","dependency_ids":[]}"#)?;
            child.wait_with_output()
        })
        .unwrap();
    assert_eq!(duplicate.status.code(), Some(2));
    eprintln!("stage: duplicate rejected");
    assert_eq!(
        result(invoke(binary, &root, &private, &["daemon", "status"]))["revision"],
        "2"
    );

    let reused = result(invoke(binary, &root, &private, &["daemon", "start"]));
    eprintln!("stage: live daemon reused");
    assert_eq!(reused["already_running"], true);
    assert_eq!(reused["started"], false);

    let stopped = result(invoke(binary, &root, &private, &["daemon", "stop"]));
    eprintln!("stage: first daemon stopped");
    assert_eq!(stopped["lifecycle"], "stopped");
    let (first_start, second_start) = thread::scope(|scope| {
        let first = scope.spawn(|| invoke(binary, &root, &private, &["daemon", "start"]));
        let second = scope.spawn(|| invoke(binary, &root, &private, &["daemon", "start"]));
        (first.join().unwrap(), second.join().unwrap())
    });
    eprintln!("stage: concurrent starts completed");
    assert_eq!(result(first_start)["workspace_id"], workspace_id);
    assert_eq!(result(second_start)["workspace_id"], workspace_id);
    let second_status = result(invoke(binary, &root, &private, &["daemon", "status"]));
    assert_eq!(second_status["revision"], "2");
    assert_ne!(second_status["generation"], first_generation);
    assert!(
        result(invoke(binary, &root, &private, &["task", "list"]))["items"]
            .as_array()
            .is_some_and(|items| items.len() == 1)
    );
    result(invoke(binary, &root, &private, &["daemon", "stop"]));
    eprintln!("stage: restarted daemon stopped");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);

    let git_root = scratch.0.join("git project");
    fs::create_dir(&git_root).unwrap();
    fs::create_dir(git_root.join(".git")).unwrap();
    let git_workspace = result(invoke(
        binary,
        &git_root,
        &private,
        &["workspace", "init", "--name", "Synthetic Git workspace"],
    ));
    eprintln!("stage: Git workspace initialized");
    assert_ne!(git_workspace["workspace_id"], workspace_id);
    result(invoke(binary, &git_root, &private, &["daemon", "stop"]));
    eprintln!("stage: Git workspace daemon stopped");
    assert_eq!(
        fs::read_dir(&git_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        vec![std::ffi::OsString::from(".git")]
    );
}
