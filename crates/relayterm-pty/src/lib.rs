//! Native PTY adapter. Durable lifecycle decisions remain outside this crate.

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    ffi::{OsStr, OsString},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PtyError {
    InvalidRequest,
    Spawn,
    Io,
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("The supervised terminal operation failed.")
    }
}

impl std::error::Error for PtyError {}

pub struct SpawnRequest {
    pub program: OsString,
    pub arguments: Vec<OsString>,
    pub working_directory: PathBuf,
    pub environment: Vec<(OsString, OsString)>,
    pub rows: u16,
    pub columns: u16,
}

pub struct NativeSession {
    control: NativeControl,
    writer: Box<dyn Write + Send>,
    reader: Box<dyn Read + Send>,
}

pub struct NativeControl {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl NativeSession {
    pub fn spawn(request: SpawnRequest) -> Result<Self, PtyError> {
        validate(&request)?;
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: request.rows,
                cols: request.columns,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| PtyError::Spawn)?;
        let mut command = CommandBuilder::new(&request.program);
        command.args(&request.arguments);
        command.cwd(&request.working_directory);
        command.env_clear();
        for (name, value) in request.environment {
            command.env(name, value);
        }
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|_| PtyError::Spawn)?;
        drop(pair.slave);
        #[cfg(unix)]
        let process_group = pair.master.process_group_leader();
        let reader = pair.master.try_clone_reader().map_err(|_| PtyError::Io)?;
        let writer = pair.master.take_writer().map_err(|_| PtyError::Io)?;
        Ok(Self {
            control: NativeControl {
                master: pair.master,
                child,
                #[cfg(unix)]
                process_group,
            },
            writer,
            reader,
        })
    }

    pub fn into_parts(self) -> (NativeControl, Box<dyn Write + Send>, Box<dyn Read + Send>) {
        (self.control, self.writer, self.reader)
    }
}

impl NativeControl {
    pub fn resize(&self, rows: u16, columns: u16) -> Result<(), PtyError> {
        validate_size(rows, columns)?;
        self.master
            .resize(PtySize {
                rows,
                cols: columns,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| PtyError::Io)
    }

    pub fn try_wait(&mut self) -> Result<Option<Exit>, PtyError> {
        self.child
            .try_wait()
            .map(|status| status.map(Exit::from))
            .map_err(|_| PtyError::Io)
    }

    pub fn terminate(&mut self) -> Result<(), PtyError> {
        #[cfg(unix)]
        if let Some(group) = self.process_group.and_then(rustix::process::Pid::from_raw) {
            return rustix::process::kill_process_group(group, rustix::process::Signal::KILL)
                .map_err(|_| PtyError::Io);
        }
        #[cfg(windows)]
        if let Some(process_id) = self.child.process_id() {
            let mut terminator = std::process::Command::new("taskkill.exe")
                .args(["/PID", &process_id.to_string(), "/T", "/F"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|_| PtyError::Io)?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                if let Some(status) = terminator.try_wait().map_err(|_| PtyError::Io)? {
                    if status.success() {
                        return Ok(());
                    }
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    let _ = terminator.kill();
                    let _ = terminator.wait();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        self.child.kill().map_err(|_| PtyError::Io)
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }
}

pub fn write_input(writer: &mut dyn Write, bytes: &[u8]) -> Result<(), PtyError> {
    writer.write_all(bytes).map_err(|_| PtyError::Io)?;
    writer.flush().map_err(|_| PtyError::Io)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exit {
    pub code: i32,
    pub signal: Option<String>,
}

impl From<portable_pty::ExitStatus> for Exit {
    fn from(value: portable_pty::ExitStatus) -> Self {
        Self {
            code: i32::try_from(value.exit_code()).unwrap_or(i32::MAX),
            signal: value.signal().map(str::to_owned),
        }
    }
}

fn validate(request: &SpawnRequest) -> Result<(), PtyError> {
    if request.program.is_empty()
        || !request.working_directory.is_absolute()
        || !request.working_directory.is_dir()
        || request.arguments.len() > 128
        || request.environment.len() > 128
    {
        return Err(PtyError::InvalidRequest);
    }
    validate_size(request.rows, request.columns)
}

fn validate_size(rows: u16, columns: u16) -> Result<(), PtyError> {
    if !(1..=1000).contains(&rows)
        || !(1..=1000).contains(&columns)
        || usize::from(rows) * usize::from(columns) > 160_000
    {
        Err(PtyError::InvalidRequest)
    } else {
        Ok(())
    }
}

pub fn default_shell() -> OsString {
    #[cfg(windows)]
    {
        std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"))
    }
}

pub fn approved_environment(
    additional: &[String],
    source: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    source
        .into_iter()
        .filter(|(name, _)| approved_name(name, additional))
        .collect()
}

pub fn terminal_environment(
    mut environment: Vec<(OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    environment.retain(|(name, _)| !name.eq_ignore_ascii_case(OsStr::new("TERM")));
    environment.push((OsString::from("TERM"), OsString::from("xterm-256color")));
    environment
}

fn approved_name(name: &OsStr, additional: &[String]) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    #[cfg(windows)]
    let equal = |candidate: &str| name.eq_ignore_ascii_case(candidate);
    #[cfg(not(windows))]
    let equal = |candidate: &str| name == candidate;
    let baseline: &[&str] = if cfg!(windows) {
        &[
            "PATH",
            "SystemRoot",
            "WINDIR",
            "COMSPEC",
            "PATHEXT",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "TEMP",
            "TMP",
        ]
    } else {
        &[
            "PATH",
            "HOME",
            "SHELL",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "TMPDIR",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_RUNTIME_DIR",
        ]
    };
    baseline.iter().any(|candidate| equal(candidate))
        || additional.iter().any(|candidate| equal(candidate))
}

pub fn canonical_working_directory(root: &Path, requested: &Path) -> Result<PathBuf, PtyError> {
    let root = root.canonicalize().map_err(|_| PtyError::InvalidRequest)?;
    let requested = requested
        .canonicalize()
        .map_err(|_| PtyError::InvalidRequest)?;
    if requested.starts_with(&root) && requested.is_dir() {
        Ok(requested)
    } else {
        Err(PtyError::InvalidRequest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_is_allowlisted_without_values_in_errors() {
        let values = vec![
            (OsString::from("PATH"), OsString::from("synthetic-path")),
            (
                OsString::from("SYNTHETIC_TOKEN"),
                OsString::from("not-secret"),
            ),
        ];
        let inherited = approved_environment(&[], values);
        assert_eq!(inherited.len(), 1);
        assert_eq!(
            PtyError::Spawn.to_string(),
            "The supervised terminal operation failed."
        );
    }

    #[test]
    fn native_child_produces_output_and_is_reaped() {
        #[cfg(windows)]
        let (program, arguments) = (
            OsString::from("cmd.exe"),
            vec![
                OsString::from("/D"),
                OsString::from("/C"),
                OsString::from("echo relayterm-pty"),
            ],
        );
        #[cfg(not(windows))]
        let (program, arguments) = (
            OsString::from("/bin/sh"),
            vec![OsString::from("-c"), OsString::from("printf relayterm-pty")],
        );
        let native = NativeSession::spawn(SpawnRequest {
            program,
            arguments,
            working_directory: std::env::current_dir().unwrap(),
            environment: approved_environment(&[], std::env::vars_os()),
            rows: 24,
            columns: 80,
        })
        .unwrap();
        let (mut control, writer, mut reader) = native.into_parts();
        drop(writer);
        let (output_tx, output_rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = reader.read_to_end(&mut output).map(|_| output);
            let _ = output_tx.send(result);
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let exit = loop {
            if let Some(exit) = control.try_wait().unwrap() {
                break exit;
            }
            assert!(std::time::Instant::now() < deadline, "child did not exit");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(exit.code == 0);
        drop(control);
        let output = output_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert!(String::from_utf8_lossy(&output).contains("relayterm-pty"));
    }
}
