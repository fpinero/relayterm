//! Bounded, argument-array Git operations for explicit worktree management.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_STDOUT: usize = 4 * 1024 * 1024;
pub const MAX_STDERR: usize = 64 * 1024;
pub const MAX_INVENTORY: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    GitMissing,
    NotRepository,
    UnsupportedRoot,
    InvalidReference,
    BranchConflict,
    DestinationConflict,
    UnsupportedCheckoutFilter,
    OutputLimit,
    Timeout,
    CommandFailed,
    MalformedOutput,
    Io,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    const fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self.kind)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repository {
    pub checkout_top: PathBuf,
    pub common_directory: PathBuf,
    pub head_commit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: bool,
    pub prunable: bool,
}

#[derive(Clone, Debug)]
pub struct Git {
    executable: PathBuf,
    read_timeout: Duration,
    create_timeout: Duration,
}

impl Default for Git {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("git"),
            read_timeout: Duration::from_secs(5),
            create_timeout: Duration::from_secs(60),
        }
    }
}

impl Git {
    pub fn with_executable(executable: PathBuf) -> Self {
        Self {
            executable,
            ..Self::default()
        }
    }

    pub fn version(&self) -> Result<String> {
        let output = self.run(None, [OsStr::new("--version")], self.read_timeout)?;
        String::from_utf8(output).map_err(|_| Error::new(ErrorKind::MalformedOutput))
    }

    pub fn inspect(&self, root: &Path) -> Result<Repository> {
        let top = self.text(root, ["rev-parse", "--show-toplevel"])?;
        let common = self.text(
            root,
            ["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        let head = self.text(root, ["rev-parse", "--verify", "HEAD^{commit}"])?;
        let checkout_top = canonical_existing(Path::new(top.trim()))?;
        let requested = canonical_existing(root)?;
        if checkout_top != requested {
            return Err(Error::new(ErrorKind::UnsupportedRoot));
        }
        Ok(Repository {
            checkout_top,
            common_directory: canonical_existing(Path::new(common.trim()))?,
            head_commit: validate_object_id(head.trim())?.to_owned(),
        })
    }

    pub fn resolve_commit(&self, root: &Path, base: &str) -> Result<String> {
        validate_base(base)?;
        let expression = format!("{base}^{{commit}}");
        let value = self.text(
            root,
            [
                "rev-parse",
                "--verify",
                "--end-of-options",
                expression.as_str(),
            ],
        )?;
        Ok(validate_object_id(value.trim())?.to_owned())
    }

    pub fn branch_available(&self, root: &Path, branch: &str) -> Result<()> {
        validate_branch(branch)?;
        let full = format!("refs/heads/{branch}");
        if self.run_status(root, ["check-ref-format", full.as_str()])? != 0 {
            return Err(Error::new(ErrorKind::InvalidReference));
        }
        match self.run_status(root, ["show-ref", "--verify", "--quiet", full.as_str()]) {
            Ok(0) => Err(Error::new(ErrorKind::BranchConflict)),
            Ok(1) => Ok(()),
            Ok(_) => Err(Error::new(ErrorKind::CommandFailed)),
            Err(error) => Err(error),
        }
    }

    pub fn list(&self, root: &Path) -> Result<Vec<WorktreeEntry>> {
        let output = self.run(
            Some(root),
            [
                OsStr::new("worktree"),
                OsStr::new("list"),
                OsStr::new("--porcelain"),
                OsStr::new("-z"),
            ],
            self.read_timeout,
        )?;
        parse_porcelain_z(&output)
    }

    pub fn add(&self, root: &Path, destination: &Path, branch: &str, commit: &str) -> Result<()> {
        validate_branch(branch)?;
        validate_object_id(commit)?;
        let parent = destination
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::DestinationConflict))?;
        if canonical_existing(parent)? != parent {
            return Err(Error::new(ErrorKind::UnsupportedRoot));
        }
        match std::fs::symlink_metadata(destination) {
            Ok(_) => return Err(Error::new(ErrorKind::DestinationConflict)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::new(ErrorKind::Io)),
        }
        self.branch_available(root, branch)?;
        self.reject_checkout_filters(root)?;
        let repository = self.inspect(root)?;
        let lock_path = repository.common_directory.join("relayterm-worktree.lock");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| Error::new(ErrorKind::Io))?;
        lock_bounded(&lock, self.read_timeout)?;
        let args = [
            OsString::from("-c"),
            OsString::from("core.hooksPath="),
            OsString::from("worktree"),
            OsString::from("add"),
            OsString::from("-b"),
            OsString::from(branch),
            OsString::from("--no-track"),
            OsString::from("--"),
            git_argument_path(destination),
            OsString::from(commit),
        ];
        self.run_quiet(
            root,
            args.iter().map(OsString::as_os_str),
            self.create_timeout,
        )?;
        Ok(())
    }

    pub fn verify_worktree(
        &self,
        source: &Path,
        destination: &Path,
        branch: &str,
        common_directory: &Path,
        initial_commit: &str,
    ) -> Result<()> {
        let destination = canonical_existing(destination)?;
        let repository = self.inspect(&destination)?;
        if repository.common_directory != common_directory
            || repository.head_commit != initial_commit
        {
            return Err(Error::new(ErrorKind::CommandFailed));
        }
        let expected = format!("refs/heads/{branch}");
        if !self.list(source)?.iter().any(|entry| {
            entry.path.canonicalize().ok().as_ref() == Some(&destination)
                && entry.branch.as_deref() == Some(expected.as_str())
        }) {
            return Err(Error::new(ErrorKind::CommandFailed));
        }
        Ok(())
    }

    fn text<const N: usize>(&self, root: &Path, args: [&str; N]) -> Result<String> {
        let output = self.run(Some(root), args.map(OsStr::new), self.read_timeout)?;
        String::from_utf8(output).map_err(|_| Error::new(ErrorKind::MalformedOutput))
    }

    fn reject_checkout_filters(&self, root: &Path) -> Result<()> {
        reject_checkout_filters(root)?;
        match self.run_status(root, ["config", "--local", "--get-regexp", "^filter\\."]) {
            Ok(0) => Err(Error::new(ErrorKind::UnsupportedCheckoutFilter)),
            Ok(1) => Ok(()),
            Ok(_) => Err(Error::new(ErrorKind::CommandFailed)),
            Err(error) => Err(error),
        }
    }

    fn run_status<const N: usize>(&self, root: &Path, args: [&str; N]) -> Result<i32> {
        let mut command = self.command(Some(root));
        command
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(map_spawn)?;
        wait_bounded(&mut child, self.read_timeout)?
            .code()
            .ok_or_else(|| Error::new(ErrorKind::CommandFailed))
    }

    fn run_quiet<I, S>(&self, root: &Path, args: I, timeout: Duration) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.command(Some(root));
        command
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(map_spawn)?;
        if wait_bounded(&mut child, timeout)?.success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::CommandFailed))
        }
    }

    fn command(&self, root: Option<&Path>) -> Command {
        let mut command = Command::new(&self.executable);
        if let Some(root) = root {
            command.current_dir(root);
        }
        command
            .stdin(Stdio::null())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "Never")
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_OBJECT_DIRECTORY")
            .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES");
        command
    }

    fn run<I, S>(&self, root: Option<&Path>, args: I, timeout: Duration) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.command(root);
        let mut stdout_file = tempfile::tempfile().map_err(|_| Error::new(ErrorKind::Io))?;
        let mut stderr_file = tempfile::tempfile().map_err(|_| Error::new(ErrorKind::Io))?;
        command
            .args(args)
            .stdout(Stdio::from(
                stdout_file
                    .try_clone()
                    .map_err(|_| Error::new(ErrorKind::Io))?,
            ))
            .stderr(Stdio::from(
                stderr_file
                    .try_clone()
                    .map_err(|_| Error::new(ErrorKind::Io))?,
            ));
        let mut child = command.spawn().map_err(map_spawn)?;
        let status = wait_bounded(&mut child, timeout)?;
        let stdout = read_bounded(&mut stdout_file, MAX_STDOUT)?;
        let stderr = read_bounded(&mut stderr_file, MAX_STDERR)?;
        if !status.success() {
            let kind = classify_failure(&stderr);
            return Err(Error::new(kind));
        }
        Ok(stdout)
    }
}

fn lock_bounded(file: &std::fs::File, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) if started.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(Error::new(ErrorKind::Timeout));
            }
            Err(std::fs::TryLockError::Error(_)) => return Err(Error::new(ErrorKind::Io)),
        }
    }
}

fn wait_bounded(child: &mut Child, timeout: Duration) -> Result<std::process::ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|_| Error::new(ErrorKind::Io))? {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(ErrorKind::Timeout));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn read_bounded(file: &mut std::fs::File, limit: usize) -> Result<Vec<u8>> {
    file.rewind().map_err(|_| Error::new(ErrorKind::Io))?;
    let mut value = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut value)
        .map_err(|_| Error::new(ErrorKind::Io))?;
    if value.len() > limit {
        Err(Error::new(ErrorKind::OutputLimit))
    } else {
        Ok(value)
    }
}

fn map_spawn(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::NotFound {
        Error::new(ErrorKind::GitMissing)
    } else {
        Error::new(ErrorKind::Io)
    }
}

fn classify_failure(stderr: &[u8]) -> ErrorKind {
    let value = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if value.contains("not a git repository") {
        ErrorKind::NotRepository
    } else if value.contains("not a valid object") || value.contains("needed a single revision") {
        ErrorKind::InvalidReference
    } else if value.contains("already exists") || value.contains("already checked out") {
        ErrorKind::BranchConflict
    } else {
        ErrorKind::CommandFailed
    }
}

fn canonical_existing(path: &Path) -> Result<PathBuf> {
    path.canonicalize().map_err(|_| Error::new(ErrorKind::Io))
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

#[cfg(not(windows))]
fn git_argument_path(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[cfg(windows)]
fn git_argument_path(path: &Path) -> OsString {
    if let Ok(relative) = path.strip_prefix(Path::new(r"\\?\UNC")) {
        return Path::new(r"\\").join(relative).into_os_string();
    }
    if let Ok(relative) = path.strip_prefix(Path::new(r"\\?\")) {
        return relative.as_os_str().to_owned();
    }
    path.as_os_str().to_owned()
}

pub fn validate_branch(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.ends_with('.')
        || value.contains("..")
        || value.contains("@{")
        || value.contains("//")
        || value.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
        || value
            .chars()
            .any(|character| matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
    {
        return Err(Error::new(ErrorKind::InvalidReference));
    }
    Ok(())
}

pub fn validate_base(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || value.starts_with('-')
        || value
            .bytes()
            .any(|byte| byte == 0 || byte == b'\n' || byte == b'\r')
    {
        return Err(Error::new(ErrorKind::InvalidReference));
    }
    Ok(())
}

pub fn validate_destination_leaf(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || value == "."
        || value == ".."
        || value.starts_with('-')
        || value.contains(['/', '\\', ':'])
        || value.chars().any(char::is_control)
    {
        return Err(Error::new(ErrorKind::DestinationConflict));
    }
    #[cfg(windows)]
    {
        let stem = value
            .split('.')
            .next()
            .unwrap_or(value)
            .to_ascii_uppercase();
        if matches!(
            stem.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return Err(Error::new(ErrorKind::DestinationConflict));
        }
    }
    Ok(())
}

fn validate_object_id(value: &str) -> Result<&str> {
    if !(4..=128).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::new(ErrorKind::MalformedOutput));
    }
    Ok(value)
}

fn reject_checkout_filters(root: &Path) -> Result<()> {
    let attributes = root.join(".gitattributes");
    let Ok(bytes) = std::fs::read(attributes) else {
        return Ok(());
    };
    if String::from_utf8_lossy(&bytes)
        .split_ascii_whitespace()
        .any(|item| item.starts_with("filter=") || item == "filter")
    {
        return Err(Error::new(ErrorKind::UnsupportedCheckoutFilter));
    }
    Ok(())
}

pub fn parse_porcelain_z(input: &[u8]) -> Result<Vec<WorktreeEntry>> {
    let mut result: Vec<WorktreeEntry> = Vec::new();
    for field in input
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
    {
        let (key, value) = field
            .iter()
            .position(|byte| *byte == b' ')
            .map_or((field, &[][..]), |position| {
                (&field[..position], &field[position + 1..])
            });
        match key {
            b"worktree" => {
                if result.len() == MAX_INVENTORY {
                    return Err(Error::new(ErrorKind::OutputLimit));
                }
                if value.is_empty() {
                    return Err(Error::new(ErrorKind::MalformedOutput));
                }
                result.push(WorktreeEntry {
                    path: bytes_to_path(value)?,
                    head: None,
                    branch: None,
                    bare: false,
                    detached: false,
                    locked: false,
                    prunable: false,
                });
            }
            b"HEAD" => current(&mut result)?.head = Some(text_value(value)?),
            b"branch" => current(&mut result)?.branch = Some(text_value(value)?),
            b"bare" => current(&mut result)?.bare = true,
            b"detached" => current(&mut result)?.detached = true,
            b"locked" => current(&mut result)?.locked = true,
            b"prunable" => current(&mut result)?.prunable = true,
            _ => {
                current(&mut result)?;
            }
        }
    }
    Ok(result)
}

fn current(values: &mut [WorktreeEntry]) -> Result<&mut WorktreeEntry> {
    values
        .last_mut()
        .ok_or_else(|| Error::new(ErrorKind::MalformedOutput))
}

fn text_value(value: &[u8]) -> Result<String> {
    String::from_utf8(value.to_vec()).map_err(|_| Error::new(ErrorKind::MalformedOutput))
}

#[cfg(unix)]
fn bytes_to_path(value: &[u8]) -> Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    Ok(PathBuf::from(OsString::from_vec(value.to_vec())))
}

#[cfg(windows)]
fn bytes_to_path(value: &[u8]) -> Result<PathBuf> {
    Ok(PathBuf::from(text_value(value)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_validators_reject_injection_and_escape_forms() {
        for branch in ["", "-force", "a..b", "a@{b", "a b", "a\\b", "a:b", "a/"] {
            assert!(validate_branch(branch).is_err(), "{branch:?}");
        }
        assert!(validate_branch("rt/task-00000000-0000-4000-8000-000000000001").is_ok());
        for leaf in ["", ".", "..", "-x", "a/b", "a\\b", "C:x", "a\0b"] {
            assert!(validate_destination_leaf(leaf).is_err(), "{leaf:?}");
        }
        assert!(validate_destination_leaf("task one-é").is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn verbatim_native_paths_are_adapted_only_at_the_git_boundary() {
        assert_eq!(
            git_argument_path(Path::new(r"\\?\C:\private\worktrees\task")),
            OsString::from(r"C:\private\worktrees\task")
        );
        assert_eq!(
            git_argument_path(Path::new(r"\\?\UNC\server\share\task")),
            OsString::from(r"\\server\share\task")
        );
    }

    #[test]
    fn porcelain_parser_keeps_unknown_fields_and_native_records() {
        let data = b"worktree /tmp/main\0HEAD 01234567\0branch refs/heads/main\0unknown future\0worktree /tmp/linked tree\0HEAD abcdef12\0detached\0locked reason\0";
        let rows = parse_porcelain_z(data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].branch.as_deref(), Some("refs/heads/main"));
        assert!(rows[1].detached);
        assert!(rows[1].locked);
        let mut excessive = Vec::new();
        for index in 0..=MAX_INVENTORY {
            excessive.extend_from_slice(format!("worktree /tmp/{index}\0").as_bytes());
        }
        assert_eq!(
            parse_porcelain_z(&excessive).unwrap_err().kind(),
            ErrorKind::OutputLimit
        );
    }

    #[test]
    fn discovery_reports_missing_non_repository_and_unsupported_subdirectory() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            Git::with_executable(directory.path().join("missing-git"))
                .version()
                .unwrap_err()
                .kind(),
            ErrorKind::GitMissing
        );
        assert_eq!(
            Git::default().inspect(directory.path()).unwrap_err().kind(),
            ErrorKind::NotRepository
        );
        let source = directory.path().join("source");
        let child = source.join("child");
        std::fs::create_dir_all(&child).unwrap();
        let status = Command::new("git")
            .current_dir(&source)
            .args(["init", "-q"])
            .status()
            .unwrap();
        assert!(status.success());
        for arguments in [
            ["config", "user.name", "Fixture"],
            ["config", "user.email", "fixture@example.invalid"],
        ] {
            assert!(
                Command::new("git")
                    .current_dir(&source)
                    .args(arguments)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::fs::write(source.join("fixture.txt"), "fixture\n").unwrap();
        assert!(
            Command::new("git")
                .current_dir(&source)
                .args(["add", "fixture.txt"])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .current_dir(&source)
                .args(["commit", "-qm", "fixture"])
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(
            Git::default().inspect(&child).unwrap_err().kind(),
            ErrorKind::UnsupportedRoot
        );
    }

    #[test]
    fn real_git_creates_a_pinned_worktree_without_moving_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let git = Git::default();
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .current_dir(&source)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        };
        run(&["init", "-q"]);
        run(&["config", "user.name", "Fixture"]);
        run(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(source.join("file.txt"), "source\n").unwrap();
        run(&["add", "file.txt"]);
        run(&["commit", "-qm", "fixture"]);
        let repository = git.inspect(&source).unwrap();
        let destination = directory.path().canonicalize().unwrap().join("linked");
        git.add(&source, &destination, "rt/test", &repository.head_commit)
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(destination.join("file.txt")).unwrap(),
            "source\n"
        );
        assert_eq!(git.list(&source).unwrap().len(), 2);
    }

    #[test]
    fn checkout_filters_and_git_ref_edge_cases_are_rejected_before_creation() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let run = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .current_dir(&source)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        run(&["init", "-q"]);
        run(&["config", "user.name", "Fixture"]);
        run(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(source.join("fixture.txt"), "fixture\n").unwrap();
        run(&["add", "fixture.txt"]);
        run(&["commit", "-qm", "fixture"]);
        let git = Git::default();
        assert_eq!(
            git.branch_available(&source, "invalid.lock")
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidReference
        );
        std::fs::write(source.join(".gitattributes"), "* filter=fixture\n").unwrap();
        let repository = git.inspect(&source).unwrap();
        assert_eq!(
            git.add(
                &source,
                &directory.path().canonicalize().unwrap().join("linked"),
                "rt/filter",
                &repository.head_commit,
            )
            .unwrap_err()
            .kind(),
            ErrorKind::UnsupportedCheckoutFilter
        );
        assert!(!directory.path().join("linked").exists());
    }
}
