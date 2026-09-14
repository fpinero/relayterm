use relayterm_client::{ClientError, ClientSnapshot, Delivery};
use serde_json::Value;
use std::{collections::VecDeque, time::Instant};

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
    AgentCreate,
    AgentEdit,
    TaskCreate,
    TaskEdit,
    Progress,
    Handover,
    SessionRename,
    ConfirmTerminate,
    WorktreeCreate,
}

#[derive(Clone, Eq, PartialEq)]
pub struct FormField {
    pub label: &'static str,
    pub value: String,
    pub cursor: usize,
    pub multiline: bool,
    pub limit: usize,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Form {
    pub kind: FormKind,
    pub guidance: Option<String>,
    pub fields: Vec<FormField>,
    pub selected: usize,
    pub error: Option<String>,
    pub pending: bool,
    pub uncertain: bool,
    pub target_id: Option<String>,
    pub base_revision: String,
}

impl Form {
    pub fn task_create() -> Self {
        Self {
            kind: FormKind::TaskCreate,
            guidance: None,
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
            uncertain: false,
            target_id: None,
            base_revision: String::new(),
        }
    }

    pub fn agent_create() -> Self {
        Self {
            kind: FormKind::AgentCreate,
            guidance: None,
            fields: vec![
                field("Display name", false, 256),
                field("Command", false, 4 * 1024),
                field(
                    "Arguments (one per line, <empty> for empty)",
                    true,
                    32 * 1024,
                ),
                field("Environment names (one per line)", true, 128 * 256),
                field("Capabilities (one per line)", true, 128 * 256),
                field("Enabled (true/false)", false, 5),
            ],
            selected: 0,
            error: None,
            pending: false,
            uncertain: false,
            target_id: None,
            base_revision: String::new(),
        }
    }

    pub fn progress() -> Self {
        Self {
            kind: FormKind::Progress,
            guidance: None,
            fields: vec![
                field("Summary", true, 8 * 1024),
                field("Verification", true, 8 * 1024),
            ],
            selected: 0,
            error: None,
            pending: false,
            uncertain: false,
            target_id: None,
            base_revision: String::new(),
        }
    }

    pub fn handover() -> Self {
        Self {
            kind: FormKind::Handover,
            guidance: None,
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
            uncertain: false,
            target_id: None,
            base_revision: String::new(),
        }
    }

    pub fn worktree_create() -> Self {
        Self {
            kind: FormKind::WorktreeCreate,
            guidance: None,
            fields: vec![
                field("Base reference", false, 256),
                field("New branch", false, 256),
                field("Approved parent", false, 8192),
                field("Destination leaf", false, 128),
                field("Operation ID", false, 64),
            ],
            selected: 0,
            error: None,
            pending: false,
            uncertain: false,
            target_id: None,
            base_revision: String::new(),
        }
    }

    pub fn session_rename(display_name: &str) -> Self {
        let mut name = field("Display name (Ctrl-U clears)", false, 128);
        name.value = display_name.to_owned();
        name.cursor = name.value.len();
        Self {
            kind: FormKind::SessionRename,
            guidance: None,
            fields: vec![name],
            selected: 0,
            error: None,
            pending: false,
            uncertain: false,
            target_id: None,
            base_revision: String::new(),
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
    pub selected_template: usize,
    pub session_page_index: usize,
    pub session_page_starts: Vec<Option<Value>>,
    pub templates: Vec<Value>,
    pub worktrees: Vec<Value>,
    pub agent_availability: Option<(String, String, String, String, Instant)>,
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
            selected_template: 0,
            session_page_index: 0,
            session_page_starts: vec![None],
            templates: Vec::new(),
            worktrees: Vec::new(),
            agent_availability: None,
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
        if self.last_revision != snapshot.revision {
            self.agent_availability = None;
        }
        let selected_task = selected_identity(self.collection("tasks"), self.selected_task, "id");
        let selected_session = self.selected_session_id().map(str::to_owned);
        let selected_agent =
            selected_identity(self.collection("definitions"), self.selected_agent, "id");
        sort_collection(&mut snapshot, "tasks", "id");
        if !snapshot.ordered_sessions {
            sort_collection(&mut snapshot, "instances", "session_id");
            self.session_page_index = 0;
            self.session_page_starts = vec![None];
        }
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
        self.selected_session = restored_session_selection(
            self.session_rows(),
            self.selected_session,
            selected_session.as_deref(),
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
        self.session_rows().get(self.selected_session)
    }

    pub fn selected_session_instance(&self) -> Option<&Value> {
        self.selected_session().map(session_instance)
    }

    pub fn selected_session_id(&self) -> Option<&str> {
        self.selected_session_instance()
            .and_then(|instance| instance.get("session_id"))
            .and_then(Value::as_str)
    }

    pub fn session_rows(&self) -> &[Value] {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.ordered_sessions)
        {
            self.collection("sessions")
        } else {
            self.collection("instances")
        }
    }

    pub fn ordered_sessions(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.ordered_sessions)
    }

    pub fn session_rename_supported(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.session_rename)
    }

    pub fn current_session_page_start(&self) -> Option<Value> {
        self.session_page_starts
            .get(self.session_page_index)
            .cloned()
            .flatten()
    }

    pub fn abbreviated_session_id(&self, index: usize) -> String {
        let rows = self.session_rows();
        let identity = rows
            .get(index)
            .map(session_instance)
            .and_then(|instance| instance.get("session_id"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        for length in 8..=identity.len() {
            if !identity.is_char_boundary(length) {
                continue;
            }
            let prefix = &identity[..length];
            if rows.iter().enumerate().all(|(other_index, row)| {
                other_index == index
                    || !session_instance(row)
                        .get("session_id")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .starts_with(prefix)
            }) {
                return prefix.to_owned();
            }
        }
        identity.to_owned()
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
            | ClientError::Cancelled(Delivery::Unknown)
            | ClientError::Rejected(relayterm_protocol::ErrorCode::ResultUnknown) => {
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
        self.selected_session = clamp(self.selected_session, self.session_rows().len());
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

fn restored_session_selection(
    collection: &[Value],
    selected: usize,
    identity: Option<&str>,
) -> usize {
    identity
        .and_then(|identity| {
            collection.iter().position(|item| {
                session_instance(item)
                    .get("session_id")
                    .and_then(Value::as_str)
                    == Some(identity)
            })
        })
        .unwrap_or(selected)
}

pub fn session_instance(row: &Value) -> &Value {
    row.get("instance").unwrap_or(row)
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
            ordered_sessions: false,
            session_rename: false,
            session_next: None,
        }
    }

    fn ordered_snapshot(revision: &str, sessions: Vec<Value>) -> ClientSnapshot {
        ClientSnapshot {
            revision: revision.into(),
            last_sequence: 0,
            retained_from_sequence: 0,
            collections: BTreeMap::from([
                ("sessions".into(), sessions.clone()),
                (
                    "instances".into(),
                    sessions.iter().map(|row| row["instance"].clone()).collect(),
                ),
            ]),
            ordered_sessions: true,
            session_rename: true,
            session_next: None,
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

    #[test]
    fn revision_change_invalidates_availability_result() {
        let mut app = App::default();
        app.install_snapshot(snapshot(Vec::new()));
        app.agent_availability = Some((
            "definition".into(),
            "available".into(),
            "available".into(),
            "1".into(),
            Instant::now(),
        ));
        let mut changed = snapshot(Vec::new());
        changed.revision = "2".into();
        app.install_snapshot(changed);
        assert!(app.agent_availability.is_none());
    }

    #[test]
    fn ordered_refresh_preserves_identity_across_names_and_background_creation() {
        let session = |id: &str, ordinal: &str, name: Value| {
            json!({
                "instance":{"session_id":id,"id":format!("instance-{id}"),"status":"running"},
                "display_name":name,
                "creation_ordinal":ordinal
            })
        };
        let first = session("session-first", "1", json!("duplicate"));
        let second = session("session-second", "2", json!("duplicate"));
        let mut app = App::default();
        app.install_snapshot(ordered_snapshot("1", vec![first.clone(), second.clone()]));
        app.selected_session = 1;

        let renamed = session("session-second", "2", Value::Null);
        let background = session("session-third", "3", json!("background"));
        app.install_snapshot(ordered_snapshot("2", vec![first, renamed, background]));

        assert_eq!(app.selected_session_id(), Some("session-second"));
        assert_eq!(app.selected_session, 1);
        assert!(app.selected_session().unwrap()["display_name"].is_null());
    }

    #[test]
    fn rename_form_keeps_the_utf8_byte_cursor_and_clear_contract() {
        let form = Form::session_rename("Build e\u{301}");
        assert_eq!(form.kind, FormKind::SessionRename);
        assert_eq!(form.fields[0].cursor, "Build e\u{301}".len());
        assert_eq!(form.fields[0].limit, 128);
        assert!(form.fields[0].label.contains("Ctrl-U"));
    }
}
