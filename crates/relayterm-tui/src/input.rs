use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    let modifier = key.modifiers;
    if modifier.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char(character) if character.is_ascii() => {
                let upper = character.to_ascii_uppercase() as u8;
                Some(vec![upper & 0x1f])
            }
            KeyCode::Left => Some(b"\x1b[1;5D".to_vec()),
            KeyCode::Right => Some(b"\x1b[1;5C".to_vec()),
            KeyCode::Up => Some(b"\x1b[1;5A".to_vec()),
            KeyCode::Down => Some(b"\x1b[1;5B".to_vec()),
            _ => None,
        };
    }
    let prefix = if modifier.contains(KeyModifiers::ALT) {
        b"\x1b".as_slice()
    } else {
        b"".as_slice()
    };
    let body: Vec<u8> = match key.code {
        KeyCode::Char(character) => character.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => cursor(application_cursor, b'A'),
        KeyCode::Down => cursor(application_cursor, b'B'),
        KeyCode::Right => cursor(application_cursor, b'C'),
        KeyCode::Left => cursor(application_cursor, b'D'),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::F(number) if (1..=4).contains(&number) => vec![0x1b, b'O', b'P' + number - 1],
        KeyCode::F(number) if (5..=12).contains(&number) => format!(
            "\x1b[{}~",
            [15, 17, 18, 19, 20, 21, 23, 24][usize::from(number - 5)]
        )
        .into_bytes(),
        _ => return None,
    };
    Some([prefix, body.as_slice()].concat())
}

pub fn encode_paste(value: &str, bracketed: bool, limit: usize) -> Option<Vec<u8>> {
    let wrapper = if bracketed { 12 } else { 0 };
    if value.is_empty() || value.len().checked_add(wrapper)? > limit {
        return None;
    }
    if bracketed {
        Some(
            [
                b"\x1b[200~".as_slice(),
                value.as_bytes(),
                b"\x1b[201~".as_slice(),
            ]
            .concat(),
        )
    } else {
        Some(value.as_bytes().to_vec())
    }
}

fn cursor(application: bool, final_byte: u8) -> Vec<u8> {
    vec![0x1b, if application { b'O' } else { b'[' }, final_byte]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn encodes_navigation_and_control_keys() {
        assert_eq!(
            encode_key(key(KeyCode::Up, KeyModifiers::NONE), false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_key(key(KeyCode::Up, KeyModifiers::NONE), true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            encode_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            Some(vec![3])
        );
        assert_eq!(
            encode_key(key(KeyCode::Char('界'), KeyModifiers::NONE), false),
            Some("界".as_bytes().to_vec())
        );
    }

    #[test]
    fn paste_is_atomic_and_bounded() {
        assert_eq!(encode_paste("x", true, 13).unwrap(), b"\x1b[200~x\x1b[201~");
        assert!(encode_paste("xx", true, 13).is_none());
    }
}
