use crate::{create_private_file, validate_private_file};
use fs4::{FileExt, TryLockError};
use std::{
    fs::{File, OpenOptions},
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockError {
    Busy,
    AccessDenied,
    Unavailable,
}

pub struct PrivateLock {
    file: File,
}

impl PrivateLock {
    pub fn acquire(path: &Path, timeout: Duration) -> Result<Self, LockError> {
        let file = if path.exists() {
            validate_private_file(path).map_err(|_| LockError::AccessDenied)?;
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(map_io)?
        } else {
            create_private_file(path).map_err(|_| LockError::AccessDenied)?
        };
        let started = Instant::now();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => return Ok(Self { file }),
                Err(TryLockError::WouldBlock) => {
                    if started.elapsed() >= timeout {
                        return Err(LockError::Busy);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(TryLockError::Error(error)) => return Err(map_io(error)),
            }
        }
    }
}

impl Drop for PrivateLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

fn map_io(error: std::io::Error) -> LockError {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => LockError::AccessDenied,
        _ => LockError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competing_lock_has_bounded_failure_and_recovers() {
        let temporary = tempfile::tempdir().unwrap();
        let private = temporary.path().join("private");
        crate::create_private_dir(&private).unwrap();
        let path = private.join("registry.lock");
        let first = PrivateLock::acquire(&path, Duration::from_millis(20)).unwrap();
        assert!(matches!(
            PrivateLock::acquire(&path, Duration::from_millis(20)),
            Err(LockError::Busy)
        ));
        drop(first);
        assert!(PrivateLock::acquire(&path, Duration::from_millis(20)).is_ok());
    }
}
