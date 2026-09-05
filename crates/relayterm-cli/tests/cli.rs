use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("relayterm-cli-{}-{nonce}", std::process::id()));
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
fn command_contracts_have_no_runtime_side_effects() {
    let cases: &[(&[&str], i32, &str)] = &[
        (&["--help"], 0, "Usage: rt"),
        (&["--version"], 0, concat!("rt ", env!("CARGO_PKG_VERSION"))),
        (&["daemon", "--help"], 0, "Usage: rt daemon"),
        (&[], 1, "The TUI is not implemented yet"),
        (&["daemon"], 1, "The daemon is not implemented yet"),
        (&["--unknown-option"], 2, "unexpected argument"),
    ];
    for (args, code, expected) in cases {
        let scratch = Scratch::new();
        let private = scratch.0.join("private");
        fs::create_dir(&private).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_rt"))
            .args(*args)
            .current_dir(&scratch.0)
            .env("RELAYTERM_TEST_MARKER", "test-token-not-secret")
            .env("XDG_DATA_HOME", &private)
            .env("XDG_CONFIG_HOME", &private)
            .env("XDG_RUNTIME_DIR", &private)
            .env("LOCALAPPDATA", &private)
            .env("APPDATA", &private)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("CLI did not terminate promptly for {args:?}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(*code), "{args:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let combined = format!("{stdout}{stderr}");
        assert!(combined.contains(expected), "{args:?}: {combined}");
        if *code == 0 {
            assert!(stderr.is_empty());
        } else {
            assert!(stdout.is_empty());
        }
        assert!(!combined.contains("test-token-not-secret"));
        assert!(!combined.contains(scratch.0.to_str().unwrap()));
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
        assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
    }
}
