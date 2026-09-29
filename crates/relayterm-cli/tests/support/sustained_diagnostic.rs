//! Opt-in bounded diagnostics, never part of sustained acceptance measurements.
use super::*;

const RUN: Duration = Duration::from_secs(12);

pub(super) fn emit(output: &mut impl Write) {
    let start = Instant::now();
    let repetitions = fs::read_to_string("diagnostic-write-size")
        .map(|value| value.parse::<usize>().unwrap())
        .unwrap_or(2048);
    // Stay below the existing 100 ms pacing cap even with larger diagnostic writes.
    assert!((1..=8192).contains(&repetitions));
    let chunk = "0123456789abcdef".repeat(repetitions);
    let duration_s = fs::read_to_string("diagnostic-duration")
        .map(|value| value.parse::<u64>().unwrap())
        .unwrap_or(RUN.as_secs());
    assert!((1..=120).contains(&duration_s));
    let duration = Duration::from_secs(duration_s);
    let mut bytes = 0_u64;
    let mut writes = Duration::ZERO;
    let mut flushes = Duration::ZERO;
    let mut checks = Duration::ZERO;
    let mut requested_sleep = Duration::ZERO;
    let mut actual_sleep = Duration::ZERO;
    let mut sleeps = 0;
    let mut intervals = vec![(0_u128, 0_u64)];
    let mut last_sample = Duration::ZERO;
    while start.elapsed() < duration {
        let before = Instant::now();
        let stop = Path::new("load.stop").exists();
        checks += before.elapsed();
        if stop {
            break;
        }
        let before = Instant::now();
        output.write_all(chunk.as_bytes()).unwrap();
        writes += before.elapsed();
        let before = Instant::now();
        output.flush().unwrap();
        flushes += before.elapsed();
        bytes += chunk.len() as u64;
        let elapsed = start.elapsed();
        if elapsed - last_sample >= Duration::from_millis(500) {
            intervals.push((elapsed.as_nanos(), bytes));
            last_sample = elapsed;
        }
        let due = Duration::from_secs_f64(bytes as f64 / (3 * 1024 * 1024) as f64);
        if let Some(delay) = due.checked_sub(start.elapsed()) {
            let delay = delay.min(Duration::from_millis(100));
            let before = Instant::now();
            std::thread::sleep(delay);
            actual_sleep += before.elapsed();
            requested_sleep += delay;
            sleeps += 1;
        }
    }
    fs::write(
        "diagnostic.done",
        serde_json::to_vec(&serde_json::json!({
        "elapsed_us":start.elapsed().as_micros(), "bytes":bytes,"chunk_bytes":chunk.len(),"duration_s":duration_s,
            "intervals_elapsed_ns_bytes":intervals,
            "write_us":writes.as_micros(), "flush_us":flushes.as_micros(),
            "stop_check_us":checks.as_micros(), "sleep_requested_us":requested_sleep.as_micros(),
            "sleep_actual_us":actual_sleep.as_micros(), "sleep_count":sleeps
        }))
        .unwrap(),
    )
    .unwrap();
}

fn drain(mut reader: impl Read, delay: Duration) -> Value {
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes = 0_u64;
    let mut reads = 0_u64;
    let mut blocked = Duration::ZERO;
    let mut largest = 0;
    let mut delayed = Duration::ZERO;
    loop {
        let before = Instant::now();
        let result = reader.read(&mut buffer);
        blocked += before.elapsed();
        let Ok(amount) = result else {
            break;
        };
        if amount == 0 {
            break;
        }
        bytes += amount as u64;
        reads += 1;
        largest = largest.max(amount);
        if !delay.is_zero() {
            let before = Instant::now();
            std::thread::sleep(delay);
            delayed += before.elapsed();
        }
    }
    serde_json::json!({"bytes":bytes,"reads":reads,"read_wait_us":blocked.as_micros(),"max_read":largest,"delay_requested_each_us":delay.as_micros(),"delay_actual_us":delayed.as_micros()})
}

fn report(root: &Path, mode: &str, consumer: Value) {
    let producer: Value =
        serde_json::from_slice(&fs::read(root.join("diagnostic.done")).unwrap()).unwrap();
    eprintln!(
        "M12 diagnostic {}",
        serde_json::json!({"mode":mode,"profile":if cfg!(debug_assertions) {"debug"} else {"release"},"producer":producer,"consumer":consumer})
    );
    if mode == "pipe" {
        let bytes = producer["bytes"].as_u64().unwrap();
        let target = 3 * 1024 * 1024 * producer["duration_s"].as_u64().unwrap();
        assert!(
            bytes.abs_diff(target) <= 131072,
            "pipe pacing control failed"
        );
    }
}

#[test]
#[ignore = "explicit serial diagnostic only"]
fn compare_pipe_native_and_daemon() {
    compare(false, Duration::ZERO, 2048, RUN, false);
}

#[test]
#[ignore = "explicit serial diagnostic only"]
#[cfg(windows)]
fn compare_native_writer() {
    compare(true, Duration::ZERO, 2048, RUN, false);
}

#[test]
#[ignore = "explicit serial diagnostic only"]
fn compare_read_batching() {
    compare(false, Duration::from_millis(1), 2048, RUN, true);
}

#[test]
#[ignore = "explicit serial diagnostic only"]
fn compare_larger_writes() {
    compare(false, Duration::ZERO, 8192, RUN, false);
}

#[test]
#[ignore = "explicit serial diagnostic only"]
fn compare_native_long() {
    compare(false, Duration::ZERO, 2048, Duration::from_secs(120), true);
}

fn wait_for_producer(root: &Path, duration: Duration) {
    let deadline = Instant::now() + duration + DEADLINE;
    while !root.join("diagnostic.done").exists() {
        assert!(Instant::now() < deadline, "diagnostic producer deadline");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn compare(
    native_writer: bool,
    reader_delay: Duration,
    repetitions: usize,
    duration: Duration,
    native_only: bool,
) {
    let _serial = native_serial::NativeSerialGuard::acquire();
    let scratch = Scratch::new();
    let executable = std::env::current_exe().unwrap();
    let fixture = if executable
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .starts_with("hardening_gate-")
    {
        "tui::interactive_fixture_process"
    } else {
        "interactive_fixture_process"
    };
    for mode in ["pipe", "native", "daemon"] {
        if native_only && mode != "native" {
            continue;
        }
        let root = scratch.0.join(mode);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("diagnostic-write-size"), repetitions.to_string()).unwrap();
        fs::write(
            root.join("diagnostic-duration"),
            duration.as_secs().to_string(),
        )
        .unwrap();
        if native_writer {
            fs::write(root.join("diagnostic-native-writer"), b"").unwrap();
        }
        if mode == "pipe" {
            let mut child = Command::new(&executable)
                .args(["--exact", fixture, "--ignored", "--nocapture"])
                .current_dir(&root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let reader = child.stdout.take().unwrap();
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = tx.send(drain(reader, Duration::ZERO));
            });
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"diagnostic-flood\n")
                .unwrap();
            let deadline = Instant::now() + DEADLINE;
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    panic!("pipe deadline");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            report(
                &root,
                mode,
                rx.recv_timeout(Duration::from_secs(2))
                    .expect("pipe drain deadline"),
            );
        } else if mode == "native" {
            let session = NativeSession::spawn(SpawnRequest {
                program: executable.clone().into_os_string(),
                arguments: ["--exact", fixture, "--ignored", "--nocapture"]
                    .map(OsString::from)
                    .to_vec(),
                working_directory: root.clone(),
                environment: approved_environment(&[], std::env::vars_os()),
                rows: 24,
                columns: 80,
            })
            .unwrap();
            let (mut control, mut writer, reader) = session.into_parts();
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = tx.send(drain(reader, reader_delay));
            });
            write_input(&mut *writer, b"diagnostic-flood\r").unwrap();
            wait_for_producer(&root, duration);
            write_input(&mut *writer, b"exit-nonzero\r").unwrap();
            wait_until(
                || control.try_wait().unwrap().is_some(),
                "native exit deadline",
            );
            drop(writer);
            drop(control);
            report(
                &root,
                mode,
                rx.recv_timeout(Duration::from_secs(2))
                    .expect("native drain deadline"),
            );
        } else {
            let private = scratch.0.join("private");
            let _cleanup = DaemonCleanup {
                root: root.clone(),
                private: private.clone(),
            };
            admin(
                &root,
                &private,
                &["workspace", "init", "--name", "Diagnostic fixture"],
            );
            let definition = register_fixture(&root, &private);
            admin(
                &root,
                &private,
                &["session", "create", "--definition-id", &definition],
            );
            let result = admin(&root, &private, &["session", "list"]);
            let session = result["result"]["items"][0]["session_id"].as_str().unwrap();
            let input = private.join("input.txt");
            fs::write(&input, b"diagnostic-flood\r").unwrap();
            admin(
                &root,
                &private,
                &[
                    "session",
                    "input",
                    session,
                    "--file",
                    input.to_str().unwrap(),
                ],
            );
            wait_for_producer(&root, duration);
            report(
                &root,
                mode,
                serde_json::json!({"observer":false,"tui":false}),
            );
            admin(&root, &private, &["daemon", "stop", "--terminate-sessions"]);
        }
    }
}
