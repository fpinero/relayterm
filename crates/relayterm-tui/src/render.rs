use crate::{
    model::{App, Screen},
    safe_text,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use serde_json::Value;
use std::time::Duration;

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    if app.small() {
        frame.render_widget(
            Paragraph::new(format!(
                "Relayterm needs at least {}x{}. Current size: {}x{}.\n\nResize the terminal, press ? for help, or q to exit.",
                crate::model::MIN_COLUMNS,
                crate::model::MIN_ROWS,
                area.width,
                area.height
            ))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Relayterm")),
            area,
        );
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);
    draw_tabs(frame, app, rows[0]);
    match app.screen {
        Screen::Overview => draw_overview(frame, app, rows[1]),
        Screen::Tasks => draw_tasks(frame, app, rows[1]),
        Screen::Sessions => draw_sessions(frame, app, rows[1]),
        Screen::Agents => draw_agents(frame, app, rows[1]),
        Screen::Events => draw_events(frame, app, rows[1]),
        Screen::Help => draw_help(frame, rows[1]),
    }
    let status = if app
        .terminal
        .as_ref()
        .is_some_and(|terminal| terminal.input_focus)
    {
        "Terminal input: Ctrl-] returns to Relayterm"
    } else {
        "Tab/1-6 screen  j/k move  Enter open  ? help  q exit"
    };
    frame.render_widget(Paragraph::new(status), rows[2]);
    if let Some(form) = &app.form {
        draw_form(frame, form, centered(area, 76, 80));
    }
    if app.confirm_quit {
        let dialog = Paragraph::new(
            "Unsaved content exists. Press y to discard it and exit, or n to continue.",
        )
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Discard draft?"),
        );
        let area = centered(area, 60, 20);
        frame.render_widget(Clear, area);
        frame.render_widget(dialog, area);
    }
    if app.confirm_discard {
        let dialog = Paragraph::new(
            "This draft has not been submitted. Press y to discard it, or n to continue editing.",
        )
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Discard draft?"),
        );
        let area = centered(area, 60, 20);
        frame.render_widget(Clear, area);
        frame.render_widget(dialog, area);
    }
}

fn draw_tabs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut spans = vec![Span::styled(
        " Relayterm ",
        Style::default().add_modifier(Modifier::BOLD),
    )];
    for (index, screen) in Screen::ALL.iter().enumerate() {
        let style = if *screen == app.screen {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        spans.push(Span::styled(
            format!(" {}:{} ", index + 1, screen.label()),
            style,
        ));
    }
    spans.push(Span::raw(format!("  [{}]", app.freshness.label())));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn draw_overview(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let tasks = app.collection("tasks");
    let instances = app.collection("instances");
    let active = tasks
        .iter()
        .filter(|item| field(item, "status") == "active")
        .count();
    let running = instances
        .iter()
        .filter(|item| field(item, "status") == "running")
        .count();
    let text = format!(
        "Workspace revision: {}\nTasks: {} ({} active)\nSessions: {} ({} running)\nAgent definitions: {}\n\nPress 2 for tasks or 3 for sessions.",
        app.last_revision,
        tasks.len(),
        active,
        instances.len(),
        running,
        app.collection("definitions").len()
    );
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Workspace overview"),
        ),
        area,
    );
}

fn draw_tasks(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);
    let items = app
        .collection("tasks")
        .iter()
        .enumerate()
        .map(|(index, task)| {
            let marker = if index == app.selected_task { ">" } else { " " };
            let title = task
                .pointer("/content/title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled");
            let owner = task
                .get("claimed_by_instance_id")
                .and_then(Value::as_str)
                .unwrap_or("unclaimed");
            ListItem::new(safe_text::single_line(
                &format!("{marker} [{}] {title} ({owner})", field(task, "status")),
                512,
            ))
        });
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("Tasks")),
        panes[0],
    );
    let detail = app
        .selected_task()
        .map(|task| task_detail(app, task))
        .unwrap_or_else(|| "No tasks. Press n to create one.".into());
    frame.render_widget(
        Paragraph::new(detail)
            .wrap(Wrap { trim: false })
            .scroll((app.task_scroll, 0))
            .block(Block::default().borders(Borders::ALL).title("Task detail")),
        panes[1],
    );
}

fn task_detail(app: &App, task: &Value) -> String {
    let content = task.get("content").unwrap_or(&Value::Null);
    let task_id = field(task, "id");
    let mut result = format!(
        "{}\n\nStatus: {}\nPriority: {}\nOwner: {}\nDependencies: {}\nScope paths: {}\n\n{}\n\nAcceptance:\n{}",
        safe_text::single_line(
            content
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled"),
            256
        ),
        field(task, "status"),
        field(content, "priority"),
        task.get("claimed_by_instance_id")
            .and_then(Value::as_str)
            .unwrap_or("unclaimed"),
        safe_list(content, "dependency_ids"),
        safe_list(content, "scope_paths"),
        safe_text::narrative(
            content
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or(""),
            16 * 1024
        ),
        safe_text::narrative(
            content
                .get("acceptance_notes")
                .and_then(Value::as_str)
                .unwrap_or(""),
            16 * 1024
        )
    );
    result.push_str("\n\nClaim history:");
    for claim in app
        .collection("claims")
        .iter()
        .filter(|item| field(item, "task_id") == task_id)
    {
        result.push_str(&format!(
            "\n  {} requested by {}, closed: {} ({})",
            safe_text::single_line(field(claim, "instance_id"), 64),
            actor_label(claim.get("requested_by")),
            actor_label(claim.get("closed_by")),
            claim
                .get("close_reason")
                .and_then(Value::as_str)
                .unwrap_or("open")
        ));
    }
    result.push_str("\n\nProgress:");
    for progress in app
        .collection("progress")
        .iter()
        .filter(|item| field(item, "task_id") == task_id)
    {
        result.push_str(&format!(
            "\n\nSummary:\n{}\nVerification:\n{}",
            safe_text::narrative(field(progress, "summary"), 8 * 1024),
            safe_text::narrative(field(progress, "verification"), 8 * 1024)
        ));
    }
    result.push_str("\n\nHandovers:");
    for handover in app
        .collection("handovers")
        .iter()
        .filter(|item| field(item, "task_id") == task_id)
    {
        let value = handover.get("content").unwrap_or(&Value::Null);
        result.push_str(&format!(
            "\n\nSummary:\n{}\nDecisions:\n{}\nChanged paths: {}\nVerification:\n{}\nOpen questions:\n{}\nNext action:\n{}",
            safe_text::narrative(field(value, "summary"), 8 * 1024),
            safe_text::narrative(field(value, "decisions"), 16 * 1024),
            safe_list(value, "changed_paths"),
            safe_text::narrative(field(value, "verification_performed"), 8 * 1024),
            safe_text::narrative(field(value, "open_questions"), 16 * 1024),
            safe_text::narrative(field(value, "recommended_next_action"), 8 * 1024),
        ));
    }
    result.push_str("\n\nPageUp/PageDown: history. Actions: n create, e edit, r ready/release, c claim, b block, d done, x cancel, p progress, h handover");
    result
}

fn safe_list(value: &Value, name: &str) -> String {
    value
        .get(name)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| safe_text::single_line(item, 4096))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn actor_label(value: Option<&Value>) -> &'static str {
    match value
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
    {
        Some("local_user") => "local user",
        Some("instance") => "instance",
        Some("system") => "system",
        _ => "none",
    }
}

fn draw_sessions(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if let Some(terminal) = &app.terminal {
        draw_terminal(
            frame,
            terminal.snapshot.as_ref(),
            area,
            terminal.input_focus,
            terminal.lease_id.is_some(),
            terminal.scrollback_rows,
        );
        return;
    }
    let items = app
        .collection("instances")
        .iter()
        .enumerate()
        .map(|(index, instance)| {
            let marker = if index == app.selected_session {
                ">"
            } else {
                " "
            };
            ListItem::new(safe_text::single_line(
                &format!(
                    "{marker} [{}] session {} instance {}",
                    field(instance, "status"),
                    field(instance, "session_id"),
                    field(instance, "id")
                ),
                512,
            ))
        });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);
    let selected = app.selected_session().map_or_else(
        || "none".to_owned(),
        |session| field(session, "session_id").to_owned(),
    );
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Sessions selected {selected}")),
        ),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new("Enter attach, s shell, a agent, t terminate"),
        rows[1],
    );
}

fn draw_terminal(
    frame: &mut Frame<'_>,
    snapshot: Option<&Value>,
    area: Rect,
    input: bool,
    writer: bool,
    scrollback_rows: u16,
) {
    let title = format!(
        "Terminal [{}] [{}]{}",
        if input { "INPUT" } else { "NAVIGATION" },
        if writer { "WRITER" } else { "READ ONLY" },
        if scrollback_rows != 0 {
            format!(" [HISTORY {scrollback_rows} rows]")
        } else {
            String::new()
        }
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(snapshot) = snapshot else {
        frame.render_widget(Paragraph::new("Attaching..."), inner);
        return;
    };
    let source_columns = snapshot.get("columns").and_then(Value::as_u64).unwrap_or(0) as usize;
    let cells = snapshot
        .get("cells")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let rows = inner
        .height
        .min(snapshot.get("rows").and_then(Value::as_u64).unwrap_or(0) as u16);
    let columns = inner.width.min(source_columns as u16);
    for row in 0..rows {
        for column in 0..columns {
            let index = usize::from(row) * source_columns + usize::from(column);
            let Some(cell) = cells.get(index) else {
                continue;
            };
            if cell_flag(cell, relayterm_protocol::TERMINAL_CELL_WIDE_CONTINUATION) {
                continue;
            }
            let text =
                safe_text::single_line(cell.get(0).and_then(Value::as_str).unwrap_or(" "), 32);
            let x = inner.x.saturating_add(column);
            let y = inner.y.saturating_add(row);
            let width = if cell_flag(cell, relayterm_protocol::TERMINAL_CELL_WIDE) {
                2
            } else {
                1
            }
            .min(inner.right().saturating_sub(x));
            if width == 0 {
                continue;
            }
            frame.buffer_mut().set_stringn(
                x,
                y,
                if text.is_empty() { " " } else { &text },
                usize::from(width),
                terminal_style(cell),
            );
        }
    }
    if input && snapshot.get("cursor_hidden").and_then(Value::as_bool) != Some(true) {
        let row = snapshot
            .get("cursor_row")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        let column = snapshot
            .get("cursor_column")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        if row < inner.height && column < inner.width {
            frame.set_cursor_position((inner.x + column, inner.y + row));
        }
    }
}

fn terminal_style(cell: &Value) -> Style {
    let mut style = Style::default()
        .fg(terminal_color(cell.get(1)))
        .bg(terminal_color(cell.get(2)));
    for (flag, modifier) in [
        (relayterm_protocol::TERMINAL_CELL_BOLD, Modifier::BOLD),
        (relayterm_protocol::TERMINAL_CELL_DIM, Modifier::DIM),
        (relayterm_protocol::TERMINAL_CELL_ITALIC, Modifier::ITALIC),
        (
            relayterm_protocol::TERMINAL_CELL_UNDERLINE,
            Modifier::UNDERLINED,
        ),
        (
            relayterm_protocol::TERMINAL_CELL_INVERSE,
            Modifier::REVERSED,
        ),
    ] {
        if cell_flag(cell, flag) {
            style = style.add_modifier(modifier);
        }
    }
    style
}

fn cell_flag(cell: &Value, flag: u8) -> bool {
    cell.get(3)
        .and_then(Value::as_u64)
        .is_some_and(|value| value & u64::from(flag) != 0)
}

fn terminal_color(value: Option<&Value>) -> Color {
    match value
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
    {
        Some("indexed") => value
            .and_then(|value| value.get("value"))
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .map_or(Color::Reset, Color::Indexed),
        Some("rgb") => value
            .and_then(|value| value.get("value"))
            .and_then(Value::as_array)
            .filter(|rgb| rgb.len() == 3)
            .and_then(|rgb| {
                Some(Color::Rgb(
                    u8::try_from(rgb[0].as_u64()?).ok()?,
                    u8::try_from(rgb[1].as_u64()?).ok()?,
                    u8::try_from(rgb[2].as_u64()?).ok()?,
                ))
            })
            .unwrap_or(Color::Reset),
        _ => Color::Reset,
    }
}

fn draw_agents(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);
    let visible = usize::from(rows[0].height.saturating_sub(2)).max(1);
    let start = app.selected_agent.saturating_sub(visible / 2);
    let items = app
        .collection("definitions")
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, definition)| {
            let marker = if index == app.selected_agent {
                ">"
            } else {
                " "
            };
            let enabled = definition
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            ListItem::new(safe_text::single_line(
                &format!(
                    "{marker} [{}] {} ({})",
                    if enabled { "enabled" } else { "disabled" },
                    field(definition, "display_name"),
                    &field(definition, "id")[..field(definition, "id").len().min(8)]
                ),
                512,
            ))
        });
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Agent definitions, a launches selected"),
        ),
        rows[0],
    );
    let template = app.templates.get(app.selected_template);
    let template_text = template.map_or_else(
        || "No template catalog available.".to_owned(),
        |value| format!(
            "Template {}/{}: {}\nCommand: {}\nDefaults are disabled; [ / ] selects, p copies into an editable definition.",
            app.selected_template + 1,
            app.templates.len(),
            safe_text::single_line(field(value, "display_name"), 256),
            safe_text::single_line(field(value, "command"), 4096),
        ),
    );
    frame.render_widget(
        Paragraph::new(template_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Built-in templates"),
        ),
        rows[1],
    );
    let selected_id = app
        .selected_agent()
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str);
    let availability = app
        .agent_availability
        .as_ref()
        .filter(|(id, _, _, revision, observed)| {
            selected_id == Some(id.as_str())
                && revision == &app.last_revision
                && observed.elapsed() <= Duration::from_secs(30)
        })
        .map_or_else(
            || "n custom  e edit  Space enable/disable  v check  a launch".to_owned(),
            |(_, status, guidance, _, _)| format!("Availability: {status} ({guidance})"),
        );
    frame.render_widget(Paragraph::new(availability), rows[2]);
}

fn draw_events(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items = app
        .diagnostics
        .iter()
        .rev()
        .map(|item| ListItem::new(format!("[{}] {}", item.category, item.message)));
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Redacted diagnostics"),
        ),
        area,
    );
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let help = "Global\n  1-6 / Tab: change screen   ?: help   q: quit   R: reconnect\n  j/k or arrows: move selection   Enter: open\n\nTasks\n  n: create   e: edit   r: ready/release   c: claim\n  b: block   d: complete   x: cancel   p: progress   h: handover\n  PageUp/PageDown: task history\n\nAgents\n  n: custom   e: edit   Space: enable/disable   v: availability   a: launch\n  [ / ]: select template   p: copy selected template into a new editable definition\n\nSessions\n  s: launch shell   a: launch selected definition   Enter: attach\n  PageUp/PageDown: terminal history   i: acquire input\n  Ctrl-]: return to navigation   Esc: detach   t: terminate\n\nForms\n  Tab/Shift-Tab: field   Ctrl-S: submit   Esc: cancel\n  Ctrl-R: reconcile an uncertain result before explicit resubmission.\n  Enter adds a newline only in multiline fields.\n\nRelayterm does not terminate sessions when the client quits.";
    frame.render_widget(
        Paragraph::new(help).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Keyboard help"),
        ),
        area,
    );
}

fn draw_form(frame: &mut Frame<'_>, form: &crate::model::Form, area: Rect) {
    frame.render_widget(Clear, area);
    let mut lines = Vec::new();
    for (index, field) in form.fields.iter().enumerate() {
        let marker = if index == form.selected { ">" } else { " " };
        lines.push(Line::from(vec![Span::styled(
            format!(
                "{marker} {} ({}/{})",
                field.label,
                field.value.len(),
                field.limit
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::raw(safe_text::narrative(&field.value, field.limit)));
    }
    if let Some(error) = &form.error {
        lines.push(Line::raw(format!(
            "Error: {}",
            safe_text::single_line(error, 512)
        )));
    }
    lines.push(Line::raw(
        "Ctrl-S submit, Esc cancel, Tab next field, Ctrl-R reconcile",
    ));
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("Form")),
        area,
    );
}

fn field<'a>(value: &'a Value, name: &str) -> &'a str {
    value.get(name).and_then(Value::as_str).unwrap_or("unknown")
}

fn centered(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn compact_and_subminimum_layouts_render_without_color_dependency() {
        for (width, height) in [(80, 24), (60, 15), (100, 30)] {
            let mut app = App {
                width,
                height,
                ..App::default()
            };
            app.freshness = crate::model::Freshness::Disconnected;
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| draw(frame, &app)).unwrap();
            let rendered = format!("{:?}", terminal.backend().buffer());
            assert!(rendered.contains(if width < 80 {
                "needs at least"
            } else {
                "Relayterm"
            }));
        }
    }
}
