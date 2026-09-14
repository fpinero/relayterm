use fs4::FileExt;
use std::{fs::File, path::PathBuf};

/// Hold an operating-system lock while a native terminal scenario is running.
///
/// Cargo can execute integration tests in parallel, including source files that
/// are retained inside a larger gate. Serializing these scenarios prevents a
/// Windows console-control event from one real ConPTY client from interrupting
/// an unrelated `rt` process in another scenario.
pub struct NativeSerialGuard(File);

impl NativeSerialGuard {
    pub fn acquire() -> Self {
        let path = lock_path();
        let file = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .expect("native gate lock file could not be opened");
        FileExt::lock(&file).expect("native gate lock could not be acquired");
        Self(file)
    }
}

impl Drop for NativeSerialGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

fn lock_path() -> PathBuf {
    std::env::temp_dir().join("relayterm-native-gate.lock")
}
