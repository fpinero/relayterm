use serde_json::Value;
use std::{
    fs,
    io::{BufRead, Read},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
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
            "r{:x}{:x}",
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
    invoke_binary(Path::new(env!("CARGO_BIN_EXE_rt")), root, home, args)
}

fn invoke_binary(binary: &Path, root: &Path, home: &Path, args: &[&str]) -> Output {
    bounded_output(
        Command::new(binary)
            .arg("--workspace")
            .arg(root)
            .arg("--home")
            .arg(home)
            .args(["--format", "json", "--timeout", "5"])
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped()),
    )
}

fn bounded_output(command: &mut Command) -> Output {
    const DEADLINE: Duration = Duration::from_secs(30);
    const OUTPUT_LIMIT: usize = 64 * 1024;

    let mut child = command.spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (output_tx, output_rx) = std::sync::mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = std::io::BufReader::new(stdout)
            .take((OUTPUT_LIMIT + 1) as u64)
            .read_until(b'\n', &mut bytes)
            .map(|_| bytes);
        let _ = output_tx.send(result);
    });
    let deadline = Instant::now() + DEADLINE;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("M11 CLI command did not terminate before its deadline");
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = output_rx
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("M11 CLI output did not close before its deadline")
        .expect("M11 CLI output could not be read");
    assert!(
        stdout.len() <= OUTPUT_LIMIT,
        "M11 CLI output exceeded its bound"
    );
    Output {
        status,
        stdout,
        stderr: Vec::new(),
    }
}

fn success(root: &Path, home: &Path, args: &[&str]) -> Value {
    let output = invoke(root, home, args);
    successful_value(output, args)
}

fn success_binary(binary: &Path, root: &Path, home: &Path, args: &[&str]) -> Value {
    let output = invoke_binary(binary, root, home, args);
    successful_value(output, args)
}

fn successful_value(output: Output, args: &[&str]) -> Value {
    assert!(
        output.status.success(),
        "command {args:?} failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["result"].clone()
}

#[test]
fn private_backup_is_exclusive_integrity_checked_and_reopenable() {
    let scratch = Scratch::new();
    let root = scratch.0.join("p λ");
    let home = scratch.0.join("h");
    let recovery = home.join("data");
    let restored = recovery.join("r");
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
    let busy = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .arg("--home")
            .arg(&home)
            .args(["--format", "json", "--timeout", "1", "backup", "create"])
            .arg("--destination")
            .arg(&busy_target)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
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
    let original_database = fs::read(backup.join("workspace.sqlite3")).unwrap();
    let original_manifest = fs::read(backup.join("manifest.json")).unwrap();
    assert!(
        fs::read_dir(backup.parent().unwrap())
            .unwrap()
            .all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains(".relayterm-backup-")
            }),
        "a completed backup must not retain private staging directories"
    );

    let redirected_destination = home.join("data").join("redirected-destination");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        home.join("data").join("missing-redirection-target"),
        &redirected_destination,
    )
    .unwrap();
    #[cfg(windows)]
    {
        let status = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&redirected_destination)
            .arg(&backup)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
    }
    let redirected_destination_result = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .arg("--home")
            .arg(&home)
            .args(["--format", "json", "backup", "create", "--destination"])
            .arg(&redirected_destination)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!redirected_destination_result.status.success());
    assert_eq!(
        fs::read(backup.join("workspace.sqlite3")).unwrap(),
        original_database
    );
    assert_eq!(
        fs::read(backup.join("manifest.json")).unwrap(),
        original_manifest
    );

    let existing_home = scratch.0.join("existing-private");
    fs::create_dir(&existing_home).unwrap();
    let sentinel = existing_home.join("preserved.txt");
    fs::write(&sentinel, b"preserve existing destination").unwrap();
    let existing_result = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&backup)
            .arg("--destination")
            .arg(&existing_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!existing_result.status.success());
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"preserve existing destination"
    );
    assert_eq!(
        fs::read(backup.join("workspace.sqlite3")).unwrap(),
        original_database
    );
    assert_eq!(
        fs::read(backup.join("manifest.json")).unwrap(),
        original_manifest
    );

    let restored_text = restored.to_str().unwrap();
    let installed_directory = scratch.0.join("installed");
    fs::create_dir(&installed_directory).unwrap();
    let installed_binary = installed_directory.join(if cfg!(windows) { "rt.exe" } else { "rt" });
    fs::copy(env!("CARGO_BIN_EXE_rt"), &installed_binary).unwrap();
    let restored_result = success_binary(
        &installed_binary,
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
    let reopened = success_binary(&installed_binary, &root, &restored, &["workspace", "open"]);
    assert_eq!(reopened["workspace_id"], workspace_id);
    success_binary(
        &installed_binary,
        &root,
        &restored,
        &["daemon", "stop", "--terminate-sessions"],
    );

    let unexpected = home.join("data").join("unexpected-member-backup");
    success(
        &root,
        &home,
        &[
            "backup",
            "create",
            "--destination",
            unexpected.to_str().unwrap(),
        ],
    );
    fs::write(unexpected.join("unexpected.txt"), b"synthetic member").unwrap();
    let unexpected_home = recovery.join("u");
    let unexpected_result = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&unexpected)
            .arg("--destination")
            .arg(&unexpected_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!unexpected_result.status.success());
    assert!(!unexpected_home.exists());

    let newer = home.join("data").join("newer-manifest-backup");
    success(
        &root,
        &home,
        &["backup", "create", "--destination", newer.to_str().unwrap()],
    );
    let mut newer_manifest: Value =
        serde_json::from_slice(&fs::read(newer.join("manifest.json")).unwrap()).unwrap();
    newer_manifest["format_version"] = Value::from(2);
    fs::write(
        newer.join("manifest.json"),
        serde_json::to_vec(&newer_manifest).unwrap(),
    )
    .unwrap();
    let newer_home = recovery.join("n");
    let newer_result = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&newer)
            .arg("--destination")
            .arg(&newer_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!newer_result.status.success());
    assert!(!newer_home.exists());

    let newer_schema = home.join("data").join("newer-schema-backup");
    success(
        &root,
        &home,
        &[
            "backup",
            "create",
            "--destination",
            newer_schema.to_str().unwrap(),
        ],
    );
    let mut newer_schema_manifest: Value =
        serde_json::from_slice(&fs::read(newer_schema.join("manifest.json")).unwrap()).unwrap();
    let schema = newer_schema_manifest["workspace_schema_version"]
        .as_i64()
        .unwrap();
    newer_schema_manifest["workspace_schema_version"] = Value::from(schema + 1);
    fs::write(
        newer_schema.join("manifest.json"),
        serde_json::to_vec(&newer_schema_manifest).unwrap(),
    )
    .unwrap();
    let newer_schema_home = recovery.join("v");
    let newer_schema_result = bounded_output(
        Command::new(&installed_binary)
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&newer_schema)
            .arg("--destination")
            .arg(&newer_schema_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!newer_schema_result.status.success());
    assert!(!newer_schema_home.exists());

    let incomplete = home.join("data").join("incomplete-backup");
    success(
        &root,
        &home,
        &[
            "backup",
            "create",
            "--destination",
            incomplete.to_str().unwrap(),
        ],
    );
    let mut incomplete_manifest: Value =
        serde_json::from_slice(&fs::read(incomplete.join("manifest.json")).unwrap()).unwrap();
    incomplete_manifest["complete"] = Value::Bool(false);
    fs::write(
        incomplete.join("manifest.json"),
        serde_json::to_vec(&incomplete_manifest).unwrap(),
    )
    .unwrap();
    let incomplete_home = recovery.join("i");
    let incomplete_result = bounded_output(
        Command::new(&installed_binary)
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&incomplete)
            .arg("--destination")
            .arg(&incomplete_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!incomplete_result.status.success());
    assert!(!incomplete_home.exists());

    let corrupt = home.join("data").join("corrupt-backup");
    success(
        &root,
        &home,
        &[
            "backup",
            "create",
            "--destination",
            corrupt.to_str().unwrap(),
        ],
    );
    let rejected_home = recovery.join("c");
    let mut database = fs::OpenOptions::new()
        .append(true)
        .open(corrupt.join("workspace.sqlite3"))
        .unwrap();
    use std::io::Write as _;
    database.write_all(b"synthetic-corruption").unwrap();
    database.sync_all().unwrap();
    let rejected = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&corrupt)
            .arg("--destination")
            .arg(&rejected_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!rejected.status.success());
    assert!(!rejected_home.exists());

    let corrupt_structure = home.join("data").join("corrupt-structure-backup");
    success(
        &root,
        &home,
        &[
            "backup",
            "create",
            "--destination",
            corrupt_structure.to_str().unwrap(),
        ],
    );
    let corrupt_database = corrupt_structure.join("workspace.sqlite3");
    let mut corrupt_bytes = fs::read(&corrupt_database).unwrap();
    corrupt_bytes[..16].copy_from_slice(b"invalid sqlite!!");
    fs::write(&corrupt_database, &corrupt_bytes).unwrap();
    let mut corrupt_manifest: Value =
        serde_json::from_slice(&fs::read(corrupt_structure.join("manifest.json")).unwrap())
            .unwrap();
    corrupt_manifest["blake3"] = Value::String(blake3::hash(&corrupt_bytes).to_hex().to_string());
    fs::write(
        corrupt_structure.join("manifest.json"),
        serde_json::to_vec(&corrupt_manifest).unwrap(),
    )
    .unwrap();
    let corrupt_structure_home = recovery.join("s");
    let corrupt_structure_result = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_rt"))
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&corrupt_structure)
            .arg("--destination")
            .arg(&corrupt_structure_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!corrupt_structure_result.status.success());
    assert!(!corrupt_structure_home.exists());
    assert_eq!(fs::read(&corrupt_database).unwrap(), corrupt_bytes);
    assert_eq!(
        fs::read(corrupt_structure.join("manifest.json")).unwrap(),
        serde_json::to_vec(&corrupt_manifest).unwrap()
    );
    assert_eq!(
        fs::read(backup.join("workspace.sqlite3")).unwrap(),
        original_database
    );
    assert_eq!(
        fs::read(backup.join("manifest.json")).unwrap(),
        original_manifest
    );

    let redirected_source = home.join("data").join("redirected-backup");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&backup, &redirected_source).unwrap();
    #[cfg(windows)]
    {
        let status = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&redirected_source)
            .arg(&backup)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
    }
    let redirected_home = recovery.join("l");
    let redirected_result = bounded_output(
        Command::new(&installed_binary)
            .arg("--workspace")
            .arg(&root)
            .args(["--format", "json", "backup", "restore", "--source"])
            .arg(&redirected_source)
            .arg("--destination")
            .arg(&redirected_home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    );
    assert!(!redirected_result.status.success());
    assert!(!redirected_home.exists());
}
