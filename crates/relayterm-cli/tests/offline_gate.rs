use serde_json::Value;
use std::{
    fs,
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
        process_has_network_endpoint(probe.id()),
        "the native monitor did not detect its TCP negative control"
    );
    stop_child(&mut probe);

    let root = scratch.0.join("project");
    let home = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
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

    for _ in 0..20 {
        assert!(
            !process_has_network_endpoint(daemon.id()),
            "the Relayterm daemon opened a TCP or UDP endpoint"
        );
        let status = successful(command(&root, &home, &["workspace", "status"]));
        assert_eq!(status["ok"], true);
        thread::sleep(Duration::from_millis(50));
    }
    successful(command(
        &root,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    ));
    wait_child(&mut daemon);
}

fn command(root: &Path, home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(home)
        .args(["--format", "json", "--timeout", "5"])
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .unwrap()
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
    let _ = child.wait();
}

#[cfg(target_os = "linux")]
fn process_has_network_endpoint(process_id: u32) -> bool {
    use std::collections::HashSet;

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
}

#[cfg(target_os = "macos")]
fn process_has_network_endpoint(process_id: u32) -> bool {
    Command::new("lsof")
        .args(["-nP", "-a", "-p", &process_id.to_string(), "-i"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn process_has_network_endpoint(process_id: u32) -> bool {
    let marker = process_id.to_string();
    Command::new("netstat.exe")
        .args(["-ano"])
        .stdin(Stdio::null())
        .output()
        .ok()
        .is_some_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.split_whitespace().next_back() == Some(marker.as_str()))
        })
}
