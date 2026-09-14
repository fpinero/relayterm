use crate::{
    model::{Form, FormField},
    safe_text,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub struct EditorLayout {
    pub lines: Vec<String>,
    pub cursor_column: u16,
    pub cursor_row: u16,
}

pub fn editor_layout(form: &Form, width: u16, height: u16) -> EditorLayout {
    let width = usize::from(width.max(1));
    let height = usize::from(height.max(1));
    let mut lines = Vec::new();
    if let Some(guidance) = &form.guidance {
        lines.extend(wrap_display(&safe_text::single_line(guidance, 512), width));
    }
    let mut cursor = (0_usize, 0_usize);
    for (index, field) in form.fields.iter().enumerate() {
        lines.extend(wrap_display(
            &format!(
                "{} {} ({}/{})",
                if index == form.selected { ">" } else { " " },
                field.label,
                field.value.len(),
                field.limit
            ),
            width,
        ));
        let base_row = lines.len();
        let (mut value_lines, field_cursor) = field_layout(field, width);
        if index == form.selected {
            cursor = (base_row + field_cursor.0, field_cursor.1);
        }
        lines.append(&mut value_lines);
    }
    if let Some(error) = &form.error {
        lines.extend(wrap_display(
            &format!("Error: {}", safe_text::single_line(error, 512)),
            width,
        ));
    }
    lines.extend(wrap_display(
        "Ctrl-S submit, Ctrl-U clear field, Esc cancel, Tab next field, Ctrl-R reconcile",
        width,
    ));
    while lines.len() <= cursor.0 {
        lines.push(String::new());
    }
    let maximum_scroll = lines.len().saturating_sub(height);
    let scroll = cursor.0.saturating_sub(height - 1).min(maximum_scroll);
    EditorLayout {
        lines: lines.into_iter().skip(scroll).take(height).collect(),
        cursor_column: cursor.1.min(width - 1) as u16,
        cursor_row: cursor.0.saturating_sub(scroll).min(height - 1) as u16,
    }
}

fn field_layout(field: &FormField, width: usize) -> (Vec<String>, (usize, usize)) {
    let display = sanitize_field(&field.value, field.multiline, field.limit);
    let prefix = sanitize_field(&field.value[..field.cursor], field.multiline, field.limit);
    let mut lines = wrap_display(&display, width);
    let cursor = display_position(&prefix, width);
    while lines.len() <= cursor.0 {
        lines.push(String::new());
    }
    (lines, cursor)
}

fn sanitize_field(value: &str, multiline: bool, limit: usize) -> String {
    let value = if multiline {
        safe_text::narrative(value, limit)
    } else {
        safe_text::single_line(value, limit)
    };
    value.replace('\t', "    ")
}

fn wrap_display(value: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut column = 0_usize;
    for grapheme in value.graphemes(true) {
        if grapheme == "\n" {
            lines.push(String::new());
            column = 0;
            continue;
        }
        let cells = UnicodeWidthStr::width(grapheme);
        if column != 0 && column.saturating_add(cells) > width {
            lines.push(String::new());
            column = 0;
        }
        lines
            .last_mut()
            .expect("one line exists")
            .push_str(grapheme);
        column = column.saturating_add(cells);
    }
    lines
}

fn display_position(value: &str, width: usize) -> (usize, usize) {
    let mut row = 0_usize;
    let mut column = 0_usize;
    for grapheme in value.graphemes(true) {
        if grapheme == "\n" {
            row += 1;
            column = 0;
            continue;
        }
        let cells = UnicodeWidthStr::width(grapheme);
        if column != 0 && column.saturating_add(cells) > width {
            row += 1;
            column = 0;
        }
        column = column.saturating_add(cells);
    }
    if column == width {
        (row + 1, 0)
    } else {
        (row, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Form;

    fn layout(value: &str, cursor: usize, width: u16, height: u16) -> EditorLayout {
        let mut form = Form::session_rename(value);
        form.fields[0].cursor = cursor;
        editor_layout(&form, width, height)
    }

    #[test]
    fn cursor_uses_cells_for_wide_and_combining_graphemes() {
        let wide = layout("a界b", "a界".len(), 10, 8);
        assert_eq!(wide.cursor_column, 3);
        let combining = layout("e\u{301}x", "e\u{301}".len(), 10, 8);
        assert_eq!(combining.cursor_column, 1);
    }

    #[test]
    fn cursor_handles_empty_end_wrap_and_trailing_newline() {
        assert_eq!(display_position("", 4), (0, 0));
        assert_eq!(display_position("abcd", 4), (1, 0));
        assert_eq!(display_position("abc\n", 10), (1, 0));
        let wrapped = layout("abcd", 4, 4, 8);
        assert!(wrapped.cursor_row < 8);
        assert_eq!(wrapped.cursor_column, 0);
    }

    #[test]
    fn active_cursor_is_vertically_clipped_into_the_viewport() {
        let value = "line\n".repeat(20);
        let visible = layout(&value, value.len(), 8, 4);
        assert_eq!(visible.lines.len(), 4);
        assert_eq!(visible.cursor_row, 3);
        assert!(visible.cursor_column < 8);
    }
}
