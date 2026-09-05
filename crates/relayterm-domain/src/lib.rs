//! Provider-neutral identifiers. Domain state machines follow in M02.

use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! identifier {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(Uuid);

        impl $name {
            /// Wrap an explicitly supplied UUID without generating runtime state.
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Return the underlying UUID for explicit boundary conversions.
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.parse().map(Self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

identifier!(WorkspaceId, "An opaque workspace identity.");
identifier!(TaskId, "An opaque task identity.");

/// Workspace and task IDs cannot be substituted for each other.
///
/// ```compile_fail
/// use relayterm_domain::{TaskId, WorkspaceId};
/// let workspace: WorkspaceId = "00000000-0000-4000-8000-000000000001".parse().unwrap();
/// let task: TaskId = workspace;
/// ```
pub mod type_safety {}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "00000000-0000-4000-8000-000000000001";

    #[test]
    fn identifiers_round_trip() {
        let workspace: WorkspaceId = ID.parse().unwrap();
        let task: TaskId = ID.parse().unwrap();
        assert_eq!(workspace.to_string(), ID);
        assert_eq!(task.to_string(), ID);
        assert_eq!(WorkspaceId::from_uuid(workspace.as_uuid()), workspace);
        assert_eq!(TaskId::from_uuid(task.as_uuid()), task);
    }

    #[test]
    fn malformed_identifiers_are_rejected() {
        for invalid in ["", "not-an-id", "00000000-0000-4000-8000-00000000000z"] {
            assert!(invalid.parse::<WorkspaceId>().is_err());
            assert!(invalid.parse::<TaskId>().is_err());
        }
    }
}
