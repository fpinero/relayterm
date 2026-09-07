//! Interactive Relayterm client. The daemon remains the sole owner of durable and process state.

mod input;
mod lifecycle;
mod model;
mod render;
mod safe_text;

pub use lifecycle::LifecycleError;
pub use model::{App, Form, FormKind, Freshness, Screen};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use relayterm_client::{Client, ClientError};
use relayterm_protocol::Operation;
use serde_json::{Value, json};
use std::{
    fmt,
    io::{IsTerminal, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

const INPUT_LIMIT: usize = 64 * 1024;
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const REFRESH_IDLE_INTERVAL: Duration = Duration::from_millis(100);
const TERMINAL_REFRESH_INTERVAL: Duration = Duration::from_millis(33);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const UI_TICK_INTERVAL: Duration = Duration::from_millis(33);

struct InputWorker {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl InputWorker {
    fn start() -> (Self, mpsc::Receiver<Event>) {
        let (sender, receiver) = mpsc::channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                if !event::poll(INPUT_POLL_INTERVAL).unwrap_or(false) {
                    continue;
                }
                let Ok(mut input) = event::read() else {
                    continue;
                };
                loop {
                    match sender.try_send(input) {
                        Ok(()) => break,
                        Err(mpsc::error::TrySendError::Full(returned)) => {
                            input = returned;
                            if worker_stop.load(Ordering::Acquire) {
                                return;
                            }
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => return,
                    }
                }
            }
        });
        (
            Self {
                stop,
                thread: Some(thread),
            },
            receiver,
        )
    }
}

impl Drop for InputWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug)]
pub enum TuiError {
    UnsupportedTerminal,
    Lifecycle,
    Client(ClientError),
    Runtime,
}

impl fmt::Display for TuiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedTerminal => {
                "Relayterm requires an interactive input and output terminal."
            }
            Self::Lifecycle => "Relayterm could not initialize or restore the terminal.",
            Self::Client(_) => "Relayterm could not synchronize with the workspace daemon.",
            Self::Runtime => "The interactive client stopped unexpectedly.",
        })
    }
}

impl std::error::Error for TuiError {}

impl From<ClientError> for TuiError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

pub fn interactive_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn confirm_initialize(path: &Path) -> Result<bool, TuiError> {
    if !interactive_terminal() {
        return Err(TuiError::UnsupportedTerminal);
    }
    let label = safe_text::single_line(&path.to_string_lossy(), 512);
    print!("No Relayterm workspace exists at {label}. Initialize it? [y/N] ");
    std::io::stdout().flush().map_err(|_| TuiError::Runtime)?;
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|_| TuiError::Runtime)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

pub async fn run(client: Client) -> Result<(), TuiError> {
    if !interactive_terminal() {
        return Err(TuiError::UnsupportedTerminal);
    }
    let mut terminal = lifecycle::TerminalGuard::enter().map_err(|_| TuiError::Lifecycle)?;
    let result = tokio::select! {
        result = run_loop(&client, &mut terminal) => result,
        () = termination_signal() => Ok(()),
    };
    let restored = terminal.restore().map_err(|_| TuiError::Lifecycle);
    result.and(restored)
}

#[cfg(unix)]
async fn termination_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).ok();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = async {
            if let Some(signal) = terminate.as_mut() {
                signal.recv().await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {}
    }
}

#[cfg(not(unix))]
async fn termination_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn run_loop(
    client: &Client,
    terminal: &mut lifecycle::TerminalGuard,
) -> Result<(), TuiError> {
    let mut app = App::default();
    refresh(client, &mut app).await?;
    let (_input_worker, mut input_rx) = InputWorker::start();
    let (event_tx, mut event_rx) = mpsc::channel::<Result<Value, ClientError>>(64);
    let event_client = client.connect_peer().await?;
    let mut event_cursor = app
        .snapshot
        .as_ref()
        .map_or(0, |snapshot| snapshot.last_sequence);
    let event_worker = tokio::spawn(async move {
        let delays = [100_u64, 250, 500, 1_000, 2_000];
        let mut delay_index = 0_usize;
        if let Err(error) = event_client.subscribe(event_cursor).await {
            if event_tx.send(Err(error)).await.is_err() {
                return;
            }
            loop {
                tokio::time::sleep(Duration::from_millis(delays[delay_index])).await;
                delay_index = (delay_index + 1).min(delays.len() - 1);
                if event_client
                    .recover_subscription(event_cursor)
                    .await
                    .is_ok()
                {
                    break;
                }
            }
        }
        loop {
            match event_client.next_event().await {
                Ok(event) => {
                    delay_index = 0;
                    if let Some(sequence) = event
                        .get("sequence")
                        .and_then(Value::as_str)
                        .and_then(|value| value.parse().ok())
                    {
                        event_cursor = sequence;
                    }
                    if event_tx.send(Ok(event)).await.is_err() {
                        return;
                    }
                }
                Err(error) => {
                    if event_tx.send(Err(error)).await.is_err() {
                        return;
                    }
                    loop {
                        tokio::time::sleep(Duration::from_millis(delays[delay_index])).await;
                        delay_index = (delay_index + 1).min(delays.len() - 1);
                        if event_client
                            .recover_subscription(event_cursor)
                            .await
                            .is_ok()
                        {
                            break;
                        }
                    }
                }
            }
        }
    });
    let mut last_refresh = Instant::now();
    let mut last_input = Instant::now() - REFRESH_IDLE_INTERVAL;
    let mut last_terminal_refresh = Instant::now() - TERMINAL_REFRESH_INTERVAL;
    loop {
        let size = terminal.terminal().size().map_err(|_| TuiError::Runtime)?;
        app.width = size.width;
        app.height = size.height;
        terminal
            .terminal()
            .draw(|frame| render::draw(frame, &app))
            .map_err(|_| TuiError::Runtime)?;
        if app.should_quit {
            cleanup_session(client, &mut app).await;
            event_worker.abort();
            return Ok(());
        }

        while let Ok(event) = event_rx.try_recv() {
            match event {
                Ok(value) => {
                    let sequence = value.get("sequence").and_then(Value::as_str).unwrap_or("?");
                    app.add_diagnostic(
                        "workspace_event",
                        format!("Workspace event {sequence} requires refresh."),
                    );
                    app.freshness = Freshness::Stale;
                    last_refresh = Instant::now() - REFRESH_INTERVAL;
                }
                Err(error) => app.record_client_error(error),
            }
        }

        if app
            .terminal
            .as_ref()
            .is_some_and(|terminal| terminal.scrollback_rows == 0)
            && last_terminal_refresh.elapsed() >= TERMINAL_REFRESH_INTERVAL
        {
            refresh_terminal(client, &mut app).await;
            last_terminal_refresh = Instant::now();
        } else if app.terminal.is_none()
            && last_refresh.elapsed() >= REFRESH_INTERVAL
            && last_input.elapsed() >= REFRESH_IDLE_INTERVAL
        {
            if let Err(error) = refresh(client, &mut app).await {
                app.record_client_error(error);
            }
            last_refresh = Instant::now();
        }

        let input = tokio::time::timeout(UI_TICK_INTERVAL, input_rx.recv())
            .await
            .ok()
            .flatten();
        if let Some(input) = input {
            last_input = Instant::now();
            handle_event(client, &mut app, input).await;
            for _ in 0..63 {
                let Ok(input) = input_rx.try_recv() else {
                    break;
                };
                last_input = Instant::now();
                handle_event(client, &mut app, input).await;
            }
        }
    }
}

async fn handle_event(client: &Client, app: &mut App, event: Event) {
    match event {
        Event::Resize(width, height) => {
            app.width = width;
            app.height = height;
            if let Some(terminal) = app.terminal.as_mut() {
                terminal.snapshot = None;
            }
            resize_attached(client, app).await;
        }
        Event::Paste(value) => handle_paste(client, app, &value).await,
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            handle_key(client, app, key).await
        }
        _ => {}
    }
}

async fn handle_key(client: &Client, app: &mut App, key: KeyEvent) {
    if app.confirm_discard {
        match key.code {
            KeyCode::Char('y') => {
                app.form = None;
                app.confirm_discard = false;
            }
            KeyCode::Char('n') | KeyCode::Esc => app.confirm_discard = false,
            _ => {}
        }
        return;
    }
    if app.confirm_quit {
        match key.code {
            KeyCode::Char('y') => app.should_quit = true,
            KeyCode::Char('n') | KeyCode::Esc => app.confirm_quit = false,
            _ => {}
        }
        return;
    }
    if app.form.is_some() {
        handle_form_key(client, app, key).await;
        return;
    }
    if app
        .terminal
        .as_ref()
        .is_some_and(|terminal| terminal.input_focus)
    {
        if release_input_chord(key) {
            release_input(client, app).await;
            return;
        }
        if let Some(bytes) = input::encode_key(key, terminal_application_cursor(app)) {
            send_input(client, app, bytes).await;
        }
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }
    if app.terminal.is_some() && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
        let page = terminal_rows(app).saturating_sub(1).max(1);
        if let Some(terminal) = app.terminal.as_mut() {
            terminal.scrollback_rows = if key.code == KeyCode::PageUp {
                terminal.scrollback_rows.saturating_add(page)
            } else {
                terminal.scrollback_rows.saturating_sub(page)
            };
            terminal.snapshot = None;
        }
        refresh_terminal(client, app).await;
        return;
    }
    match key.code {
        KeyCode::Char('q') => {
            if app.form.is_some() {
                app.confirm_quit = true
            } else {
                app.should_quit = true
            }
        }
        KeyCode::Char('?') => app.screen = Screen::Help,
        KeyCode::Char('R') => {
            app.freshness = Freshness::Reconnecting;
            if let Err(error) = refresh(client, app).await {
                app.record_client_error(error);
            }
        }
        KeyCode::Char(value @ '1'..='6') => {
            app.screen = Screen::ALL[(value as usize) - ('1' as usize)]
        }
        KeyCode::Tab => next_screen(app),
        KeyCode::BackTab => previous_screen(app),
        KeyCode::Down | KeyCode::Char('j') => move_selection(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_selection(app, -1),
        KeyCode::PageDown if app.screen == Screen::Tasks => {
            app.task_scroll = app.task_scroll.saturating_add(10)
        }
        KeyCode::PageUp if app.screen == Screen::Tasks => {
            app.task_scroll = app.task_scroll.saturating_sub(10)
        }
        KeyCode::Esc if app.terminal.is_some() => detach(client, app).await,
        KeyCode::Enter if app.screen == Screen::Sessions => attach_selected(client, app).await,
        KeyCode::Char('i') if app.terminal.is_some() => acquire_input(client, app).await,
        KeyCode::Char('s') if app.screen == Screen::Sessions => launch(client, app, None).await,
        KeyCode::Char('a') if matches!(app.screen, Screen::Sessions | Screen::Agents) => {
            let definition = app
                .selected_agent()
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            launch(client, app, definition).await;
        }
        KeyCode::Char('t') if app.screen == Screen::Sessions => confirm_termination(app),
        KeyCode::Char('n') if app.screen == Screen::Tasks => app.form = Some(Form::task_create()),
        KeyCode::Char('e') if app.screen == Screen::Tasks => edit_task(app),
        KeyCode::Char('p') if app.screen == Screen::Tasks => app.form = Some(Form::progress()),
        KeyCode::Char('h') if app.screen == Screen::Tasks => app.form = Some(Form::handover()),
        KeyCode::Char('r') if app.screen == Screen::Tasks => ready_or_release(client, app).await,
        KeyCode::Char('c') if app.screen == Screen::Tasks => claim(client, app).await,
        KeyCode::Char('b') if app.screen == Screen::Tasks => {
            transition(client, app, "blocked").await
        }
        KeyCode::Char('d') if app.screen == Screen::Tasks => transition(client, app, "done").await,
        KeyCode::Char('x') if app.screen == Screen::Tasks => {
            transition(client, app, "cancelled").await
        }
        _ => {}
    }
}

fn release_input_chord(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(']') | KeyCode::Char('5'))
}

async fn handle_form_key(client: &Client, app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        submit_form(client, app).await;
        return;
    }
    let Some(form) = app.form.as_mut() else {
        return;
    };
    if form.pending {
        return;
    }
    match key.code {
        KeyCode::Esc => app.confirm_discard = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.confirm_discard = true
        }
        KeyCode::Tab => form.selected = (form.selected + 1) % form.fields.len(),
        KeyCode::BackTab => {
            form.selected = form
                .selected
                .checked_sub(1)
                .unwrap_or(form.fields.len() - 1)
        }
        KeyCode::Backspace => {
            let field = &mut form.fields[form.selected];
            if field.cursor > 0 {
                let previous = field.value[..field.cursor]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(index, _)| index);
                field.value.drain(previous..field.cursor);
                field.cursor = previous;
            }
        }
        KeyCode::Delete => {
            let field = &mut form.fields[form.selected];
            if field.cursor < field.value.len() {
                let next = field.value[field.cursor..]
                    .char_indices()
                    .nth(1)
                    .map_or(field.value.len(), |(index, _)| field.cursor + index);
                field.value.drain(field.cursor..next);
            }
        }
        KeyCode::Left => {
            let field = &mut form.fields[form.selected];
            field.cursor = field.value[..field.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(index, _)| index);
        }
        KeyCode::Right => {
            let field = &mut form.fields[form.selected];
            if field.cursor < field.value.len() {
                field.cursor = field.value[field.cursor..]
                    .char_indices()
                    .nth(1)
                    .map_or(field.value.len(), |(index, _)| field.cursor + index);
            }
        }
        KeyCode::Home => form.fields[form.selected].cursor = 0,
        KeyCode::End => {
            let field = &mut form.fields[form.selected];
            field.cursor = field.value.len();
        }
        KeyCode::Enter if form.fields[form.selected].multiline => push_form(form, '\n'),
        KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            push_form(form, character)
        }
        _ => {}
    }
}

fn push_form(form: &mut Form, character: char) {
    let total = form.bytes();
    let field = &mut form.fields[form.selected];
    if field.value.len().saturating_add(character.len_utf8()) <= field.limit
        && total.saturating_add(character.len_utf8()) <= model::MAX_DRAFT_BYTES
    {
        field.value.insert(field.cursor, character);
        field.cursor += character.len_utf8();
        form.error = None;
    } else {
        form.error = Some("The field or total draft byte limit was reached.".into());
    }
}

async fn handle_paste(client: &Client, app: &mut App, value: &str) {
    if app
        .terminal
        .as_ref()
        .is_some_and(|terminal| terminal.input_focus)
    {
        let bracketed = app
            .terminal
            .as_ref()
            .and_then(|terminal| terminal.snapshot.as_ref())
            .and_then(|snapshot| snapshot.get("bracketed_paste"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if let Some(bytes) = input::encode_paste(value, bracketed, INPUT_LIMIT) {
            send_input(client, app, bytes).await;
        } else {
            app.add_diagnostic("resource_limit", "Paste rejected without partial delivery.");
        }
        return;
    }
    if let Some(form) = app.form.as_mut() {
        for character in value.chars() {
            push_form(form, character);
        }
    }
}

async fn refresh(client: &Client, app: &mut App) -> Result<(), ClientError> {
    app.freshness = Freshness::Loading;
    let snapshot = client.refresh_snapshot().await?;
    app.install_snapshot(snapshot);
    Ok(())
}

async fn mutate(client: &Client, app: &mut App, operation: Operation, params: Value) {
    match client.call::<_, Value>(operation, &params).await {
        Ok(_) => {
            app.add_diagnostic("committed", "Operation committed.");
            if let Err(error) = refresh(client, app).await {
                app.record_client_error(error);
            }
        }
        Err(error) => {
            app.record_client_error(error);
            let _ = refresh(client, app).await;
        }
    }
}

async fn submit_form(client: &Client, app: &mut App) {
    let Some(mut form) = app.form.take() else {
        return;
    };
    if form.pending {
        app.form = Some(form);
        return;
    }
    if form.kind == FormKind::ConfirmTerminate {
        if form.fields[0].value != "TERMINATE" {
            form.error = Some("Type TERMINATE exactly to confirm.".into());
            app.form = Some(form);
            return;
        }
        let Some(session_id) = selected_session_id(app) else {
            app.form = Some(form);
            return;
        };
        mutate(
            client,
            app,
            Operation::SessionTerminate,
            json!({"session_id":session_id}),
        )
        .await;
        return;
    }
    let task_id = match selected_task_id(app) {
        Some(task_id) => task_id,
        None if form.kind == FormKind::TaskCreate => String::new(),
        None => {
            form.error = Some("Select a task first.".into());
            app.form = Some(form);
            return;
        }
    };
    let result = form_params(app, &form, &task_id);
    let (operation, params) = match result {
        Ok(value) => value,
        Err(error) => {
            form.error = Some(error);
            app.form = Some(form);
            return;
        }
    };
    form.pending = true;
    match client.call::<_, Value>(operation, &params).await {
        Ok(_) => {
            app.add_diagnostic("committed", "Form operation committed.");
            if let Err(error) = refresh(client, app).await {
                app.record_client_error(error);
            }
        }
        Err(error) => {
            form.pending = false;
            form.error = Some(error.to_string());
            app.record_client_error(error);
            let _ = refresh(client, app).await;
            app.form = Some(form);
        }
    }
}

fn form_params(app: &App, form: &Form, task_id: &str) -> Result<(Operation, Value), String> {
    let value = |index: usize| form.fields[index].value.clone();
    let list = |index: usize| -> Vec<String> {
        form.fields[index]
            .value
            .split(',')
            .filter(|item| !item.is_empty())
            .map(str::to_owned)
            .collect()
    };
    match form.kind {
        FormKind::TaskCreate | FormKind::TaskEdit => {
            if value(0).is_empty() {
                return Err("Title is required.".into());
            }
            let priority = if value(2).is_empty() {
                "normal".into()
            } else {
                value(2)
            };
            if !matches!(priority.as_str(), "low" | "normal" | "high" | "urgent") {
                return Err("Priority must be low, normal, high, or urgent.".into());
            }
            let mut params = json!({
                "expected_revision":app.last_revision,"title":value(0),"description":value(1),"priority":priority,
                "scope_paths":list(3),"acceptance_notes":value(4),"dependency_ids":list(5)
            });
            let operation = if form.kind == FormKind::TaskEdit {
                params["task_id"] = Value::String(task_id.into());
                Operation::TaskUpdate
            } else {
                Operation::TaskCreate
            };
            Ok((operation, params))
        }
        FormKind::Progress => {
            if value(0).is_empty() {
                return Err("Summary is required.".into());
            }
            Ok((
                Operation::ProgressAppend,
                json!({"task_id":task_id,"summary":value(0),"verification":value(1)}),
            ))
        }
        FormKind::Handover => {
            if value(0).is_empty() || value(3).is_empty() || value(5).is_empty() {
                return Err("Summary, verification, and next action are required.".into());
            }
            Ok((
                Operation::HandoverCreate,
                json!({
                    "task_id":task_id,"expected_revision":app.last_revision,"summary":value(0),"decisions":value(1),
                    "changed_paths":list(2),"verification_performed":value(3),"open_questions":value(4),
                    "recommended_next_action":value(5)
                }),
            ))
        }
        FormKind::ConfirmTerminate => Err("Use the termination confirmation action.".into()),
    }
}

fn edit_task(app: &mut App) {
    let Some(task) = app.selected_task().cloned() else {
        return;
    };
    if matches!(
        task.get("status").and_then(Value::as_str),
        Some("done" | "cancelled")
    ) {
        app.add_diagnostic("final", "Final tasks cannot be edited.");
        return;
    }
    let mut form = Form::task_create();
    form.kind = FormKind::TaskEdit;
    let content = task.get("content").unwrap_or(&Value::Null);
    form.fields[0].value = string(content, "title");
    form.fields[1].value = string(content, "description");
    form.fields[2].value = string(content, "priority");
    form.fields[3].value = joined(content, "scope_paths");
    form.fields[4].value = string(content, "acceptance_notes");
    form.fields[5].value = joined(content, "dependency_ids");
    for field in &mut form.fields {
        field.cursor = field.value.len();
    }
    app.form = Some(form);
}

async fn ready_or_release(client: &Client, app: &mut App) {
    match app
        .selected_task()
        .and_then(|task| task.get("status"))
        .and_then(Value::as_str)
    {
        Some("active") => {
            if let Some(task_id) = selected_task_id(app) {
                mutate(
                    client,
                    app,
                    Operation::TaskRelease,
                    json!({"task_id":task_id,"expected_revision":app.last_revision}),
                )
                .await;
            }
        }
        Some("backlog" | "blocked") => transition(client, app, "ready").await,
        _ => app.add_diagnostic(
            "invalid_state",
            "Ready/release is not valid for the selected state.",
        ),
    }
}

async fn transition(client: &Client, app: &mut App, status: &str) {
    if let Some(task_id) = selected_task_id(app) {
        mutate(
            client,
            app,
            Operation::TaskTransition,
            json!({"task_id":task_id,"expected_revision":app.last_revision,"status":status}),
        )
        .await;
    }
}

async fn claim(client: &Client, app: &mut App) {
    let task = selected_task_id(app);
    let instance = app
        .selected_session()
        .filter(|instance| {
            instance.get("status").and_then(Value::as_str) == Some("running")
                && !app.collection("claims").iter().any(|claim| {
                    claim.get("instance_id") == instance.get("id")
                        && claim.get("closed_at").is_some_and(Value::is_null)
                })
        })
        .and_then(|instance| instance.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    match (task, instance) {
        (Some(task_id), Some(instance_id)) => {
            mutate(
                client,
                app,
                Operation::TaskClaim,
                json!({"task_id":task_id,"instance_id":instance_id}),
            )
            .await
        }
        _ => app.add_diagnostic(
            "invalid_reference",
            "The selected session is not eligible to claim this task.",
        ),
    }
}

async fn launch(client: &Client, app: &mut App, definition_id: Option<String>) {
    let task_id = selected_task_id(app);
    let params = json!({
        "receipt_id":uuid::Uuid::new_v4().to_string(),
        "launch_kind":if definition_id.is_some(){"definition"}else{"default_shell"},
        "definition_id":definition_id,"task_id":task_id,"working_directory":null,
        "rows":terminal_rows(app),"columns":terminal_columns(app)
    });
    mutate(client, app, Operation::SessionCreate, params).await;
    app.screen = Screen::Sessions;
}

async fn attach_selected(client: &Client, app: &mut App) {
    let Some(session_id) = selected_session_id(app) else {
        return;
    };
    match client
        .call::<_, Value>(
            Operation::SessionReadDisplay,
            &display_params(app, &session_id, None, None, 0, None),
        )
        .await
    {
        Ok(value) => {
            app.terminal = Some(model::TerminalView {
                session_id,
                attachment_id: string(&value, "attachment_id"),
                snapshot: value.get("snapshot").cloned(),
                ..model::TerminalView::default()
            })
        }
        Err(error) => app.record_client_error(error),
    }
}

async fn refresh_terminal(client: &Client, app: &mut App) {
    let Some((session_id, attachment_id, revision, scrollback_rows, after_scrollback)) =
        app.terminal.as_ref().map(|terminal| {
            (
                terminal.session_id.clone(),
                terminal.attachment_id.clone(),
                terminal
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.get("revision"))
                    .and_then(Value::as_u64),
                terminal.scrollback_rows,
                terminal.snapshot.as_ref().map(|_| terminal.scrollback_rows),
            )
        })
    else {
        return;
    };
    match client
        .call::<_, Value>(
            Operation::SessionReadDisplay,
            &display_params(
                app,
                &session_id,
                Some(&attachment_id),
                revision,
                scrollback_rows,
                after_scrollback,
            ),
        )
        .await
    {
        Ok(value) => {
            if let Some(terminal) = app.terminal.as_mut() {
                if terminal.attachment_id == string(&value, "attachment_id") {
                    if !value
                        .get("unchanged")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        terminal.snapshot = value
                            .get("snapshot")
                            .cloned()
                            .filter(|snapshot| !snapshot.is_null());
                    }
                    terminal.scrollback_rows = value
                        .get("scrollback_offset")
                        .and_then(Value::as_u64)
                        .and_then(|value| u16::try_from(value).ok())
                        .unwrap_or(0);
                    terminal.retained_scrollback_rows = value
                        .get("retained_scrollback_rows")
                        .and_then(Value::as_u64)
                        .and_then(|value| u16::try_from(value).ok())
                        .unwrap_or(0);
                    app.freshness = Freshness::Current;
                } else {
                    app.freshness = Freshness::Stale;
                    app.add_diagnostic(
                        "generation",
                        "Attachment identity changed. Reattach explicitly.",
                    );
                }
            }
        }
        Err(error) => app.record_client_error(error),
    }
}

async fn acquire_input(client: &Client, app: &mut App) {
    if let Some(terminal) = app.terminal.as_mut()
        && terminal.scrollback_rows != 0
    {
        terminal.scrollback_rows = 0;
        terminal.snapshot = None;
        refresh_terminal(client, app).await;
    }
    let Some(session_id) = app
        .terminal
        .as_ref()
        .map(|terminal| terminal.session_id.clone())
    else {
        return;
    };
    match client
        .call::<_, Value>(
            Operation::SessionAcquireInput,
            &json!({"session_id":session_id}),
        )
        .await
    {
        Ok(value) => {
            if let Some(terminal) = app.terminal.as_mut() {
                terminal.lease_id = value
                    .get("lease_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                terminal.input_sequence = 1;
                terminal.input_focus = terminal.lease_id.is_some();
            }
        }
        Err(error) => app.record_client_error(error),
    }
}

async fn release_input(client: &Client, app: &mut App) {
    let Some(terminal) = app.terminal.as_ref() else {
        return;
    };
    if let Some(lease_id) = terminal.lease_id.clone() {
        let session_id = terminal.session_id.clone();
        let _ = client
            .call::<_, Value>(
                Operation::SessionReleaseInput,
                &json!({"session_id":session_id,"lease_id":lease_id}),
            )
            .await;
    }
    if let Some(terminal) = app.terminal.as_mut() {
        terminal.lease_id = None;
        terminal.input_focus = false;
    }
}

async fn send_input(client: &Client, app: &mut App, bytes: Vec<u8>) {
    let Some(terminal) = app.terminal.as_ref() else {
        return;
    };
    let (session_id, lease_id, sequence) = match &terminal.lease_id {
        Some(lease) => (
            terminal.session_id.clone(),
            lease.clone(),
            terminal.input_sequence,
        ),
        None => return,
    };
    match client.call::<_, Value>(Operation::SessionInput, &json!({"session_id":session_id,"lease_id":lease_id,"sequence":sequence.to_string(),"data":STANDARD.encode(bytes)})).await {
        Ok(_) => if let Some(terminal) = app.terminal.as_mut() { terminal.input_sequence += 1; },
        Err(error) => {
            if let Some(terminal) = app.terminal.as_mut() {
                terminal.uncertain_input = true;
                terminal.input_focus = false;
                terminal.lease_id = None;
            }
            app.record_client_error(error);
        }
    }
}

async fn resize_attached(client: &Client, app: &mut App) {
    let Some(terminal) = app.terminal.as_ref() else {
        return;
    };
    let Some(lease_id) = terminal.lease_id.clone() else {
        return;
    };
    let session_id = terminal.session_id.clone();
    let result = client.call::<_, Value>(Operation::SessionResize, &json!({"session_id":session_id,"lease_id":lease_id,"rows":terminal_rows(app),"columns":terminal_columns(app)})).await;
    if let Err(error) = result {
        app.record_client_error(error);
    }
}

fn confirm_termination(app: &mut App) {
    let Some(session_id) = selected_session_id(app) else {
        return;
    };
    app.form = Some(Form {
        kind: FormKind::ConfirmTerminate,
        fields: vec![model::FormField {
            label: "Type TERMINATE to confirm",
            value: String::new(),
            cursor: 0,
            multiline: false,
            limit: 9,
        }],
        selected: 0,
        error: Some(format!(
            "Session {} will stop; an owned active task becomes blocked.",
            safe_text::single_line(&session_id, 64)
        )),
        pending: false,
    });
}

async fn detach(client: &Client, app: &mut App) {
    release_input(client, app).await;
    if let Some(terminal) = app.terminal.take() {
        let _ = client
            .call::<_, Value>(
                Operation::SessionDetach,
                &json!({"session_id":terminal.session_id}),
            )
            .await;
    }
}

async fn cleanup_session(client: &Client, app: &mut App) {
    detach(client, app).await;
}

fn next_screen(app: &mut App) {
    let index = Screen::ALL
        .iter()
        .position(|screen| *screen == app.screen)
        .unwrap_or(0);
    app.screen = Screen::ALL[(index + 1) % Screen::ALL.len()];
}

fn previous_screen(app: &mut App) {
    let index = Screen::ALL
        .iter()
        .position(|screen| *screen == app.screen)
        .unwrap_or(0);
    app.screen = Screen::ALL[index.checked_sub(1).unwrap_or(Screen::ALL.len() - 1)];
}

fn move_selection(app: &mut App, amount: isize) {
    if app.screen == Screen::Tasks {
        app.task_scroll = 0;
    }
    let (selected, len) = match app.screen {
        Screen::Tasks => (
            &mut app.selected_task,
            app.snapshot
                .as_ref()
                .and_then(|s| s.collections.get("tasks"))
                .map_or(0, Vec::len),
        ),
        Screen::Sessions => (
            &mut app.selected_session,
            app.snapshot
                .as_ref()
                .and_then(|s| s.collections.get("instances"))
                .map_or(0, Vec::len),
        ),
        Screen::Agents => (
            &mut app.selected_agent,
            app.snapshot
                .as_ref()
                .and_then(|s| s.collections.get("definitions"))
                .map_or(0, Vec::len),
        ),
        _ => return,
    };
    if len == 0 {
        *selected = 0;
        return;
    }
    *selected = if amount < 0 {
        selected
            .checked_sub(amount.unsigned_abs())
            .unwrap_or(len - 1)
    } else {
        (*selected + amount as usize) % len
    };
}

fn selected_task_id(app: &App) -> Option<String> {
    app.selected_task()
        .and_then(|task| task.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}
fn selected_session_id(app: &App) -> Option<String> {
    app.selected_session()
        .and_then(|instance| instance.get("session_id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}
fn terminal_application_cursor(app: &App) -> bool {
    app.terminal
        .as_ref()
        .and_then(|terminal| terminal.snapshot.as_ref())
        .and_then(|snapshot| snapshot.get("application_cursor"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
fn terminal_rows(app: &App) -> u16 {
    let columns = terminal_columns(app);
    app.height
        .saturating_sub(7)
        .clamp(1, 1_000)
        .min((relayterm_protocol::MAX_DISPLAY_CELLS / usize::from(columns)) as u16)
}
fn terminal_columns(app: &App) -> u16 {
    app.width.saturating_sub(2).clamp(1, 1_000)
}
fn display_params(
    app: &App,
    session_id: &str,
    attachment_id: Option<&str>,
    after_revision: Option<u64>,
    scrollback_rows: u16,
    after_scrollback_offset: Option<u16>,
) -> Value {
    json!({
        "session_id":session_id,
        "attachment_id":attachment_id,
        "after_revision":after_revision,
        "after_scrollback_offset":after_scrollback_offset,
        "top":0,"left":0,
        "scrollback_rows":scrollback_rows,
        "rows":terminal_rows(app),"columns":terminal_columns(app)
    })
}
fn string(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
fn joined(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_wraps_and_small_dimensions_remain_valid() {
        let mut app = App::default();
        previous_screen(&mut app);
        assert_eq!(app.screen, Screen::Help);
        next_screen(&mut app);
        assert_eq!(app.screen, Screen::Overview);
        app.width = 1;
        app.height = 1;
        assert_eq!(terminal_rows(&app), 1);
        assert_eq!(terminal_columns(&app), 1);
    }

    #[test]
    fn task_form_requires_title_and_handover_requires_context() {
        let app = App::default();
        assert!(form_params(&app, &Form::task_create(), "").is_err());
        assert!(form_params(&app, &Form::handover(), "task").is_err());
    }

    #[test]
    fn focus_escape_accepts_terminal_and_crossterm_representations() {
        for character in [']', '5'] {
            assert!(release_input_chord(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::CONTROL
            )));
        }
        assert!(!release_input_chord(KeyEvent::new(
            KeyCode::Char(']'),
            KeyModifiers::NONE
        )));
    }
}
