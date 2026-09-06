use clap::{Args, Parser, Subcommand, ValueEnum};
use relayterm_client::{ClientError, Delivery};
use relayterm_daemon::{BootstrapAction, BootstrapRequest, RuntimeError, WorkspaceRoute};
use relayterm_protocol::{ErrorCode, JSON_FRAME_LIMIT, Operation, decode_json};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, ExitCode, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const MAX_INPUT: usize = JSON_FRAME_LIMIT;
const MAX_BOOTSTRAP: usize = 64 * 1024;

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Parser)]
#[command(
    name = "rt",
    bin_name = "rt",
    version,
    about = "A persistent, agent-neutral development workspace"
)]
struct Cli {
    #[arg(long, global = true)]
    workspace: Option<PathBuf>,
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    #[arg(long, global = true, value_enum, default_value = "human")]
    format: OutputFormat,
    #[arg(long, global = true, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=120))]
    timeout: u64,
    #[command(subcommand)]
    command: Option<TopCommand>,
}

#[derive(Subcommand)]
enum TopCommand {
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Progress {
        #[command(subcommand)]
        command: ProgressCommand,
    },
    Handover {
        #[command(subcommand)]
        command: HandoverCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Event {
        #[command(subcommand)]
        command: EventCommand,
    },
    #[command(name = "__bootstrap", hide = true)]
    InternalBootstrap,
    #[command(name = "__daemon-run", hide = true)]
    InternalDaemon(InternalDaemon),
}

#[derive(Subcommand)]
enum WorkspaceCommand {
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    Open,
    Status,
}
#[derive(Subcommand)]
enum DaemonCommand {
    Start,
    Status,
    Stop,
}

#[derive(Args)]
struct InputArgs {
    #[arg(long, conflicts_with = "stdin")]
    file: Option<PathBuf>,
    #[arg(long)]
    stdin: bool,
}

#[derive(Args)]
struct PageArgs {
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=200))]
    limit: u16,
    #[arg(long)]
    after: Option<String>,
    #[arg(long)]
    expected_revision: Option<String>,
}

#[derive(Subcommand)]
enum AgentCommand {
    List(PageArgs),
    Register {
        #[arg(long)]
        expected_revision: String,
        #[command(flatten)]
        input: InputArgs,
    },
    Update {
        definition_id: String,
        #[arg(long)]
        expected_revision: String,
        #[command(flatten)]
        input: InputArgs,
    },
    Import {
        #[arg(long)]
        expected_revision: Option<String>,
        #[command(flatten)]
        input: InputArgs,
    },
}

#[derive(Subcommand)]
enum TaskCommand {
    Create {
        #[arg(long)]
        expected_revision: String,
        #[command(flatten)]
        input: InputArgs,
    },
    List(PageArgs),
    Get {
        task_id: String,
        #[arg(long)]
        expected_revision: Option<String>,
    },
    Update {
        task_id: String,
        #[arg(long)]
        expected_revision: String,
        #[command(flatten)]
        input: InputArgs,
    },
    Transition {
        task_id: String,
        status: String,
        #[arg(long)]
        expected_revision: String,
    },
    Claim {
        task_id: String,
        #[arg(long)]
        instance: String,
    },
    Release {
        task_id: String,
        #[arg(long)]
        expected_revision: String,
    },
    History {
        task_id: String,
        #[command(flatten)]
        page: PageArgs,
    },
    Claims {
        task_id: String,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Subcommand)]
enum ProgressCommand {
    Append {
        task_id: String,
        #[command(flatten)]
        input: InputArgs,
    },
}
#[derive(Subcommand)]
enum HandoverCommand {
    Create {
        task_id: String,
        #[arg(long)]
        expected_revision: String,
        #[command(flatten)]
        input: InputArgs,
    },
    Get {
        handover_id: String,
    },
}
#[derive(Subcommand)]
enum SessionCommand {
    List(PageArgs),
}
#[derive(Subcommand)]
enum EventCommand {
    List(PageArgs),
    Watch {
        #[arg(long, default_value = "0")]
        after: String,
    },
}

#[derive(Args)]
struct InternalDaemon {
    #[arg(long = "root")]
    workspace: PathBuf,
    #[arg(long)]
    workspace_id: String,
    #[arg(long = "private-home")]
    home: Option<PathBuf>,
}

#[derive(Debug)]
enum CliError {
    Tui,
    Usage(&'static str),
    Runtime(RuntimeError),
    Client(ClientError),
    InvalidInput,
    Io,
    Interrupted,
}

impl CliError {
    fn code(&self) -> u8 {
        match self {
            Self::Tui => 1,
            Self::Usage(_) | Self::InvalidInput => 2,
            Self::Interrupted => 130,
            Self::Runtime(RuntimeError::WorkspaceNotInitialized) => 3,
            Self::Runtime(
                RuntimeError::Transport | RuntimeError::Spawn | RuntimeError::Timeout,
            ) => 4,
            Self::Runtime(
                RuntimeError::AccessDenied
                | RuntimeError::RecoveryRequired
                | RuntimeError::Protocol,
            ) => 6,
            Self::Client(
                ClientError::Transport(Delivery::Unknown)
                | ClientError::Cancelled(Delivery::Unknown),
            ) => 7,
            Self::Client(ClientError::Rejected(ErrorCode::InvalidReference)) => 3,
            Self::Client(ClientError::Rejected(_)) => 5,
            Self::Client(
                ClientError::VersionMismatch
                | ClientError::WorkspaceMismatch
                | ClientError::Protocol,
            ) => 6,
            Self::Client(ClientError::Transport(_) | ClientError::Cancelled(_)) => 4,
            Self::Runtime(RuntimeError::Storage) => 8,
            _ => 1,
        }
    }
    fn machine_code(&self) -> &'static str {
        match self {
            Self::Tui => "tui_unavailable",
            Self::Usage(_) => "invalid_usage",
            Self::InvalidInput => "invalid_input",
            Self::Io => "input_unavailable",
            Self::Interrupted => "user_interrupted",
            Self::Runtime(RuntimeError::WorkspaceNotInitialized) => "workspace_not_initialized",
            Self::Runtime(RuntimeError::Timeout) => "daemon_timeout",
            Self::Runtime(RuntimeError::Protocol) => "protocol_incompatible",
            Self::Runtime(RuntimeError::RecoveryRequired) => "recovery_required",
            Self::Runtime(RuntimeError::AccessDenied) => "access_denied",
            Self::Runtime(RuntimeError::Busy) => "workspace_busy",
            Self::Runtime(RuntimeError::Storage) => "storage_error",
            Self::Runtime(RuntimeError::Spawn) => "daemon_spawn_failed",
            Self::Runtime(RuntimeError::Transport) => "daemon_unavailable",
            Self::Runtime(RuntimeError::InvalidLocation) => "invalid_location",
            Self::Runtime(RuntimeError::InvalidWorkspace) => "invalid_workspace",
            Self::Client(
                ClientError::Transport(Delivery::Unknown)
                | ClientError::Cancelled(Delivery::Unknown),
            ) => "result_unknown",
            Self::Client(ClientError::Rejected(ErrorCode::InvalidReference)) => "entity_not_found",
            Self::Client(ClientError::Rejected(_)) => "request_rejected",
            Self::Client(ClientError::VersionMismatch) => "protocol_incompatible",
            Self::Client(ClientError::WorkspaceMismatch) => "workspace_mismatch",
            Self::Client(ClientError::Protocol) => "invalid_response",
            Self::Client(ClientError::ResourceLimit) => "resource_limit",
            Self::Client(ClientError::Transport(_) | ClientError::Cancelled(_)) => {
                "transport_error"
            }
        }
    }
    fn message(&self) -> String {
        match self {
            Self::Tui => "The TUI is not implemented yet.".into(),
            Self::Usage(v) => (*v).into(),
            Self::InvalidInput => "The command input is invalid.".into(),
            Self::Io => "The command input could not be read.".into(),
            Self::Interrupted => "The command was interrupted by the user.".into(),
            Self::Runtime(v) => v.to_string(),
            Self::Client(v) => v.to_string(),
        }
    }
}
impl From<RuntimeError> for CliError {
    fn from(v: RuntimeError) -> Self {
        Self::Runtime(v)
    }
}
impl From<ClientError> for CliError {
    fn from(v: ClientError) -> Self {
        Self::Client(v)
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            let _ = error.print();
            return ExitCode::SUCCESS;
        }
        Err(_) => {
            eprintln!("invalid_usage: The command arguments are invalid.");
            return ExitCode::from(2);
        }
    };
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| CliError::Io)
        .and_then(|runtime| runtime.block_on(run(&cli)));
    match result {
        Ok(value) => {
            render_success(cli.format, command_name(cli.command.as_ref()), &value);
            ExitCode::SUCCESS
        }
        Err(error) => {
            render_error(cli.format, command_name(cli.command.as_ref()), &error);
            ExitCode::from(error.code())
        }
    }
}

async fn run(cli: &Cli) -> Result<Value, CliError> {
    let Some(command) = &cli.command else {
        return relayterm_tui::run()
            .map(|_| json!({}))
            .map_err(|_| CliError::Tui);
    };
    if let TopCommand::InternalBootstrap = command {
        let input: BootstrapRequest =
            decode_json(read_limited(io::stdin(), MAX_BOOTSTRAP)?.as_slice(), false)
                .map_err(|_| CliError::InvalidInput)?;
        let input = input.decode()?;
        let executable = std::env::current_exe().map_err(|_| CliError::Io)?;
        return serde_json::to_value(
            relayterm_daemon::bootstrap(
                input.action,
                &input.workspace,
                input.home,
                input.display_name,
                &executable,
                input.timeout,
            )
            .await?,
        )
        .map_err(|_| CliError::InvalidInput);
    }
    if let TopCommand::InternalDaemon(input) = command {
        let id = input
            .workspace_id
            .parse()
            .map_err(|_| CliError::InvalidInput)?;
        relayterm_daemon::run_workspace(&input.workspace, input.home.clone(), id).await?;
        return Ok(json!({"stopped":true}));
    }
    let root = cli
        .workspace
        .clone()
        .map_or_else(|| std::env::current_dir().map_err(|_| CliError::Io), Ok)?;
    match command {
        TopCommand::Workspace {
            command: WorkspaceCommand::Init { name },
        } => route_value(invoke_bootstrap(
            "initialize",
            &root,
            cli.home.as_deref(),
            name.as_deref(),
            cli.timeout,
        )?),
        TopCommand::Workspace {
            command: WorkspaceCommand::Open,
        }
        | TopCommand::Daemon {
            command: DaemonCommand::Start,
        } => route_value(invoke_bootstrap(
            "open",
            &root,
            cli.home.as_deref(),
            None,
            cli.timeout,
        )?),
        TopCommand::Workspace {
            command: WorkspaceCommand::Status,
        }
        | TopCommand::Daemon {
            command: DaemonCommand::Status,
        } => status(cli, &root).await,
        TopCommand::Daemon {
            command: DaemonCommand::Stop,
        } => stop(cli, &root).await,
        TopCommand::Event {
            command: EventCommand::Watch { after },
        } => watch_events(cli, &root, after).await,
        _ => dispatch_admin(cli, &root, command).await,
    }
}

async fn watch_events(cli: &Cli, root: &Path, after: &str) -> Result<Value, CliError> {
    let cursor = after.parse::<u64>().map_err(|_| CliError::InvalidInput)?;
    let route = invoke_bootstrap("locate", root, cli.home.as_deref(), None, cli.timeout)?;
    let client = relayterm_daemon::connect_route(&route, cli.home.clone()).await?;
    let accepted = client.subscribe(cursor).await?;
    render_stream(cli.format, "subscribed", &accepted);
    loop {
        tokio::select! {
            event = client.next_event() => render_stream(cli.format, "event", &event?),
            interrupted = tokio::signal::ctrl_c() => {
                let _ = interrupted;
                client.unsubscribe().await?;
                return Err(CliError::Interrupted);
            }
        }
    }
}

fn route_value(route: WorkspaceRoute) -> Result<Value, CliError> {
    serde_json::to_value(route).map_err(|_| CliError::InvalidInput)
}

fn invoke_bootstrap(
    action: &str,
    root: &Path,
    home: Option<&Path>,
    name: Option<&str>,
    timeout: u64,
) -> Result<WorkspaceRoute, CliError> {
    let operation_started = Instant::now();
    let mut command = ProcessCommand::new(std::env::current_exe().map_err(|_| CliError::Io)?);
    command
        .arg("__bootstrap")
        .arg("--format")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let request = BootstrapRequest::new(
        match action {
            "initialize" => BootstrapAction::Initialize,
            "open" => BootstrapAction::Open,
            "locate" => BootstrapAction::Locate,
            _ => return Err(CliError::InvalidInput),
        },
        root,
        home,
        name.map(str::to_owned),
        timeout,
    )?;
    let request = serde_json::to_vec(&request).map_err(|_| CliError::InvalidInput)?;
    if request.len() > MAX_BOOTSTRAP {
        return Err(CliError::InvalidInput);
    }
    let mut child = command
        .spawn()
        .map_err(|_| CliError::Runtime(RuntimeError::Spawn))?;
    child
        .stdin
        .take()
        .ok_or(CliError::Io)?
        .write_all(&request)
        .map_err(|_| CliError::Runtime(RuntimeError::Transport))?;
    let stdout = child.stdout.take().ok_or(CliError::Io)?;
    let (response_tx, response_rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut response = Vec::new();
        let result = BufReader::new(stdout)
            .take((MAX_BOOTSTRAP + 1) as u64)
            .read_until(b'\n', &mut response)
            .map(|_| response);
        let _ = response_tx.send(result);
    });
    let operation_limit = Duration::from_secs(timeout.saturating_add(2));
    let stdout = match response_rx.recv_timeout(operation_limit) {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => return Err(CliError::Runtime(RuntimeError::Transport)),
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CliError::Runtime(RuntimeError::Timeout));
        }
    };
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| CliError::Runtime(RuntimeError::Transport))?
        {
            break status;
        }
        if operation_started.elapsed() >= operation_limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CliError::Runtime(RuntimeError::Timeout));
        }
        thread::sleep(Duration::from_millis(10));
    };
    if stdout.len() > MAX_BOOTSTRAP {
        return Err(CliError::InvalidInput);
    }
    let envelope: Value =
        serde_json::from_slice(&stdout).map_err(|_| CliError::Runtime(RuntimeError::Protocol))?;
    if !status.success() {
        return Err(map_bootstrap_error(&envelope));
    }
    serde_json::from_value(
        envelope
            .get("result")
            .cloned()
            .ok_or(CliError::InvalidInput)?,
    )
    .map_err(|_| CliError::InvalidInput)
}

fn map_bootstrap_error(value: &Value) -> CliError {
    match value.pointer("/error/code").and_then(Value::as_str) {
        Some("workspace_not_initialized") => RuntimeError::WorkspaceNotInitialized.into(),
        Some("recovery_required") => RuntimeError::RecoveryRequired.into(),
        Some("access_denied") => RuntimeError::AccessDenied.into(),
        Some("workspace_busy") => RuntimeError::Busy.into(),
        Some("daemon_timeout") => RuntimeError::Timeout.into(),
        Some("invalid_workspace") => RuntimeError::InvalidWorkspace.into(),
        Some("storage_error") => RuntimeError::Storage.into(),
        Some("invalid_location") => RuntimeError::InvalidLocation.into(),
        Some("daemon_spawn_failed") => RuntimeError::Spawn.into(),
        Some("protocol_incompatible") => RuntimeError::Protocol.into(),
        Some("invalid_input") | Some("invalid_usage") => CliError::InvalidInput,
        _ => RuntimeError::Transport.into(),
    }
}

async fn status(cli: &Cli, root: &Path) -> Result<Value, CliError> {
    let route = invoke_bootstrap("locate", root, cli.home.as_deref(), None, cli.timeout)?;
    match relayterm_daemon::connect_route(&route, cli.home.clone()).await {
        Ok(client) => client
            .call(Operation::DaemonStatus, &json!({}))
            .await
            .map_err(Into::into),
        Err(RuntimeError::Transport) => {
            Ok(json!({"workspace_id":route.workspace_id,"lifecycle":"stopped"}))
        }
        Err(error) => Err(error.into()),
    }
}

async fn stop(cli: &Cli, root: &Path) -> Result<Value, CliError> {
    let route = invoke_bootstrap("locate", root, cli.home.as_deref(), None, cli.timeout)?;
    let client = match relayterm_daemon::connect_route(&route, cli.home.clone()).await {
        Ok(v) => v,
        Err(RuntimeError::Transport) => {
            return Ok(json!({"workspace_id":route.workspace_id,"lifecycle":"stopped"}));
        }
        Err(e) => return Err(e.into()),
    };
    let state: Value = client.call(Operation::DaemonStatus, &json!({})).await?;
    let generation = state["generation"]
        .as_str()
        .ok_or(CliError::InvalidInput)?
        .to_owned();
    let _: Value = client
        .call(Operation::DaemonShutdown, &json!({"generation":generation}))
        .await?;
    let deadline = Instant::now() + Duration::from_secs(cli.timeout);
    while Instant::now() < deadline {
        if relayterm_daemon::connect_route(&route, cli.home.clone())
            .await
            .is_err()
        {
            return Ok(
                json!({"workspace_id":route.workspace_id,"generation":generation,"lifecycle":"stopped"}),
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(RuntimeError::Timeout.into())
}

async fn dispatch_admin(cli: &Cli, root: &Path, command: &TopCommand) -> Result<Value, CliError> {
    let route = invoke_bootstrap("locate", root, cli.home.as_deref(), None, cli.timeout)?;
    let client = relayterm_daemon::connect_route(&route, cli.home.clone()).await?;
    let (operation, params) = operation_and_params(command)?;
    client.call(operation, &params).await.map_err(Into::into)
}

fn operation_and_params(command: &TopCommand) -> Result<(Operation, Value), CliError> {
    Ok(match command {
        TopCommand::Agent {
            command: AgentCommand::List(page),
        } => (Operation::AgentListDefinitions, page_params(page)),
        TopCommand::Agent {
            command:
                AgentCommand::Register {
                    expected_revision,
                    input,
                },
        } => (
            Operation::AgentRegisterDefinition,
            extend(
                read_json(input)?,
                &[("expected_revision", expected_revision)],
            )?,
        ),
        TopCommand::Agent {
            command:
                AgentCommand::Update {
                    definition_id,
                    expected_revision,
                    input,
                },
        } => (
            Operation::AgentUpdateDefinition,
            extend(
                read_json(input)?,
                &[
                    ("definition_id", definition_id),
                    ("expected_revision", expected_revision),
                ],
            )?,
        ),
        TopCommand::Agent {
            command:
                AgentCommand::Import {
                    expected_revision,
                    input,
                },
        } => {
            let mut value = json!({"document":read_text(input, 1024*1024)?});
            if let Some(v) = expected_revision {
                value["expected_revision"] = Value::String(v.clone())
            };
            (Operation::AgentImportDefinitions, value)
        }
        TopCommand::Task {
            command:
                TaskCommand::Create {
                    expected_revision,
                    input,
                },
        } => (
            Operation::TaskCreate,
            extend(
                read_json(input)?,
                &[("expected_revision", expected_revision)],
            )?,
        ),
        TopCommand::Task {
            command: TaskCommand::List(page),
        } => (Operation::TaskList, page_params(page)),
        TopCommand::Task {
            command:
                TaskCommand::Get {
                    task_id,
                    expected_revision,
                },
        } => {
            let mut value = json!({"task_id":task_id});
            if let Some(v) = expected_revision {
                value["expected_revision"] = Value::String(v.clone())
            };
            (Operation::TaskGet, value)
        }
        TopCommand::Task {
            command:
                TaskCommand::Update {
                    task_id,
                    expected_revision,
                    input,
                },
        } => (
            Operation::TaskUpdate,
            extend(
                read_json(input)?,
                &[
                    ("task_id", task_id),
                    ("expected_revision", expected_revision),
                ],
            )?,
        ),
        TopCommand::Task {
            command:
                TaskCommand::Transition {
                    task_id,
                    status,
                    expected_revision,
                },
        } => (
            Operation::TaskTransition,
            json!({"task_id":task_id,"status":status,"expected_revision":expected_revision}),
        ),
        TopCommand::Task {
            command: TaskCommand::Claim { task_id, instance },
        } => (
            Operation::TaskClaim,
            json!({"task_id":task_id,"instance_id":instance}),
        ),
        TopCommand::Task {
            command:
                TaskCommand::Release {
                    task_id,
                    expected_revision,
                },
        } => (
            Operation::TaskRelease,
            json!({"task_id":task_id,"expected_revision":expected_revision}),
        ),
        TopCommand::Task {
            command: TaskCommand::History { task_id, page },
        } => (Operation::TaskGetHistory, history_params(task_id, page)?),
        TopCommand::Task {
            command: TaskCommand::Claims { task_id, page },
        } => (
            Operation::TaskGetClaimHistory,
            history_params(task_id, page)?,
        ),
        TopCommand::Progress {
            command: ProgressCommand::Append { task_id, input },
        } => (
            Operation::ProgressAppend,
            extend(read_json(input)?, &[("task_id", task_id)])?,
        ),
        TopCommand::Handover {
            command:
                HandoverCommand::Create {
                    task_id,
                    expected_revision,
                    input,
                },
        } => (
            Operation::HandoverCreate,
            extend(
                read_json(input)?,
                &[
                    ("task_id", task_id),
                    ("expected_revision", expected_revision),
                ],
            )?,
        ),
        TopCommand::Handover {
            command: HandoverCommand::Get { handover_id },
        } => (Operation::HandoverGet, json!({"handover_id":handover_id})),
        TopCommand::Session {
            command: SessionCommand::List(page),
        } => (Operation::SessionList, page_params(page)),
        TopCommand::Event {
            command: EventCommand::List(page),
        } => (
            Operation::EventList,
            json!({"after_sequence":page.after.clone().unwrap_or_else(||"0".into()),"limit":page.limit}),
        ),
        TopCommand::Event {
            command: EventCommand::Watch { after },
        } => (
            Operation::EventList,
            json!({"after_sequence":after,"limit":200}),
        ),
        _ => return Err(CliError::InvalidInput),
    })
}

fn page_params(page: &PageArgs) -> Value {
    let mut value = json!({"limit":page.limit});
    if let Some(v) = &page.after {
        value["after_id"] = Value::String(v.clone())
    };
    if let Some(v) = &page.expected_revision {
        value["expected_revision"] = Value::String(v.clone())
    };
    value
}
fn history_params(task_id: &str, page: &PageArgs) -> Result<Value, CliError> {
    Ok(
        json!({"task_id":task_id,"after_sequence":page.after.clone().unwrap_or_else(||"0".into()),"expected_revision":page.expected_revision.clone().ok_or(CliError::Usage("--expected-revision is required"))?,"limit":page.limit}),
    )
}
fn read_json(input: &InputArgs) -> Result<Value, CliError> {
    decode_json(read_text(input, MAX_INPUT)?.as_bytes(), false).map_err(|_| CliError::InvalidInput)
}
fn read_text(input: &InputArgs, limit: usize) -> Result<String, CliError> {
    if input.file.is_none() && !input.stdin {
        return Err(CliError::Usage("Specify --file or --stdin."));
    }
    let reader: Box<dyn Read> = match &input.file {
        Some(path) => Box::new(File::open(path).map_err(|_| CliError::Io)?),
        None => Box::new(io::stdin()),
    };
    let bytes = read_limited(reader, limit)?;
    String::from_utf8(bytes).map_err(|_| CliError::InvalidInput)
}
fn read_limited(reader: impl Read, limit: usize) -> Result<Vec<u8>, CliError> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CliError::Io)?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(CliError::InvalidInput);
    }
    Ok(bytes)
}
fn extend(mut value: Value, fields: &[(&str, &String)]) -> Result<Value, CliError> {
    let object = value.as_object_mut().ok_or(CliError::InvalidInput)?;
    for (name, field) in fields {
        if object
            .insert((*name).into(), Value::String((*field).clone()))
            .is_some()
        {
            return Err(CliError::InvalidInput);
        }
    }
    Ok(value)
}

fn render_success(format: OutputFormat, command: &str, result: &Value) {
    match format {
        OutputFormat::Json => println!(
            "{}",
            json!({"schema_version":1,"command":command,"ok":true,"result":result})
        ),
        OutputFormat::Human => println!(
            "{}",
            escape_human(&serde_json::to_string_pretty(result).unwrap_or_default())
        ),
    }
}
fn render_error(format: OutputFormat, command: &str, error: &CliError) {
    let value = json!({"code":error.machine_code(),"message":error.message()});
    match format {
        OutputFormat::Json => println!(
            "{}",
            json!({"schema_version":1,"command":command,"ok":false,"error":value})
        ),
        OutputFormat::Human => eprintln!("{}: {}", error.machine_code(), error.message()),
    }
}

fn render_stream(format: OutputFormat, record_type: &str, value: &Value) {
    match format {
        OutputFormat::Json => println!(
            "{}",
            json!({"schema_version":1,"record_type":record_type,"value":value})
        ),
        OutputFormat::Human => println!(
            "{}",
            escape_human(&serde_json::to_string(value).unwrap_or_default())
        ),
    }
}
fn escape_human(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
        .collect()
}
fn command_name(command: Option<&TopCommand>) -> &'static str {
    match command {
        Some(TopCommand::Workspace { .. }) => "workspace",
        Some(TopCommand::Daemon { .. }) => "daemon",
        Some(TopCommand::Agent { .. }) => "agent",
        Some(TopCommand::Task { .. }) => "task",
        Some(TopCommand::Progress { .. }) => "progress",
        Some(TopCommand::Handover { .. }) => "handover",
        Some(TopCommand::Session { .. }) => "session",
        Some(TopCommand::Event { .. }) => "event",
        Some(TopCommand::InternalBootstrap) => "bootstrap",
        Some(TopCommand::InternalDaemon(_)) => "daemon_internal",
        None => "tui",
    }
}
