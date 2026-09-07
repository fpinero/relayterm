pub fn single_line(value: &str, max_bytes: usize) -> String {
    sanitize(value, max_bytes, false)
}

pub fn narrative(value: &str, max_bytes: usize) -> String {
    sanitize(value, max_bytes, true)
}

fn sanitize(value: &str, max_bytes: usize, multiline: bool) -> String {
    let mut output = String::new();
    for character in value.chars() {
        let safe = if character == '\n' && multiline {
            '\n'
        } else if character == '\t' && multiline {
            '\t'
        } else if character.is_control() || is_unsafe_bidi(character) {
            '\u{fffd}'
        } else {
            character
        };
        if output.len().saturating_add(safe.len_utf8()) > max_bytes {
            break;
        }
        output.push(safe);
    }
    output
}

fn is_unsafe_bidi(value: char) -> bool {
    matches!(value as u32, 0x061c | 0x200e | 0x200f | 0x202a..=0x202e | 0x2066..=0x2069)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_controls_and_bidi_without_splitting_unicode() {
        assert_eq!(single_line("safe\u{1b}[2J\u{202e}界", 64), "safe�[2J�界");
        assert_eq!(single_line("界界", 4), "界");
        assert_eq!(narrative("a\nb\t", 8), "a\nb\t");
    }
}
