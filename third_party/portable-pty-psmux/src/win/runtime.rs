//! Pinned, executable-local runtime. No PATH, CWD, environment override or fallback.
use anyhow::{Context, Result, bail, ensure};
use std::convert::TryInto;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use winapi::shared::minwindef::{FARPROC, HMODULE};
use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryExW};

const FILES: [(&str, &str); 2] = [
    ("conpty.dll", "39fba2713e2495117b1591ae8c32a3b904bea7aa66069cf7815e2844c76d75d8"),
    ("OpenConsole.exe", "b7fd936c2668b87b9ecf7b3366dc6568afc1c6f981874cba3e955a1c35cf8160"),
];

pub(super) struct Runtime {
    module: HMODULE,
    // Deny write/delete replacement from verification through the last console.
    _files: Vec<File>,
}
unsafe impl Send for Runtime {}
unsafe impl Sync for Runtime {}

fn verify(path: &Path, expected: &str) -> Result<File> {
    let metadata = std::fs::symlink_metadata(path)
        .context("ConPTY runtime unavailable (stage=metadata)")?;
    ensure!(metadata.is_file() && metadata.file_attributes() & 0x400 == 0,
        "ConPTY runtime rejected (stage=regular_file, code=13)");
    let mut file = OpenOptions::new().read(true).share_mode(1).open(path)
        .context("ConPTY runtime unavailable (stage=open)")?;
    ensure!(file.metadata()?.len() <= 16 * 1024 * 1024,
        "ConPTY runtime rejected (stage=size, code=13)");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    ensure!(bytes.get(..2) == Some(b"MZ"), "ConPTY runtime rejected (stage=PE, code=193)");
    let offset = bytes.get(60..64).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let header = offset.and_then(|i| bytes.get(i..i.checked_add(6)?));
    ensure!(header == Some(b"PE\0\0\x64\x86"),
        "ConPTY runtime rejected (stage=architecture, code=193)");
    ensure!(format!("{:x}", Sha256::digest(&bytes)) == expected,
        "ConPTY runtime rejected (stage=sha256, code=13)");
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}

impl Runtime {
    pub(super) fn load() -> Result<Self> {
        ensure!(cfg!(target_arch = "x86_64"), "ConPTY package supports x64 only (code=193)");
        let executable = std::env::current_exe().context("ConPTY executable location unavailable")?;
        let directory = executable.parent().context("ConPTY executable directory unavailable")?;
        let files = FILES.iter().map(|(name, hash)| verify(&directory.join(name), hash))
            .collect::<Result<Vec<_>>>()?;
        let path: Vec<u16> = directory.join("conpty.dll").as_os_str().encode_wide()
            .chain(Some(0)).collect();
        // DLL dependencies can resolve only in the verified module directory or System32.
        let module = unsafe { LoadLibraryExW(path.as_ptr(), std::ptr::null_mut(), 0x100 | 0x800) };
        if module.is_null() {
            bail!("ConPTY runtime unavailable (stage=LoadLibraryExW): {}", std::io::Error::last_os_error());
        }
        Ok(Self { module, _files: files })
    }

    pub(super) unsafe fn symbol(&self, name: &[u8]) -> Result<FARPROC, String> {
        let address = GetProcAddress(self.module, name.as_ptr().cast());
        if address.is_null() {
            Err(format!("ConPTY runtime unavailable (stage=GetProcAddress): {}", std::io::Error::last_os_error()))
        } else { Ok(address) }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) { unsafe { FreeLibrary(self.module); } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_corrupt_and_wrong_architecture_fail_closed() {
        let path = std::env::temp_dir().join(format!("rt-conpty-negative-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(verify(&path, FILES[0].1).is_err());
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(format!("{:#}", verify(&path, FILES[0].1).unwrap_err()).contains("code=193"));
        let mut bytes = vec![0u8; 128];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..70].copy_from_slice(b"PE\0\0\x4c\x01");
        std::fs::write(&path, &bytes).unwrap();
        assert!(format!("{:#}", verify(&path, FILES[0].1).unwrap_err()).contains("architecture"));
        bytes[68..70].copy_from_slice(b"\x64\x86");
        std::fs::write(&path, &bytes).unwrap();
        assert!(format!("{:#}", verify(&path, FILES[0].1).unwrap_err()).contains("sha256"));
        std::fs::remove_file(path).unwrap();
    }
}
