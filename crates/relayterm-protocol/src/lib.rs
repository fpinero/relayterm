//! Pure versioned wire contracts for Relayterm local IPC.

mod frame;
mod message;

pub use frame::*;
pub use message::*;

use std::fmt;

pub const PROTOCOL_VERSION: u16 = 1;
pub const JSON_FRAME_LIMIT: usize = 8 * 1024 * 1024;
pub const HELLO_FRAME_LIMIT: usize = 4 * 1024;
pub const TERMINAL_DATA_LIMIT: usize = 16 * 1024;
pub const TERMINAL_METADATA_SIZE: usize = 32;
pub const TERMINAL_FRAME_LIMIT: usize = TERMINAL_METADATA_SIZE + TERMINAL_DATA_LIMIT;
pub const CONNECTION_BUFFER_LIMIT: usize = 16 * 1024 * 1024;
pub const SNAPSHOT_STAGING_LIMIT: usize = 64 * 1024 * 1024;
pub const COLLECTION_PAGE_BYTES: usize = 6 * 1024 * 1024;
pub const DEFAULT_PAGE_SIZE: u16 = 50;
pub const MAX_PAGE_SIZE: u16 = 200;
pub const MAX_JSON_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncompatibleVersion {
    pub expected: u16,
    pub received: u16,
}

impl fmt::Display for IncompatibleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Unsupported protocol version. Use matching client and daemon versions.")
    }
}
impl std::error::Error for IncompatibleVersion {}

pub fn check_version(received: u16) -> Result<(), IncompatibleVersion> {
    if received == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(IncompatibleVersion {
            expected: PROTOCOL_VERSION,
            received,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_current_version_without_echoing_input() {
        for received in 0..=u16::MAX {
            match check_version(received) {
                Ok(()) => assert_eq!(received, PROTOCOL_VERSION),
                Err(error) => assert!(!error.to_string().contains(&received.to_string())),
            }
        }
    }
}
