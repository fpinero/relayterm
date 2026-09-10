use serde_json::Value;
use std::{
    fs,
    io::{BufRead, Read},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(30);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        #[cfg(unix)]
        let base = PathBuf::from(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        });
        #[cfg(windows)]
        let base = std::env::temp_dir();
        let path = base.join(format!(
            "rt11-offline-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
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
#[ignore]
fn synthetic_network_probe() {
    let ready = PathBuf::from(std::env::var_os("RELAYTERM_OFFLINE_PROBE_READY").unwrap());
    let _listener = TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(ready, b"ready").unwrap();
    thread::sleep(DEADLINE);
}

#[test]
#[ignore]
fn synthetic_offline_agent() {
    let ready = PathBuf::from(std::env::var_os("RELAYTERM_OFFLINE_AGENT_READY").unwrap());
    let stop = PathBuf::from(std::env::var_os("RELAYTERM_OFFLINE_AGENT_STOP").unwrap());
    fs::write(ready, std::process::id().to_string()).unwrap();
    wait_until(|| stop.exists(), "offline agent stop marker");
}

#[test]
fn daemon_uses_local_ipc_without_network_endpoints() {
    let scratch = Scratch::new();
    let probe_ready = scratch.0.join("probe.ready");
    let mut probe = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "synthetic_network_probe",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RELAYTERM_OFFLINE_PROBE_READY", &probe_ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_until(|| probe_ready.exists(), "network monitor probe readiness");
    assert!(
        processes_have_network_endpoint(&[probe.id()]),
        "the native monitor did not detect its TCP negative control"
    );
    stop_child(&mut probe);
    eprintln!("M11 offline phase=negative-control-complete");

    let root = scratch.0.join("project");
    let home = scratch.0.join("private");
    let agent_ready = scratch.0.join("agent.ready");
    let agent_stop = scratch.0.join("agent.stop");
    fs::create_dir(&root).unwrap();
    run_git(&root, &["init", "-q"]);
    run_git(&root, &["config", "user.name", "Fixture"]);
    run_git(&root, &["config", "user.email", "fixture@example.invalid"]);
    fs::write(root.join("fixture.txt"), b"offline fixture\n").unwrap();
    run_git(&root, &["add", "fixture.txt"]);
    run_git(&root, &["commit", "-qm", "offline fixture"]);
    let initialized = successful(command(
        &root,
        &home,
        &["workspace", "init", "--name", "Offline fixture"],
    ));
    let workspace_id = initialized["result"]["workspace_id"]
        .as_str()
        .unwrap()
        .to_owned();
    successful(command(
        &root,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    ));

    let mut daemon = Command::new(env!("CARGO_BIN_EXE_rt"))
        .args(["--workspace"])
        .arg(&root)
        .args(["--home"])
        .arg(&home)
        .args(["__daemon-run", "--root"])
        .arg(&root)
        .args(["--workspace-id", &workspace_id, "--private-home"])
        .arg(&home)
        .env("RELAYTERM_OFFLINE_AGENT_READY", &agent_ready)
        .env("RELAYTERM_OFFLINE_AGENT_STOP", &agent_stop)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_until(
        || {
            command(&root, &home, &["daemon", "status"])
                .status
                .success()
        },
        "foreground daemon readiness",
    );
    eprintln!("M11 offline phase=daemon-ready");

    let status = successful(command(&root, &home, &["workspace", "status"]));
    let revision = status["result"]["revision"].as_str().unwrap();
    let task_file = scratch.0.join("offline-task.json");
    fs::write(
        &task_file,
        serde_json::to_vec(&serde_json::json!({
            "title":"Offline worktree task",
            "description":"",
            "priority":"normal",
            "scope_paths":[],
            "acceptance_notes":"",
            "dependency_ids":[]
        }))
        .unwrap(),
    )
    .unwrap();
    let task = successful(command(
        &root,
        &home,
        &[
            "task",
            "create",
            "--expected-revision",
            revision,
            "--file",
            task_file.to_str().unwrap(),
        ],
    ));
    let task_id = task["result"]["entity_ids"][0].as_str().unwrap();
    let ready = successful(command(
        &root,
        &home,
        &[
            "task",
            "transition",
            task_id,
            "ready",
            "--expected-revision",
            task["result"]["revision"].as_str().unwrap(),
        ],
    ));
    let operation_id = "00000000-0000-4000-8000-000000001814";
    let worktree = successful(command(
        &root,
        &home,
        &[
            "worktree",
            "create",
            task_id,
            "--expected-revision",
            ready["result"]["revision"].as_str().unwrap(),
            "--operation-id",
            operation_id,
            "--branch",
            "rt/offline-gate",
            "--leaf",
            "offline-gate",
        ],
    ));
    assert_eq!(worktree["result"]["phase"], "ready");
    let status = successful(command(&root, &home, &["workspace", "status"]));
    let revision = status["result"]["revision"].as_str().unwrap();
    let definition = scratch.0.join("offline-agent.json");
    fs::write(
        &definition,
        serde_json::to_vec(&serde_json::json!({
            "display_name":"Offline fixture",
            "command":std::env::current_exe().unwrap().to_string_lossy(),
            "arguments":["--ignored","--exact","synthetic_offline_agent","--nocapture","--test-threads=1"],
            "environment_allowlist":["RELAYTERM_OFFLINE_AGENT_READY","RELAYTERM_OFFLINE_AGENT_STOP"],
            "capabilities":["interactive_terminal"],
            "enabled":true
        }))
        .unwrap(),
    )
    .unwrap();
    let registered = successful(command(
        &root,
        &home,
        &[
            "agent",
            "register",
            "--expected-revision",
            revision,
            "--file",
            definition.to_str().unwrap(),
        ],
    ));
    let definition_id = registered["result"]["entity_ids"][0].as_str().unwrap();
    let created = successful(command(
        &root,
        &home,
        &["session", "create", "--definition-id", definition_id],
    ));
    let session_id = created["result"]["session_id"].as_str().unwrap();
    wait_until(|| agent_ready.exists(), "offline agent readiness");
    let agent_pid = fs::read_to_string(&agent_ready)
        .unwrap()
        .parse::<u32>()
        .unwrap();

    for index in 0..20 {
        if !cfg!(windows) || matches!(index, 0 | 9 | 19) {
            assert!(
                !processes_have_network_endpoint(&[daemon.id(), agent_pid]),
                "the Relayterm daemon or synthetic agent opened a TCP or UDP endpoint"
            );
        }
        let status = successful(command(&root, &home, &["workspace", "status"]));
        assert_eq!(status["ok"], true);
        thread::sleep(Duration::from_millis(50));
    }
    eprintln!("M11 offline phase=runtime-observation-complete");
    fs::write(&agent_stop, b"stop").unwrap();
    wait_until(
        || {
            successful(command(&root, &home, &["session", "list"]))["result"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["session_id"] == session_id && item["status"] == "exited")
        },
        "offline agent exit observation",
    );
    let backup = home.join("data").join("b");
    let restored = home.join("data").join("r");
    successful(command(
        &root,
        &home,
        &[
            "backup",
            "create",
            "--destination",
            backup.to_str().unwrap(),
        ],
    ));
    eprintln!("M11 offline phase=backup-complete");
    assert!(!processes_have_network_endpoint(&[daemon.id()]));
    successful(command(
        &root,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    ));
    wait_child(&mut daemon);
    successful(command(
        &root,
        &home,
        &[
            "backup",
            "restore",
            "--source",
            backup.to_str().unwrap(),
            "--destination",
            restored.to_str().unwrap(),
        ],
    ));
    let restored_status = successful(command(&root, &restored, &["workspace", "open"]));
    assert_eq!(restored_status["result"]["workspace_id"], workspace_id);
    successful(command(
        &root,
        &restored,
        &["daemon", "stop", "--terminate-sessions"],
    ));
    eprintln!("M11 offline phase=restore-complete");
}

fn run_git(root: &Path, arguments: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}

fn command(root: &Path, home: &Path, arguments: &[&str]) -> Output {
    const OUTPUT_LIMIT: usize = 64 * 1024;
    let mut child = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(home)
        .args(["--format", "json", "--timeout", "5"])
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = std::io::BufReader::new(stdout)
            .take((OUTPUT_LIMIT + 1) as u64)
            .read_until(b'\n', &mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    let deadline = Instant::now() + DEADLINE;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            stop_child(&mut child);
            panic!("offline command exceeded its deadline");
        }
        thread::sleep(Duration::from_millis(20));
    };
    let stdout = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("offline command output did not close before its deadline")
        .expect("offline command output could not be read");
    assert!(stdout.len() <= OUTPUT_LIMIT);
    Output {
        status,
        stdout,
        stderr: Vec::new(),
    }
}

fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "offline command failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn wait_until(mut condition: impl FnMut() -> bool, label: &str) {
    let deadline = Instant::now() + DEADLINE;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_child(child: &mut Child) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "foreground daemon exited unsuccessfully");
            return;
        }
        if Instant::now() >= deadline {
            stop_child(child);
            panic!("foreground daemon did not stop before its deadline");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "fixture child did not terminate after kill"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(target_os = "linux")]
fn processes_have_network_endpoint(process_ids: &[u32]) -> bool {
    use std::collections::HashSet;

    process_ids.iter().any(|process_id| {
        let descriptors = PathBuf::from(format!("/proc/{process_id}/fd"));
        let inodes = fs::read_dir(descriptors)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| fs::read_link(entry.path()).ok())
            .filter_map(|target| {
                let value = target.to_string_lossy();
                value
                    .strip_prefix("socket:[")
                    .and_then(|value| value.strip_suffix(']'))
                    .map(str::to_owned)
            })
            .collect::<HashSet<_>>();
        ["tcp", "tcp6", "udp", "udp6"].into_iter().any(|name| {
            fs::read_to_string(format!("/proc/{process_id}/net/{name}"))
                .ok()
                .is_some_and(|table| {
                    table
                        .lines()
                        .skip(1)
                        .filter_map(|line| line.split_whitespace().nth(9))
                        .any(|inode| inodes.contains(inode))
                })
        })
    })
}

#[cfg(target_os = "macos")]
fn processes_have_network_endpoint(process_ids: &[u32]) -> bool {
    Command::new("lsof")
        .args([
            "-nP",
            "-a",
            "-p",
            &process_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
            "-i",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn processes_have_network_endpoint(process_ids: &[u32]) -> bool {
    static NEXT_PROBE: AtomicU64 = AtomicU64::new(1);
    let prefix = std::env::temp_dir().join(format!(
        "rt11-netstat-{}-{}",
        std::process::id(),
        NEXT_PROBE.fetch_add(1, Ordering::Relaxed)
    ));
    ["tcp", "udp"].into_iter().any(|protocol| {
        let output = prefix.with_extension(protocol);
        let writer = fs::File::create(&output).expect("native netstat output could not be created");
        let mut monitor = Command::new("netstat.exe")
            .args(["-ano", "-p", protocol])
            .stdin(Stdio::null())
            .stdout(writer)
            .stderr(Stdio::null())
            .spawn()
            .expect("native Windows netstat monitor could not start");
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = monitor.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                stop_child(&mut monitor);
                panic!("native Windows netstat monitor exceeded its deadline");
            }
            thread::sleep(Duration::from_millis(20));
        };
        assert!(status.success(), "native Windows netstat monitor failed");
        let table = fs::read_to_string(&output).expect("native netstat output was not written");
        let _ = fs::remove_file(output);
        table.lines().any(|line| {
            line.split_whitespace()
                .next_back()
                .and_then(|value| value.parse::<u32>().ok())
                .is_some_and(|owner| process_ids.contains(&owner))
        })
    })
}
