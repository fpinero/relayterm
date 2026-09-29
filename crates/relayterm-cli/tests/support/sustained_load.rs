//! Sustained native measurements, separate from the historical finite burst.
use super::*;
use std::sync::{atomic::AtomicBool, mpsc};

const LOAD_DURATION: Duration = Duration::from_secs(120);
const PRODUCER_LIMIT: Duration = Duration::from_secs(240);
const MIN_RATE: u64 = 2 * 1024 * 1024;
const TARGET_RATE: u64 = 3 * 1024 * 1024;
const MAX_GAP: Duration = Duration::from_secs(3);

/// Emit only synthetic data. Fixture files are private, numeric or empty controls.
pub(super) fn emit(output: &mut impl Write, flood: bool) {
    let role = if flood { "flood" } else { "screen" };
    let started = Instant::now();
    let mut bytes = 0_u64;
    let mut frames = 0_u64;
    let chunk = "0123456789abcdef".repeat(2048);
    fs::write(format!("{role}.ready"), b"").unwrap();
    while started.elapsed() < PRODUCER_LIMIT && !Path::new("load.stop").exists() {
        if flood {
            output.write_all(chunk.as_bytes()).unwrap();
            bytes += chunk.len() as u64;
        } else {
            let mut frame = format!("\x1b[H\x1b[2Jsynthetic-frame-{frames:06}\r\n");
            for row in 1..23 {
                frame.push_str(&format!("\x1b[{};1H{:078}", row + 1, frames % 10));
            }
            output.write_all(frame.as_bytes()).unwrap();
            bytes += frame.len() as u64;
        }
        output.flush().unwrap();
        frames += 1;
        let due = if flood {
            Duration::from_secs_f64(bytes as f64 / TARGET_RATE as f64)
        } else {
            Duration::from_millis(frames * 100)
        };
        if let Some(delay) = due.checked_sub(started.elapsed()) {
            std::thread::sleep(delay.min(Duration::from_millis(100)));
        }
    }
    fs::write(
        format!("{role}.done"),
        format!("{} {bytes} {frames}\n", started.elapsed().as_micros()),
    )
    .unwrap();
}

/// Bound process exit and pipe EOF independently; descendants may retain pipes.
pub(super) fn bounded_output(command: &mut Command) -> std::process::Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        // Administrative JSON is a single line; do not wait for descendant EOF.
        let mut data = Vec::new();
        let result = std::io::BufReader::new(&mut stdout)
            .take(8 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut data);
        let _ = tx.send((result, data));
    });
    let deadline = Instant::now() + DEADLINE;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("administrative process exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let (result, stdout) = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("administrative JSON response did not arrive within drain deadline");
    result.unwrap();
    assert!(stdout.len() <= 8 * 1024 * 1024);
    std::process::Output {
        status,
        stdout,
        stderr: Vec::new(),
    }
}

#[derive(Clone, Copy)]
struct Point {
    begin: Duration,
    end: Duration,
    flood: u64,
    screen: u64,
}

fn rate(first: Point, last: Point) -> Option<u64> {
    let elapsed = last.end.checked_sub(first.begin)?.as_nanos();
    if elapsed == 0 {
        return None;
    }
    Some((u128::from(last.flood.checked_sub(first.flood)?) * 1_000_000_000 / elapsed) as u64)
}

fn check_window(points: &[Point], start: Duration, end: Duration) -> Result<u64, &'static str> {
    let selected = points
        .iter()
        .copied()
        .filter(|point| point.begin >= start && point.end <= end)
        .collect::<Vec<_>>();
    let first = *selected
        .first()
        .ok_or("no load samples in latency window")?;
    let last = *selected.last().unwrap();
    if selected.len() < 2 || first.begin - start > MAX_GAP || end - last.end > MAX_GAP {
        return Err("insufficient load coverage of latency window");
    }
    for pair in selected.windows(2) {
        if pair[1].end.saturating_sub(pair[0].begin) > MAX_GAP {
            return Err("load sampling gap exceeded bound");
        }
        if pair[1].screen <= pair[0].screen {
            return Err("full-screen producer stopped during measurement");
        }
        if rate(pair[0], pair[1]).is_none_or(|rate| rate < MIN_RATE) {
            return Err("consumed flood rate below two MiB/s during measurement");
        }
    }
    let measured = rate(first, last).ok_or("invalid load counters")?;
    if measured < MIN_RATE {
        return Err("window rate below two MiB/s");
    }
    Ok(measured)
}

#[test]
fn load_contract_rejects_bursts_gaps_and_stopped_screen() {
    let points = (0..=5)
        .map(|second| Point {
            begin: Duration::from_secs(second),
            end: Duration::from_secs(second) + Duration::from_millis(10),
            flood: second * TARGET_RATE,
            screen: second * 1000,
        })
        .collect::<Vec<_>>();
    let end = Duration::from_secs(6);
    assert!(check_window(&points, Duration::ZERO, end).is_ok());
    let mut burst = points.clone();
    for point in &mut burst[2..] {
        point.flood = TARGET_RATE;
    }
    assert!(check_window(&burst, Duration::ZERO, end).is_err());
    let mut stopped = points.clone();
    stopped[3].screen = stopped[2].screen;
    assert!(check_window(&stopped, Duration::ZERO, end).is_err());
    assert!(check_window(&[points[0], points[5]], Duration::ZERO, end).is_err());
    assert!(check_window(&points, end, end + Duration::from_secs(1)).is_err());
    let mut slow = points.clone();
    for point in &mut slow {
        point.flood /= 2;
    }
    assert!(check_window(&slow, Duration::ZERO, end).is_err());
}

struct Monitor {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Vec<Point>>>,
    root: PathBuf,
}

impl Monitor {
    fn start(root: &Path, private: &Path, flood: String, screen: String, epoch: Instant) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let project = root.to_owned();
        let home = private.to_owned();
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let (route, _) = tokio::time::timeout(
                    DEADLINE,
                    relayterm_daemon::locate_workspace(&project, Some(home.clone())),
                )
                .await
                .unwrap()
                .unwrap();
                let client = relayterm_daemon::connect_route(&route, Some(home))
                    .await
                    .unwrap();
                let mut points = Vec::new();
                while !done.load(Ordering::Relaxed) && epoch.elapsed() < PRODUCER_LIMIT {
                    let begin = epoch.elapsed();
                    let mut offsets = [0_u64; 2];
                    for (index, id) in [&flood, &screen].iter().enumerate() {
                        let value: Value = tokio::time::timeout(
                            Duration::from_secs(2),
                            client.call(
                                Operation::SessionReadDisplay,
                                &serde_json::json!({"session_id":id,"rows":1,"columns":1}),
                            ),
                        )
                        .await
                        .expect("load snapshot deadline")
                        .unwrap();
                        offsets[index] = value["snapshot"]["raw_offset"]
                            .as_u64()
                            .expect("daemon snapshot has a raw byte counter");
                    }
                    points.push(Point {
                        begin,
                        end: epoch.elapsed(),
                        flood: offsets[0],
                        screen: offsets[1],
                    });
                    for _ in 0..5 {
                        if done.load(Ordering::Relaxed) {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
                for id in [&flood, &screen] {
                    let _: Value = tokio::time::timeout(
                        Duration::from_secs(2),
                        client.call(
                            Operation::SessionDetach,
                            &serde_json::json!({"session_id":id}),
                        ),
                    )
                    .await
                    .expect("load detach deadline")
                    .unwrap();
                }
                points
            })
        });
        Self {
            stop,
            worker: Some(worker),
            root: root.to_owned(),
        }
    }

    fn finish(&mut self) -> Vec<Point> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("load monitor failed")
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = fs::write(self.root.join("load.stop"), b"");
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn send_fixture_command(root: &Path, private: &Path, session: &str, command: &str) {
    let input = private.join("fixture-input.txt");
    fs::write(&input, format!("{command}\r")).unwrap();
    admin(
        root,
        private,
        &[
            "session",
            "input",
            session,
            "--file",
            input.to_str().unwrap(),
        ],
    );
}

#[test]
fn sustained_output_navigation_and_echo_meet_declared_contract() {
    let _native_serial = native_serial::NativeSerialGuard::acquire();
    let _serial = NATIVE_GATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let scratch = Scratch::new();
    let root = scratch.0.join("project");
    let private = scratch.0.join("private");
    fs::create_dir(&root).unwrap();
    let _cleanup = DaemonCleanup {
        root: root.clone(),
        private: private.clone(),
    };
    admin(
        &root,
        &private,
        &["workspace", "init", "--name", "Sustained fixture"],
    );
    let definition = register_fixture(&root, &private);
    for _ in 0..3 {
        admin(
            &root,
            &private,
            &["session", "create", "--definition-id", &definition],
        );
    }
    let sessions = admin(&root, &private, &["session", "list"]);
    let ids = sessions["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["session_id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    let mut terminal = OuterTerminal::spawn(&root, &private);
    terminal.finish_startup();
    terminal.send(b"3");
    select_session(&mut terminal, &ids[2]);
    terminal.send(b"\r");
    terminal.wait_for("fixture-ready");
    terminal.send(b"i");
    terminal.wait_for("WRITER");
    terminal.send(&[0x1d]);
    terminal.wait_for("READ ONLY");
    terminal.send(b"\x1b");
    terminal.wait_for("Sessions selected");

    send_fixture_command(&root, &private, &ids[0], "sustained-flood");
    send_fixture_command(&root, &private, &ids[1], "sustained-screen");
    wait_until(
        || root.join("flood.ready").exists() && root.join("screen.ready").exists(),
        "producer readiness",
    );
    let epoch = Instant::now();
    let mut monitor = Monitor::start(&root, &private, ids[0].clone(), ids[1].clone(), epoch);
    // Measure navigation in the first half and echo in the second half of the load.
    let navigation_start = epoch.elapsed();
    let mut navigation = Vec::with_capacity(100);
    for sample in 0..100 {
        assert!(
            epoch.elapsed() < PRODUCER_LIMIT - DEADLINE,
            "measurement exceeded producer safety window"
        );
        let due = navigation_start + Duration::from_millis(sample * 550);
        while epoch.elapsed() < due {
            std::thread::sleep(Duration::from_millis(5));
        }
        let previous = wait_for_selected_session_change(&terminal, None);
        let started = Instant::now();
        terminal.send(next_selection_input());
        wait_for_selected_session_change(&terminal, Some(&previous));
        navigation.push(started.elapsed());
    }
    let navigation_end = epoch.elapsed();
    select_session(&mut terminal, &ids[2]);
    terminal.send(b"\r");
    terminal.wait_for("Terminal");
    terminal.send(b"i");
    terminal.wait_for("WRITER");
    let input_start = epoch.elapsed();
    let mut input = Vec::with_capacity(100);
    for sample in 0..100 {
        assert!(
            epoch.elapsed() < PRODUCER_LIMIT - DEADLINE,
            "measurement exceeded producer safety window"
        );
        let due = input_start + Duration::from_millis(sample * 550);
        while epoch.elapsed() < due {
            std::thread::sleep(Duration::from_millis(5));
        }
        let started = Instant::now();
        terminal.send(b"x\r");
        terminal.wait_for(&format!("fixture-echo-{sample:03}:x"));
        input.push(started.elapsed());
    }
    let input_end = epoch.elapsed();
    while epoch.elapsed() < LOAD_DURATION + Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(100));
    }
    let end = epoch.elapsed();
    let points = monitor.finish();
    fs::write(root.join("load.stop"), b"").unwrap();
    wait_until(
        || root.join("flood.done").exists() && root.join("screen.done").exists(),
        "producer shutdown",
    );
    terminal.send(&[0x1d]);
    terminal.wait_for("READ ONLY");
    terminal.send(b"q");
    terminal.wait_exit();
    admin(&root, &private, &["daemon", "stop", "--terminate-sessions"]);

    // Report all observations before asserting, including invalid-load attempts.
    eprintln!("M12 load points begin_us end_us consumed_flood consumed_screen");
    for point in &points {
        eprintln!(
            "M12 load {} {} {} {}",
            point.begin.as_micros(),
            point.end.as_micros(),
            point.flood,
            point.screen
        );
    }
    for (label, samples) in [("navigation", &navigation), ("echo", &input)] {
        eprintln!(
            "M12 raw {label} latency_us={:?}",
            samples.iter().map(Duration::as_micros).collect::<Vec<_>>()
        );
    }
    let mut load_valid = points.last().unwrap().end - points[0].begin >= LOAD_DURATION;
    for (label, start, end) in [
        ("entire-load", Duration::ZERO, end),
        ("navigation", navigation_start, navigation_end),
        ("echo", input_start, input_end),
    ] {
        let result = check_window(&points, start, end);
        eprintln!(
            "M12 sustained {label} start_ms={} end_ms={} consumed_rate_result={result:?}",
            start.as_millis(),
            end.as_millis()
        );
        load_valid &= result.is_ok();
    }
    for role in ["flood", "screen"] {
        let report = fs::read_to_string(root.join(format!("{role}.done"))).unwrap();
        eprintln!(
            "M12 producer {role} elapsed_us bytes frames={}",
            report.trim()
        );
    }
    eprintln!(
        "M12 sustained harness_profile={} hosted={} load_samples={} product_override={}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        std::env::var_os("CI").is_some(),
        points.len(),
        std::env::var_os("RELAYTERM_TEST_RT").is_some()
    );
    let navigation_valid = latency_within_budget(
        "sustained-navigation",
        &mut navigation,
        Duration::from_millis(100),
        Duration::from_millis(500),
    );
    let input_valid = latency_within_budget(
        "sustained-input-to-rendered-echo",
        &mut input,
        Duration::from_millis(250),
        Duration::from_secs(1),
    );
    assert!(
        load_valid,
        "sustained load contract not met; latency is not acceptance evidence"
    );
    assert!(
        navigation_valid && input_valid,
        "sustained latency budget exceeded"
    );
}
