use crate::StorageError;
use relayterm_domain::Timestamp;

pub fn encode_counter(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

pub fn decode_counter(bytes: &[u8]) -> Result<u64, StorageError> {
    Ok(u64::from_be_bytes(
        bytes.try_into().map_err(|_| StorageError::Integrity)?,
    ))
}

pub fn encode_timestamp(value: Timestamp) -> (i64, i64) {
    let nanos: i128 = value.into();
    let seconds = nanos.div_euclid(1_000_000_000);
    let subsecond = nanos.rem_euclid(1_000_000_000);
    (
        i64::try_from(seconds).expect("domain timestamp seconds fit i64"),
        i64::try_from(subsecond).expect("subsecond fits i64"),
    )
}

pub fn decode_timestamp(seconds: i64, nanoseconds: i64) -> Result<Timestamp, StorageError> {
    if !(0..1_000_000_000).contains(&nanoseconds) {
        return Err(StorageError::Integrity);
    }
    let value = i128::from(seconds) * 1_000_000_000 + i128::from(nanoseconds);
    Timestamp::try_from(value).map_err(|_| StorageError::Integrity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_preserve_unsigned_order_and_boundaries() {
        let values = [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];
        for value in values {
            assert_eq!(decode_counter(&encode_counter(value)).unwrap(), value);
        }
        assert!(
            values
                .windows(2)
                .all(|pair| encode_counter(pair[0]) < encode_counter(pair[1]))
        );
    }

    #[test]
    fn timestamps_preserve_negative_nanoseconds() {
        for nanos in [-1_000_000_001_i128, -1, 0, 1, 1_000_000_001] {
            let timestamp = Timestamp::try_from(nanos).unwrap();
            let (seconds, subsecond) = encode_timestamp(timestamp);
            assert_eq!(decode_timestamp(seconds, subsecond).unwrap(), timestamp);
        }
    }
}
