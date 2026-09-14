use serde_json::Value;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

fn rt_binary() -> OsString {
    std::env::var_os("RELAYTERM_TEST_RT")
        .unwrap_or_else(|| OsString::from(env!("CARGO_BIN_EXE_rt")))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let base = if cfg!(target_os = "macos") {
            PathBuf::from("/private/tmp")
        } else {
            std::env::temp_dir()
        };
        let path = tempfile::Builder::new()
            .prefix("rt12-session-")
            .tempdir_in(base)
            .unwrap()
            .keep();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn invoke(root: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(rt_binary())
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(home)
        .args(["--format", "json", "--timeout", "10"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn success(root: &Path, home: &Path, args: &[&str]) -> Value {
    let output = invoke(root, home, args);
    assert!(
        output.status.success(),
        "command {args:?} failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["result"].clone()
}

#[test]
fn installed_cli_lists_and_renames_sessions_by_stable_identity() {
    let scratch = Scratch::new();
    let root = scratch.0.join("project λ");
    let home = scratch.0.join("private home");
    fs::create_dir(&root).unwrap();
    success(
        &root,
        &home,
        &["workspace", "init", "--name", "M12 fixture"],
    );
    let created = success(&root, &home, &["session", "create"]);
    let session_id = created["session_id"].as_str().unwrap();
    let initial = success(&root, &home, &["session", "list-ordered"]);
    assert_eq!(initial["items"].as_array().unwrap().len(), 1);
    assert_eq!(initial["items"][0]["creation_ordinal"], "1");
    assert_eq!(initial["items"][0]["display_name"], Value::Null);
    assert_eq!(initial["items"][0]["instance"]["session_id"], session_id);
    let initial_revision = initial["revision"].as_str().unwrap();

    let renamed = success(
        &root,
        &home,
        &[
            "session",
            "rename",
            session_id,
            "--name",
            "  Build e\u{301}  ",
            "--expected-revision",
            initial_revision,
        ],
    );
    assert_eq!(renamed["changed"], true);
    let current_revision = renamed["revision"].as_str().unwrap();
    let listed = success(
        &root,
        &home,
        &[
            "session",
            "list-ordered",
            "--expected-revision",
            current_revision,
        ],
    );
    assert_eq!(listed["items"][0]["display_name"], "Build e\u{301}");

    let stale = invoke(
        &root,
        &home,
        &[
            "session",
            "rename",
            session_id,
            "--name",
            "Stale draft",
            "--expected-revision",
            initial_revision,
        ],
    );
    assert!(!stale.status.success());
    let stale: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale["error"]["code"], "request_rejected");
    let final_list = success(&root, &home, &["session", "list-ordered"]);
    assert_eq!(final_list["items"][0]["display_name"], "Build e\u{301}");

    success(&root, &home, &["daemon", "stop", "--terminate-sessions"]);
}
