use relayterm_pty::{
    NativeControl, NativeSession, SpawnRequest, approved_environment, terminal_environment,
    write_input,
};
use serde_json::Value;
use std::{
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub const DEADLINE: Duration = Duration::from_secs(45);
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;

pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new() -> Self {
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
            "rt9-{}-{}",
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

pub struct OuterTerminal {
    control: NativeControl,
    writer: Box<dyn std::io::Write + Send>,
    #[cfg(unix)]
    output: Arc<Mutex<Vec<u8>>>,
    screen: Arc<Mutex<vt100::Parser>>,
}

impl OuterTerminal {
    pub fn spawn(root: &Path, private: &Path) -> Self {
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
            #[cfg(unix)]
            output,
            screen,
        }
    }

    pub fn send(&mut self, bytes: &[u8]) {
        write_input(&mut *self.writer, bytes).unwrap();
    }

    pub fn finish_startup(&mut self) {
        #[cfg(unix)]
        {
            self.wait_for_raw("\x1b[6n");
            self.send(b"\x1b[30;100R");
        }
        self.wait_for("Workspace overview");
    }

    pub fn wait_for(&self, marker: &str) {
        let deadline = Instant::now() + DEADLINE;
        while !self.screen_contents().contains(marker) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {marker}; screen={:?}",
                self.screen_contents()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    fn wait_for_raw(&self, marker: &str) {
        wait_until(
            || String::from_utf8_lossy(&self.output.lock().unwrap()).contains(marker),
            marker,
        );
    }

    pub fn screen_contents(&self) -> String {
        self.screen.lock().unwrap().screen().contents()
    }

    pub fn wait_exit(&mut self) {
        wait_until(
            || self.control.try_wait().unwrap().is_some(),
            "interactive client exit",
        );
    }
}

impl Drop for OuterTerminal {
    fn drop(&mut self) {
        let _ = self.control.terminate();
    }
}

pub struct DaemonCleanup {
    root: PathBuf,
    private: PathBuf,
}

impl DaemonCleanup {
    pub fn new(root: &Path, private: &Path) -> Self {
        Self {
            root: root.to_owned(),
            private: private.to_owned(),
        }
    }
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

pub fn admin(root: &Path, private: &Path, args: &[&str]) -> Value {
    let output = admin_output(root, private, args);
    assert!(
        output.status.success(),
        "administrative command {args:?} failed: {}",
        output.status
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

pub fn admin_output(root: &Path, private: &Path, args: &[&str]) -> std::process::Output {
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

pub fn wait_until(mut condition: impl FnMut() -> bool, label: &str) {
    let deadline = Instant::now() + DEADLINE;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn next_selection_input() -> &'static [u8] {
    if cfg!(windows) { b"\x1b[B" } else { b"j" }
}

pub fn previous_selection_input() -> &'static [u8] {
    if cfg!(windows) { b"\x1b[A" } else { b"k" }
}
