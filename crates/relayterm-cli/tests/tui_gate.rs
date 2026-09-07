use relayterm_protocol::Operation;
use relayterm_pty::{
    NativeControl, NativeSession, SpawnRequest, approved_environment, terminal_environment,
    write_input,
};
use serde_json::Value;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(45);
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;
const FLOOD_CHUNK_REPETITIONS: usize = 2_048;
const FLOOD_CHUNKS: usize = 80;
static NATIVE_GATE_LOCK: Mutex<()> = Mutex::new(());

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
            "rt8-{}-{}",
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

struct OuterTerminal {
    control: NativeControl,
    writer: Box<dyn std::io::Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    screen: Arc<Mutex<vt100::Parser>>,
}

impl OuterTerminal {
    fn spawn(root: &Path, private: &Path) -> Self {
        let environment = terminal_environment(approved_environment(&[], std::env::vars_os()));
        let session = NativeSession::spawn(SpawnRequest {
            program: OsString::from(env!("CARGO_BIN_EXE_rt")),
            arguments: vec![
                OsString::from("--workspace"),
                root.as_os_str().to_owned(),
                OsString::from("--home"),
                private.as_os_str().to_owned(),
            ],
            working_directory: root.to_owned(),
            environment,
            rows: 30,
            columns: 100,
        })
        .unwrap();
        let (control, writer, mut reader) = session.into_parts();
        let output = Arc::new(Mutex::new(Vec::new()));
        let captured = output.clone();
        let screen = Arc::new(Mutex::new(vt100::Parser::new(30, 100, 0)));
        let parsed = screen.clone();
        std::thread::spawn(move || {
            let mut chunk = [0_u8; 4096];
            while let Ok(amount) = reader.read(&mut chunk) {
                if amount == 0 {
                    break;
                }
                parsed.lock().unwrap().process(&chunk[..amount]);
                let mut output = captured.lock().unwrap();
                output.extend_from_slice(&chunk[..amount]);
                if output.len() > OUTPUT_LIMIT {
                    let remove = output.len() - OUTPUT_LIMIT;
                    output.drain(..remove);
                }
            }
        });
        Self {
            control,
            writer,
            output,
            screen,
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        write_input(&mut *self.writer, bytes).unwrap();
    }

    fn finish_startup(&mut self) {
        #[cfg(unix)]
        {
            self.wait_for("\x1b[6n");
            self.send(b"\x1b[30;100R");
        }
        self.wait_for("Workspace overview");
    }

    fn resize(&mut self, rows: u16, columns: u16) {
        self.control.resize(rows, columns).unwrap();
        self.screen
            .lock()
            .unwrap()
            .screen_mut()
            .set_size(rows, columns);
    }

    fn wait_for(&self, marker: &str) {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let present = {
                let output = self.output.lock().unwrap();
                if marker.starts_with('\u{1b}') {
                    String::from_utf8_lossy(&output).contains(marker)
                } else {
                    drop(output);
                    self.screen
                        .lock()
                        .unwrap()
                        .screen()
                        .contents()
                        .contains(marker)
                }
            };
            if present {
                return;
            }
            if Instant::now() >= deadline {
                let bytes = self.output.lock().unwrap();
                panic!(
                    "TUI marker {marker:?} did not arrive after {DEADLINE:?}; captured bytes={}",
                    bytes.len()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn screen_contents(&self) -> String {
        self.screen.lock().unwrap().screen().contents()
    }

    fn wait_exit(&mut self) {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(exit) = self.control.try_wait().unwrap() {
                assert_eq!(exit.code, 0, "interactive rt exited unsuccessfully");
                return;
            }
            assert!(Instant::now() < deadline, "interactive rt did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for OuterTerminal {
    fn drop(&mut self) {
        let _ = self.control.terminate();
    }
}

#[test]
#[ignore]
fn interactive_fixture_process() {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "fixture-ready").unwrap();
    stdout.flush().unwrap();
    let mut echoes = 0_u64;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        if line == "flood" {
            let started = Instant::now();
            let chunk = format!("{}\r\n", "0123456789abcdef".repeat(FLOOD_CHUNK_REPETITIONS));
            for _ in 0..FLOOD_CHUNKS {
                stdout.write_all(chunk.as_bytes()).unwrap();
            }
            stdout.flush().unwrap();
            writeln!(
                stdout,
                "flood-complete-us-{}",
                started.elapsed().as_micros()
            )
            .unwrap();
        } else if line == "exit-nonzero" {
            stdout.flush().unwrap();
            std::process::exit(7);
        } else {
            writeln!(stdout, "fixture-echo-{echoes:03}:{line}").unwrap();
            echoes += 1;
        }
        stdout.flush().unwrap();
    }
}

struct DaemonCleanup {
    root: PathBuf,
    private: PathBuf,
}

impl Drop for DaemonCleanup {
    fn drop(&mut self) {
        let _ = admin_output(
            &self.root,
            &self.private,
            &["daemon", "stop", "--terminate-sessions"],
        );
    }
}

#[test]
fn tui_initializes_launches_detaches_and_reopens_without_stopping_children() {
    let _serial = NATIVE_GATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let scratch = Scratch::new();
    let root = scratch.0.join("project with space λ");
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    let _cleanup = DaemonCleanup {
        root: root.clone(),
        private: private.clone(),
    };

    let mut first = OuterTerminal::spawn(&root, &private);
    first.wait_for("Initialize it?");
    first.send(b"y\r");
    first.finish_startup();
    register_fixture(&root, &private);
    first.send(b"R4aa3s");
    wait_for_session_count(&root, &private, 3);
    first.send(b"2nM08 task\tCoordinate three sessions\tnormal\tsrc/lib.rs\tVerified in TUI\t\x13");
    wait_for_task_status(&root, &private, "backlog");
    first.send(b"rc");
    wait_for_task_status(&root, &private, "active");
    let mut competitor = OuterTerminal::spawn(&root, &private);
    competitor.finish_startup();
    competitor.send(b"2c5");
    competitor.wait_for("Redacted diagnostics");
    let diagnostic = competitor.screen_contents();
    assert!(
        diagnostic.contains("rejected")
            || diagnostic.contains("conflict")
            || diagnostic.contains("invalid"),
        "competing claim did not produce a contextual diagnostic"
    );
    competitor.send(b"q");
    competitor.wait_exit();
    first.send(b"r");
    wait_for_task_status(&root, &private, "blocked");
    first.send(b"rc");
    wait_for_task_status(&root, &private, "active");
    first.send(b"pFirst instance progress\tNative PTY verified\x13");
    wait_for_history_marker(&root, &private, "First instance progress");
    first.send(b"hIncomplete handover\x13");
    first.wait_for("Summary, verification, and next action are required");
    first.send(b"\x1b");
    first.wait_for("Discard draft?");
    first.send(b"y");
    first.send(
        b"hContinue elsewhere\tKeep neutral ownership\t\tNative TUI gate\t\tClaim and finish\x13",
    );
    wait_for_task_status(&root, &private, "handover_ready");
    first.send(b"3j2cd");
    wait_for_task_status(&root, &private, "done");
    first.send(b"q");
    first.wait_exit();
    #[cfg(unix)]
    {
        let first_output = first.output.lock().unwrap().clone();
        assert!(first_output.windows(8).any(|part| part == b"\x1b[?1049h"));
        assert!(first_output.windows(8).any(|part| part == b"\x1b[?1049l"));
    }

    let sessions = admin(&root, &private, &["session", "list"]);
    assert_eq!(sessions["ok"], true);
    assert!(sessions["result"]["items"].as_array().is_some_and(|items| {
        items
            .iter()
            .filter(|item| item["status"] == "running")
            .count()
            >= 3
    }));
    let session_items = sessions["result"]["items"].as_array().unwrap();
    let fixture_indices = session_items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.get("agent_definition_id")
                .is_some_and(|value| !value.is_null())
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(fixture_indices.len(), 2);
    let flood_session_id = session_items[fixture_indices[0]]["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let quiet_session_id = session_items[fixture_indices[1]]["session_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut second = OuterTerminal::spawn(&root, &private);
    second.finish_startup();
    second.send(b"3");
    select_session(&mut second, &flood_session_id);
    second.wait_for("running");
    second.send(b"\r");
    second.wait_for("Terminal");
    second.send(b"i");
    second.wait_for("WRITER");
    second.resize(28, 90);
    wait_for_terminal_size(&root, &private, &flood_session_id, 21, 88);
    second.resize(30, 100);
    wait_for_terminal_size(&root, &private, &flood_session_id, 23, 98);
    second.send(b"echo tui-reattach\r");
    second.wait_for("tui-reattach");
    second.send(b"flood\r");
    second.send(&[0x1d]);
    second.wait_for("READ ONLY");
    second.send(b"\x1b");
    second.wait_for("Sessions selected");
    second.send(b"?");
    second.wait_for("Keyboard help");
    second.send(b"3");
    second.wait_for("Sessions selected");
    wait_for_selected_session(&second, &flood_session_id);
    let mut navigation = Vec::with_capacity(100);
    for _ in 0..50 {
        let selected_session_id = wait_for_selected_session_change(&second, None);
        let started = Instant::now();
        second.send(next_selection_input());
        let next_session_id = wait_for_selected_session_change(&second, Some(&selected_session_id));
        navigation.push(started.elapsed());
        let started = Instant::now();
        second.send(previous_selection_input());
        wait_for_selected_session(&second, &selected_session_id);
        navigation.push(started.elapsed());
        assert_ne!(selected_session_id, next_session_id);
    }
    assert_latency(
        "navigation",
        &mut navigation,
        Duration::from_millis(100),
        Duration::from_millis(500),
    );

    select_session(&mut second, &quiet_session_id);
    second.send(b"\r");
    second.wait_for("Terminal");
    second.send(b"i");
    second.wait_for("WRITER");
    let mut input_latencies = Vec::with_capacity(100);
    for index in 0..100 {
        let marker = format!("fixture-echo-{index:03}:x");
        let started = Instant::now();
        second.send(b"x\r");
        second.wait_for(&marker);
        input_latencies.push(started.elapsed());
    }
    assert_latency(
        "input-to-rendered-echo",
        &mut input_latencies,
        Duration::from_millis(250),
        Duration::from_secs(1),
    );
    second.send(b"exit-nonzero\r");
    wait_for_session_status(&root, &private, &quiet_session_id, "exited");
    second.send(&[0x1d]);
    second.wait_for("READ ONLY");
    second.send(b"\x1b");
    second.wait_for("Sessions selected");
    select_session(&mut second, &flood_session_id);
    second.send(b"\r");
    second.wait_for("flood-complete-us-");
    let screen = second.screen_contents();
    let microseconds = screen
        .split("flood-complete-us-")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<u128>().ok())
        .expect("fixture reported its bounded flood duration");
    let flood_bytes = FLOOD_CHUNKS as u128 * (FLOOD_CHUNK_REPETITIONS as u128 * 16 + 2);
    let bytes_per_second = flood_bytes * 1_000_000 / microseconds.max(1);
    eprintln!("M08 flood throughput_bytes_per_second={bytes_per_second}");
    assert!(
        bytes_per_second >= 1024 * 1024,
        "synthetic output did not reach one MiB per second"
    );
    second.send(b"\x1b[5~");
    second.wait_for("HISTORY");
    second.send(b"\x1b[6~");
    second.send(b"\x1b");
    second.send(b"tTERMINATE\x13");
    wait_for_session_status(&root, &private, &flood_session_id, "terminated");
    second.send(b"q");
    second.wait_exit();

    let stopped = admin(&root, &private, &["daemon", "stop", "--terminate-sessions"]);
    assert_eq!(stopped["ok"], true);
}

fn select_session(terminal: &mut OuterTerminal, target_session_id: &str) {
    let mut visited = HashSet::new();
    for _ in 0..relayterm_protocol::MAX_PAGE_SIZE {
        let selected = wait_for_selected_session_change(terminal, None);
        if selected == target_session_id {
            return;
        }
        if !visited.insert(selected.clone()) {
            break;
        }
        terminal.send(next_selection_input());
        wait_for_selected_session_change(terminal, Some(&selected));
    }
    panic!(
        "target session was not selected after one rendered cycle: target={target_session_id} visited={visited:?}"
    );
}

fn wait_for_selected_session(terminal: &OuterTerminal, expected: &str) {
    let selected = wait_for_selected_session_change(terminal, None);
    if selected == expected {
        return;
    }
    let deadline = Instant::now() + DEADLINE;
    loop {
        let selected = wait_for_selected_session_change(terminal, None);
        if selected == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "expected rendered session was not selected"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_selected_session_change(terminal: &OuterTerminal, previous: Option<&str>) -> String {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let screen = terminal.screen_contents();
        if let Some(selected) = rendered_selected_session_id(&screen)
            && previous.is_none_or(|previous| previous != selected)
        {
            return selected.to_owned();
        }
        assert!(
            Instant::now() < deadline,
            "rendered session selection did not become coherent"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn visible_session_rows(screen: &str) -> Vec<(bool, String)> {
    let selected_session_id = rendered_selected_session_id(screen);
    screen
        .lines()
        .filter_map(|line| {
            let (_, remainder) = line.split_once("] session ")?;
            let session_id = remainder.split_whitespace().next()?;
            Some((
                selected_session_id == Some(session_id),
                session_id.to_owned(),
            ))
        })
        .collect()
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

fn next_selection_input() -> &'static [u8] {
    if cfg!(windows) { b"\x1b[B" } else { b"j" }
}

fn previous_selection_input() -> &'static [u8] {
    if cfg!(windows) { b"\x1b[A" } else { b"k" }
}

#[test]
fn rendered_session_rows_accept_native_border_prefixes() {
    let rows = visible_session_rows(
        "\u{250c}Sessions selected selected-id\u{2500}\u{2500}\n\u{2502}> [running] session selected-id instance one\n|  [exited] session other-id instance two",
    );
    assert_eq!(
        rows,
        vec![
            (true, "selected-id".to_owned()),
            (false, "other-id".to_owned())
        ]
    );
}

fn assert_latency(
    label: &str,
    samples: &mut [Duration],
    reference_target: Duration,
    hosted_guardrail: Duration,
) {
    samples.sort_unstable();
    let median = samples[samples.len() / 2 - 1];
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    let maximum = samples[samples.len() - 1];
    eprintln!(
        "M08 {label} samples={} median_ms={} p95_ms={} max_ms={}",
        samples.len(),
        median.as_millis(),
        p95.as_millis(),
        maximum.as_millis()
    );
    if std::env::var_os("CI").is_some() {
        eprintln!(
            "M08 {label} reference_target_ms={} reference_target_met={}",
            reference_target.as_millis(),
            p95 <= reference_target
        );
        assert!(
            p95 <= hosted_guardrail,
            "{label} p95 exceeded hosted guardrail {hosted_guardrail:?}"
        );
    } else {
        assert!(
            p95 <= reference_target,
            "{label} p95 exceeded reference target {reference_target:?}"
        );
    }
}

#[test]
fn existing_daemon_reaches_usable_screen_within_budget() {
    let _serial = NATIVE_GATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let scratch = Scratch::new();
    let root = scratch.0.join("representative project λ");
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    let _cleanup = DaemonCleanup {
        root: root.clone(),
        private: private.clone(),
    };
    create_representative_workspace(&root, &private);

    let mut samples = Vec::with_capacity(20);
    for sample in 0..=20 {
        let started = Instant::now();
        let mut terminal = OuterTerminal::spawn(&root, &private);
        terminal.finish_startup();
        let elapsed = started.elapsed();
        terminal.send(b"q");
        terminal.wait_exit();
        if sample != 0 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    let median = samples[9];
    let p95 = samples[18];
    let maximum = samples[19];
    eprintln!(
        "M08 startup samples=20 median_ms={} p95_ms={} max_ms={}",
        median.as_millis(),
        p95.as_millis(),
        maximum.as_millis()
    );
    assert!(
        p95 <= Duration::from_secs(2),
        "connection-to-usable-screen p95 exceeded two seconds"
    );
}

fn create_representative_workspace(root: &Path, private: &Path) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let route = relayterm_daemon::bootstrap(
            relayterm_daemon::BootstrapAction::Initialize,
            root,
            Some(private.to_owned()),
            Some("Representative workspace".into()),
            Path::new(env!("CARGO_BIN_EXE_rt")),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
        let client = relayterm_daemon::connect_route(&route, Some(private.to_owned()))
            .await
            .unwrap();
        let snapshot: Value = client
            .call(
                Operation::WorkspaceGetSnapshot,
                &serde_json::json!({"collection":"workspace","limit":50}),
            )
            .await
            .unwrap();
        let mut revision = snapshot["revision"].as_str().unwrap().to_owned();
        let mut first_task = String::new();
        for index in 0..100 {
            let receipt: Value = client
                .call(
                    Operation::TaskCreate,
                    &serde_json::json!({
                        "expected_revision":revision,
                        "title":format!("Representative task {index:03}"),
                        "description":"Bounded startup workload",
                        "priority":"normal",
                        "scope_paths":[],
                        "acceptance_notes":"Visible from the TUI",
                        "dependency_ids":[]
                    }),
                )
                .await
                .unwrap();
            if index == 0 {
                first_task = receipt["entity_ids"][0].as_str().unwrap().to_owned();
            }
            revision = receipt["revision"].as_str().unwrap().to_owned();
        }
        for _ in 0..3 {
            let _: Value = client
                .call(
                    Operation::SessionCreate,
                    &serde_json::json!({
                        "receipt_id":uuid::Uuid::new_v4().to_string(),
                        "launch_kind":"default_shell",
                        "definition_id":null,
                        "task_id":null,
                        "working_directory":null,
                        "rows":24,
                        "columns":80
                    }),
                )
                .await
                .unwrap();
        }
        let snapshot = client.refresh_snapshot().await.unwrap();
        revision = snapshot.revision;
        let instance_id = snapshot.collections["instances"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let receipt: Value = client
            .call(
                Operation::TaskTransition,
                &serde_json::json!({"task_id":first_task,"expected_revision":revision,"status":"ready"}),
            )
            .await
            .unwrap();
        let _: Value = client
            .call(
                Operation::TaskClaim,
                &serde_json::json!({"task_id":first_task,"instance_id":instance_id}),
            )
            .await
            .unwrap();
        for index in 0..100 {
            let _: Value = client
                .call(
                    Operation::ProgressAppend,
                    &serde_json::json!({
                        "task_id":first_task,
                        "summary":format!("History entry {index:03}"),
                        "verification":"Synthetic startup workload"
                    }),
                )
                .await
                .unwrap();
        }
        assert!(receipt["revision"].as_str().is_some());
    });
}

fn register_fixture(root: &Path, private: &Path) {
    let status = admin(root, private, &["daemon", "status"]);
    let revision = status["result"]["revision"].as_str().unwrap();
    let definition = private.join("fixture-definition.json");
    fs::write(
        &definition,
        serde_json::to_vec(&serde_json::json!({
            "display_name":"Neutral fixture",
            "command":std::env::current_exe().unwrap().to_string_lossy(),
            "arguments":["--exact","interactive_fixture_process","--ignored","--nocapture"],
            "environment_allowlist":[],
            "capabilities":["interactive_terminal"],
            "enabled":true
        }))
        .unwrap(),
    )
    .unwrap();
    let result = admin(
        root,
        private,
        &[
            "agent",
            "register",
            "--expected-revision",
            revision,
            "--file",
            definition.to_str().unwrap(),
        ],
    );
    assert_eq!(result["ok"], true);
}

fn admin(root: &Path, private: &Path, args: &[&str]) -> Value {
    let output = admin_output(root, private, args);
    assert!(
        output.status.success(),
        "administrative command {args:?} failed with status {}",
        output.status
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn admin_output(root: &Path, private: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(private)
        .args(["--format", "json"])
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap()
}

fn wait_for_session_count(root: &Path, private: &Path, expected: usize) {
    wait_until(
        || {
            admin(root, private, &["session", "list"])["result"]["items"]
                .as_array()
                .is_some_and(|items| {
                    items
                        .iter()
                        .filter(|item| item["status"] == "running")
                        .count()
                        >= expected
                })
        },
        "session count",
    );
}

fn wait_for_session_status(root: &Path, private: &Path, session_id: &str, status: &str) {
    wait_until(
        || {
            admin(root, private, &["session", "list"])["result"]["items"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["session_id"] == session_id))
                .is_some_and(|item| item["status"] == status)
        },
        "session status",
    );
}

fn wait_for_terminal_size(root: &Path, private: &Path, session_id: &str, rows: u64, columns: u64) {
    wait_until(
        || {
            let value = admin(root, private, &["session", "attach", session_id]);
            value["result"]["snapshot"]["rows"] == rows
                && value["result"]["snapshot"]["columns"] == columns
        },
        "terminal resize",
    );
}

fn task_list(root: &Path, private: &Path) -> Value {
    admin(root, private, &["task", "list"])
}

fn wait_for_task_status(root: &Path, private: &Path, status: &str) {
    wait_until(
        || {
            task_list(root, private)["result"]["items"]
                .as_array()
                .and_then(|items| items.first())
                .is_some_and(|task| task["status"] == status)
        },
        "task status",
    );
}

fn wait_for_history_marker(root: &Path, private: &Path, marker: &str) {
    wait_until(
        || {
            let tasks = task_list(root, private);
            let Some(task_id) = tasks["result"]["items"]
                .as_array()
                .and_then(|items| items.first())
                .and_then(|task| task["id"].as_str())
            else {
                return false;
            };
            let Some(revision) = tasks["result"]["revision"].as_str() else {
                return false;
            };
            let output = admin_output(
                root,
                private,
                &["task", "history", task_id, "--expected-revision", revision],
            );
            output.status.success()
                && serde_json::from_slice::<Value>(&output.stdout)
                    .is_ok_and(|history| history.to_string().contains(marker))
        },
        "task history",
    );
}

fn wait_until(mut condition: impl FnMut() -> bool, label: &str) {
    let deadline = Instant::now() + DEADLINE;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        std::thread::sleep(Duration::from_millis(50));
    }
}
