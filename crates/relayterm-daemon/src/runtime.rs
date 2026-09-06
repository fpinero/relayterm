use crate::{
    DaemonControl, WorkspaceServer, diagnostics::DiagnosticLog, supervisor::SessionSupervisor,
};
use relayterm_application::{Clock, EventNotifier, IdGenerator, Service, Store, Transaction};
use relayterm_client::{Client, ClientError, Delivery};
use relayterm_config::StorageSettings;
use relayterm_domain::{AgentInstanceId, InstanceStatus, Observation, WorkspaceId};
use relayterm_ipc::{Endpoint, LocalListener};
use relayterm_persistence_sqlite::{
    Database, DatabaseKind, InitializationError, OpenMode, PoolSettings, RegistrationState,
    Registry, SqliteStore, StorageError, initialize_workspace,
};
use relayterm_platform::{
    LocationOptions, PrivateLocations, PrivateLock, RandomIdGenerator, SystemClock,
    WorkspaceRootIdentity, decode_native_path, detach_current_process, encode_native_path,
    spawn_detached as spawn_detached_process,
};
use relayterm_protocol::{NativePathDto, WorkspaceId as WireWorkspaceId};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    InvalidLocation,
    InvalidWorkspace,
    WorkspaceNotInitialized,
    RecoveryRequired,
    AccessDenied,
    Busy,
    Storage,
    Transport,
    Protocol,
    Spawn,
    Timeout,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLocation => "The private Relayterm location is invalid.",
            Self::InvalidWorkspace => "The workspace directory is invalid or unavailable.",
            Self::WorkspaceNotInitialized => "The workspace is not initialized. Run `rt workspace init`.",
            Self::RecoveryRequired => "The workspace requires recovery. Preserve its private data and inspect diagnostics.",
            Self::AccessDenied => "Relayterm private state failed its access policy.",
            Self::Busy => "The workspace is busy. Another Relayterm operation may be starting it.",
            Self::Storage => "The workspace storage is unavailable.",
            Self::Transport => "The private workspace endpoint is unavailable.",
            Self::Protocol => "The daemon uses an incompatible or invalid protocol.",
            Self::Spawn => "The Relayterm daemon process could not be started.",
            Self::Timeout => "The daemon did not become ready before the deadline.",
        })
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRoute {
    pub schema_version: u8,
    pub workspace_id: String,
    pub initialized: bool,
    pub already_initialized: bool,
    pub started: bool,
    pub already_running: bool,
}

impl WorkspaceRoute {
    pub fn domain_id(&self) -> Result<WorkspaceId, RuntimeError> {
        self.workspace_id
            .parse()
            .map_err(|_| RuntimeError::Protocol)
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapAction {
    Initialize,
    Open,
    Locate,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapRequest {
    pub schema_version: u8,
    pub action: BootstrapAction,
    pub workspace: NativePathDto,
    pub home: Option<NativePathDto>,
    pub display_name: Option<String>,
    pub timeout_seconds: u64,
}

pub struct DecodedBootstrapRequest {
    pub action: BootstrapAction,
    pub workspace: PathBuf,
    pub home: Option<PathBuf>,
    pub display_name: Option<String>,
    pub timeout: Duration,
}

impl BootstrapRequest {
    pub fn new(
        action: BootstrapAction,
        workspace: &Path,
        home: Option<&Path>,
        display_name: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, RuntimeError> {
        if !(1..=120).contains(&timeout_seconds) {
            return Err(RuntimeError::Protocol);
        }
        Ok(Self {
            schema_version: 1,
            action,
            workspace: encode_path(workspace)?,
            home: home.map(encode_path).transpose()?,
            display_name,
            timeout_seconds,
        })
    }

    pub fn decode(self) -> Result<DecodedBootstrapRequest, RuntimeError> {
        if self.schema_version != 1 || !(1..=120).contains(&self.timeout_seconds) {
            return Err(RuntimeError::Protocol);
        }
        let workspace = decode_path(self.workspace)?;
        let home = self.home.map(decode_path).transpose()?;
        Ok(DecodedBootstrapRequest {
            action: self.action,
            workspace,
            home,
            display_name: self.display_name,
            timeout: Duration::from_secs(self.timeout_seconds),
        })
    }
}

fn encode_path(path: &Path) -> Result<NativePathDto, RuntimeError> {
    let encoded = encode_native_path(path).map_err(|_| RuntimeError::InvalidLocation)?;
    NativePathDto::from_bytes(encoded.tag, &encoded.bytes).map_err(|_| RuntimeError::Protocol)
}

pub fn encode_session_path(path: &Path) -> Result<NativePathDto, RuntimeError> {
    encode_path(path)
}

fn decode_path(path: NativePathDto) -> Result<PathBuf, RuntimeError> {
    let bytes = path.decode().map_err(|_| RuntimeError::Protocol)?;
    decode_native_path(&path.encoding, &bytes).map_err(|_| RuntimeError::Protocol)
}

#[doc(hidden)]
#[derive(Clone)]
pub struct RuntimeNotifier(watch::Sender<u64>);

impl EventNotifier for RuntimeNotifier {
    async fn notify(&self, _: WorkspaceId, revision: u64) -> relayterm_domain::Result<()> {
        let _ = self.0.send(revision);
        Ok(())
    }
}

fn locations(home: Option<PathBuf>) -> Result<PrivateLocations, RuntimeError> {
    PrivateLocations::resolve(LocationOptions::from_process(home))
        .map_err(|_| RuntimeError::InvalidLocation)
}

fn pool_settings(config: Option<&relayterm_config::ParsedConfig>) -> PoolSettings {
    let value = config.map_or(StorageSettings::default(), |parsed| parsed.storage);
    PoolSettings {
        busy_timeout: value.busy_timeout,
        max_connections: u32::from(value.max_connections),
    }
}

fn load_startup_config(
    locations: &PrivateLocations,
) -> Result<Option<relayterm_config::ParsedConfig>, RuntimeError> {
    let path = locations.config().join("config.toml");
    if !path.exists() {
        return Ok(None);
    }
    relayterm_config::load(&path)
        .map(Some)
        .map_err(|_| RuntimeError::RecoveryRequired)
}

pub async fn locate_workspace(
    root: &Path,
    home: Option<PathBuf>,
) -> Result<(WorkspaceRoute, PrivateLocations), RuntimeError> {
    let locations = locations(home)?;
    let identity =
        WorkspaceRootIdentity::resolve(root).map_err(|_| RuntimeError::InvalidWorkspace)?;
    let registry_path = locations.data().join("registry.sqlite3");
    if !registry_path.exists() {
        return Err(RuntimeError::WorkspaceNotInitialized);
    }
    let registry = Registry::open_read_only(&registry_path, PoolSettings::default())
        .await
        .map_err(map_storage)?;
    let registration = registry
        .lookup(&identity)
        .await
        .map_err(map_storage)?
        .ok_or(RuntimeError::WorkspaceNotInitialized)?;
    if registration.state != RegistrationState::Ready {
        return Err(RuntimeError::RecoveryRequired);
    }
    let database = workspace_database_path(&locations, registration.workspace_id);
    if !database.exists() {
        return Err(RuntimeError::RecoveryRequired);
    }
    Ok((
        WorkspaceRoute {
            schema_version: 1,
            workspace_id: registration.workspace_id.to_string(),
            initialized: true,
            already_initialized: true,
            started: false,
            already_running: false,
        },
        locations,
    ))
}

pub async fn initialize(
    root: &Path,
    home: Option<PathBuf>,
    display_name: String,
) -> Result<(WorkspaceRoute, PrivateLocations), RuntimeError> {
    let locations = locations(home)?;
    let config = load_startup_config(&locations)?;
    let initialized = initialize_workspace(
        &locations,
        root,
        display_name,
        pool_settings(config.as_ref()),
        SystemClock,
        RandomIdGenerator::default(),
        RuntimeNotifier(watch::channel(0).0),
    )
    .await
    .map_err(map_initialization)?;
    let route = WorkspaceRoute {
        schema_version: 1,
        workspace_id: initialized.workspace_id.to_string(),
        initialized: true,
        already_initialized: false,
        started: false,
        already_running: false,
    };
    drop(initialized.database);
    Ok((route, locations))
}

pub async fn bootstrap(
    action: BootstrapAction,
    root: &Path,
    home: Option<PathBuf>,
    display_name: Option<String>,
    executable: &Path,
    timeout: Duration,
) -> Result<WorkspaceRoute, RuntimeError> {
    let was_initialized = if matches!(action, BootstrapAction::Initialize) {
        locate_workspace(root, home.clone()).await.is_ok()
    } else {
        true
    };
    let (mut route, locations) = match action {
        BootstrapAction::Initialize => {
            initialize(
                root,
                home.clone(),
                display_name.unwrap_or_else(|| "Relayterm workspace".into()),
            )
            .await?
        }
        BootstrapAction::Open | BootstrapAction::Locate => {
            locate_workspace(root, home.clone()).await?
        }
    };
    route.already_initialized = was_initialized;
    if matches!(action, BootstrapAction::Locate) {
        return Ok(route);
    }
    let id = route.domain_id()?;
    let endpoint = endpoint(&locations, id)?;
    let startup_started = Instant::now();
    let _startup_lock = PrivateLock::acquire(
        &locations
            .runtime()
            .join(format!("workspace-{id}.start.lock")),
        timeout,
    )
    .map_err(|error| match error {
        relayterm_platform::LockError::Busy => RuntimeError::Busy,
        relayterm_platform::LockError::AccessDenied => RuntimeError::AccessDenied,
        relayterm_platform::LockError::Unavailable => RuntimeError::Transport,
    })?;
    if probe(&endpoint, id).await? {
        route.already_running = true;
        return Ok(route);
    }
    spawn_detached(executable, root, home.as_deref(), id)?;
    while startup_started.elapsed() < timeout {
        if probe(&endpoint, id).await? {
            route.started = true;
            return Ok(route);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(RuntimeError::Timeout)
}

pub async fn connect_route(
    route: &WorkspaceRoute,
    home: Option<PathBuf>,
) -> Result<Client, RuntimeError> {
    let locations = locations(home)?;
    let id = route.domain_id()?;
    tokio::time::timeout(
        Duration::from_secs(2),
        Client::connect(
            &endpoint(&locations, id)?,
            WireWorkspaceId::from_uuid(id.as_uuid()),
        ),
    )
    .await
    .map_err(|_| RuntimeError::Transport)?
    .map_err(map_client)
}

pub async fn run_workspace(
    root: &Path,
    home: Option<PathBuf>,
    expected: WorkspaceId,
) -> Result<(), RuntimeError> {
    detach_current_process().map_err(|_| RuntimeError::Spawn)?;
    serve_workspace(root, home, expected).await
}

/// Serve an already detached workspace process. Process creation stays in the platform boundary.
pub async fn serve_workspace(
    root: &Path,
    home: Option<PathBuf>,
    expected: WorkspaceId,
) -> Result<(), RuntimeError> {
    prepare_workspace(root, home, expected).await?.run().await
}

/// Prepared production runtime composition. Exposed for process-level integration tests and
/// the future supervisor adapter; normal clients cannot inject lifecycle observations.
pub struct PreparedWorkspace {
    expected: WorkspaceId,
    diagnostics: DiagnosticLog,
    database: Database,
    store: SqliteStore,
    service: Arc<Service<SqliteStore, SystemClock, RandomIdGenerator, RuntimeNotifier>>,
    notify_rx: watch::Receiver<u64>,
    listener: LocalListener,
}

impl PreparedWorkspace {
    #[doc(hidden)]
    pub fn service(
        &self,
    ) -> Arc<Service<SqliteStore, SystemClock, RandomIdGenerator, RuntimeNotifier>> {
        self.service.clone()
    }

    pub async fn run(self) -> Result<(), RuntimeError> {
        self.run_with_ready(|| {}).await
    }

    #[doc(hidden)]
    pub async fn run_with_ready<F>(self, ready: F) -> Result<(), RuntimeError>
    where
        F: FnOnce(),
    {
        let (control, shutdown) = DaemonControl::new(
            RandomIdGenerator::default()
                .next()
                .map_err(|_| RuntimeError::Spawn)?
                .to_string(),
        );
        let supervisor = Arc::new(SessionSupervisor::default());
        let server = WorkspaceServer::from_listener(
            self.expected,
            self.listener,
            self.service.clone(),
            self.store,
        )
        .with_event_wakeups(self.notify_rx)
        .with_lifecycle(control.clone())
        .with_supervisor(supervisor.clone());
        self.diagnostics.write(
            "info",
            "daemon.ready",
            self.expected,
            Some(control.generation()),
        );
        ready();
        let signal_control = control.clone();
        let signal = tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                signal_control.request_shutdown();
            }
        });
        let result = server.run(shutdown).await;
        signal.abort();
        result.map_err(|_| RuntimeError::Transport)?;
        supervisor.terminate_all();
        let cleanup_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while supervisor.live_count() != 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            for exit in supervisor.poll_exits() {
                let observed_at = SystemClock.now().map_err(|_| RuntimeError::Storage)?;
                self.service
                    .observe(
                        self.expected,
                        Observation::Status {
                            id: AgentInstanceId::from_uuid(exit.instance_id),
                            status: if exit.terminated {
                                InstanceStatus::Terminated
                            } else {
                                InstanceStatus::Exited
                            },
                            observed_at,
                            exit_code: Some(exit.code),
                        },
                    )
                    .await
                    .map_err(|_| RuntimeError::Storage)?;
            }
            supervisor.terminate_all();
            if tokio::time::Instant::now() >= cleanup_deadline {
                return Err(RuntimeError::Timeout);
            }
        }
        self.diagnostics.write(
            "info",
            "daemon.stopped",
            self.expected,
            Some(control.generation()),
        );
        drop(self.database);
        Ok(())
    }
}

/// Open and reconcile a workspace only after acquiring exclusive runtime ownership.
pub async fn prepare_workspace(
    root: &Path,
    home: Option<PathBuf>,
    expected: WorkspaceId,
) -> Result<PreparedWorkspace, RuntimeError> {
    let (route, locations) = locate_workspace(root, home).await?;
    if route.domain_id()? != expected {
        return Err(RuntimeError::InvalidWorkspace);
    }
    let endpoint = endpoint(&locations, expected)?;
    let listener = LocalListener::bind(&endpoint)
        .await
        .map_err(|_| RuntimeError::Busy)?;
    let diagnostics =
        DiagnosticLog::open(&locations, expected).map_err(|_| RuntimeError::AccessDenied)?;
    diagnostics.write("info", "daemon.starting", expected, None);
    let config = load_startup_config(&locations)?;
    let database = Database::open(
        &workspace_database_path(&locations, expected),
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        pool_settings(config.as_ref()),
    )
    .await
    .map_err(map_storage)?;
    let store = SqliteStore::new(database.pool().clone());
    let (notify_tx, notify_rx) = watch::channel(0);
    let service = Arc::new(Service::new(
        store.clone(),
        SystemClock,
        RandomIdGenerator::default(),
        RuntimeNotifier(notify_tx),
    ));
    let snapshot = store
        .begin(expected)
        .await
        .map_err(|_| RuntimeError::Storage)?;
    if snapshot
        .snapshot()
        .state()
        .map_err(|_| RuntimeError::RecoveryRequired)?
        .instances()
        .iter()
        .any(|instance| !instance.record().status.is_final())
        && service
            .observe(expected, Observation::ReconcileLost)
            .await
            .is_err()
    {
        diagnostics.write("error", "daemon.recovery_failed", expected, None);
        return Err(RuntimeError::RecoveryRequired);
    }
    Ok(PreparedWorkspace {
        expected,
        diagnostics,
        database,
        store,
        service,
        notify_rx,
        listener,
    })
}

fn workspace_database_path(locations: &PrivateLocations, id: WorkspaceId) -> PathBuf {
    locations
        .data()
        .join("workspaces")
        .join(id.to_string())
        .join("workspace.sqlite3")
}

fn endpoint(locations: &PrivateLocations, id: WorkspaceId) -> Result<Endpoint, RuntimeError> {
    Endpoint::derive(
        locations.runtime(),
        WireWorkspaceId::from_uuid(id.as_uuid()),
    )
    .map_err(|_| RuntimeError::Transport)
}

async fn probe(endpoint: &Endpoint, id: WorkspaceId) -> Result<bool, RuntimeError> {
    let result = tokio::time::timeout(
        Duration::from_millis(500),
        Client::connect(endpoint, WireWorkspaceId::from_uuid(id.as_uuid())),
    )
    .await;
    match result {
        Err(_) => Ok(false),
        Ok(Ok(_)) => Ok(true),
        Ok(Err(ClientError::Transport(Delivery::NotSent))) => Ok(false),
        Ok(Err(ClientError::Cancelled(Delivery::NotSent))) => Ok(false),
        Ok(Err(error)) => Err(map_client(error)),
    }
}

fn map_client(error: ClientError) -> RuntimeError {
    match error {
        ClientError::VersionMismatch
        | ClientError::WorkspaceMismatch
        | ClientError::Protocol
        | ClientError::Rejected(_) => RuntimeError::Protocol,
        ClientError::Transport(_) | ClientError::Cancelled(_) | ClientError::ResourceLimit => {
            RuntimeError::Transport
        }
    }
}

fn spawn_detached(
    executable: &Path,
    root: &Path,
    home: Option<&Path>,
    id: WorkspaceId,
) -> Result<(), RuntimeError> {
    let mut arguments = vec![
        OsString::from("__daemon-run"),
        OsString::from("--root"),
        root.as_os_str().to_owned(),
        OsString::from("--workspace-id"),
        OsString::from(id.to_string()),
    ];
    if let Some(home) = home {
        arguments.push(OsString::from("--private-home"));
        arguments.push(home.as_os_str().to_owned());
    }
    spawn_detached_process(executable, &arguments).map_err(|_| RuntimeError::Spawn)
}

fn map_storage(error: StorageError) -> RuntimeError {
    match error {
        StorageError::NotFound => RuntimeError::WorkspaceNotInitialized,
        StorageError::AccessDenied => RuntimeError::AccessDenied,
        StorageError::Busy => RuntimeError::Busy,
        StorageError::IncompatibleVersion | StorageError::Integrity | StorageError::Migration => {
            RuntimeError::RecoveryRequired
        }
        _ => RuntimeError::Storage,
    }
}

fn map_initialization(error: InitializationError) -> RuntimeError {
    match error {
        InitializationError::Location => RuntimeError::InvalidLocation,
        InitializationError::Identity => RuntimeError::InvalidWorkspace,
        InitializationError::Busy => RuntimeError::Busy,
        InitializationError::RecoveryRequired => RuntimeError::RecoveryRequired,
        InitializationError::Storage => RuntimeError::Storage,
        InitializationError::Domain => RuntimeError::RecoveryRequired,
    }
}
