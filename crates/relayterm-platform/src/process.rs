use std::{
    ffi::OsString,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessLaunchError;

/// Start the long-lived runtime without inheriting the caller's standard streams.
pub fn spawn_detached(executable: &Path, arguments: &[OsString]) -> Result<(), ProcessLaunchError> {
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
    command.spawn().map_err(|_| ProcessLaunchError)?;
    Ok(())
}

/// Separate the Unix runtime from the launching session after process creation.
pub fn detach_current_process() -> Result<(), ProcessLaunchError> {
    #[cfg(unix)]
    rustix::process::setsid().map_err(|_| ProcessLaunchError)?;
    Ok(())
}
