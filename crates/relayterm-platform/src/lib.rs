//! Native private locations, path identity, locking, clock, and identifiers.

mod identity;
mod locations;
mod locking;
mod permissions;

pub use identity::*;
pub use locations::*;
pub use locking::*;
pub use permissions::*;

use relayterm_application::{Clock, IdGenerator};
use relayterm_domain::{Error as DomainError, EventId, Timestamp};
use std::sync::atomic::{AtomicU64, Ordering};
use time::OffsetDateTime;
use uuid::Uuid;

/// Production UTC clock used at the application boundary.
#[derive(Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Result<Timestamp, DomainError> {
        Timestamp::new(OffsetDateTime::now_utc())
    }
}

/// Collision-resistant production identifier source.
pub struct RandomIdGenerator {
    counter: AtomicU64,
}

impl Default for RandomIdGenerator {
    fn default() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl IdGenerator for RandomIdGenerator {
    fn next(&self) -> Result<EventId, DomainError> {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let now = OffsetDateTime::now_utc()
            .unix_timestamp_nanos()
            .to_be_bytes();
        let mut input = [0_u8; 24];
        input[..16].copy_from_slice(&now);
        input[16..].copy_from_slice(&counter.to_be_bytes());
        let mut bytes = *blake3::hash(&input).as_bytes();
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(EventId::from_uuid(Uuid::from_bytes(
            bytes[..16].try_into().expect("fixed length"),
        )))
    }
}
