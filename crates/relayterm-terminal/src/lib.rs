//! Bounded, provider-neutral terminal state owned by the daemon.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const DEFAULT_SCROLLBACK_BYTES: usize = 1024 * 1024;
pub const MAX_SCROLLBACK_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SCREEN_CELLS: usize = 160_000;
pub const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CONTROL_SEQUENCE_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalError {
    InvalidSize,
    ResourceLimit,
    CounterOverflow,
    ResnapshotRequired,
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("The terminal state request is invalid or exceeds a resource limit.")
    }
}

impl std::error::Error for TerminalError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb([u8; 3]),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub contents: String,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub wide: bool,
    pub wide_continuation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSnapshot {
    pub schema_version: u8,
    pub revision: u64,
    pub raw_offset: u64,
    pub retained_from_offset: u64,
    pub rows: u16,
    pub columns: u16,
    pub cursor_row: u16,
    pub cursor_column: u16,
    pub cursor_hidden: bool,
    pub alternate_screen: bool,
    pub application_cursor: bool,
    pub application_keypad: bool,
    pub bracketed_paste: bool,
    pub cells: Vec<Cell>,
}

/// Server-parsed replacement snapshots make reconnect independent of a client parser's hidden
/// state. Raw retained bytes are diagnostic history, not the source of reconstruction truth.
pub struct TerminalState {
    parser: vt100::Parser,
    revision: u64,
    raw_offset: u64,
    retained_from_offset: u64,
    retained: VecDeque<u8>,
    retained_limit: usize,
    control_sequence_bytes: usize,
}

impl TerminalState {
    pub fn new(rows: u16, columns: u16, retained_limit: usize) -> Result<Self, TerminalError> {
        validate_size(rows, columns)?;
        if retained_limit == 0 || retained_limit > MAX_SCROLLBACK_BYTES {
            return Err(TerminalError::ResourceLimit);
        }
        Ok(Self {
            parser: vt100::Parser::new(rows, columns, 10_000),
            revision: 1,
            raw_offset: 0,
            retained_from_offset: 0,
            retained: VecDeque::with_capacity(retained_limit.min(64 * 1024)),
            retained_limit,
            control_sequence_bytes: 0,
        })
    }

    pub fn process(&mut self, bytes: &[u8]) -> Result<u64, TerminalError> {
        let amount = u64::try_from(bytes.len()).map_err(|_| TerminalError::CounterOverflow)?;
        self.raw_offset = self
            .raw_offset
            .checked_add(amount)
            .ok_or(TerminalError::CounterOverflow)?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(TerminalError::CounterOverflow)?;
        let mut bounded = Vec::with_capacity(bytes.len().saturating_add(1));
        for byte in bytes {
            if *byte == 0x1b {
                self.control_sequence_bytes = 1;
            } else if self.control_sequence_bytes != 0 {
                self.control_sequence_bytes = self.control_sequence_bytes.saturating_add(1);
                if *byte == 0x07
                    || (self.control_sequence_bytes > 2 && (0x40..=0x7e).contains(byte))
                {
                    self.control_sequence_bytes = 0;
                } else if self.control_sequence_bytes >= MAX_CONTROL_SEQUENCE_BYTES {
                    bounded.push(0x18);
                    self.control_sequence_bytes = 0;
                }
            }
            bounded.push(*byte);
        }
        self.parser.process(&bounded);
        for byte in bytes {
            if self.retained.len() == self.retained_limit {
                self.retained.pop_front();
                self.retained_from_offset = self
                    .retained_from_offset
                    .checked_add(1)
                    .ok_or(TerminalError::CounterOverflow)?;
            }
            self.retained.push_back(*byte);
        }
        Ok(self.revision)
    }

    pub fn resize(&mut self, rows: u16, columns: u16) -> Result<u64, TerminalError> {
        validate_size(rows, columns)?;
        self.parser.screen_mut().set_size(rows, columns);
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(TerminalError::CounterOverflow)?;
        Ok(self.revision)
    }

    pub fn snapshot(&self) -> Result<TerminalSnapshot, TerminalError> {
        let screen = self.parser.screen();
        let (rows, columns) = screen.size();
        let capacity = usize::from(rows)
            .checked_mul(usize::from(columns))
            .ok_or(TerminalError::ResourceLimit)?;
        if capacity > MAX_SCREEN_CELLS {
            return Err(TerminalError::ResourceLimit);
        }
        let mut cells = Vec::with_capacity(capacity);
        for row in 0..rows {
            for column in 0..columns {
                let cell = screen.cell(row, column).ok_or(TerminalError::InvalidSize)?;
                cells.push(Cell {
                    contents: cell.contents().to_owned(),
                    foreground: color(cell.fgcolor()),
                    background: color(cell.bgcolor()),
                    bold: cell.bold(),
                    dim: cell.dim(),
                    italic: cell.italic(),
                    underline: cell.underline(),
                    inverse: cell.inverse(),
                    wide: cell.is_wide(),
                    wide_continuation: cell.is_wide_continuation(),
                });
            }
        }
        let (cursor_row, cursor_column) = screen.cursor_position();
        Ok(TerminalSnapshot {
            schema_version: 1,
            revision: self.revision,
            raw_offset: self.raw_offset,
            retained_from_offset: self.retained_from_offset,
            rows,
            columns,
            cursor_row,
            cursor_column,
            cursor_hidden: screen.hide_cursor(),
            alternate_screen: screen.alternate_screen(),
            application_cursor: screen.application_cursor(),
            application_keypad: screen.application_keypad(),
            bracketed_paste: screen.bracketed_paste(),
            cells,
        })
    }

    pub fn retained_bytes(&self) -> Vec<u8> {
        self.retained.iter().copied().collect()
    }

    pub fn output_since(
        &self,
        after_offset: u64,
        limit: usize,
    ) -> Result<(u64, Vec<u8>), TerminalError> {
        if limit == 0 || limit > 16 * 1024 || after_offset > self.raw_offset {
            return Err(TerminalError::ResourceLimit);
        }
        if after_offset < self.retained_from_offset {
            return Err(TerminalError::ResnapshotRequired);
        }
        let start = usize::try_from(after_offset - self.retained_from_offset)
            .map_err(|_| TerminalError::ResourceLimit)?;
        let data = self
            .retained
            .iter()
            .skip(start)
            .take(limit)
            .copied()
            .collect::<Vec<_>>();
        let next = after_offset
            .checked_add(u64::try_from(data.len()).map_err(|_| TerminalError::CounterOverflow)?)
            .ok_or(TerminalError::CounterOverflow)?;
        Ok((next, data))
    }
}

fn validate_size(rows: u16, columns: u16) -> Result<(), TerminalError> {
    if !(1..=1000).contains(&rows)
        || !(1..=1000).contains(&columns)
        || usize::from(rows) * usize::from(columns) > MAX_SCREEN_CELLS
    {
        return Err(TerminalError::InvalidSize);
    }
    Ok(())
}

fn color(value: vt100::Color) -> Color {
    match value {
        vt100::Color::Default => Color::Default,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(red, green, blue) => Color::Rgb([red, green, blue]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_full_screen_and_continues_after_truncation() {
        let mut terminal = TerminalState::new(3, 8, 8).unwrap();
        for part in [
            b"normal".as_slice(),
            b"\x1b[?1049",
            b"h\x1b[2J",
            "wide界".as_bytes(),
        ] {
            terminal.process(part).unwrap();
        }
        let alternate = terminal.snapshot().unwrap();
        assert!(alternate.alternate_screen);
        assert!(alternate.cells.iter().any(|cell| cell.contents == "界"));
        assert!(alternate.retained_from_offset > 0);

        terminal.process(b"\x1b[?1049l!").unwrap();
        let restored = terminal.snapshot().unwrap();
        assert!(!restored.alternate_screen);
        let visible: String = restored
            .cells
            .iter()
            .map(|cell| cell.contents.as_str())
            .collect();
        assert!(visible.contains("normal!"));
        assert!(restored.revision > alternate.revision);
    }

    #[test]
    fn enforces_cell_and_history_limits() {
        assert!(TerminalState::new(1000, 1000, 1).is_err());
        assert!(TerminalState::new(24, 80, MAX_SCROLLBACK_BYTES + 1).is_err());
        let mut terminal = TerminalState::new(2, 2, 3).unwrap();
        terminal.process(b"abcdef").unwrap();
        assert_eq!(terminal.retained_bytes(), b"def");
    }

    #[test]
    fn split_sequences_match_uninterrupted_state() {
        let bytes = concat!(
            "start\r\n",
            "\x1b[2;3H\x1b[31mred",
            "e\u{301}",
            "\x1b[s\x1b[2;4r\x1b[4;11H界x\x1b[u",
            "\x1b[?1h\x1b[?2004h"
        )
        .as_bytes();
        let mut expected = TerminalState::new(4, 12, 128).unwrap();
        expected.process(bytes).unwrap();
        let expected = expected.snapshot().unwrap();
        for split in 0..=bytes.len() {
            let mut candidate = TerminalState::new(4, 12, 128).unwrap();
            candidate.process(&bytes[..split]).unwrap();
            candidate.process(&bytes[split..]).unwrap();
            let candidate = candidate.snapshot().unwrap();
            assert_eq!(candidate.cells, expected.cells, "split at {split}");
            assert_eq!(candidate.cursor_row, expected.cursor_row);
            assert_eq!(candidate.cursor_column, expected.cursor_column);
            assert_eq!(candidate.bracketed_paste, expected.bracketed_paste);
            assert_eq!(candidate.application_cursor, expected.application_cursor);
        }
    }

    #[test]
    fn unterminated_controls_are_cancelled_at_the_bound() {
        let mut terminal = TerminalState::new(2, 10, 32).unwrap();
        let mut hostile = b"\x1b]".to_vec();
        hostile.extend(std::iter::repeat_n(b'a', MAX_CONTROL_SEQUENCE_BYTES + 20));
        hostile.extend_from_slice(b"visible");
        terminal.process(&hostile).unwrap();
        terminal.process(b"\x07safe").unwrap();
        let visible: String = terminal
            .snapshot()
            .unwrap()
            .cells
            .iter()
            .map(|cell| cell.contents.as_str())
            .collect();
        assert!(visible.contains("safe"));
    }

    #[test]
    fn output_cursor_detects_truncation_and_accepts_zero() {
        let mut terminal = TerminalState::new(2, 2, 4).unwrap();
        terminal.process(b"abcdef").unwrap();
        assert_eq!(
            terminal.output_since(0, 4),
            Err(TerminalError::ResnapshotRequired)
        );
        assert_eq!(terminal.output_since(2, 2).unwrap(), (4, b"cd".to_vec()));
        assert_eq!(terminal.output_since(6, 2).unwrap(), (6, Vec::new()));
    }
}
