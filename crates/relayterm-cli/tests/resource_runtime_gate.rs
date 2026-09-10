#[path = "support/m09_native.rs"]
#[allow(dead_code)]
mod native;

use native::{
    DaemonCleanup, OuterTerminal, Scratch, admin, admin_output, next_selection_input, wait_until,
};
use relayterm_terminal::DEFAULT_SCROLLBACK_BYTES;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const SAMPLE_DURATION: Duration = Duration::from_secs(180);
const WARMUP: Duration = Duration::from_secs(60);
const MEMORY_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
const PLATEAU_ALLOWANCE_BYTES: u64 = 32 * 1024 * 1024;
const SESSION_COUNT: usize = 8;
const BOUNDED_SESSION_OUTPUT: u64 = 3 * DEFAULT_SCROLLBACK_BYTES as u64;

#[test]
#[ignore]
fn synthetic_resource_flood() {
    let stop = PathBuf::from(std::env::var_os("RELAYTERM_RESOURCE_STOP").unwrap());
    let reports = PathBuf::from(std::env::var_os("RELAYTERM_RESOURCE_REPORT").unwrap());
    let processes = PathBuf::from(std::env::var_os("RELAYTERM_RESOURCE_PROCESS").unwrap());
    let index = (0..SESSION_COUNT)
        .find(|index| {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(processes.join(index.to_string()))
                .and_then(|mut file| write!(file, "{}", std::process::id()))
                .is_ok()
        })
        .expect("resource fixture could not reserve a session index");
    let chunk = vec![b'x'; 256 * 1024];
    let mut output = std::io::stdout().lock();
    let started = Instant::now();
    let mut bytes = 0_u64;
    while !stop.exists() && (index == 0 || bytes < BOUNDED_SESSION_OUTPUT) {
        output.write_all(&chunk).unwrap();
        output.write_all(b"\r\nresource-frame\r\n").unwrap();
        output.flush().unwrap();
        bytes += u64::try_from(chunk.len() + b"\r\nresource-frame\r\n".len()).unwrap();
    }
    fs::write(
        reports.join(index.to_string()),
        format!("{bytes} {}", started.elapsed().as_millis()),
    )
    .unwrap();
    while index != 0 && !stop.exists() {
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore]
fn sustained_output_memory_and_reconnect_resources_are_bounded() {
    let scratch = Scratch::new();
    let root = scratch.0.join("project");
    let home = scratch.0.join("private");
    let stop = scratch.0.join("stop-flood");
    let reports = scratch.0.join("flood-reports");
    let processes = scratch.0.join("flood-processes");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&reports).unwrap();
    fs::create_dir(&processes).unwrap();

    let initialized = successful(admin(
        &root,
        &home,
        &["workspace", "init", "--name", "Resource fixture"],
    ));
    let workspace_id = initialized["workspace_id"].as_str().unwrap().to_owned();
    successful(admin(
        &root,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    ));

    let mut daemon = foreground_daemon(&root, &home, &workspace_id, &stop, &reports, &processes);
    let _cleanup = DaemonCleanup::new(&root, &home);
    wait_until(
        || {
            admin_output(&root, &home, &["daemon", "status"])
                .status
                .success()
        },
        "foreground daemon readiness",
    );

    let revision = successful(admin(&root, &home, &["workspace", "status"]))["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let definition_file = scratch.0.join("definition.json");
    fs::write(
        &definition_file,
        serde_json::to_vec(&json!({
            "display_name":"Resource fixture",
            "command":std::env::current_exe().unwrap().to_string_lossy(),
            "arguments":["--ignored","--exact","synthetic_resource_flood","--nocapture","--test-threads=1"],
            "environment_allowlist":["RELAYTERM_RESOURCE_STOP","RELAYTERM_RESOURCE_REPORT","RELAYTERM_RESOURCE_PROCESS"],
            "capabilities":["interactive_terminal"],
            "enabled":true
        }))
        .unwrap(),
    )
    .unwrap();
    let registered = successful(admin(
        &root,
        &home,
        &[
            "agent",
            "register",
            "--expected-revision",
            &revision,
            "--file",
            definition_file.to_str().unwrap(),
        ],
    ));
    let definition_id = registered["entity_ids"][0].as_str().unwrap();

    let mut session_ids = Vec::with_capacity(SESSION_COUNT);
    for expected in 1..=SESSION_COUNT {
        let mut session_command = command(&root, &home);
        session_command
            .args([
                "session",
                "create",
                "--definition-id",
                definition_id,
                "--rows",
                "40",
                "--columns",
                "120",
            ])
            .env("RELAYTERM_RESOURCE_STOP", &stop);
        let session = successful_output(session_command.output().unwrap());
        session_ids.push(session["session_id"].as_str().unwrap().to_owned());
        wait_until(
            || fs::read_dir(&processes).unwrap().count() == expected,
            "resource child process identifier",
        );
    }
    wait_until(
        || {
            successful(admin(&root, &home, &["session", "list"]))["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|item| item["status"] == "running")
                .count()
                == SESSION_COUNT
        },
        "resource session readiness",
    );
    wait_until(
        || fs::read_dir(&reports).unwrap().count() == SESSION_COUNT - 1,
        "bounded session output completion",
    );
    let ninth = command(&root, &home)
        .args(["session", "create", "--definition-id", definition_id])
        .output()
        .unwrap();
    assert!(
        !ninth.status.success(),
        "the ninth session must be rejected before spawn"
    );
    assert_eq!(fs::read_dir(&processes).unwrap().count(), SESSION_COUNT);
    let listed_sessions = successful(admin(&root, &home, &["session", "list"]))["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        listed_sessions
            .iter()
            .filter(|item| item["status"] == "running")
            .count(),
        SESSION_COUNT
    );
    let child_pids = (0..SESSION_COUNT)
        .map(|index| {
            fs::read_to_string(processes.join(index.to_string()))
                .unwrap()
                .parse::<u32>()
                .unwrap()
        })
        .collect::<Vec<_>>();

    let mut tui = OuterTerminal::spawn(&root, &home);
    tui.finish_startup();
    tui.send(b"3");
    tui.wait_for("Sessions selected");
    select_rendered_running_session(&mut tui, listed_sessions.len());
    tui.send(b"\r");
    tui.wait_for("Terminal");

    let daemon_pid = daemon.id();
    let tui_pid = tui.process_id();
    let baseline_handles = process_handles(daemon_pid);
    let started = Instant::now();
    let mut daemon_memory = Vec::new();
    let mut tui_memory = Vec::new();
    let mut child_memory = Vec::new();
    while started.elapsed() < SAMPLE_DURATION {
        let process_ids = [daemon_pid, tui_pid]
            .into_iter()
            .chain(child_pids.iter().copied())
            .collect::<Vec<_>>();
        let measured = process_memories(&process_ids);
        daemon_memory.push(measured[0]);
        tui_memory.push(measured[1]);
        child_memory.push(measured[2..].iter().sum());
        thread::sleep(Duration::from_secs(3));
    }
    assert_memory("daemon", &daemon_memory);
    assert_memory("tui", &tui_memory);
    report_child_memory(&child_memory);

    for _ in 0..80 {
        successful(admin(&root, &home, &["workspace", "status"]));
    }
    for _ in 0..20 {
        let mut watcher = command(&root, &home);
        watcher
            .args(["event", "watch", "--after", "0"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut watcher = watcher.spawn().unwrap();
        thread::sleep(Duration::from_millis(20));
        let _ = watcher.kill();
        let _ = watcher.wait();
    }
    wait_until(
        || process_handles(daemon_pid) <= baseline_handles.saturating_add(16),
        "daemon handles after reconnect churn",
    );
    let final_handles = process_handles(daemon_pid);
    eprintln!(
        "M11 resource reconnects=100 abrupt=20 daemon_handles_baseline={baseline_handles} daemon_handles_final={final_handles}"
    );

    fs::write(&stop, b"stop").unwrap();
    wait_until(|| reports.join("0").exists(), "sustained output report");
    let report = fs::read_to_string(reports.join("0")).unwrap();
    let mut fields = report.split_whitespace();
    let output_bytes = fields.next().unwrap().parse::<u64>().unwrap();
    let output_milliseconds = fields.next().unwrap().parse::<u64>().unwrap();
    let output_bytes_per_second = output_bytes.saturating_mul(1_000) / output_milliseconds.max(1);
    eprintln!(
        "M11 resource sustained_output_bytes={output_bytes} duration_ms={output_milliseconds} throughput_bytes_per_second={output_bytes_per_second}"
    );
    assert!(
        output_bytes_per_second >= 2 * 1024 * 1024,
        "sustained fixture output did not reach two MiB per second"
    );
    tui.send(&[0x1d]);
    tui.send(b"\x1b");
    tui.send(b"q");
    tui.wait_exit();
    successful(admin(
        &root,
        &home,
        &["daemon", "stop", "--terminate-sessions"],
    ));
    wait_child(&mut daemon);
}

fn select_rendered_running_session(tui: &mut OuterTerminal, session_count: usize) {
    for _ in 0..session_count {
        let screen = tui.screen_contents();
        if screen
            .lines()
            .any(|line| line.contains("> [running] session "))
        {
            return;
        }
        let previous = rendered_selected_session_id(&screen).map(str::to_owned);
        tui.send(next_selection_input());
        wait_until(
            || {
                let screen = tui.screen_contents();
                let current = rendered_selected_session_id(&screen);
                current.is_some() && current != previous.as_deref()
            },
            "resource TUI selection change",
        );
    }
    panic!(
        "no running session could be selected from the rendered TUI; screen={:?}",
        tui.screen_contents()
    );
}

fn rendered_selected_session_id(screen: &str) -> Option<&str> {
    screen.lines().find_map(|line| {
        let (_, value) = line.split_once("Sessions selected ")?;
        let length = value
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_alphanumeric() || *character == '-')
            .last()
            .map_or(0, |(index, character)| index + character.len_utf8());
        (length != 0).then_some(&value[..length])
    })
}

fn command(root: &Path, home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rt"));
    command
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(home)
        .args(["--format", "json", "--timeout", "5"])
        .stdin(Stdio::null());
    command
}

fn successful(value: Value) -> Value {
    assert_eq!(value["ok"], true, "administrative command failed: {value}");
    value["result"].clone()
}

fn successful_output(output: Output) -> Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    successful(serde_json::from_slice(&output.stdout).unwrap())
}

fn foreground_daemon(
    root: &Path,
    home: &Path,
    workspace_id: &str,
    stop: &Path,
    report: &Path,
    process: &Path,
) -> Child {
    Command::new(env!("CARGO_BIN_EXE_rt"))
        .args(["--workspace"])
        .arg(root)
        .args(["--home"])
        .arg(home)
        .args(["__daemon-run", "--root"])
        .arg(root)
        .args(["--workspace-id", workspace_id, "--private-home"])
        .arg(home)
        .env("RELAYTERM_RESOURCE_STOP", stop)
        .env("RELAYTERM_RESOURCE_REPORT", report)
        .env("RELAYTERM_RESOURCE_PROCESS", process)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_child(child: &mut Child) {
    wait_until(
        || child.try_wait().unwrap().is_some(),
        "foreground daemon shutdown",
    );
}

fn assert_memory(label: &str, samples: &[u64]) {
    assert!(samples.len() >= 40, "insufficient {label} memory samples");
    assert!(
        samples.iter().all(|value| *value <= MEMORY_LIMIT_BYTES),
        "{label} exceeded its memory limit: {samples:?}"
    );
    let warmup_samples = usize::try_from(WARMUP.as_secs() / 3).unwrap();
    let steady = &samples[warmup_samples.min(samples.len())..];
    let window = 10.min(steady.len() / 2);
    let first = median(&steady[..window]);
    let last = median(&steady[steady.len() - window..]);
    let maximum = *samples.iter().max().unwrap();
    eprintln!(
        "M11 resource process={label} samples={} first_median_bytes={first} last_median_bytes={last} maximum_bytes={maximum}",
        samples.len()
    );
    assert!(
        last <= first.saturating_add(PLATEAU_ALLOWANCE_BYTES),
        "{label} memory did not plateau: first={first} last={last}"
    );
}

fn median(values: &[u64]) -> u64 {
    let mut values = values.to_vec();
    values.sort_unstable();
    values[values.len() / 2]
}

fn report_child_memory(samples: &[u64]) {
    let maximum = samples.iter().max().unwrap();
    eprintln!(
        "M11 resource process=fixture_child samples={} maximum_bytes={maximum}",
        samples.len()
    );
}

#[cfg(target_os = "linux")]
fn process_memory(process_id: u32) -> u64 {
    let status = fs::read_to_string(format!("/proc/{process_id}/status")).unwrap();
    let kib = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap();
    kib * 1024
}

#[cfg(not(windows))]
fn process_memories(process_ids: &[u32]) -> Vec<u64> {
    process_ids
        .iter()
        .map(|process_id| process_memory(*process_id))
        .collect()
}

#[cfg(target_os = "macos")]
fn process_memory(process_id: u32) -> u64 {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &process_id.to_string()])
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse::<u64>()
        .unwrap()
        * 1024
}

#[cfg(windows)]
fn process_memories(process_ids: &[u32]) -> Vec<u64> {
    let identifiers = process_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let expression = format!(
        "Get-Process -Id {identifiers} | ForEach-Object {{ Write-Output ($_.Id.ToString() + ' ' + $_.WorkingSet64.ToString()) }}"
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &expression])
        .output()
        .unwrap();
    assert!(output.status.success(), "process measurement failed");
    let measured = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let mut fields = line.split_whitespace();
            (
                fields.next().unwrap().parse::<u32>().unwrap(),
                fields.next().unwrap().parse::<u64>().unwrap(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    process_ids
        .iter()
        .map(|process_id| measured[process_id])
        .collect()
}

#[cfg(target_os = "linux")]
fn process_handles(process_id: u32) -> usize {
    fs::read_dir(format!("/proc/{process_id}/fd"))
        .unwrap()
        .count()
}

#[cfg(target_os = "macos")]
fn process_handles(process_id: u32) -> usize {
    let output = Command::new("lsof")
        .args(["-nP", "-p", &process_id.to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).lines().count()
}

#[cfg(windows)]
fn process_handles(process_id: u32) -> usize {
    usize::try_from(powershell_process_value(process_id, "HandleCount")).unwrap()
}

#[cfg(windows)]
fn powershell_process_value(process_id: u32, property: &str) -> u64 {
    let expression = format!("(Get-Process -Id {process_id}).{property}");
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &expression])
        .output()
        .unwrap();
    assert!(output.status.success(), "process measurement failed");
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
