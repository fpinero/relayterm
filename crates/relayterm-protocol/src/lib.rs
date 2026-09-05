//! Version compatibility only. Framing and message schemas belong to M04.

use std::fmt;

/// Initial local protocol version. No stable wire format is published yet.
pub const PROTOCOL_VERSION: u16 = 1;

/// The peer and this implementation require different protocol versions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncompatibleVersion {
    pub expected: u16,
    pub received: u16,
}

impl fmt::Display for IncompatibleVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Unsupported protocol version {}; expected {}. Use matching client and daemon versions.",
            self.received, self.expected
        )
    }
}

impl std::error::Error for IncompatibleVersion {}

/// Validate a peer version without opening a transport.
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
    fn accepts_only_the_current_version() {
        for received in 0..=u16::MAX {
            match check_version(received) {
                Ok(()) => assert_eq!(received, PROTOCOL_VERSION),
                Err(error) => {
                    assert_ne!(received, PROTOCOL_VERSION);
                    assert_eq!(error.expected, PROTOCOL_VERSION);
                    assert_eq!(error.received, received);
                }
            }
        }
    }
}
