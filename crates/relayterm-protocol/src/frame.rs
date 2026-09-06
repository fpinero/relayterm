use crate::{JSON_FRAME_LIMIT, PROTOCOL_VERSION, TERMINAL_FRAME_LIMIT, check_version};
use std::fmt;

pub const HEADER_SIZE: usize = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameKind {
    Json = 0x01,
    Terminal = 0x02,
}

impl TryFrom<u8> for FrameKind {
    type Error = FrameError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Json),
            2 => Ok(Self::Terminal),
            _ => Err(FrameError::UnsupportedKind),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Frame {
    pub kind: FrameKind,
    pub payload: Vec<u8>,
}
impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("kind", &self.kind)
            .field("payload_bytes", &self.payload.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    EmptyControl,
    Oversized,
    UnsupportedKind,
    UnsupportedVersion,
    Truncated,
    BufferLimit,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyControl => "The control frame is empty.",
            Self::Oversized => "The frame exceeds its resource limit.",
            Self::UnsupportedKind => "The frame kind is unsupported.",
            Self::UnsupportedVersion => "The protocol version is unsupported.",
            Self::Truncated => "The transport ended inside a frame.",
            Self::BufferLimit => "The connection buffer limit was exceeded.",
        })
    }
}
impl std::error::Error for FrameError {}

fn limit(kind: FrameKind) -> usize {
    match kind {
        FrameKind::Json => JSON_FRAME_LIMIT,
        FrameKind::Terminal => TERMINAL_FRAME_LIMIT,
    }
}

pub fn encode_frame(kind: FrameKind, payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    if kind == FrameKind::Json && payload.is_empty() {
        return Err(FrameError::EmptyControl);
    }
    if payload.len() > limit(kind) || payload.len() > u32::MAX as usize {
        return Err(FrameError::Oversized);
    }
    if kind == FrameKind::Terminal && payload.len() <= crate::TERMINAL_METADATA_SIZE {
        return Err(FrameError::Oversized);
    }
    let mut output = Vec::with_capacity(HEADER_SIZE + payload.len());
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    output.push(kind as u8);
    output.extend_from_slice(payload);
    Ok(output)
}

pub struct FrameDecoder {
    bytes: Vec<u8>,
    buffer_limit: usize,
}
impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new(crate::CONNECTION_BUFFER_LIMIT)
    }
}
impl FrameDecoder {
    pub fn new(buffer_limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            buffer_limit,
        }
    }
    pub fn feed(&mut self, input: &[u8]) -> Result<Vec<Frame>, FrameError> {
        let total = self
            .bytes
            .len()
            .checked_add(input.len())
            .ok_or(FrameError::BufferLimit)?;
        if total > self.buffer_limit {
            return Err(FrameError::BufferLimit);
        }
        self.bytes.extend_from_slice(input);
        let mut frames = Vec::new();
        let mut consumed = 0;
        while self.bytes.len().saturating_sub(consumed) >= HEADER_SIZE {
            let header = &self.bytes[consumed..consumed + HEADER_SIZE];
            let length = u32::from_be_bytes(header[..4].try_into().expect("fixed header")) as usize;
            if check_version(u16::from_be_bytes([header[4], header[5]])).is_err() {
                return Err(FrameError::UnsupportedVersion);
            }
            let kind = FrameKind::try_from(header[6])?;
            if length > limit(kind) {
                return Err(FrameError::Oversized);
            }
            if kind == FrameKind::Terminal && length <= crate::TERMINAL_METADATA_SIZE {
                return Err(FrameError::Oversized);
            }
            if kind == FrameKind::Json && length == 0 {
                return Err(FrameError::EmptyControl);
            }
            let end = consumed
                .checked_add(HEADER_SIZE)
                .and_then(|v| v.checked_add(length))
                .ok_or(FrameError::Oversized)?;
            if end > self.bytes.len() {
                break;
            }
            frames.push(Frame {
                kind,
                payload: self.bytes[consumed + HEADER_SIZE..end].to_vec(),
            });
            consumed = end;
        }
        if consumed != 0 {
            self.bytes.drain(..consumed);
        }
        Ok(frames)
    }
    pub fn finish(self) -> Result<(), FrameError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(FrameError::Truncated)
        }
    }
    pub fn buffered_len(&self) -> usize {
        self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_and_coalesced_frames_decode() {
        let first = encode_frame(FrameKind::Json, br#"{"type":"one"}"#).unwrap();
        let second =
            encode_frame(FrameKind::Terminal, &[7; crate::TERMINAL_METADATA_SIZE + 1]).unwrap();
        for split in 0..=first.len() {
            let mut decoder = FrameDecoder::default();
            let mut frames = decoder.feed(&first[..split]).unwrap();
            frames.extend(decoder.feed(&first[split..]).unwrap());
            assert_eq!(frames.len(), 1);
            decoder.finish().unwrap();
        }
        let mut combined = first;
        combined.extend(second);
        assert_eq!(FrameDecoder::default().feed(&combined).unwrap().len(), 2);
    }
    #[test]
    fn rejects_header_failures_before_payload() {
        let cases = [
            ([0, 0, 0, 0, 0, 1, 1], FrameError::EmptyControl),
            ([0, 0, 0, 1, 0, 2, 1], FrameError::UnsupportedVersion),
            ([0, 0, 0, 1, 0, 1, 9], FrameError::UnsupportedKind),
        ];
        for (header, expected) in cases {
            assert_eq!(FrameDecoder::default().feed(&header), Err(expected));
        }
        let mut header = [0_u8; HEADER_SIZE];
        header[..4].copy_from_slice(&((JSON_FRAME_LIMIT + 1) as u32).to_be_bytes());
        header[5] = 1;
        header[6] = 1;
        assert_eq!(
            FrameDecoder::default().feed(&header),
            Err(FrameError::Oversized)
        );
    }
    #[test]
    fn eof_reports_partial_frame() {
        let bytes = encode_frame(FrameKind::Json, b"{}").unwrap();
        let mut decoder = FrameDecoder::default();
        assert!(decoder.feed(&bytes[..bytes.len() - 1]).unwrap().is_empty());
        assert_eq!(decoder.finish(), Err(FrameError::Truncated));
    }

    #[test]
    fn exact_frame_limits_are_accepted_and_one_byte_over_is_rejected() {
        assert!(encode_frame(FrameKind::Json, &vec![b'x'; JSON_FRAME_LIMIT]).is_ok());
        assert_eq!(
            encode_frame(FrameKind::Json, &vec![b'x'; JSON_FRAME_LIMIT + 1]),
            Err(FrameError::Oversized)
        );
        assert!(encode_frame(FrameKind::Terminal, &vec![0; crate::TERMINAL_FRAME_LIMIT]).is_ok());
        assert_eq!(
            encode_frame(
                FrameKind::Terminal,
                &vec![0; crate::TERMINAL_FRAME_LIMIT + 1]
            ),
            Err(FrameError::Oversized)
        );
    }
}
