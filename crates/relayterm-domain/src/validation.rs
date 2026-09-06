use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Component, Path},
};
use time::{OffsetDateTime, UtcOffset};

/// Safe categories never contain input values or adapter diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Validation(&'static str),
    State,
    Unauthorized,
    Reference,
    Conflict,
    Time,
    Storage,
    StorageBusy,
    ReadOnly,
    Integrity,
    Migration,
    InvalidCursor,
    ResnapshotRequired,
    Uncertain,
    Unavailable,
    Version,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// UTC timestamp serialized as a checked Unix nanosecond count.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "i128", into = "i128")]
pub struct Timestamp(OffsetDateTime);
impl Timestamp {
    pub fn new(value: OffsetDateTime) -> Result<Self> {
        value
            .checked_to_offset(UtcOffset::UTC)
            .map(Self)
            .ok_or(Error::Time)
    }
    pub fn value(self) -> OffsetDateTime {
        self.0
    }
    pub fn not_before(self, earlier: Self) -> Result<()> {
        if self < earlier {
            Err(Error::Time)
        } else {
            Ok(())
        }
    }
}
impl TryFrom<i128> for Timestamp {
    type Error = Error;
    fn try_from(n: i128) -> Result<Self> {
        OffsetDateTime::from_unix_timestamp_nanos(n)
            .map(Self)
            .map_err(|_| Error::Time)
    }
}
impl From<Timestamp> for i128 {
    fn from(t: Timestamp) -> Self {
        t.0.unix_timestamp_nanos()
    }
}

pub(crate) fn text(
    value: &str,
    field: &'static str,
    max: usize,
    required: bool,
    multiline: bool,
) -> Result<()> {
    if value.len() > max
        || (required && value.trim().is_empty())
        || value
            .chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\t')))
    {
        return Err(Error::Validation(field));
    }
    // Heuristics intentionally cannot identify arbitrary secrets.
    if [
        "ghp_",
        "github_pat_",
        "sk-proj-",
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "AKIA",
    ]
    .iter()
    .any(|p| value.contains(p))
    {
        return Err(Error::Validation(field));
    }
    Ok(())
}
pub(crate) fn set<T: PartialEq>(values: &[T], field: &'static str) -> Result<()> {
    if values.len() > 128
        || values
            .iter()
            .enumerate()
            .any(|(i, x)| values[..i].contains(x))
    {
        Err(Error::Validation(field))
    } else {
        Ok(())
    }
}
pub(crate) fn native_path(value: &Path, field: &'static str) -> Result<()> {
    if value.as_os_str().is_empty()
        || value
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(Error::Validation(field));
    }
    text(&value.to_string_lossy(), field, 4096, true, false)
}
pub(crate) fn paths(values: &[String], field: &'static str) -> Result<()> {
    set(values, field)?;
    for value in values {
        text(value, field, 4096, true, false)?;
        // Reject both Windows and Unix escape syntax on every host.
        if value.starts_with(['/', '\\'])
            || value.contains(':')
            || value.split(['/', '\\']).any(|c| c == "..")
        {
            return Err(Error::Validation(field));
        }
    }
    Ok(())
}
