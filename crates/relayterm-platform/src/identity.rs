use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathCodecError {
    UnsupportedPlatform,
    InvalidEncoding,
    TooLarge,
}

#[derive(Clone, Eq, PartialEq)]
pub struct NativePathEncoding {
    pub tag: &'static str,
    pub bytes: Vec<u8>,
}

pub fn encode_native_path(path: &Path) -> Result<NativePathEncoding, PathCodecError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes().to_vec();
        if bytes.len() > 8192 {
            return Err(PathCodecError::TooLarge);
        }
        Ok(NativePathEncoding {
            tag: "unix_bytes_v1",
            bytes,
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = path.as_os_str().encode_wide().collect();
        if units.len().saturating_mul(2) > 8192 {
            return Err(PathCodecError::TooLarge);
        }
        let bytes = units.into_iter().flat_map(u16::to_le_bytes).collect();
        Ok(NativePathEncoding {
            tag: "windows_utf16le_v1",
            bytes,
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(PathCodecError::UnsupportedPlatform)
    }
}

pub fn decode_native_path(tag: &str, bytes: &[u8]) -> Result<PathBuf, PathCodecError> {
    if bytes.len() > 8192 {
        return Err(PathCodecError::TooLarge);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        if tag != "unix_bytes_v1" {
            return Err(PathCodecError::UnsupportedPlatform);
        }
        Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        if tag != "windows_utf16le_v1" || !bytes.len().is_multiple_of(2) {
            return Err(PathCodecError::InvalidEncoding);
        }
        let units = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|x| u16::from_le_bytes(*x))
            .collect::<Vec<_>>();
        Ok(PathBuf::from(OsString::from_wide(&units)))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (tag, bytes);
        Err(PathCodecError::UnsupportedPlatform)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct WorkspaceRootIdentity {
    canonical_root: PathBuf,
    lookup_key: [u8; 32],
    filesystem_guard: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    Missing,
    NotDirectory,
    Unavailable,
}

impl WorkspaceRootIdentity {
    pub fn resolve(root: &Path) -> Result<Self, IdentityError> {
        let canonical_root = fs::canonicalize(root).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => IdentityError::Missing,
            _ => IdentityError::Unavailable,
        })?;
        let metadata = fs::metadata(&canonical_root).map_err(|_| IdentityError::Unavailable)?;
        if !metadata.is_dir() {
            return Err(IdentityError::NotDirectory);
        }
        let encoded =
            encode_native_path(&canonical_root).map_err(|_| IdentityError::Unavailable)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(encoded.tag.as_bytes());
        hasher.update(&[0]);
        hasher.update(&encoded.bytes);
        Ok(Self {
            canonical_root,
            lookup_key: *hasher.finalize().as_bytes(),
            filesystem_guard: filesystem_guard(&metadata),
        })
    }

    pub fn canonical_root(&self) -> &Path {
        &self.canonical_root
    }
    pub fn lookup_key(&self) -> &[u8; 32] {
        &self.lookup_key
    }
    pub fn filesystem_guard(&self) -> Option<&[u8]> {
        self.filesystem_guard.as_deref()
    }
}

#[cfg(unix)]
fn filesystem_guard(metadata: &fs::Metadata) -> Option<Vec<u8>> {
    use std::os::unix::fs::MetadataExt;
    let mut value = Vec::with_capacity(16);
    value.extend_from_slice(&metadata.dev().to_be_bytes());
    value.extend_from_slice(&metadata.ino().to_be_bytes());
    Some(value)
}

#[cfg(windows)]
fn filesystem_guard(_: &fs::Metadata) -> Option<Vec<u8>> {
    // Stable std does not expose Windows volume/file IDs yet. Keep this guard optional.
    None
}

#[cfg(not(any(unix, windows)))]
fn filesystem_guard(_: &fs::Metadata) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_share_one_identity_and_native_paths_round_trip() {
        let temporary = tempfile::tempdir().unwrap();
        let direct = WorkspaceRootIdentity::resolve(temporary.path()).unwrap();
        let alias = WorkspaceRootIdentity::resolve(&temporary.path().join(".")).unwrap();
        assert_eq!(direct.lookup_key(), alias.lookup_key());
        let encoded = encode_native_path(direct.canonical_root()).unwrap();
        assert_eq!(
            decode_native_path(encoded.tag, &encoded.bytes).unwrap(),
            direct.canonical_root()
        );
    }
}
