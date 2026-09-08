#[path = "support/m09_native.rs"]
mod native;

use native::{
    DaemonCleanup, OuterTerminal, Scratch, admin, admin_output, next_selection_input,
    previous_selection_input, wait_until,
};
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, Write},
    sync::Mutex,
    time::Instant,
};

static GATE_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[ignore]
fn unknown_interactive_process() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let separator = arguments.iter().position(|value| value == "--").unwrap();
    let supplied = &arguments[separator + 1..];
    println!("unknown-ready");
    println!("unknown-argument-count:{}", supplied.len());
    for (index, value) in supplied.iter().enumerate() {
        println!("unknown-argument-{index}:<{value}>");
    }
    std::io::stdout().flush().unwrap();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        println!("unknown-input:<{line}>");
        std::io::stdout().flush().unwrap();
    }
}

#[test]
fn unknown_cli_is_configured_and_continued_through_the_real_tui() {
    let _serial = GATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let scratch = Scratch::new();
    let root = scratch.0.join("unknown project λ");
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    let fixture_name = if cfg!(windows) {
        "unknown fixture λ.exe"
    } else {
        "unknown fixture λ"
    };
    fs::copy(std::env::current_exe().unwrap(), root.join(fixture_name)).unwrap();
    let fixture_command = format!("./{fixture_name}");
    let _cleanup = DaemonCleanup::new(&root, &private);

    let mut first = OuterTerminal::spawn(&root, &private);
    first.wait_for("Initialize it?");
    first.send(b"y\r");
    first.finish_startup();
    first.send(b"4");
    first.wait_for("Agent definitions");
    eprintln!("M09 stage: empty workspace and template previews");
    assert_eq!(agent_items(&root, &private).len(), 0);
    first.send(b"a");
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(session_items(&root, &private).len(), 0);

    for index in 0..3 {
        first.send(b"p");
        first.wait_for("Display name");
        first.send(b"\x1b");
        first.wait_for("Discard draft?");
        first.send(b"y");
        first.wait_for("Agent definitions");
        if index != 2 {
            first.send(b"]");
        }
    }
    assert_eq!(agent_items(&root, &private).len(), 0);

    first.send(b"p");
    first.wait_for("Display name");
    first.send(b"\x13");
    wait_until(
        || {
            agent_items(&root, &private)
                .iter()
                .any(|definition| definition["display_name"] == "OpenCode")
        },
        "disabled template copy",
    );
    wait_until(
        || !first.screen_contents().contains("┌Form"),
        "template copy form completion",
    );
    let copied_template = agent_by_name(&root, &private, "OpenCode");
    assert_eq!(copied_template["command"], "opencode");
    assert_eq!(copied_template["enabled"], false);
    eprintln!("M09 stage: disabled template copied");

    first.send(b"n");
    first.wait_for("Display name");
    fill_agent_form(
        &mut first,
        "Unknown native CLI",
        "relayterm-m09-missing",
        &fixture_arguments(),
        "",
        "interactive_terminal",
        false,
    );
    wait_until(
        || !first.screen_contents().contains("┌Form"),
        "custom definition form completion",
    );
    first.wait_for("Unknown native CLI");
    if !agent_selected(&first, "Unknown native CLI") {
        first.send(previous_selection_input());
        wait_until(
            || agent_selected(&first, "Unknown native CLI"),
            "unknown definition selection",
        );
    }
    eprintln!("M09 stage: unknown definition selected");
    let unavailable_started = Instant::now();
    first.send(b"v");
    first.wait_for("Availability: not_found");
    eprintln!("M09 stage: unavailable definition checked");
    let unavailable_latency = unavailable_started.elapsed();
    let listed = admin(&root, &private, &["agent", "list"]);
    let definition_id = listed["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["display_name"] == "Unknown native CLI")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = listed["result"]["revision"].as_str().unwrap();
    let unavailable = admin_output(
        &root,
        &private,
        &[
            "agent",
            "check",
            &definition_id,
            "--expected-revision",
            revision,
        ],
    );
    assert_eq!(unavailable.status.code(), Some(3));
    let unavailable_json: Value = serde_json::from_slice(&unavailable.stdout).unwrap();
    assert_eq!(unavailable_json["result"]["status"], "not_found");
    assert!(!String::from_utf8_lossy(&unavailable.stdout).contains("relayterm-m09-missing"));

    first.send(b"e");
    first.wait_for("Enabled (true/false)");
    eprintln!("M09 stage: command edit form opened");
    first.send(b"\t");
    wait_until(
        || form_field_selected(&first, "Command"),
        "command field selection",
    );
    first.send(b"\x15");
    wait_until(
        || !first.screen_contents().contains("relayterm-m09-missing"),
        "command field clearing",
    );
    send_form_text(&mut first, &fixture_command);
    first.wait_for("unknown fixture");
    first.send(b"\x13");
    eprintln!("M09 stage: command edit submitted");
    wait_until(
        || !first.screen_contents().contains("┌Form"),
        "command edit form completion",
    );
    let edited = agent_by_name(&root, &private, "Unknown native CLI");
    assert_eq!(edited["id"], definition_id);
    assert_eq!(edited["command"], fixture_command);
    let available_started = Instant::now();
    first.send(b"v");
    first.wait_for("Availability: available");
    eprintln!("M09 stage: custom definition and availability");
    let available_latency = available_started.elapsed();
    let listed = admin(&root, &private, &["agent", "list"]);
    let available = admin_output(
        &root,
        &private,
        &[
            "agent",
            "check",
            &definition_id,
            "--expected-revision",
            listed["result"]["revision"].as_str().unwrap(),
        ],
    );
    assert!(available.status.success());
    first.send(b" ");
    first.wait_for("[enabled] Unknown native CLI");

    let definition = agent_by_name(&root, &private, "Unknown native CLI");
    assert_eq!(definition["id"], definition_id);
    assert_eq!(
        definition["arguments"],
        serde_json::json!(fixture_arguments())
    );
    first.send(b"a");
    wait_until(
        || session_items(&root, &private).len() == 1,
        "first unknown session",
    );
    first.send(b"\r");
    first.wait_for("unknown-ready");
    first.wait_for("unknown-argument-0:<value with space>");
    first.wait_for("unknown-argument-1:<>");
    first.wait_for("unknown-argument-3:<repeat>");
    eprintln!("M09 stage: first argv and terminal input");

    let first_session = session_items(&root, &private).pop().unwrap();
    let first_session_id = first_session["session_id"].as_str().unwrap().to_owned();
    assert_eq!(first_session["agent_definition_id"], definition_id);
    assert_eq!(
        first_session["launch_definition"]["arguments"],
        serde_json::json!(fixture_arguments())
    );
    first.send(b"ihello from tui\r");
    first.wait_for("unknown-input:<hello from tui>");
    first.send(&[0x1d]);
    first.wait_for("READ ONLY");
    first.send(b"\x1b");
    first.wait_for("Sessions selected");
    first.send(b"2nM09 continuity\tExplicit task state only\tnormal\t\tNative gate\t\x13");
    wait_for_task_status(&root, &private, "backlog");
    first.send(b"rc");
    wait_for_task_status(&root, &private, "active");
    first.send(b"pFirst instance progress\tReal TUI and PTY verified\x13");
    wait_for_progress(&root, &private, "First instance progress");
    eprintln!("M09 stage: first claim and progress");

    first.send(b"4e\t\t\x1b[F\rreplacement\x13");
    first.wait_for("Unknown native CLI");
    assert_eq!(
        session_by_id(&root, &private, &first_session_id)["launch_definition"]["arguments"],
        serde_json::json!(fixture_arguments())
    );
    first.send(b" a");
    first.wait_for("Sessions selected");
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(session_items(&root, &private).len(), 1);
    first.send(b"4 a");
    wait_until(
        || session_items(&root, &private).len() == 2,
        "replacement session",
    );

    first.send(b"2hContinue with replacement\tDefinition edited while running\t\tNative gate\t\tClaim and complete\x13");
    wait_for_task_status(&root, &private, "handover_ready");
    first.send(b"3");
    first.wait_for("Sessions selected");
    first.send(next_selection_input());
    first.send(b"2c");
    wait_for_task_status(&root, &private, "active");
    first.send(b"d");
    wait_for_task_status(&root, &private, "done");
    eprintln!("M09 stage: replacement handover and completion");
    first.send(b"\x03");
    first.wait_exit();
    eprintln!("M09 stage: first TUI client closed");

    let mut second = OuterTerminal::spawn(&root, &private);
    second.finish_startup();
    eprintln!("M09 stage: second TUI client reconnected");
    second.send(b"3");
    second.wait_for("Sessions selected");
    assert_eq!(agent_items(&root, &private).len(), 2);
    assert_eq!(session_items(&root, &private).len(), 2);
    second.send(b"\x03");
    second.wait_exit();
    eprintln!("M09 stage: second TUI client closed");

    eprintln!("M09 stage: stopping daemon and sessions");
    let stopped = admin(&root, &private, &["daemon", "stop", "--terminate-sessions"]);
    assert_eq!(stopped["ok"], true);
    eprintln!("M09 stage: restarting daemon");
    let restarted = admin(&root, &private, &["daemon", "start"]);
    assert_eq!(restarted["ok"], true);
    let persisted = agent_by_name(&root, &private, "Unknown native CLI");
    assert_eq!(persisted["id"], definition_id);
    assert_eq!(persisted["enabled"], true);
    eprintln!(
        "M09 native gate definitions=2 sessions=2 arguments={} unavailable_check_ms={} available_check_ms={} credentials=not_required providers=not_run",
        fixture_arguments().len(),
        unavailable_latency.as_millis(),
        available_latency.as_millis()
    );
}

fn fill_agent_form(
    terminal: &mut OuterTerminal,
    name: &str,
    command: &str,
    arguments: &[String],
    environment: &str,
    capabilities: &str,
    enabled: bool,
) {
    terminal.send(name.as_bytes());
    terminal.send(b"\t");
    terminal.send(command.as_bytes());
    terminal.send(b"\t");
    terminal.send(
        arguments
            .iter()
            .map(|value| if value.is_empty() { "<empty>" } else { value })
            .collect::<Vec<_>>()
            .join("\r")
            .as_bytes(),
    );
    terminal.send(b"\t");
    terminal.send(environment.as_bytes());
    terminal.send(b"\t");
    terminal.send(capabilities.as_bytes());
    terminal.send(b"\t");
    terminal.send(if enabled { b"true" } else { b"false" });
    terminal.send(b"\x13");
}

fn fixture_arguments() -> Vec<String> {
    vec![
        "--exact",
        "unknown_interactive_process",
        "--ignored",
        "--nocapture",
        "--",
        "value with space",
        "",
        "repeat",
        "repeat",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn agent_items(root: &std::path::Path, private: &std::path::Path) -> Vec<Value> {
    admin(root, private, &["agent", "list"])["result"]["items"]
        .as_array()
        .unwrap()
        .clone()
}

fn agent_by_name(root: &std::path::Path, private: &std::path::Path, name: &str) -> Value {
    agent_items(root, private)
        .into_iter()
        .find(|definition| definition["display_name"] == name)
        .unwrap()
}

fn send_form_text(terminal: &mut OuterTerminal, value: &str) {
    #[cfg(unix)]
    {
        let mut paste = b"\x1b[200~".to_vec();
        paste.extend_from_slice(value.as_bytes());
        paste.extend_from_slice(b"\x1b[201~");
        terminal.send(&paste);
    }
    #[cfg(windows)]
    terminal.send(value.as_bytes());
}

fn agent_selected(terminal: &OuterTerminal, name: &str) -> bool {
    terminal
        .screen_contents()
        .lines()
        .any(|line| line.contains('>') && line.contains(name))
}

fn form_field_selected(terminal: &OuterTerminal, label: &str) -> bool {
    terminal
        .screen_contents()
        .lines()
        .any(|line| line.contains(&format!("> {label} (")))
}

fn session_items(root: &std::path::Path, private: &std::path::Path) -> Vec<Value> {
    admin(root, private, &["session", "list"])["result"]["items"]
        .as_array()
        .unwrap()
        .clone()
}

fn session_by_id(root: &std::path::Path, private: &std::path::Path, id: &str) -> Value {
    session_items(root, private)
        .into_iter()
        .find(|value| value["session_id"] == id)
        .unwrap()
}

fn wait_for_task_status(root: &std::path::Path, private: &std::path::Path, status: &str) {
    wait_until(
        || {
            admin(root, private, &["task", "list"])["result"]["items"]
                .as_array()
                .unwrap()
                .first()
                .is_some_and(|task| task["status"] == status)
        },
        "task status",
    )
}

fn wait_for_progress(root: &std::path::Path, private: &std::path::Path, marker: &str) {
    wait_until(
        || {
            let tasks = admin(root, private, &["task", "list"]);
            let task = &tasks["result"]["items"][0];
            let Some(id) = task["id"].as_str() else {
                return false;
            };
            let Some(revision) = tasks["result"]["revision"].as_str() else {
                return false;
            };
            let output = admin_output(
                root,
                private,
                &["task", "history", id, "--expected-revision", revision],
            );
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains(marker)
        },
        "progress",
    )
}
