//! Bounded, argument-array Git operations for explicit worktree management.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
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
        let top = self.path(root, ["rev-parse", "--show-toplevel"])?;
        let common = self.path(
            root,
            ["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        let head = self.text(root, ["rev-parse", "--verify", "HEAD^{commit}"])?;
        let checkout_top = canonical_existing(&top)?;
        let requested = canonical_existing(root)?;
        if checkout_top != requested {
            return Err(Error::new(ErrorKind::UnsupportedRoot));
        }
        Ok(Repository {
            checkout_top,
            common_directory: canonical_existing(&common)?,
            head_commit: validate_object_id(single_line(&head)?)?.to_owned(),
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
        Ok(validate_object_id(single_line(&value)?)?.to_owned())
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
        // Checks which protect an external effect must run while every Relayterm
        // process sharing this Git common directory is excluded.
        match std::fs::symlink_metadata(destination) {
            Ok(_) => return Err(Error::new(ErrorKind::DestinationConflict)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::new(ErrorKind::Io)),
        }
        self.branch_available(root, branch)?;
        self.reject_checkout_filters(root, commit)?;
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
        _initial_commit: &str,
    ) -> Result<()> {
        self.verify_worktree_identity(source, destination, branch, common_directory)
    }

    pub fn verify_created_worktree(
        &self,
        source: &Path,
        destination: &Path,
        branch: &str,
        common_directory: &Path,
        initial_commit: &str,
    ) -> Result<()> {
        self.verify_worktree_identity(source, destination, branch, common_directory)?;
        let repository = self.inspect(destination)?;
        if repository.head_commit != initial_commit {
            return Err(Error::new(ErrorKind::CommandFailed));
        }
        Ok(())
    }

    fn verify_worktree_identity(
        &self,
        source: &Path,
        destination: &Path,
        branch: &str,
        common_directory: &Path,
    ) -> Result<()> {
        let destination = canonical_existing(destination)?;
        let repository = self.inspect(&destination)?;
        if repository.common_directory != common_directory {
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

    fn path<const N: usize>(&self, root: &Path, args: [&str; N]) -> Result<PathBuf> {
        let output = self.run(Some(root), args.map(OsStr::new), self.read_timeout)?;
        bytes_to_path(single_line_bytes(&output)?)
    }

    fn reject_checkout_filters(&self, root: &Path, commit: &str) -> Result<()> {
        reject_checkout_filters(root)?;
        match self.run_status(root, ["config", "--get-regexp", "^filter\\."]) {
            Ok(0) => Err(Error::new(ErrorKind::UnsupportedCheckoutFilter)),
            Ok(1) => self.reject_committed_checkout_filters(root, commit),
            Ok(_) => Err(Error::new(ErrorKind::CommandFailed)),
            Err(error) => Err(error),
        }
    }

    fn reject_committed_checkout_filters(&self, root: &Path, commit: &str) -> Result<()> {
        let output = self.run(
            Some(root),
            [
                OsStr::new("ls-tree"),
                OsStr::new("-r"),
                OsStr::new("-z"),
                OsStr::new("--name-only"),
                OsStr::new(commit),
            ],
            self.read_timeout,
        )?;
        let mut count = 0usize;
        for path in output
            .split(|byte| *byte == 0)
            .filter(|value| !value.is_empty())
        {
            if path == b".gitattributes" || path.ends_with(b"/.gitattributes") {
                count = count.saturating_add(1);
                if count > 128 {
                    return Err(Error::new(ErrorKind::OutputLimit));
                }
                let path = std::str::from_utf8(path)
                    .map_err(|_| Error::new(ErrorKind::UnsupportedCheckoutFilter))?;
                let object = format!("{commit}:{path}");
                let attributes = self.run(
                    Some(root),
                    [
                        OsStr::new("show"),
                        OsStr::new("--no-textconv"),
                        OsStr::new(&object),
                    ],
                    self.read_timeout,
                )?;
                if contains_filter_attribute(&attributes) {
                    return Err(Error::new(ErrorKind::UnsupportedCheckoutFilter));
                }
            }
        }
        Ok(())
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
        let inherited = [
            "PATH",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "TMPDIR",
            "TEMP",
            "TMP",
            "SystemRoot",
            "WINDIR",
            "PATHEXT",
            "COMSPEC",
        ]
        .into_iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (name, value)))
        .collect::<Vec<_>>();
        command
            .stdin(Stdio::null())
            .env_clear()
            .envs(inherited)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "Never")
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_PROTOCOL_FROM_USER", "0");
        isolate_process_group(&mut command);
        command
    }

    fn run<I, S>(&self, root: Option<&Path>, args: I, timeout: Duration) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.command(root);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(map_spawn)?;
        let process_group = child.id();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::new(ErrorKind::Io))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::new(ErrorKind::Io))?;
        let (stdout_tx, stdout_rx) = mpsc::sync_channel(1);
        let (stderr_tx, stderr_rx) = mpsc::sync_channel(1);
        let command_finished = Arc::new(AtomicBool::new(false));
        let stdout_finished = command_finished.clone();
        let stderr_finished = command_finished.clone();
        thread::spawn(move || {
            let _ = stdout_tx.send(read_stream_bounded(stdout, MAX_STDOUT, stdout_finished));
        });
        thread::spawn(move || {
            let _ = stderr_tx.send(read_stream_bounded(stderr, MAX_STDERR, stderr_finished));
        });
        let status = wait_bounded(&mut child, timeout);
        terminate_process_group(process_group);
        command_finished.store(true, Ordering::Release);
        let stdout = receive_stream(&stdout_rx, timeout)?;
        let stderr = receive_stream(&stderr_rx, timeout)?;
        let status = status?;
        if !status.success() {
            let kind = classify_failure(&stderr);
            return Err(Error::new(kind));
        }
        Ok(stdout)
    }
}

#[cfg(unix)]
fn isolate_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn isolate_process_group(_: &mut Command) {}

#[cfg(unix)]
fn terminate_process_group(raw_pid: u32) {
    let Ok(raw_pid) = i32::try_from(raw_pid) else {
        return;
    };
    let Some(group) = rustix::process::Pid::from_raw(raw_pid) else {
        return;
    };
    let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
}

#[cfg(windows)]
fn terminate_process_group(_: u32) {}

#[cfg(unix)]
fn read_stream_bounded(
    mut input: impl Read + std::os::fd::AsFd,
    limit: usize,
    command_finished: Arc<AtomicBool>,
) -> Result<Vec<u8>> {
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

    let flags = fcntl_getfl(&input).map_err(|_| Error::new(ErrorKind::Io))?;
    fcntl_setfl(&input, flags | OFlags::NONBLOCK).map_err(|_| Error::new(ErrorKind::Io))?;
    read_nonblocking(&mut input, limit, &command_finished)
}

#[cfg(unix)]
fn read_nonblocking(
    input: &mut impl Read,
    limit: usize,
    command_finished: &AtomicBool,
) -> Result<Vec<u8>> {
    let mut value = Vec::with_capacity(limit.min(64 * 1024));
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let read = match input.read(&mut chunk) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if command_finished.load(Ordering::Acquire) {
                    return Ok(value);
                }
                thread::sleep(Duration::from_millis(2));
                continue;
            }
            Err(_) => return Err(Error::new(ErrorKind::Io)),
        };
        if read == 0 {
            return Ok(value);
        }
        if value.len().saturating_add(read) > limit {
            return Err(Error::new(ErrorKind::OutputLimit));
        }
        value.extend_from_slice(&chunk[..read]);
    }
}

#[cfg(windows)]
fn read_stream_bounded(
    mut input: impl Read,
    limit: usize,
    _command_finished: Arc<AtomicBool>,
) -> Result<Vec<u8>> {
    let mut value = Vec::with_capacity(limit.min(64 * 1024));
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let read = input
            .read(&mut chunk)
            .map_err(|_| Error::new(ErrorKind::Io))?;
        if read == 0 {
            return Ok(value);
        }
        if value.len().saturating_add(read) > limit {
            return Err(Error::new(ErrorKind::OutputLimit));
        }
        value.extend_from_slice(&chunk[..read]);
    }
}

fn receive_stream(
    receiver: &mpsc::Receiver<Result<Vec<u8>>>,
    timeout: Duration,
) -> Result<Vec<u8>> {
    receiver
        .recv_timeout(timeout.min(Duration::from_secs(1)))
        .map_err(|_| Error::new(ErrorKind::Timeout))?
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
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let verbatim_unc = r"\\?\UNC\".encode_utf16().collect::<Vec<_>>();
    if units.starts_with(&verbatim_unc) {
        let mut native = r"\\".encode_utf16().collect::<Vec<_>>();
        native.extend_from_slice(&units[verbatim_unc.len()..]);
        return OsString::from_wide(&native);
    }
    let verbatim = r"\\?\".encode_utf16().collect::<Vec<_>>();
    if units.starts_with(&verbatim) {
        return OsString::from_wide(&units[verbatim.len()..]);
    }
    OsString::from_wide(&units)
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
        || value.ends_with(['.', ' '])
        || value.contains(['/', '\\', ':'])
        || value.chars().any(char::is_control)
    {
        return Err(Error::new(ErrorKind::DestinationConflict));
    }
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
    Ok(())
}

fn validate_object_id(value: &str) -> Result<&str> {
    if !(4..=128).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::new(ErrorKind::MalformedOutput));
    }
    Ok(value)
}

fn single_line(value: &str) -> Result<&str> {
    let bytes = single_line_bytes(value.as_bytes())?;
    std::str::from_utf8(bytes).map_err(|_| Error::new(ErrorKind::MalformedOutput))
}

fn single_line_bytes(value: &[u8]) -> Result<&[u8]> {
    let value = value
        .strip_suffix(b"\n")
        .and_then(|value| value.strip_suffix(b"\r").or(Some(value)))
        .unwrap_or(value);
    if value.is_empty() || value.contains(&0) || value.contains(&b'\n') || value.contains(&b'\r') {
        return Err(Error::new(ErrorKind::MalformedOutput));
    }
    Ok(value)
}

fn contains_filter_attribute(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(|line| line.split_once('#').map_or(line, |(value, _)| value))
        .flat_map(str::split_ascii_whitespace)
        .any(|item| item == "filter" || item.starts_with("filter=") || item == "-filter")
}

fn reject_checkout_filters(root: &Path) -> Result<()> {
    let attributes = root.join(".gitattributes");
    let Ok(bytes) = std::fs::read(attributes) else {
        return Ok(());
    };
    if contains_filter_attribute(&bytes) {
        return Err(Error::new(ErrorKind::UnsupportedCheckoutFilter));
    }
    Ok(())
}

pub fn parse_porcelain_z(input: &[u8]) -> Result<Vec<WorktreeEntry>> {
    if !input.is_empty() && !input.ends_with(&[0]) {
        return Err(Error::new(ErrorKind::MalformedOutput));
    }
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
            b"HEAD" => {
                let current = current(&mut result)?;
                if current.head.replace(text_value(value)?).is_some() {
                    return Err(Error::new(ErrorKind::MalformedOutput));
                }
            }
            b"branch" => {
                let current = current(&mut result)?;
                if current.branch.replace(text_value(value)?).is_some() {
                    return Err(Error::new(ErrorKind::MalformedOutput));
                }
            }
            b"bare" => set_flag(&mut result, |value| &mut value.bare)?,
            b"detached" => set_flag(&mut result, |value| &mut value.detached)?,
            b"locked" => set_flag(&mut result, |value| &mut value.locked)?,
            b"prunable" => set_flag(&mut result, |value| &mut value.prunable)?,
            _ => {
                current(&mut result)?;
            }
        }
    }
    Ok(result)
}

fn set_flag(
    values: &mut [WorktreeEntry],
    select: impl FnOnce(&mut WorktreeEntry) -> &mut bool,
) -> Result<()> {
    let flag = select(current(values)?);
    if *flag {
        return Err(Error::new(ErrorKind::MalformedOutput));
    }
    *flag = true;
    Ok(())
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
    #[ignore]
    fn injected_git_environment_probe() {
        let root = PathBuf::from(std::env::var_os("RELAYTERM_GIT_PROBE_ROOT").unwrap());
        assert!(Git::default().inspect(&root).is_ok());
    }

    #[test]
    #[ignore]
    fn concurrent_add_probe() {
        let root = PathBuf::from(std::env::var_os("RELAYTERM_GIT_PROBE_ROOT").unwrap());
        let destination =
            PathBuf::from(std::env::var_os("RELAYTERM_GIT_PROBE_DESTINATION").unwrap());
        let start = PathBuf::from(std::env::var_os("RELAYTERM_GIT_PROBE_START").unwrap());
        let commit = std::env::var("RELAYTERM_GIT_PROBE_COMMIT").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !start.exists() {
            assert!(Instant::now() < deadline, "probe start barrier timed out");
            thread::sleep(Duration::from_millis(5));
        }
        Git::default()
            .add(&root, &destination, "rt/concurrent", &commit)
            .unwrap();
    }

    #[test]
    fn portable_validators_reject_injection_and_escape_forms() {
        for branch in ["", "-force", "a..b", "a@{b", "a b", "a\\b", "a:b", "a/"] {
            assert!(validate_branch(branch).is_err(), "{branch:?}");
        }
        assert!(validate_branch("rt/task-00000000-0000-4000-8000-000000000001").is_ok());
        for leaf in [
            "", ".", "..", "-x", "a/b", "a\\b", "C:x", "a\0b", "task ", "task.", "CON", "nul.txt",
        ] {
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
        for malformed in [
            &b"worktree /tmp/main"[..],
            &b"worktree /tmp/main\0HEAD 01234567\0HEAD abcdef12\0"[..],
            &b"worktree /tmp/main\0detached\0detached\0"[..],
        ] {
            assert_eq!(
                parse_porcelain_z(malformed).unwrap_err().kind(),
                ErrorKind::MalformedOutput
            );
        }
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

    #[cfg(target_os = "linux")]
    #[test]
    fn repository_discovery_preserves_non_utf8_native_paths() {
        use std::os::unix::ffi::OsStringExt;

        let directory = tempfile::tempdir().unwrap();
        let source = directory
            .path()
            .join(OsString::from_vec(b"source-\xff".to_vec()));
        std::fs::create_dir(&source).unwrap();
        for arguments in [
            vec!["init", "-q"],
            vec!["config", "user.name", "Fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
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
            Git::default().inspect(&source).unwrap().checkout_top,
            source.canonicalize().unwrap()
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
        let hook = source.join(".git/hooks/post-checkout");
        std::fs::write(&hook, "#!/bin/sh\ntouch relayterm-hook-ran\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let environment_probe = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "tests::injected_git_environment_probe",
                "--test-threads=1",
            ])
            .env("RELAYTERM_GIT_PROBE_ROOT", &source)
            .env("GIT_DIR", directory.path().join("foreign-git-dir"))
            .env("GIT_WORK_TREE", directory.path().join("foreign-work-tree"))
            .status()
            .unwrap();
        assert!(environment_probe.success());
        let repository = git.inspect(&source).unwrap();
        let destination = directory.path().canonicalize().unwrap().join("linked");
        git.add(&source, &destination, "rt/test", &repository.head_commit)
            .unwrap();
        assert!(!source.join("relayterm-hook-ran").exists());
        assert!(!destination.join("relayterm-hook-ran").exists());
        assert_eq!(
            std::fs::read_to_string(destination.join("file.txt")).unwrap(),
            "source\n"
        );
        assert_eq!(git.list(&source).unwrap().len(), 2);

        let initial_commit = repository.head_commit;
        let run_linked = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .current_dir(&destination)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        run_linked(&["config", "user.name", "Fixture"]);
        run_linked(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(destination.join("second.txt"), "user work\n").unwrap();
        run_linked(&["add", "second.txt"]);
        run_linked(&["commit", "-qm", "user work"]);
        assert!(
            git.verify_worktree(
                &source,
                &destination,
                "rt/test",
                &repository.common_directory,
                &initial_commit,
            )
            .is_ok()
        );
        assert_eq!(
            git.verify_created_worktree(
                &source,
                &destination,
                "rt/test",
                &repository.common_directory,
                &initial_commit,
            )
            .unwrap_err()
            .kind(),
            ErrorKind::CommandFailed
        );
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
        let included = directory.path().join("included-config");
        let included_marker = source.join("relayterm-included-filter-ran");
        std::fs::write(
            &included,
            "[filter \"included\"]\n\tsmudge = touch relayterm-included-filter-ran\n",
        )
        .unwrap();
        run(&["config", "include.path", included.to_str().unwrap()]);
        let repository = git.inspect(&source).unwrap();
        assert_eq!(
            git.add(
                &source,
                &directory.path().canonicalize().unwrap().join("included"),
                "rt/included-filter",
                &repository.head_commit,
            )
            .unwrap_err()
            .kind(),
            ErrorKind::UnsupportedCheckoutFilter
        );
        assert!(!included_marker.exists());
        assert!(!directory.path().join("included").exists());
        run(&["config", "--unset", "include.path"]);
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

        std::fs::create_dir(source.join("nested")).unwrap();
        std::fs::write(source.join("nested/.gitattributes"), "* filter=fixture\n").unwrap();
        run(&["add", "nested/.gitattributes"]);
        run(&["commit", "-qm", "nested attributes"]);
        std::fs::remove_file(source.join(".gitattributes")).unwrap();
        let repository = git.inspect(&source).unwrap();
        assert_eq!(
            git.add(
                &source,
                &directory
                    .path()
                    .canonicalize()
                    .unwrap()
                    .join("nested-filter"),
                "rt/nested-filter",
                &repository.head_commit,
            )
            .unwrap_err()
            .kind(),
            ErrorKind::UnsupportedCheckoutFilter
        );
        assert!(!directory.path().join("nested-filter").exists());
    }

    #[test]
    fn separate_processes_serialize_branch_and_destination_admission() {
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
        let commit = Git::default().inspect(&source).unwrap().head_commit;
        let canonical_parent = directory.path().canonicalize().unwrap();
        let start = canonical_parent.join("start");
        let spawn = |destination: &Path| {
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "tests::concurrent_add_probe",
                    "--test-threads=1",
                ])
                .env("RELAYTERM_GIT_PROBE_ROOT", &source)
                .env("RELAYTERM_GIT_PROBE_DESTINATION", destination)
                .env("RELAYTERM_GIT_PROBE_START", &start)
                .env("RELAYTERM_GIT_PROBE_COMMIT", &commit)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        };
        let mut first = spawn(&canonical_parent.join("first"));
        let mut second = spawn(&canonical_parent.join("second"));
        std::fs::write(&start, b"start").unwrap();
        let results = [
            first.wait().unwrap().success(),
            second.wait().unwrap().success(),
        ];
        assert_eq!(results.into_iter().filter(|result| *result).count(), 1);
        assert_eq!(Git::default().list(&source).unwrap().len(), 2);
    }

    #[test]
    fn line_decoding_preserves_identity_whitespace_and_rejects_extra_records() {
        assert_eq!(single_line(" value \n").unwrap(), " value ");
        assert_eq!(single_line_bytes(b"native \n").unwrap(), b"native ");
        assert_eq!(
            single_line("one\ntwo\n").unwrap_err().kind(),
            ErrorKind::MalformedOutput
        );
        assert_eq!(
            single_line_bytes(b"value\0tail\n").unwrap_err().kind(),
            ErrorKind::MalformedOutput
        );
    }

    #[cfg(unix)]
    #[test]
    fn stream_reader_stops_at_the_exact_output_budget() {
        assert_eq!(
            read_nonblocking(
                &mut std::io::Cursor::new(vec![b'x'; 16]),
                16,
                &AtomicBool::new(false),
            )
            .unwrap(),
            vec![b'x'; 16]
        );
        assert_eq!(
            read_nonblocking(
                &mut std::io::Cursor::new(vec![b'x'; 17]),
                16,
                &AtomicBool::new(false),
            )
            .unwrap_err()
            .kind(),
            ErrorKind::OutputLimit
        );
    }

    #[cfg(unix)]
    #[test]
    fn completed_git_does_not_wait_for_a_descendant_holding_stdout() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("git-fixture");
        std::fs::write(
            &executable,
            "#!/bin/sh\n(sleep 0.2; touch \"${0}.marker\") &\nprintf 'git version 2.99.0\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let started = Instant::now();
        assert_eq!(
            Git::with_executable(executable).version().unwrap(),
            "git version 2.99.0\n"
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        thread::sleep(Duration::from_millis(400));
        assert!(!directory.path().join("git-fixture.marker").exists());
    }
}
