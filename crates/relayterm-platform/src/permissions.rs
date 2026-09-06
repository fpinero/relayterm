use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivatePathError {
    AccessDenied,
    InvalidType,
    LinkRejected,
    Unavailable,
}

pub fn create_private_dir(path: &Path) -> Result<(), PrivatePathError> {
    if path.exists() {
        return validate_private_dir(path);
    }
    fs::create_dir_all(path).map_err(map_io)?;
    secure_dir(path)?;
    validate_private_dir(path)
}

pub fn create_private_file(path: &Path) -> Result<File, PrivatePathError> {
    if let Some(parent) = path.parent() {
        validate_private_dir(parent)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.access_mode(windows_private_file_access());
    }
    let mut file = options.open(path).map_err(map_io)?;
    secure_file(&mut file)?;
    validate_private_file(path)?;
    Ok(file)
}

pub fn validate_private_dir(path: &Path) -> Result<(), PrivatePathError> {
    let metadata = fs::symlink_metadata(path).map_err(map_io)?;
    if metadata.file_type().is_symlink() {
        return Err(PrivatePathError::LinkRejected);
    }
    if !metadata.is_dir() {
        return Err(PrivatePathError::InvalidType);
    }
    validate_metadata(&metadata, true)?;
    #[cfg(windows)]
    validate_windows_acl(path)?;
    Ok(())
}

pub fn validate_private_file(path: &Path) -> Result<(), PrivatePathError> {
    let metadata = fs::symlink_metadata(path).map_err(map_io)?;
    if metadata.file_type().is_symlink() {
        return Err(PrivatePathError::LinkRejected);
    }
    if !metadata.is_file() {
        return Err(PrivatePathError::InvalidType);
    }
    validate_metadata(&metadata, false)?;
    #[cfg(windows)]
    validate_windows_acl(path)?;
    Ok(())
}

/// Secure a file created by a trusted library inside an already private directory.
pub fn secure_generated_file(path: &Path) -> Result<(), PrivatePathError> {
    let metadata = fs::symlink_metadata(path).map_err(map_io)?;
    if metadata.file_type().is_symlink() {
        return Err(PrivatePathError::LinkRejected);
    }
    if !metadata.is_file() {
        return Err(PrivatePathError::InvalidType);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(map_io)?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        let mut file = OpenOptions::new()
            .read(true)
            .access_mode(windows_private_file_access())
            .open(path)
            .map_err(map_io)?;
        secure_windows_handle(&mut file, false)?;
    }
    validate_private_file(path)
}

#[cfg(unix)]
fn secure_dir(path: &Path) -> Result<(), PrivatePathError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(map_io)
}

#[cfg(windows)]
fn secure_dir(path: &Path) -> Result<(), PrivatePathError> {
    secure_windows_path(path)
}

#[cfg(not(any(unix, windows)))]
fn secure_dir(_: &Path) -> Result<(), PrivatePathError> {
    Err(PrivatePathError::Unavailable)
}

#[cfg(unix)]
fn secure_file(_: &mut File) -> Result<(), PrivatePathError> {
    Ok(())
}

#[cfg(windows)]
fn secure_file(file: &mut File) -> Result<(), PrivatePathError> {
    secure_windows_handle(file, false)
}

#[cfg(not(any(unix, windows)))]
fn secure_file(_: &mut File) -> Result<(), PrivatePathError> {
    Err(PrivatePathError::Unavailable)
}

#[cfg(unix)]
fn validate_metadata(metadata: &fs::Metadata, directory: bool) -> Result<(), PrivatePathError> {
    use std::os::unix::fs::MetadataExt;
    let expected = if directory { 0o700 } else { 0o600 };
    if metadata.uid() != rustix::process::getuid().as_raw() || metadata.mode() & 0o777 != expected {
        return Err(PrivatePathError::AccessDenied);
    }
    Ok(())
}

#[cfg(windows)]
fn validate_metadata(_: &fs::Metadata, _: bool) -> Result<(), PrivatePathError> {
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn validate_metadata(_: &fs::Metadata, _: bool) -> Result<(), PrivatePathError> {
    Err(PrivatePathError::Unavailable)
}

#[cfg(windows)]
fn private_descriptor(
    inheritable: bool,
) -> Result<windows_permissions::LocalBox<windows_permissions::SecurityDescriptor>, PrivatePathError>
{
    let current = windows_permissions::utilities::current_process_sid()
        .map_err(|_| PrivatePathError::AccessDenied)?;
    let descriptor = if inheritable {
        format!("D:P(A;OICI;FA;;;{current})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)")
    } else {
        format!("D:P(A;;FA;;;{current})(A;;FA;;;SY)(A;;FA;;;BA)")
    };
    windows_permissions::wrappers::ConvertStringSecurityDescriptorToSecurityDescriptor(
        std::ffi::OsStr::new(&descriptor),
    )
    .map_err(|_| PrivatePathError::AccessDenied)
}

#[cfg(windows)]
fn secure_windows_handle(file: &mut File, inheritable: bool) -> Result<(), PrivatePathError> {
    use windows_permissions::{
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let descriptor = private_descriptor(inheritable)?;
    wrappers::SetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        Some(descriptor.dacl().ok_or(PrivatePathError::AccessDenied)?),
        None,
    )
    .map_err(|_| PrivatePathError::AccessDenied)
}

#[cfg(windows)]
fn secure_windows_path(path: &Path) -> Result<(), PrivatePathError> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let mut file = OpenOptions::new()
        .read(true)
        .access_mode(windows_security_access())
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(map_io)?;
    secure_windows_handle(&mut file, true)
}

#[cfg(windows)]
fn validate_windows_acl(path: &Path) -> Result<(), PrivatePathError> {
    use windows_permissions::{WindowsSecure, constants::SecurityInformation, wrappers};
    let descriptor = path
        .as_os_str()
        .security_descriptor(SecurityInformation::Owner | SecurityInformation::Dacl)
        .map_err(|_| PrivatePathError::AccessDenied)?;
    let owner = descriptor.owner().ok_or(PrivatePathError::AccessDenied)?;
    let current = windows_permissions::utilities::current_process_sid()
        .map_err(|_| PrivatePathError::AccessDenied)?;
    if !wrappers::EqualSid(owner, &current) {
        return Err(PrivatePathError::AccessDenied);
    }
    let sddl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &descriptor,
        SecurityInformation::Dacl,
    )
    .map_err(|_| PrivatePathError::AccessDenied)?;
    if !sddl.to_string_lossy().starts_with("D:P") {
        return Err(PrivatePathError::AccessDenied);
    }
    let dacl = descriptor.dacl().ok_or(PrivatePathError::AccessDenied)?;
    for sid_text in ["S-1-1-0", "S-1-5-32-545"] {
        let sid = wrappers::ConvertStringSidToSid(std::ffi::OsStr::new(sid_text))
            .map_err(|_| PrivatePathError::AccessDenied)?;
        let trustee = windows_permissions::Trustee::from(&*sid);
        if !dacl
            .effective_rights(&trustee)
            .map_err(|_| PrivatePathError::AccessDenied)?
            .is_empty()
        {
            return Err(PrivatePathError::AccessDenied);
        }
    }
    Ok(())
}

#[cfg(windows)]
const fn windows_security_access() -> u32 {
    const READ_CONTROL: u32 = 0x0002_0000;
    const WRITE_DAC: u32 = 0x0004_0000;
    READ_CONTROL | WRITE_DAC
}

#[cfg(windows)]
const fn windows_private_file_access() -> u32 {
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    GENERIC_READ | GENERIC_WRITE | windows_security_access()
}

fn map_io(error: io::Error) -> PrivatePathError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => PrivatePathError::AccessDenied,
        _ => PrivatePathError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_private_directory_and_file() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("managed");
        create_private_dir(&directory).unwrap();
        let file = directory.join("state.sqlite3");
        create_private_file(&file).unwrap();
        validate_private_file(&file).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_permissive_existing_directory() {
        use std::os::unix::fs::PermissionsExt;
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("managed");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            validate_private_dir(&directory),
            Err(PrivatePathError::AccessDenied)
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_broad_existing_windows_acl() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_permissions::WindowsSecure;

        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("managed");
        create_private_dir(&directory).unwrap();
        let descriptor =
            windows_permissions::wrappers::ConvertStringSecurityDescriptorToSecurityDescriptor(
                std::ffi::OsStr::new("D:P(A;;FA;;;WD)"),
            )
            .unwrap();
        let mut handle = OpenOptions::new()
            .read(true)
            .access_mode(windows_security_access())
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(&directory)
            .unwrap();
        handle.set_dacl(descriptor.dacl().unwrap()).unwrap();
        assert_eq!(
            validate_private_dir(&directory),
            Err(PrivatePathError::AccessDenied)
        );
    }
}
