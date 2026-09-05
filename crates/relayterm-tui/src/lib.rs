//! Separate TUI entry boundary. Runtime implementation belongs to later milestones.

use std::fmt;

/// The bootstrap does not implement this runtime yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotImplemented;

impl fmt::Display for NotImplemented {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("The TUI is not implemented yet. See the MVP roadmap in TODO.md.")
    }
}

impl std::error::Error for NotImplemented {}

/// Report unavailable behavior without creating processes, files, or terminals.
pub fn run() -> Result<(), NotImplemented> {
    Err(NotImplemented)
}
