use base64::{Engine as _, engine::general_purpose::STANDARD};
use relayterm_application::Request;
use relayterm_daemon::{connect_route, initialize, prepare_workspace};
use relayterm_domain::{Actor, WorkspaceId};
use relayterm_protocol::{
    AttachmentId, DecimalOffset, Operation, SessionId, SessionReadOutputParams,
};
use serde_json::{Value, json};
use std::{
    io::{Read as _, Write as _},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[cfg(unix)]
fn native_temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("rt7-")
        .tempdir_in(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        })
        .unwrap()
}

#[cfg(windows)]
fn native_temp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
#[ignore]
fn interactive_fixture_child() {
    print!("\x1b[?1049h\x1b[2J\x1b[2;3Hfixture-ready界\x1b[?2004h\r\n");
    println!(
        "fixture-term:{}",
        std::env::var("TERM").as_deref() == Ok("xterm-256color")
    );
    println!(
        "fixture-private-canary-absent:{}",
        std::env::var_os("RELAYTERM_PRIVATE_CANARY").is_none()
    );
    std::io::stdout().flush().unwrap();
    let mut input = std::io::stdin().lock();
    let mut byte = [0_u8; 1];
    let mut pending = Vec::new();
    while input.read_exact(&mut byte).is_ok() {
        if !matches!(byte[0], b'\r' | b'\n') {
            pending.push(byte[0]);
            continue;
        }
        if pending.is_empty() {
            continue;
        }
        let line = String::from_utf8(std::mem::take(&mut pending)).unwrap();
        if line == "flood" {
            for _ in 0..32_768 {
                print!("0123456789abcdef0123456789abcdef\r\n");
            }
            print!("flood-complete\r\n");
            std::io::stdout().flush().unwrap();
            continue;
        }
        if line == "descendant" {
            #[cfg(windows)]
            let child = std::process::Command::new("cmd.exe")
                .args(["/D", "/C", "ping -n 300 127.0.0.1 >nul"])
                .spawn()
                .unwrap();
            #[cfg(not(windows))]
            let child = std::process::Command::new("sleep")
                .arg("300")
                .spawn()
                .unwrap();
            println!("fixture-descendant:{}", child.id());
            drop(child);
            std::io::stdout().flush().unwrap();
            continue;
        }
        println!("fixture-echo:{line}");
        std::io::stdout().flush().unwrap();
        if line == "quit" {
            return;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn three_real_ptys_survive_client_disconnect_and_reconstruct() {
    let scenario_complete = Arc::new(AtomicBool::new(false));
    let watchdog_state = scenario_complete.clone();
    std::thread::spawn(move || {
        std::thread::sleep(whole_scenario_timeout());
        if !watchdog_state.load(Ordering::Acquire) {
            eprintln!("M07 PTY gate: whole-scenario deadline exceeded");
            std::process::abort();
        }
    });
    let temporary = native_temp();
    let project = temporary.path().join("project with space");
    std::fs::create_dir(&project).unwrap();
    let private = temporary.path().join("private");
    let (route, _) = initialize(&project, Some(private.clone()), "PTY gate workspace".into())
        .await
        .unwrap();
    let workspace: WorkspaceId = route.workspace_id.parse().unwrap();
    let prepared = prepare_workspace(&project, Some(private.clone()), workspace)
        .await
        .unwrap();
    let (fixture, fixture_arguments) = interactive_fixture(&project);
    let outcome = prepared
        .service()
        .execute(
            workspace,
            Actor::LocalUser,
            Request::AddDefinition {
                display_name: "Neutral interactive fixture".into(),
                command: fixture,
                arguments: fixture_arguments,
                environment_allowlist: Vec::new(),
                capabilities: vec!["interactive_terminal".into()],
                enabled: true,
            },
        )
        .await
        .unwrap();
    let definition = outcome.committed.snapshot.state().unwrap().definitions()[0]
        .record()
        .id
        .to_string();
    let bad_outcome = prepared
        .service()
        .execute(
            workspace,
            Actor::LocalUser,
            Request::AddDefinition {
                display_name: "Missing executable fixture".into(),
                command: project
                    .join("missing-relayterm-fixture")
                    .to_string_lossy()
                    .into_owned(),
                arguments: Vec::new(),
                environment_allowlist: Vec::new(),
                capabilities: vec!["interactive_terminal".into()],
                enabled: true,
            },
        )
        .await
        .unwrap();
    let bad_definition = bad_outcome
        .committed
        .snapshot
        .state()
        .unwrap()
        .definitions()
        .iter()
        .find(|definition| definition.record().display_name == "Missing executable fixture")
        .unwrap()
        .record()
        .id
        .to_string();
    let (ready_tx, ready_rx) = mpsc::channel();
    let server = tokio::spawn(prepared.run_with_ready(move || ready_tx.send(()).unwrap()));
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let client = connect_route(&route, Some(private.clone())).await.unwrap();
    let bad_receipt = uuid::Uuid::new_v4().to_string();
    assert!(
        client
            .call::<_, Value>(
                Operation::SessionCreate,
                &json!({
                    "receipt_id":bad_receipt,
                    "launch_kind":"definition","definition_id":bad_definition,"task_id":null,
                    "working_directory":null,"rows":24,"columns":80
                }),
            )
            .await
            .is_err()
    );
    let failed_instances: Value = client
        .call(Operation::SessionList, &json!({"limit":50}))
        .await
        .unwrap();
    assert!(
        failed_instances["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|instance| instance["status"] == "failed" && instance["exit_code"].is_null())
    );
    let failed_count = failed_instances["items"].as_array().unwrap().len();
    assert!(
        client
            .call::<_, Value>(
                Operation::SessionCreate,
                &json!({
                    "receipt_id":bad_receipt,"launch_kind":"definition",
                    "definition_id":bad_definition,"task_id":null,
                    "working_directory":null,"rows":24,"columns":80
                }),
            )
            .await
            .is_err()
    );
    let after_failed_retry: Value = client
        .call(Operation::SessionList, &json!({"limit":50}))
        .await
        .unwrap();
    assert_eq!(
        after_failed_retry["items"].as_array().unwrap().len(),
        failed_count,
        "a failed launch receipt must not create a second durable instance"
    );
    let first_receipt = uuid::Uuid::new_v4().to_string();
    let first = create_definition_session(&client, &definition, &first_receipt).await;
    eprintln!("M07 PTY gate: first native session launched");
    wait_for_text(&client, &first, "fixture-ready").await;
    eprintln!("M07 PTY gate: initial terminal output reconstructed");
    let repeated = create_definition_session(&client, &definition, &first_receipt).await;
    assert_eq!(repeated, first, "a launch receipt must be idempotent");
    assert!(
        client
            .call::<_, Value>(
                Operation::SessionCreate,
                &json!({
                    "receipt_id":first_receipt,"launch_kind":"default_shell","definition_id":null,
                    "task_id":null,"working_directory":null,"rows":24,"columns":80
                }),
            )
            .await
            .is_err(),
        "conflicting receipt reuse must fail"
    );
    let second =
        create_definition_session(&client, &definition, &uuid::Uuid::new_v4().to_string()).await;
    let shell: Value = client
        .call(
            Operation::SessionCreate,
            &json!({
                "receipt_id":uuid::Uuid::new_v4().to_string(),
                "launch_kind":"default_shell","definition_id":null,"task_id":null,
                "working_directory":null,"rows":24,"columns":80
            }),
        )
        .await
        .unwrap();
    let shell_id = shell["session_id"].as_str().unwrap().to_owned();
    let mut capacity_sessions = Vec::new();
    for _ in 0..5 {
        capacity_sessions.push(
            create_definition_session(&client, &definition, &uuid::Uuid::new_v4().to_string())
                .await,
        );
    }
    assert!(
        client
            .call::<_, Value>(
                Operation::SessionCreate,
                &json!({
                    "receipt_id":uuid::Uuid::new_v4().to_string(),
                    "launch_kind":"definition","definition_id":definition,"task_id":null,
                    "working_directory":null,"rows":24,"columns":80
                }),
            )
            .await
            .is_err(),
        "the ninth concurrent session must be rejected"
    );
    let instances: Value = client
        .call(Operation::SessionList, &json!({"limit":50}))
        .await
        .unwrap();
    let first_instance = instance_for_session(&instances, &first);
    let second_instance = instance_for_session(&instances, &second);
    let coordination_peer = connect_route(&route, Some(private.clone())).await.unwrap();
    let created: Value = client
        .call(
            Operation::TaskCreate,
            &json!({
                "expected_revision":instances["revision"],"title":"Real PTY coordination",
                "description":"Coordinate work owned by real supervised instances",
                "priority":"normal","scope_paths":[],"acceptance_notes":"Complete through handover",
                "dependency_ids":[]
            }),
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
    client
        .call::<_, Value>(
            Operation::TaskClaim,
            &json!({"task_id":task_id,"instance_id":first_instance}),
        )
        .await
        .unwrap();
    assert!(
        coordination_peer
            .call::<_, Value>(
                Operation::TaskClaim,
                &json!({"task_id":task_id,"instance_id":second_instance}),
            )
            .await
            .is_err()
    );
    let progress: Value = client
        .call(
            Operation::ProgressAppend,
            &json!({
                "task_id":task_id,"summary":"First real instance produced progress",
                "verification":"PTY output and input verified"
            }),
        )
        .await
        .unwrap();
    client
        .call::<_, Value>(
            Operation::HandoverCreate,
            &json!({
                "task_id":task_id,"expected_revision":progress["revision"],
                "summary":"Continue in the second real instance","decisions":"Use neutral session IDs",
                "changed_paths":[],"verification_performed":"Real PTY gate",
                "open_questions":"","recommended_next_action":"Claim and complete"
            }),
        )
        .await
        .unwrap();
    let resumed: Value = coordination_peer
        .call(
            Operation::TaskClaim,
            &json!({"task_id":task_id,"instance_id":second_instance}),
        )
        .await
        .unwrap();
    coordination_peer
        .call::<_, Value>(
            Operation::TaskTransition,
            &json!({"task_id":task_id,"expected_revision":resumed["revision"],"status":"done"}),
        )
        .await
        .unwrap();
    let _ = ready;
    let lease: Value = client
        .call(Operation::SessionAcquireInput, &json!({"session_id":first}))
        .await
        .unwrap();
    let lease_id = lease["lease_id"].as_str().unwrap();
    client
        .call::<_, Value>(
            Operation::SessionInput,
            &json!({"session_id":first,"lease_id":lease_id,"sequence":"1","data":STANDARD.encode(terminal_line("hello"))}),
        )
        .await
        .unwrap();
    wait_for_text(&client, &first, "fixture-echo:hello").await;
    assert!(
        client
            .call::<_, Value>(
                Operation::SessionInput,
                &json!({"session_id":first,"lease_id":lease_id,"sequence":"1","data":STANDARD.encode(terminal_line("duplicate"))}),
            )
            .await
            .is_err(),
        "a duplicate input sequence must fail before writing"
    );
    client
        .call::<_, Value>(
            Operation::SessionInput,
            &json!({"session_id":first,"lease_id":lease_id,"sequence":"2","data":STANDARD.encode(terminal_line("descendant"))}),
        )
        .await
        .unwrap();
    let descendant_state = wait_for_text(&client, &first, "fixture-descendant:").await;
    let descendant_text = visible_text(&descendant_state);
    let descendant_pid: u32 = descendant_text
        .split("fixture-descendant:")
        .nth(1)
        .unwrap()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap();
    eprintln!("M07 PTY gate: descendant process observed");
    client
        .call::<_, Value>(
            Operation::SessionResize,
            &json!({"session_id":first,"lease_id":lease_id,"rows":30,"columns":100}),
        )
        .await
        .unwrap();
    client
        .call::<_, Value>(
            Operation::SessionInput,
            &json!({"session_id":first,"lease_id":lease_id,"sequence":"3","data":STANDARD.encode(terminal_line("after-resize"))}),
        )
        .await
        .unwrap();
    let attached = wait_for_text(&client, &first, "fixture-echo:after-resize").await;
    let attached_again = wait_for_text(&client, &first, "fixture-echo:after-resize").await;
    assert_eq!(attached["attachment_id"], attached_again["attachment_id"]);
    assert_eq!(attached["snapshot"]["rows"], 30);
    assert_eq!(attached["snapshot"]["columns"], 100);
    assert_native_full_screen_mode(&attached);
    let environment = wait_for_text(&client, &first, "fixture-private-canary-absent:true").await;
    let environment_text: String = environment["snapshot"]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|cell| cell["contents"].as_str())
        .collect();
    assert!(environment_text.contains("fixture-term:true"));

    let observer = connect_route(&route, Some(private.clone())).await.unwrap();
    let conflict = observer
        .call::<_, Value>(Operation::SessionAcquireInput, &json!({"session_id":first}))
        .await;
    assert!(conflict.is_err(), "a second writer must be rejected");
    drop(client);
    let deadline = Instant::now() + Duration::from_secs(15);
    let replacement_lease = loop {
        match observer
            .call::<_, Value>(Operation::SessionAcquireInput, &json!({"session_id":first}))
            .await
        {
            Ok(value) => break value,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(20)).await
            }
            Err(error) => panic!("input lease was not released: {error:?}"),
        }
    };
    assert!(replacement_lease["lease_id"].as_str().is_some());
    let reconstructed = wait_for_text(&observer, &first, "fixture-echo:after-resize").await;
    assert_ne!(attached["attachment_id"], reconstructed["attachment_id"]);
    assert_eq!(reconstructed["snapshot"]["rows"], 30);
    assert_native_full_screen_mode(&reconstructed);
    assert_eq!(
        attached["snapshot"]["cells"], reconstructed["snapshot"]["cells"],
        "reattachment must reproduce the authoritative visible grid"
    );
    assert_eq!(
        attached["snapshot"]["cursor_row"],
        reconstructed["snapshot"]["cursor_row"]
    );
    assert_eq!(
        attached["snapshot"]["cursor_column"],
        reconstructed["snapshot"]["cursor_column"]
    );
    assert_eq!(
        attached["snapshot"]["bracketed_paste"],
        reconstructed["snapshot"]["bracketed_paste"]
    );
    let replacement_lease_id = replacement_lease["lease_id"].as_str().unwrap();
    let stream_start = reconstructed["snapshot"]["raw_offset"].as_u64().unwrap();
    observer
        .call::<_, Value>(
            Operation::SessionInput,
            &json!({"session_id":first,"lease_id":replacement_lease_id,"sequence":"1","data":STANDARD.encode(terminal_line("stream-check"))}),
        )
        .await
        .unwrap();
    let mut stream_offset = stream_start;
    let mut streamed = Vec::new();
    let stream_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let (metadata, frame) = observer
            .read_session_output(&SessionReadOutputParams {
                session_id: first.parse::<SessionId>().unwrap(),
                attachment_id: reconstructed["attachment_id"]
                    .as_str()
                    .unwrap()
                    .parse::<AttachmentId>()
                    .unwrap(),
                after_offset: DecimalOffset::new(stream_offset),
            })
            .await
            .unwrap();
        assert!(!metadata.resnapshot_required);
        stream_offset = metadata.next_offset.get();
        if let Some(frame) = frame {
            streamed.extend(frame.data);
        }
        if String::from_utf8_lossy(&streamed).contains("fixture-echo:stream-check") {
            break;
        }
        assert!(
            Instant::now() < stream_deadline,
            "bounded binary terminal output did not arrive"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    observer
        .call::<_, Value>(
            Operation::SessionInput,
            &json!({"session_id":first,"lease_id":replacement_lease_id,"sequence":"2","data":STANDARD.encode(terminal_line("flood"))}),
        )
        .await
        .unwrap();
    let flooded = wait_for_text(&observer, &first, "flood-complete").await;
    assert!(
        flooded["snapshot"]["retained_from_offset"]
            .as_u64()
            .unwrap()
            > 0,
        "high-volume output must truncate bounded raw history"
    );
    let (expired, frame) = observer
        .read_session_output(&SessionReadOutputParams {
            session_id: first.parse().unwrap(),
            attachment_id: reconstructed["attachment_id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            after_offset: DecimalOffset::new(stream_start),
        })
        .await
        .unwrap();
    assert!(expired.resnapshot_required);
    assert!(frame.is_none());

    observer
        .call::<_, Value>(Operation::SessionTerminate, &json!({"session_id":first}))
        .await
        .unwrap();
    eprintln!("M07 PTY gate: primary process tree terminated");
    let descendant_deadline = Instant::now() + Duration::from_secs(15);
    while process_exists(descendant_pid) {
        assert!(
            Instant::now() < descendant_deadline,
            "owned descendant survived session termination"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    for session in [&second, &shell_id]
        .into_iter()
        .chain(capacity_sessions.iter())
    {
        observer
            .call::<_, Value>(Operation::SessionTerminate, &json!({"session_id":session}))
            .await
            .unwrap();
    }
    let status: Value = observer
        .call(Operation::DaemonStatus, &json!({}))
        .await
        .unwrap();
    observer
        .call::<_, Value>(
            Operation::DaemonShutdown,
            &json!({"generation":status["generation"],"terminate_sessions":true}),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_private_tree_has_no_terminal_capture(&private);
    scenario_complete.store(true, Ordering::Release);
    eprintln!("M07 PTY gate: completed");
}

fn terminal_line(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(b'\r');
    #[cfg(windows)]
    bytes.push(b'\n');
    bytes
}

#[cfg(not(windows))]
fn interactive_fixture(_: &Path) -> (String, Vec<String>) {
    (
        std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        vec![
            "interactive_fixture_child".into(),
            "--exact".into(),
            "--ignored".into(),
            "--nocapture".into(),
            "--test-threads=1".into(),
        ],
    )
}

#[cfg(windows)]
fn interactive_fixture(project: &Path) -> (String, Vec<String>) {
    let script = project.join("interactive-fixture.ps1");
    let content = r#"$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RelaytermConsoleMode {
    [DllImport("kernel32.dll")] public static extern IntPtr GetStdHandle(int value);
    [DllImport("kernel32.dll")] public static extern bool GetConsoleMode(IntPtr handle, out uint mode);
    [DllImport("kernel32.dll")] public static extern bool SetConsoleMode(IntPtr handle, uint mode);
}
'@
$stdout = [RelaytermConsoleMode]::GetStdHandle(-11)
$mode = [uint32]0
if (-not [RelaytermConsoleMode]::GetConsoleMode($stdout, [ref]$mode)) { throw 'stdout console mode unavailable' }
if (-not [RelaytermConsoleMode]::SetConsoleMode($stdout, $mode -bor 4)) { throw 'VT output mode unavailable' }
$escape = [char]27
$wide = [char]0x754c
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::Write("$escape[?1049h$escape[2J$escape[2;3Hfixture-ready$wide$escape[?2004h`r`n")
[Console]::WriteLine("fixture-term:" + (($env:TERM -eq 'xterm-256color').ToString().ToLowerInvariant()))
[Console]::WriteLine("fixture-private-canary-absent:" + (($null -eq $env:RELAYTERM_PRIVATE_CANARY).ToString().ToLowerInvariant()))
while (($line = [Console]::ReadLine()) -ne $null) {
    if ($line -eq 'flood') {
        [Console]::Write("$escape[?1049h")
        [Console]::Write(('0123456789abcdef0123456789abcdef' + "`r`n") * 32768)
        [Console]::WriteLine('flood-complete')
        continue
    }
    if ($line -eq 'descendant') {
        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = $env:ComSpec
        $start.Arguments = '/D /C ping -n 300 127.0.0.1 >nul'
        $start.UseShellExecute = $false
        $start.CreateNoWindow = $true
        $child = [Diagnostics.Process]::Start($start)
        [Console]::Write("$escape[?1049h")
        [Console]::WriteLine("fixture-descendant:" + $child.Id)
        continue
    }
    [Console]::Write("$escape[?1049h")
    [Console]::WriteLine("fixture-echo:" + $line)
    if ($line -eq 'quit') { exit 0 }
}
"#;
    std::fs::write(&script, content).unwrap();
    let system_root = std::env::var_os("SystemRoot").unwrap();
    let powershell = Path::new(&system_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    (
        powershell.to_string_lossy().into_owned(),
        vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script.to_string_lossy().into_owned(),
        ],
    )
}

async fn create_definition_session(
    client: &relayterm_client::Client,
    definition: &str,
    receipt: &str,
) -> String {
    let value: Value = client
        .call(
            Operation::SessionCreate,
            &json!({
                "receipt_id":receipt,
                "launch_kind":"definition","definition_id":definition,"task_id":null,
                "working_directory":null,"rows":24,"columns":80
            }),
        )
        .await
        .unwrap();
    value["session_id"].as_str().unwrap().to_owned()
}

async fn wait_for_text(client: &relayterm_client::Client, session: &str, expected: &str) -> Value {
    let deadline = Instant::now() + output_wait_timeout();
    loop {
        let value: Value = client
            .call(Operation::SessionAttach, &json!({"session_id":session}))
            .await
            .unwrap();
        let text = visible_text(&value);
        if text.contains(expected) {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "terminal output marker {expected:?} did not arrive; visible text: {text:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn output_wait_timeout() -> Duration {
    if cfg!(windows) {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(15)
    }
}

fn whole_scenario_timeout() -> Duration {
    if cfg!(windows) {
        Duration::from_secs(120)
    } else {
        Duration::from_secs(60)
    }
}

fn assert_native_full_screen_mode(value: &Value) {
    #[cfg(windows)]
    assert_eq!(
        value["snapshot"]["alternate_screen"], false,
        "ConPTY must expose its rendered primary-screen representation"
    );
    #[cfg(not(windows))]
    assert_eq!(
        value["snapshot"]["alternate_screen"], true,
        "Unix PTYs must preserve alternate-screen mode"
    );
}

fn visible_text(value: &Value) -> String {
    value["snapshot"]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|cell| cell["contents"].as_str())
        .collect()
}

fn instance_for_session<'a>(instances: &'a Value, session_id: &str) -> &'a str {
    instances["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|instance| instance["session_id"] == session_id)
        .and_then(|instance| instance["id"].as_str())
        .unwrap()
}

#[cfg(not(windows))]
fn process_exists(process_id: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &process_id.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn process_exists(process_id: u32) -> bool {
    let filter = format!("PID eq {process_id}");
    std::process::Command::new("tasklist.exe")
        .args(["/FI", &filter, "/NH"])
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout).contains(&process_id.to_string())
        })
}

fn assert_private_tree_has_no_terminal_capture(root: &Path) {
    fn visit(path: &Path) {
        for item in std::fs::read_dir(path).unwrap() {
            let item = item.unwrap();
            let path = item.path();
            if path.is_dir() {
                visit(&path);
            } else {
                let name = path.file_name().unwrap().to_string_lossy();
                assert!(!name.contains("terminal") && !name.contains("scrollback"));
            }
        }
    }
    visit(root);
}
