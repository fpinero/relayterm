use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

pub const MAX_EXECUTABLE_PATH_BYTES: usize = 64 * 1024;
pub const MAX_EXECUTABLE_PATH_ENTRIES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableStatus {
    Available,
    NotFound,
    NotExecutable,
    UnsupportedLauncher,
    InvalidCommand,
    Unavailable,
}

pub struct ResolvedExecutable {
    pub path: PathBuf,
}

pub struct ExecutableResolution {
    pub status: ExecutableStatus,
    pub executable: Option<ResolvedExecutable>,
}

/// Resolve one executable without running it or consulting a shell or credential store.
pub fn resolve_executable(
    command: &str,
    workspace_root: &Path,
    working_directory: &Path,
    search_path: Option<&OsStr>,
) -> ExecutableResolution {
    if command.is_empty() || command.contains(['\0', '\n', '\r']) {
        return resolution(ExecutableStatus::InvalidCommand, None);
    }
    let configured = Path::new(command);
    if configured.is_absolute() || configured.components().count() > 1 {
        let candidate = if configured.is_absolute() {
            configured.to_path_buf()
        } else {
            let candidate = working_directory.join(configured);
            let Ok(candidate) = candidate.canonicalize() else {
                return resolution(ExecutableStatus::NotFound, None);
            };
            let Ok(root) = workspace_root.canonicalize() else {
                return resolution(ExecutableStatus::Unavailable, None);
            };
            if !candidate.starts_with(root) {
                return resolution(ExecutableStatus::InvalidCommand, None);
            }
            candidate
        };
        return inspect_candidate(candidate);
    }
    let Some(search_path) = search_path else {
        return resolution(ExecutableStatus::NotFound, None);
    };
    if search_path.to_string_lossy().len() > MAX_EXECUTABLE_PATH_BYTES {
        return resolution(ExecutableStatus::Unavailable, None);
    }
    let entries: Vec<_> = std::env::split_paths(search_path).collect();
    if entries.len() > MAX_EXECUTABLE_PATH_ENTRIES {
        return resolution(ExecutableStatus::Unavailable, None);
    }
    let mut non_executable = None;
    for directory in entries
        .into_iter()
        .filter(|directory| directory.is_absolute() && !directory.as_os_str().is_empty())
    {
        for candidate in executable_candidates(&directory, command) {
            let inspected = inspect_candidate(candidate);
            if inspected.status == ExecutableStatus::Available {
                return inspected;
            }
            if matches!(
                inspected.status,
                ExecutableStatus::NotExecutable | ExecutableStatus::UnsupportedLauncher
            ) {
                non_executable = Some(inspected.status);
            }
        }
    }
    resolution(non_executable.unwrap_or(ExecutableStatus::NotFound), None)
}

fn resolution(status: ExecutableStatus, path: Option<PathBuf>) -> ExecutableResolution {
    ExecutableResolution {
        status,
        executable: path.map(|path| ResolvedExecutable { path }),
    }
}

fn inspect_candidate(path: PathBuf) -> ExecutableResolution {
    let Ok(metadata) = std::fs::metadata(&path) else {
        return resolution(ExecutableStatus::NotFound, None);
    };
    if !metadata.is_file() {
        return resolution(ExecutableStatus::NotExecutable, None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return resolution(ExecutableStatus::NotExecutable, None);
        }
    }
    #[cfg(windows)]
    {
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(extension.as_str(), "cmd" | "bat" | "ps1") {
            return resolution(ExecutableStatus::UnsupportedLauncher, None);
        }
        if !matches!(extension.as_str(), "exe" | "com") {
            return resolution(ExecutableStatus::NotExecutable, None);
        }
    }
    resolution(ExecutableStatus::Available, Some(path))
}

fn executable_candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let configured = Path::new(command);
        if configured.extension().is_some() {
            vec![directory.join(configured)]
        } else {
            vec![
                directory.join(format!("{command}.exe")),
                directory.join(format!("{command}.com")),
                directory.join(configured),
            ]
        }
    }
    #[cfg(not(windows))]
    {
        vec![directory.join(command)]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessLaunchError;

/// Start the long-lived runtime without inheriting the caller's standard streams.
pub fn spawn_detached(
    executable: &Path,
    arguments: &[OsString],
) -> Result<Child, ProcessLaunchError> {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    command.spawn().map_err(|_| ProcessLaunchError)
}

/// Separate the Unix runtime from the launching session after process creation.
pub fn detach_current_process() -> Result<(), ProcessLaunchError> {
    #[cfg(unix)]
    rustix::process::setsid().map_err(|_| ProcessLaunchError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_executable(path: &Path) {
        std::fs::write(path, b"resolution must not execute this file").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    #[test]
    fn resolution_is_side_effect_free_bounded_and_skips_relative_path_entries() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join(if cfg!(windows) {
            "neutral.exe"
        } else {
            "neutral"
        });
        make_executable(&executable);
        let found = resolve_executable(
            "neutral",
            directory.path(),
            directory.path(),
            Some(directory.path().as_os_str()),
        );
        assert_eq!(found.status, ExecutableStatus::Available);
        assert_eq!(found.executable.unwrap().path, executable);
        assert_eq!(
            resolve_executable("", directory.path(), directory.path(), None).status,
            ExecutableStatus::InvalidCommand
        );
        assert_eq!(
            resolve_executable(
                "neutral",
                directory.path(),
                directory.path(),
                Some(OsStr::new("."))
            )
            .status,
            ExecutableStatus::NotFound
        );
        let excessive = std::iter::repeat_n("/synthetic", MAX_EXECUTABLE_PATH_ENTRIES + 1)
            .collect::<Vec<_>>()
            .join(if cfg!(windows) { ";" } else { ":" });
        assert_eq!(
            resolve_executable(
                "neutral",
                directory.path(),
                directory.path(),
                Some(OsStr::new(&excessive))
            )
            .status,
            ExecutableStatus::Unavailable
        );
    }

    #[test]
    fn explicit_and_path_candidates_obey_native_boundaries_without_running() {
        let workspace = tempfile::tempdir().unwrap();
        let bin = workspace.path().join("directory with space λ");
        std::fs::create_dir(&bin).unwrap();
        let name = if cfg!(windows) {
            "unknown.EXE"
        } else {
            "unknown"
        };
        let executable = bin.join(name);
        make_executable(&executable);
        let marker = workspace.path().join("must-not-exist");

        let bare = resolve_executable(
            name,
            workspace.path(),
            workspace.path(),
            Some(bin.as_os_str()),
        );
        assert_eq!(bare.status, ExecutableStatus::Available);
        assert_eq!(bare.executable.unwrap().path, executable);
        let relative = resolve_executable(
            &format!("directory with space λ/{name}"),
            workspace.path(),
            workspace.path(),
            None,
        );
        assert_eq!(relative.status, ExecutableStatus::Available);
        assert!(!marker.exists(), "executable resolution ran the candidate");

        let outside = tempfile::tempdir().unwrap();
        let escaped = outside.path().join(name);
        make_executable(&escaped);
        let relative_escape = Path::new("..")
            .join(
                outside
                    .path()
                    .file_name()
                    .expect("temporary directory has a file name"),
            )
            .join(name);
        assert_eq!(
            resolve_executable(
                relative_escape.to_str().unwrap(),
                workspace.path(),
                workspace.path(),
                None,
            )
            .status,
            ExecutableStatus::InvalidCommand
        );
        assert_eq!(
            resolve_executable(
                bin.to_str().unwrap(),
                workspace.path(),
                workspace.path(),
                None,
            )
            .status,
            ExecutableStatus::NotExecutable
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_rejects_non_executable_and_broken_targets() {
        use std::os::unix::fs::symlink;
        let workspace = tempfile::tempdir().unwrap();
        let plain = workspace.path().join("plain");
        std::fs::write(&plain, b"not executable").unwrap();
        assert_eq!(
            resolve_executable(
                plain.to_str().unwrap(),
                workspace.path(),
                workspace.path(),
                None,
            )
            .status,
            ExecutableStatus::NotExecutable
        );
        let broken = workspace.path().join("broken");
        symlink(workspace.path().join("missing"), &broken).unwrap();
        assert_eq!(
            resolve_executable(
                broken.to_str().unwrap(),
                workspace.path(),
                workspace.path(),
                None,
            )
            .status,
            ExecutableStatus::NotFound
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_accepts_native_suffixes_and_rejects_shell_launchers() {
        let workspace = tempfile::tempdir().unwrap();
        let executable = workspace.path().join("Tool.EXE");
        make_executable(&executable);
        assert_eq!(
            resolve_executable(
                "Tool.EXE",
                workspace.path(),
                workspace.path(),
                Some(workspace.path().as_os_str()),
            )
            .status,
            ExecutableStatus::Available
        );
        for name in ["shim.cmd", "shim.bat", "shim.ps1"] {
            std::fs::write(workspace.path().join(name), b"echo unsupported").unwrap();
            assert_eq!(
                resolve_executable(
                    name,
                    workspace.path(),
                    workspace.path(),
                    Some(workspace.path().as_os_str()),
                )
                .status,
                ExecutableStatus::UnsupportedLauncher
            );
        }
    }
}
