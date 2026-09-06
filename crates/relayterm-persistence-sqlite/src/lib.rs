//! Transactional SQLite persistence with strict, validated reconstruction.

mod codecs;
mod connection;
mod registry;
mod store;

pub use codecs::*;
pub use connection::*;
pub use registry::*;
pub use store::*;

pub const MINIMUM_SQLITE_VERSION: (u32, u32, u32) = (3, 51, 3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    NotFound,
    AlreadyExists,
    AccessDenied,
    Busy,
    ReadOnly,
    IncompatibleVersion,
    Integrity,
    Migration,
    Unavailable,
}
