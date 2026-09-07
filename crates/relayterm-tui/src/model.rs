use relayterm_client::{ClientError, ClientSnapshot, Delivery};
use serde_json::Value;
use std::collections::VecDeque;

pub const MIN_COLUMNS: u16 = 80;
pub const MIN_ROWS: u16 = 24;
pub const MAX_DIAGNOSTICS: usize = 1_000;
pub const MAX_DIAGNOSTIC_BYTES: usize = 1024 * 1024;
pub const MAX_DRAFT_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Overview,
    Tasks,
    Sessions,
    Agents,
    Events,
    Help,
}

impl Screen {
    pub const ALL: [Self; 6] = [
        Self::Overview,
        Self::Tasks,
        Self::Sessions,
        Self::Agents,
        Self::Events,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Tasks => "Tasks",
            Self::Sessions => "Sessions",
            Self::Agents => "Agents",
            Self::Events => "Events",
            Self::Help => "Help",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Connecting,
    Loading,
    Current,
    Stale,
    Disconnected,
    Reconnecting,
    Incompatible,
    Retryable,
}

impl Freshness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Connecting => "CONNECTING",
            Self::Loading => "LOADING",
            Self::Current => "CURRENT",
            Self::Stale => "STALE",
            Self::Disconnected => "DISCONNECTED",
            Self::Reconnecting => "RECONNECTING",
            Self::Incompatible => "INCOMPATIBLE",
            Self::Retryable => "RETRY REQUIRED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormKind {
    TaskCreate,
    TaskEdit,
    Progress,
    Handover,
    ConfirmTerminate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormField {
    pub label: &'static str,
    pub value: String,
    pub cursor: usize,
    pub multiline: bool,
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Form {
    pub kind: FormKind,
    pub fields: Vec<FormField>,
    pub selected: usize,
    pub error: Option<String>,
    pub pending: bool,
}

impl Form {
    pub fn task_create() -> Self {
        Self {
            kind: FormKind::TaskCreate,
            fields: vec![
                field("Title", false, 256),
                field("Description", true, 16 * 1024),
                field("Priority (low/normal/high/urgent)", false, 16),
                field("Scope paths (comma separated)", false, 128 * 4096),
                field("Acceptance notes", true, 16 * 1024),
                field("Dependency IDs (comma separated)", false, 128 * 64),
            ],
            selected: 0,
            error: None,
            pending: false,
        }
    }

    pub fn progress() -> Self {
        Self {
            kind: FormKind::Progress,
            fields: vec![
                field("Summary", true, 8 * 1024),
                field("Verification", true, 8 * 1024),
            ],
            selected: 0,
            error: None,
            pending: false,
        }
    }

    pub fn handover() -> Self {
        Self {
            kind: FormKind::Handover,
            fields: vec![
                field("Summary", true, 8 * 1024),
                field("Decisions", true, 16 * 1024),
                field("Changed paths (comma separated)", false, 128 * 4096),
                field("Verification performed", true, 8 * 1024),
                field("Open questions", true, 16 * 1024),
                field("Recommended next action", true, 8 * 1024),
            ],
            selected: 0,
            error: None,
            pending: false,
        }
    }

    pub fn bytes(&self) -> usize {
        self.fields.iter().map(|field| field.value.len()).sum()
    }
}

fn field(label: &'static str, multiline: bool, limit: usize) -> FormField {
    FormField {
        label,
        value: String::new(),
        cursor: 0,
        multiline,
        limit,
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub sequence: Option<u64>,
    pub category: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct TerminalView {
    pub session_id: String,
    pub attachment_id: String,
    pub lease_id: Option<String>,
    pub input_sequence: u64,
    pub snapshot: Option<Value>,
    pub input_focus: bool,
    pub uncertain_input: bool,
    pub scrollback_rows: u16,
    pub retained_scrollback_rows: u16,
}

#[derive(Clone)]
pub struct App {
    pub screen: Screen,
    pub freshness: Freshness,
    pub snapshot: Option<ClientSnapshot>,
    pub selected_task: usize,
    pub selected_session: usize,
    pub selected_agent: usize,
    pub task_scroll: u16,
    pub form: Option<Form>,
    pub terminal: Option<TerminalView>,
    pub diagnostics: VecDeque<Diagnostic>,
    pub diagnostic_bytes: usize,
    pub should_quit: bool,
    pub confirm_quit: bool,
    pub confirm_discard: bool,
    pub width: u16,
    pub height: u16,
    pub last_revision: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::Overview,
            freshness: Freshness::Connecting,
            snapshot: None,
            selected_task: 0,
            selected_session: 0,
            selected_agent: 0,
            task_scroll: 0,
            form: None,
            terminal: None,
            diagnostics: VecDeque::new(),
            diagnostic_bytes: 0,
            should_quit: false,
            confirm_quit: false,
            confirm_discard: false,
            width: MIN_COLUMNS,
            height: MIN_ROWS,
            last_revision: "0".into(),
        }
    }
}

impl App {
    pub fn install_snapshot(&mut self, mut snapshot: ClientSnapshot) {
        let selected_task = selected_identity(self.collection("tasks"), self.selected_task, "id");
        let selected_session = selected_identity(
            self.collection("instances"),
            self.selected_session,
            "session_id",
        );
        let selected_agent =
            selected_identity(self.collection("definitions"), self.selected_agent, "id");
        sort_collection(&mut snapshot, "tasks", "id");
        sort_collection(&mut snapshot, "instances", "session_id");
        sort_collection(&mut snapshot, "definitions", "id");
        self.last_revision = snapshot.revision.clone();
        self.snapshot = Some(snapshot);
        self.freshness = Freshness::Current;
        self.clamp_selections();
        self.selected_task = restored_selection(
            self.collection("tasks"),
            self.selected_task,
            selected_task.as_deref(),
            "id",
        );
        self.selected_session = restored_selection(
            self.collection("instances"),
            self.selected_session,
            selected_session.as_deref(),
            "session_id",
        );
        self.selected_agent = restored_selection(
            self.collection("definitions"),
            self.selected_agent,
            selected_agent.as_deref(),
            "id",
        );
    }

    pub fn collection(&self, name: &str) -> &[Value] {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.collections.get(name))
            .map_or(&[], Vec::as_slice)
    }

    pub fn selected_task(&self) -> Option<&Value> {
        self.collection("tasks").get(self.selected_task)
    }

    pub fn selected_session(&self) -> Option<&Value> {
        self.collection("instances").get(self.selected_session)
    }

    pub fn selected_agent(&self) -> Option<&Value> {
        self.collection("definitions").get(self.selected_agent)
    }

    pub fn small(&self) -> bool {
        self.width < MIN_COLUMNS || self.height < MIN_ROWS
    }

    pub fn add_diagnostic(&mut self, category: &'static str, message: impl Into<String>) {
        let message = crate::safe_text::single_line(&message.into(), 512);
        self.diagnostic_bytes = self.diagnostic_bytes.saturating_add(message.len());
        self.diagnostics.push_back(Diagnostic {
            sequence: None,
            category,
            message,
        });
        while self.diagnostics.len() > MAX_DIAGNOSTICS
            || self.diagnostic_bytes > MAX_DIAGNOSTIC_BYTES
        {
            if let Some(removed) = self.diagnostics.pop_front() {
                self.diagnostic_bytes = self.diagnostic_bytes.saturating_sub(removed.message.len());
            } else {
                break;
            }
        }
    }

    pub fn record_client_error(&mut self, error: ClientError) {
        match error {
            ClientError::Transport(Delivery::Unknown)
            | ClientError::Cancelled(Delivery::Unknown) => {
                self.freshness = Freshness::Stale;
                self.add_diagnostic(
                    "result_unknown",
                    "The result is unknown. Refresh before deciding whether to submit again.",
                );
            }
            ClientError::Transport(_) | ClientError::Cancelled(_) => {
                self.freshness = Freshness::Disconnected;
                self.add_diagnostic("transport", "Connection lost. Press R to reconnect.");
            }
            ClientError::VersionMismatch => {
                self.freshness = Freshness::Incompatible;
                self.add_diagnostic("version", "The daemon uses an incompatible protocol.");
            }
            ClientError::Rejected(_) => {
                self.freshness = Freshness::Stale;
                self.add_diagnostic(
                    "rejected",
                    "The daemon rejected the operation. State was refreshed.",
                );
            }
            _ => {
                self.freshness = Freshness::Retryable;
                self.add_diagnostic("protocol", "The operation could not be completed safely.");
            }
        }
    }

    fn clamp_selections(&mut self) {
        self.selected_task = clamp(self.selected_task, self.collection("tasks").len());
        self.selected_session = clamp(self.selected_session, self.collection("instances").len());
        self.selected_agent = clamp(self.selected_agent, self.collection("definitions").len());
    }
}

fn clamp(value: usize, len: usize) -> usize {
    value.min(len.saturating_sub(1))
}

fn selected_identity(collection: &[Value], selected: usize, field: &str) -> Option<String> {
    collection
        .get(selected)
        .and_then(|item| item.get(field))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn sort_collection(snapshot: &mut ClientSnapshot, collection: &str, field: &str) {
    if let Some(items) = snapshot.collections.get_mut(collection) {
        items.sort_by(|left, right| {
            let left = left.get(field).and_then(Value::as_str).unwrap_or_default();
            let right = right.get(field).and_then(Value::as_str).unwrap_or_default();
            left.cmp(right)
        });
    }
}

fn restored_selection(
    collection: &[Value],
    selected: usize,
    identity: Option<&str>,
    field: &str,
) -> usize {
    identity
        .and_then(|identity| {
            collection
                .iter()
                .position(|item| item.get(field).and_then(Value::as_str) == Some(identity))
        })
        .unwrap_or(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn snapshot(instances: Vec<Value>) -> ClientSnapshot {
        ClientSnapshot {
            revision: "1".into(),
            last_sequence: 0,
            retained_from_sequence: 0,
            collections: BTreeMap::from([("instances".into(), instances)]),
        }
    }

    #[test]
    fn diagnostics_are_bounded_and_safe() {
        let mut app = App::default();
        for _ in 0..1_100 {
            app.add_diagnostic("test", "bad\u{1b}[2Jvalue");
        }
        assert_eq!(app.diagnostics.len(), MAX_DIAGNOSTICS);
        assert!(
            app.diagnostics
                .iter()
                .all(|item| !item.message.contains('\u{1b}'))
        );
    }

    #[test]
    fn forms_have_bounded_complete_fields() {
        let task = Form::task_create();
        assert_eq!(task.fields.len(), 6);
        let handover = Form::handover();
        assert_eq!(handover.fields.len(), 6);
        assert!(handover.bytes() <= MAX_DRAFT_BYTES);
    }

    #[test]
    fn snapshot_refresh_preserves_selected_session_identity() {
        let first = json!({"session_id":"first"});
        let second = json!({"session_id":"second"});
        let mut app = App::default();
        app.install_snapshot(snapshot(vec![first.clone(), second.clone()]));
        app.selected_session = 1;

        app.install_snapshot(snapshot(vec![second, first]));

        assert_eq!(app.selected_session, 1);
        assert_eq!(
            app.selected_session()
                .and_then(|item| item["session_id"].as_str()),
            Some("second")
        );
    }
}
