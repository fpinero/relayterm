use relayterm_protocol::{
    COLLECTION_PAGE_BYTES, CONNECTION_BUFFER_LIMIT, JSON_FRAME_LIMIT, MAX_PAGE_SIZE,
    SNAPSHOT_STAGING_LIMIT, TERMINAL_DATA_LIMIT,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

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
            "rt11-{}-{}",
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

struct DaemonCleanup {
    root: PathBuf,
    home: PathBuf,
}

impl Drop for DaemonCleanup {
    fn drop(&mut self) {
        let _ = invoke(
            &self.root,
            &self.home,
            &["daemon", "stop", "--terminate-sessions"],
        );
    }
}

fn invoke(root: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(root)
        .arg("--home")
        .arg(home)
        .args(["--format", "json", "--timeout", "5"])
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
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
fn public_resource_limits_are_finite_and_internally_ordered() {
    assert_eq!(MAX_PAGE_SIZE, 200);
    assert_eq!(TERMINAL_DATA_LIMIT, 16 * 1024);
    assert_eq!(JSON_FRAME_LIMIT, 8 * 1024 * 1024);
    assert_eq!(CONNECTION_BUFFER_LIMIT, 16 * 1024 * 1024);
    assert_eq!(COLLECTION_PAGE_BYTES, 6 * 1024 * 1024);
    assert_eq!(SNAPSHOT_STAGING_LIMIT, 64 * 1024 * 1024);

    let help = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("backup")
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("create"));
    assert!(help.contains("restore"));
}

#[test]
fn private_backup_is_exclusive_integrity_checked_and_reopenable() {
    let scratch = Scratch::new();
    let root = scratch.0.join("synthetic project λ");
    let home = scratch.0.join("private");
    let restored = scratch.0.join("restored-private");
    fs::create_dir(&root).unwrap();
    let _cleanup = DaemonCleanup {
        root: root.clone(),
        home: home.clone(),
    };

    let initialized = success(
        &root,
        &home,
        &["workspace", "init", "--name", "M11 fixture"],
    );
    let workspace_id = initialized["workspace_id"].as_str().unwrap().to_owned();
    let busy_target = home.join("data").join("busy-backup");
    let busy = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .arg("--home")
        .arg(&home)
        .args(["--format", "json", "--timeout", "1", "backup", "create"])
        .arg("--destination")
        .arg(&busy_target)
        .output()
        .unwrap();
    assert!(!busy.status.success());
    assert!(!busy_target.exists());

    success(&root, &home, &["daemon", "stop", "--terminate-sessions"]);
    let backup = home.join("data").join("backup");
    let backup_text = backup.to_str().unwrap();
    let created = success(
        &root,
        &home,
        &["backup", "create", "--destination", backup_text],
    );
    assert_eq!(created["workspace_id"], workspace_id);
    assert_eq!(fs::read_dir(&backup).unwrap().count(), 2);

    let restored_text = restored.to_str().unwrap();
    let restored_result = success(
        &root,
        &home,
        &[
            "backup",
            "restore",
            "--source",
            backup_text,
            "--destination",
            restored_text,
        ],
    );
    assert_eq!(restored_result["workspace_id"], workspace_id);
    let reopened = success(&root, &restored, &["workspace", "open"]);
    assert_eq!(reopened["workspace_id"], workspace_id);
    success(
        &root,
        &restored,
        &["daemon", "stop", "--terminate-sessions"],
    );

    let rejected_home = scratch.0.join("rejected-private");
    let mut database = fs::OpenOptions::new()
        .append(true)
        .open(backup.join("workspace.sqlite3"))
        .unwrap();
    use std::io::Write as _;
    database.write_all(b"synthetic-corruption").unwrap();
    database.sync_all().unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_rt"))
        .arg("--workspace")
        .arg(&root)
        .args(["--format", "json", "backup", "restore", "--source"])
        .arg(&backup)
        .arg("--destination")
        .arg(&rejected_home)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(!rejected_home.exists());
}
