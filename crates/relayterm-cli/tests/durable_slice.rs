use relayterm_daemon::{
    RuntimeAgentDefinitionId as AgentDefinitionId, RuntimeClock as Clock,
    RuntimeInstanceStatus as InstanceStatus, RuntimeLaunchContext as LaunchContext,
    RuntimeObservation as Observation, RuntimeSystemClock as SystemClock,
    RuntimeTerminalSize as TerminalSize,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const HOST_TIMEOUT: Duration = Duration::from_secs(30);
const OUTPUT_LIMIT: usize = 1024 * 1024;

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
        let path = base.join(format!("rt6-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Host(Child);

impl Host {
    fn wait(mut self) {
        let deadline = Instant::now() + HOST_TIMEOUT;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "synthetic host failed");
                return;
            }
            assert!(Instant::now() < deadline, "synthetic host did not stop");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn kill_and_wait(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
#[ignore]
fn synthetic_lifecycle_host() {
    if std::env::var_os("RELAYTERM_M06_HOST").is_none() {
        return;
    }
    let root = PathBuf::from(std::env::var_os("RELAYTERM_M06_ROOT").unwrap());
    let private = PathBuf::from(std::env::var_os("RELAYTERM_M06_PRIVATE").unwrap());
    let ready = PathBuf::from(std::env::var_os("RELAYTERM_M06_READY").unwrap());
    let workspace = std::env::var("RELAYTERM_M06_WORKSPACE")
        .unwrap()
        .parse()
        .unwrap();
    let definitions = std::env::var("RELAYTERM_M06_DEFINITIONS")
        .unwrap()
        .split(',')
        .map(|value| value.parse::<AgentDefinitionId>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(definitions.len(), 2);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let prepared = relayterm_daemon::prepare_workspace(&root, Some(private.clone()), workspace)
            .await
            .unwrap();
        let service = prepared.service();
        let mut instance_ids = Vec::new();
        for definition in definitions {
            let outcome = service
                .register_instance(
                    workspace,
                    LaunchContext {
                        agent_definition_id: Some(definition),
                        task_id: None,
                        working_directory: root.clone(),
                        terminal_size: TerminalSize::new(24, 80).unwrap(),
                    },
                )
                .await
                .unwrap();
            let id = outcome
                .committed
                .snapshot
                .state()
                .unwrap()
                .instances()
                .last()
                .unwrap()
                .record()
                .id;
            service
                .observe(
                    workspace,
                    Observation::Status {
                        id,
                        status: InstanceStatus::Running,
                        observed_at: SystemClock.now().unwrap(),
                        exit_code: None,
                    },
                )
                .await
                .unwrap();
            instance_ids.push(id.to_string());
        }
        let encoded = serde_json::to_vec(&instance_ids).unwrap();
        prepared
            .run_with_ready(|| fs::write(&ready, encoded).unwrap())
            .await
            .unwrap();
    });
}

fn run_cli(root: &Path, private: &Path, args: &[&str], input: Option<&Value>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rt"));
    command
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(private)
        .args(["--format", "json"])
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(value) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(value).unwrap())
            .unwrap();
    }
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI stage exceeded its deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.stdout.len() <= OUTPUT_LIMIT);
    assert!(output.stderr.len() <= OUTPUT_LIMIT);
    output
}

fn success(root: &Path, private: &Path, args: &[&str], input: Option<&Value>) -> Value {
    let output = run_cli(root, private, args, input);
    assert!(
        output.status.success(),
        "command {args:?} failed with code {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["ok"], true);
    envelope["result"].clone()
}

fn rejected(root: &Path, private: &Path, args: &[&str], input: Option<&Value>) -> Value {
    failed_with(root, private, args, input, 5)
}

fn failed_with(
    root: &Path,
    private: &Path,
    args: &[&str],
    input: Option<&Value>,
    code: i32,
) -> Value {
    let output = run_cli(root, private, args, input);
    assert_eq!(output.status.code(), Some(code));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    envelope
}

fn entity_id(receipt: &Value) -> String {
    receipt["entity_ids"][0].as_str().unwrap().to_owned()
}

fn revision(root: &Path, private: &Path) -> String {
    success(root, private, &["daemon", "status"], None)["revision"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn start_host(
    root: &Path,
    private: &Path,
    workspace: &str,
    definitions: &[String],
    ready: &Path,
) -> (Host, Vec<String>) {
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "synthetic_lifecycle_host",
            "--ignored",
            "--exact",
            "--nocapture",
        ])
        .env("RELAYTERM_M06_HOST", "1")
        .env("RELAYTERM_M06_ROOT", root)
        .env("RELAYTERM_M06_PRIVATE", private)
        .env("RELAYTERM_M06_READY", ready)
        .env("RELAYTERM_M06_WORKSPACE", workspace)
        .env("RELAYTERM_M06_DEFINITIONS", definitions.join(","))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + HOST_TIMEOUT;
    while !ready.exists() {
        assert!(Instant::now() < deadline, "synthetic host was not ready");
        thread::sleep(Duration::from_millis(10));
    }
    let ids: Vec<String> = serde_json::from_slice(&fs::read(ready).unwrap()).unwrap();
    assert_eq!(ids.len(), 2);
    (Host(child), ids)
}

fn definition(root: &Path, private: &Path, name: &str) -> String {
    let current = revision(root, private);
    entity_id(&success(
        root,
        private,
        &[
            "agent",
            "register",
            "--expected-revision",
            &current,
            "--stdin",
        ],
        Some(&json!({
            "display_name": name,
            "command": "synthetic-agent",
            "arguments": [],
            "environment_allowlist": ["PATH"],
            "capabilities": ["terminal"],
            "enabled": true
        })),
    ))
}

fn create_task(root: &Path, private: &Path, title: &str) -> String {
    let current = revision(root, private);
    entity_id(&success(
        root,
        private,
        &["task", "create", "--expected-revision", &current, "--stdin"],
        Some(&json!({
            "title": title,
            "description": "Durable coordination context",
            "priority": "normal",
            "scope_paths": ["src/lib.rs"],
            "acceptance_notes": "Resume from structured state",
            "dependency_ids": []
        })),
    ))
}

fn transition(root: &Path, private: &Path, task: &str, status: &str) -> Value {
    let current = revision(root, private);
    success(
        root,
        private,
        &[
            "task",
            "transition",
            task,
            status,
            "--expected-revision",
            &current,
        ],
        None,
    )
}

fn diagnostic_logs(path: &Path, output: &mut Vec<u8>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            diagnostic_logs(&entry.path(), output);
        } else if entry.file_name() == "daemon.log" {
            output.extend(fs::read(entry.path()).unwrap());
        }
    }
}

fn run_journey(git: bool) {
    eprintln!("m06 stage: initialize {} fixture", if git { "git" } else { "non-git" });
    let scratch = Scratch::new();
    let root = scratch.0.join(if git {
        "Git project with spaces Ω"
    } else {
        "Project with spaces Ω"
    });
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    if git {
        fs::create_dir(root.join(".git")).unwrap();
    }
    let initial_entries = fs::read_dir(&root).unwrap().count();
    let initialized = success(
        &root,
        &private,
        &["workspace", "init", "--name", "M06 fixture"],
        None,
    );
    let workspace = initialized["workspace_id"].as_str().unwrap().to_owned();
    let definitions = vec![
        definition(&root, &private, "Fixture alpha"),
        definition(&root, &private, "Fixture beta"),
    ];
    success(&root, &private, &["daemon", "stop"], None);

    eprintln!("m06 stage: start first synthetic host");
    let ready = private.join(format!("host-{}.json", u8::from(git)));
    let (host, instances) = start_host(&root, &private, &workspace, &definitions, &ready);
    let task = create_task(&root, &private, "Durable handover task");
    eprintln!("m06 stage: execute handover journey");
    transition(&root, &private, &task, "ready");
    success(
        &root,
        &private,
        &["task", "claim", &task, "--instance", &instances[0]],
        None,
    );
    let before_rejection = revision(&root, &private);
    let failure = rejected(
        &root,
        &private,
        &["task", "claim", &task, "--instance", &instances[1]],
        None,
    );
    assert_eq!(failure["error"]["code"], "request_rejected");
    assert_eq!(revision(&root, &private), before_rejection);
    rejected(
        &root,
        &private,
        &["task", "claim", &task, "--instance", &instances[0]],
        None,
    );
    assert_eq!(revision(&root, &private), before_rejection);
    success(
        &root,
        &private,
        &["progress", "append", &task, "--stdin"],
        Some(&json!({
            "summary": "First instance completed the bounded stage",
            "verification": "cargo test -p relayterm-cli --test durable_slice --locked"
        })),
    );
    let handover_revision = revision(&root, &private);
    rejected(
        &root,
        &private,
        &[
            "handover",
            "create",
            &task,
            "--expected-revision",
            &handover_revision,
            "--stdin",
        ],
        Some(&json!({
            "summary": "Invalid handover", "decisions": "None", "changed_paths": [],
            "verification_performed": "", "open_questions": "None",
            "recommended_next_action": "Continue"
        })),
    );
    assert_eq!(revision(&root, &private), handover_revision);
    let handover = success(
        &root,
        &private,
        &[
            "handover",
            "create",
            &task,
            "--expected-revision",
            &handover_revision,
            "--stdin",
        ],
        Some(&json!({
            "summary": "Continue from durable state",
            "decisions": "Keep the protocol provider-neutral",
            "changed_paths": ["src/lib.rs"],
            "verification_performed": "cargo test -p relayterm-cli --test durable_slice --locked",
            "open_questions": "None",
            "recommended_next_action": "Claim and complete the task"
        })),
    );
    let handover_id = handover["entity_ids"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();
    let handed_task = success(&root, &private, &["task", "get", &task], None);
    assert_eq!(handed_task["status"], "handover_ready");
    assert!(handed_task["claimed_by_instance_id"].is_null());
    let context = success(&root, &private, &["handover", "get", &handover_id], None);
    assert_eq!(
        context["content"]["verification_performed"],
        "cargo test -p relayterm-cli --test durable_slice --locked"
    );
    assert_eq!(
        context["content"]["recommended_next_action"],
        "Claim and complete the task"
    );
    let released_revision = revision(&root, &private);
    rejected(
        &root,
        &private,
        &[
            "task",
            "release",
            &task,
            "--expected-revision",
            &released_revision,
        ],
        None,
    );
    assert_eq!(revision(&root, &private), released_revision);
    success(
        &root,
        &private,
        &["task", "claim", &task, "--instance", &instances[1]],
        None,
    );
    transition(&root, &private, &task, "done");
    let first_final_read = success(&root, &private, &["task", "get", &task], None);
    let second_final_read = success(&root, &private, &["task", "get", &task], None);
    assert_eq!(first_final_read, second_final_read);

    let history_revision = revision(&root, &private);
    let first_history = success(
        &root,
        &private,
        &[
            "task",
            "history",
            &task,
            "--limit",
            "2",
            "--expected-revision",
            &history_revision,
        ],
        None,
    );
    assert_eq!(first_history["entries"].as_array().unwrap().len(), 2);
    let after = first_history["entries"][1]["sequence"]
        .as_str()
        .unwrap()
        .to_owned();
    let second_history = success(
        &root,
        &private,
        &[
            "task",
            "history",
            &task,
            "--limit",
            "2",
            "--after",
            &after,
            "--expected-revision",
            &history_revision,
        ],
        None,
    );
    assert!(!second_history["entries"].as_array().unwrap().is_empty());

    let race = create_task(&root, &private, "Competing claim task");
    transition(&root, &private, &race, "ready");
    let (first_race, second_race) = thread::scope(|scope| {
        let first = scope.spawn(|| {
            run_cli(
                &root,
                &private,
                &["task", "claim", &race, "--instance", &instances[0]],
                None,
            )
        });
        let second = scope.spawn(|| {
            run_cli(
                &root,
                &private,
                &["task", "claim", &race, "--instance", &instances[1]],
                None,
            )
        });
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_eq!(
        [first_race.status.success(), second_race.status.success()]
            .into_iter()
            .filter(|won| *won)
            .count(),
        1
    );
    transition(&root, &private, &race, "done");

    eprintln!("m06 stage: prepare recovery journey");
    let recovery = create_task(&root, &private, "Interrupted active task");
    transition(&root, &private, &recovery, "ready");
    success(
        &root,
        &private,
        &["task", "claim", &recovery, "--instance", &instances[0]],
        None,
    );
    let active_revision = revision(&root, &private);
    success(
        &root,
        &private,
        &[
            "task",
            "release",
            &recovery,
            "--expected-revision",
            &active_revision,
        ],
        None,
    );
    let blocked_revision = revision(&root, &private);
    rejected(
        &root,
        &private,
        &[
            "task",
            "release",
            &recovery,
            "--expected-revision",
            &blocked_revision,
        ],
        None,
    );
    rejected(
        &root,
        &private,
        &["task", "claim", &recovery, "--instance", &instances[0]],
        None,
    );
    assert_eq!(revision(&root, &private), blocked_revision);
    transition(&root, &private, &recovery, "ready");
    success(
        &root,
        &private,
        &["task", "claim", &recovery, "--instance", &instances[0]],
        None,
    );
    let occupied = create_task(&root, &private, "Instance exclusivity task");
    transition(&root, &private, &occupied, "ready");
    let occupied_revision = revision(&root, &private);
    rejected(
        &root,
        &private,
        &["task", "claim", &occupied, "--instance", &instances[0]],
        None,
    );
    assert_eq!(revision(&root, &private), occupied_revision);
    let dependent_revision = revision(&root, &private);
    let dependent = entity_id(&success(
        &root,
        &private,
        &[
            "task",
            "create",
            "--expected-revision",
            &dependent_revision,
            "--stdin",
        ],
        Some(&json!({
            "title": "Informational dependency task",
            "description": "The active dependency does not schedule work",
            "priority": "normal",
            "scope_paths": [],
            "acceptance_notes": "Claim remains permitted",
            "dependency_ids": [&recovery]
        })),
    ));
    transition(&root, &private, &dependent, "ready");
    success(
        &root,
        &private,
        &["task", "claim", &dependent, "--instance", &instances[1]],
        None,
    );
    transition(&root, &private, &dependent, "done");
    if !git {
        let foreign_root = scratch.0.join("Foreign workspace");
        fs::create_dir(&foreign_root).unwrap();
        success(
            &foreign_root,
            &private,
            &["workspace", "init", "--name", "Foreign fixture"],
            None,
        );
        let foreign_revision = revision(&foreign_root, &private);
        failed_with(
            &foreign_root,
            &private,
            &[
                "task",
                "create",
                "--expected-revision",
                &foreign_revision,
                "--stdin",
            ],
            Some(&json!({
                "title": "Foreign dependency rejection", "description": "",
                "priority": "normal", "scope_paths": [], "acceptance_notes": "",
                "dependency_ids": [&recovery]
            })),
            3,
        );
        assert_eq!(revision(&foreign_root, &private), foreign_revision);
        success(&foreign_root, &private, &["daemon", "stop"], None);
        assert_eq!(fs::read_dir(foreign_root).unwrap().count(), 0);
    }
    let events_before = success(&root, &private, &["event", "list", "--limit", "200"], None);
    let prefix = events_before["events"].as_array().unwrap().clone();
    let mut host = host;
    if git {
        eprintln!("m06 stage: orderly host stop");
        success(&root, &private, &["daemon", "stop"], None);
        host.wait();
    } else {
        eprintln!("m06 stage: abrupt host stop");
        host.kill_and_wait();
    }

    let started = success(&root, &private, &["daemon", "start"], None);
    eprintln!("m06 stage: verify production recovery");
    assert_eq!(started["workspace_id"], workspace);
    let done = success(&root, &private, &["task", "get", &task], None);
    assert_eq!(done["status"], "done");
    let blocked = success(&root, &private, &["task", "get", &recovery], None);
    assert_eq!(blocked["status"], "blocked");
    assert!(blocked["claimed_by_instance_id"].is_null());
    let sessions = success(&root, &private, &["session", "list"], None);
    assert!(
        sessions["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["status"] == "lost")
    );
    let current = revision(&root, &private);
    let claims = success(
        &root,
        &private,
        &["task", "claims", &recovery, "--expected-revision", &current],
        None,
    );
    assert_eq!(claims["entries"].as_array().unwrap().len(), 2);
    assert_eq!(
        claims["entries"][0]["item"]["close_reason"],
        "explicit_release"
    );
    assert_eq!(claims["entries"][1]["item"]["close_reason"], "instance_end");
    let events_after = success(&root, &private, &["event", "list", "--limit", "200"], None);
    assert!(
        events_after["events"]
            .as_array()
            .unwrap()
            .starts_with(&prefix)
    );
    let sequences = events_after["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["sequence"].as_str().unwrap().parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    assert!(sequences.windows(2).all(|pair| pair[1] == pair[0] + 1));
    let encoded_events = serde_json::to_string(&events_after).unwrap();
    assert!(!encoded_events.contains("First instance completed the bounded stage"));
    assert!(!encoded_events.contains(root.to_str().unwrap()));
    let first_event_page = success(&root, &private, &["event", "list", "--limit", "2"], None);
    assert_eq!(first_event_page["events"].as_array().unwrap().len(), 2);
    let event_after = first_event_page["events"][1]["sequence"].as_str().unwrap();
    let second_event_page = success(
        &root,
        &private,
        &["event", "list", "--limit", "2", "--after", event_after],
        None,
    );
    assert!(!second_event_page["events"].as_array().unwrap().is_empty());
    let recovered_revision = revision(&root, &private);
    success(&root, &private, &["daemon", "stop"], None);
    success(&root, &private, &["daemon", "start"], None);
    assert_eq!(revision(&root, &private), recovered_revision);
    success(&root, &private, &["daemon", "stop"], None);

    eprintln!("m06 stage: start successor host");
    fs::remove_file(&ready).unwrap();
    let (successor_host, successors) =
        start_host(&root, &private, &workspace, &definitions, &ready);
    transition(&root, &private, &recovery, "ready");
    success(
        &root,
        &private,
        &["task", "claim", &recovery, "--instance", &successors[0]],
        None,
    );
    success(
        &root,
        &private,
        &["progress", "append", &recovery, "--stdin"],
        Some(&json!({
            "summary": "Successor resumed from durable context",
            "verification": "Prior claim and state inspected through IPC"
        })),
    );
    transition(&root, &private, &recovery, "done");
    success(&root, &private, &["daemon", "stop"], None);
    successor_host.wait();
    eprintln!("m06 stage: final production restart");
    success(&root, &private, &["daemon", "start"], None);
    assert_eq!(
        success(&root, &private, &["task", "get", &recovery], None)["status"],
        "done"
    );
    success(&root, &private, &["daemon", "stop"], None);
    let mut logs = Vec::new();
    diagnostic_logs(&private, &mut logs);
    let logs = String::from_utf8(logs).unwrap();
    assert!(!logs.contains("First instance completed the bounded stage"));
    assert!(!logs.contains(root.to_str().unwrap()));
    assert_eq!(fs::read_dir(&root).unwrap().count(), initial_entries);
}

#[test]
fn durable_handover_survives_process_restart() {
    run_journey(false);
    run_journey(true);
}

#[test]
fn helper_has_no_effect_without_test_marker() {
    synthetic_lifecycle_host();
}

#[test]
fn completed_host_can_be_joined() {
    if std::env::var_os("RELAYTERM_M06_HOST_JOIN_SMOKE").is_some() {
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["completed_host_can_be_joined", "--exact", "--nocapture"])
        .env("RELAYTERM_M06_HOST_JOIN_SMOKE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + HOST_TIMEOUT;
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    Host(child).wait();
}
