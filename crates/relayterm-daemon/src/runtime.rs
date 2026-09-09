use crate::{
    DaemonControl, WorkspaceServer, diagnostics::DiagnosticLog, supervisor::SessionSupervisor,
};
use relayterm_application::{
    Clock, DurableReadStore, EventNotifier, IdGenerator, Service, Store, Transaction,
};
use relayterm_client::{Client, ClientError, Delivery};
use relayterm_config::StorageSettings;
use relayterm_domain::{AgentInstanceId, InstanceStatus, Observation, WorkspaceId};
use relayterm_ipc::{Endpoint, LocalListener};
use relayterm_persistence_sqlite::{
    Database, DatabaseKind, InitializationError, OpenMode, PoolSettings, RegistrationState,
    Registry, SqliteStore, StorageError, WORKSPACE_SCHEMA_VERSION, initialize_workspace,
};
use relayterm_platform::{
    LocationAlias, LocationOptions, PrivateLocations, PrivateLock, RandomIdGenerator, SystemClock,
    WorkspaceRootIdentity, create_private_dir, create_private_file, decode_native_path,
    detach_current_process, encode_native_path, spawn_detached as spawn_detached_process,
    validate_private_dir, validate_private_file,
};
use relayterm_protocol::{NativePathDto, WorkspaceId as WireWorkspaceId};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fmt,
    fs::File,
    io::{Read, Write},
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

const BACKUP_MANIFEST: &str = "manifest.json";
const BACKUP_WORKSPACE: &str = "workspace.sqlite3";
const MAX_BACKUP_MANIFEST: u64 = 64 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupManifest {
    format_version: u8,
    application_version: String,
    workspace_schema_version: i64,
    workspace_id: String,
    database_member: String,
    workspace_revision: String,
    last_event_sequence: String,
    blake3: String,
    complete: bool,
}

#[derive(Serialize)]
pub struct BackupReport {
    pub format_version: u8,
    pub application_version: String,
    pub workspace_schema_version: i64,
    pub workspace_id: String,
    pub workspace_revision: String,
    pub last_event_sequence: String,
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
    let mut launched = spawn_detached(executable, root, home.as_deref(), id)?;
    while startup_started.elapsed() < timeout {
        match probe(&endpoint, id).await {
            Ok(true) => {
                route.started = true;
                return Ok(route);
            }
            Ok(false) => {}
            Err(error) => {
                stop_failed_launch(&mut launched);
                return Err(error);
            }
        }
        match launched.try_wait() {
            Ok(Some(_)) => return Err(RuntimeError::Spawn),
            Ok(None) => {}
            Err(_) => {
                stop_failed_launch(&mut launched);
                return Err(RuntimeError::Spawn);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    stop_failed_launch(&mut launched);
    Err(RuntimeError::Timeout)
}

fn stop_failed_launch(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
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
    runtime_lock: PrivateLock,
    worktree_parent: PathBuf,
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
        let (control, mut shutdown) = DaemonControl::new(
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
        .with_supervisor(supervisor.clone())
        .with_worktrees(self.worktree_parent.clone());
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
        // Keep the listener owner alive while supervised children are terminated and their final
        // observations are committed. A new runtime must not enter recovery against the database
        // while the previous generation is still completing orderly shutdown.
        let server_ownership = server.run_retaining_listener(&mut shutdown).await;
        signal.abort();
        let server_ownership = server_ownership.map_err(|_| RuntimeError::Transport)?;
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
        self.database.pool().close().await;
        drop(self.database);
        drop(server_ownership);
        drop(self.runtime_lock);
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
    let runtime_lock = acquire_runtime_lock(&locations, expected, Duration::from_millis(100))?;
    let endpoint = endpoint(&locations, expected)?;
    let listener = LocalListener::bind(&endpoint)
        .await
        .map_err(|error| match error {
            relayterm_ipc::IpcError::EndpointInUse => RuntimeError::Busy,
            relayterm_ipc::IpcError::AccessDenied => RuntimeError::AccessDenied,
            _ => RuntimeError::Transport,
        })?;
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
    let worktrees = locations.data().join("worktrees");
    if !worktrees.exists() {
        relayterm_platform::create_private_dir(&worktrees)
            .map_err(|_| RuntimeError::AccessDenied)?;
    }
    let worktree_parent = worktrees.join(expected.to_string());
    if !worktree_parent.exists() {
        relayterm_platform::create_private_dir(&worktree_parent)
            .map_err(|_| RuntimeError::AccessDenied)?;
    }
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
        runtime_lock,
        worktree_parent,
    })
}

pub fn wait_for_workspace_release(
    home: Option<PathBuf>,
    expected: WorkspaceId,
    timeout: Duration,
) -> Result<(), RuntimeError> {
    let locations = locations(home)?;
    drop(acquire_runtime_lock(&locations, expected, timeout)?);
    Ok(())
}

/// Create a consistent private workspace backup while the workspace daemon is stopped.
pub async fn backup_workspace(
    root: &Path,
    home: Option<PathBuf>,
    destination: &Path,
    timeout: Duration,
) -> Result<BackupReport, RuntimeError> {
    let (route, locations) = locate_workspace(root, home).await?;
    let workspace_id = route.domain_id()?;
    let _runtime_lock = acquire_runtime_lock(&locations, workspace_id, timeout)?;
    let parent = destination.parent().ok_or(RuntimeError::InvalidLocation)?;
    validate_private_dir(parent).map_err(|_| RuntimeError::AccessDenied)?;
    if destination.exists() {
        return Err(RuntimeError::InvalidLocation);
    }
    create_private_dir(destination).map_err(|_| RuntimeError::AccessDenied)?;

    let source_path = workspace_database_path(&locations, workspace_id);
    let source = Database::open(
        &source_path,
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .map_err(map_storage)?;
    let database_member = destination.join(BACKUP_WORKSPACE);
    source
        .snapshot_to(&database_member)
        .await
        .map_err(map_storage)?;
    source.pool().close().await;
    let captured = Database::open(
        &database_member,
        DatabaseKind::Workspace,
        OpenMode::ReadOnly,
        PoolSettings::default(),
    )
    .await
    .map_err(map_storage)?;
    let workspace_schema_version = captured.schema_version().await.map_err(map_storage)?;
    let snapshot = SqliteStore::new(captured.pool().clone())
        .consistent_snapshot(workspace_id)
        .await
        .map_err(|_| RuntimeError::Storage)?;
    let workspace_revision = snapshot.snapshot.revision().to_string();
    let last_event_sequence = snapshot.last_sequence.to_string();
    captured.pool().close().await;
    let checksum = hash_private_file(&database_member)?;
    let manifest = BackupManifest {
        format_version: 1,
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        workspace_schema_version,
        workspace_id: workspace_id.to_string(),
        database_member: BACKUP_WORKSPACE.to_owned(),
        workspace_revision: workspace_revision.clone(),
        last_event_sequence: last_event_sequence.clone(),
        blake3: checksum,
        complete: true,
    };
    let bytes = serde_json::to_vec(&manifest).map_err(|_| RuntimeError::Storage)?;
    let mut output = create_private_file(&destination.join(BACKUP_MANIFEST))
        .map_err(|_| RuntimeError::AccessDenied)?;
    output
        .write_all(&bytes)
        .map_err(|_| RuntimeError::Storage)?;
    output.sync_all().map_err(|_| RuntimeError::Storage)?;
    validate_backup_members(destination)?;
    Ok(BackupReport {
        format_version: 1,
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        workspace_schema_version,
        workspace_id: workspace_id.to_string(),
        workspace_revision,
        last_event_sequence,
    })
}

/// Restore a private workspace backup into a new Relayterm home.
pub async fn restore_workspace(
    root: &Path,
    source: &Path,
    destination_home: &Path,
) -> Result<BackupReport, RuntimeError> {
    validate_backup_members(source)?;
    if destination_home.exists() || !destination_home.is_absolute() {
        return Err(RuntimeError::InvalidLocation);
    }
    let manifest = read_backup_manifest(source)?;
    if manifest.format_version != 1
        || !manifest.complete
        || manifest.database_member != BACKUP_WORKSPACE
        || manifest.application_version.is_empty()
        || manifest.workspace_schema_version <= 0
        || manifest.workspace_schema_version > WORKSPACE_SCHEMA_VERSION
    {
        return Err(RuntimeError::RecoveryRequired);
    }
    let workspace_id = manifest
        .workspace_id
        .parse::<WorkspaceId>()
        .map_err(|_| RuntimeError::RecoveryRequired)?;
    let backup_database = source.join(BACKUP_WORKSPACE);
    if hash_private_file(&backup_database)? != manifest.blake3 {
        return Err(RuntimeError::RecoveryRequired);
    }
    let locations = locations(Some(destination_home.to_path_buf()))?;
    for alias in [LocationAlias::Data, LocationAlias::Runtime] {
        create_private_dir(locations.path(alias)).map_err(|_| RuntimeError::AccessDenied)?;
    }
    let workspaces = locations.data().join("workspaces");
    create_private_dir(&workspaces).map_err(|_| RuntimeError::AccessDenied)?;
    let workspace_dir = workspaces.join(workspace_id.to_string());
    create_private_dir(&workspace_dir).map_err(|_| RuntimeError::AccessDenied)?;
    copy_private_file(&backup_database, &workspace_dir.join(BACKUP_WORKSPACE))?;
    let restored = Database::open(
        &workspace_dir.join(BACKUP_WORKSPACE),
        DatabaseKind::Workspace,
        OpenMode::Reopen,
        PoolSettings::default(),
    )
    .await
    .map_err(map_storage)?;
    let snapshot = SqliteStore::new(restored.pool().clone())
        .consistent_snapshot(workspace_id)
        .await
        .map_err(|_| RuntimeError::RecoveryRequired)?;
    if snapshot.snapshot.revision().to_string() != manifest.workspace_revision
        || snapshot.last_sequence.to_string() != manifest.last_event_sequence
    {
        return Err(RuntimeError::RecoveryRequired);
    }
    restored.pool().close().await;

    let identity =
        WorkspaceRootIdentity::resolve(root).map_err(|_| RuntimeError::InvalidWorkspace)?;
    let registry = Registry::open(
        &locations.data().join("registry.sqlite3"),
        PoolSettings::default(),
    )
    .await
    .map_err(map_storage)?;
    let now = SystemClock.now().map_err(|_| RuntimeError::Storage)?;
    let registration = registry
        .reserve_or_get(&identity, workspace_id, now)
        .await
        .map_err(map_storage)?;
    if registration.workspace_id != workspace_id {
        return Err(RuntimeError::RecoveryRequired);
    }
    registry
        .mark_ready(workspace_id, now)
        .await
        .map_err(map_storage)?;
    Ok(BackupReport {
        format_version: manifest.format_version,
        application_version: manifest.application_version,
        workspace_schema_version: manifest.workspace_schema_version,
        workspace_id: manifest.workspace_id,
        workspace_revision: manifest.workspace_revision,
        last_event_sequence: manifest.last_event_sequence,
    })
}

fn validate_backup_members(directory: &Path) -> Result<(), RuntimeError> {
    validate_private_dir(directory).map_err(|_| RuntimeError::AccessDenied)?;
    let mut names = std::fs::read_dir(directory)
        .map_err(|_| RuntimeError::Storage)?
        .map(|entry| {
            entry.map_err(|_| RuntimeError::Storage).and_then(|entry| {
                let kind = entry.file_type().map_err(|_| RuntimeError::Storage)?;
                if !kind.is_file() || kind.is_symlink() {
                    return Err(RuntimeError::RecoveryRequired);
                }
                Ok(entry.file_name())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    let mut expected = vec![
        OsString::from(BACKUP_MANIFEST),
        OsString::from(BACKUP_WORKSPACE),
    ];
    expected.sort();
    if names != expected {
        return Err(RuntimeError::RecoveryRequired);
    }
    for name in expected {
        validate_private_file(&directory.join(name)).map_err(|_| RuntimeError::AccessDenied)?;
    }
    Ok(())
}

fn read_backup_manifest(directory: &Path) -> Result<BackupManifest, RuntimeError> {
    let file = File::open(directory.join(BACKUP_MANIFEST)).map_err(|_| RuntimeError::Storage)?;
    let mut bytes = Vec::new();
    file.take(MAX_BACKUP_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| RuntimeError::Storage)?;
    if bytes.len() as u64 > MAX_BACKUP_MANIFEST {
        return Err(RuntimeError::RecoveryRequired);
    }
    serde_json::from_slice(&bytes).map_err(|_| RuntimeError::RecoveryRequired)
}

fn hash_private_file(path: &Path) -> Result<String, RuntimeError> {
    validate_private_file(path).map_err(|_| RuntimeError::AccessDenied)?;
    let mut file = File::open(path).map_err(|_| RuntimeError::Storage)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| RuntimeError::Storage)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn copy_private_file(source: &Path, destination: &Path) -> Result<(), RuntimeError> {
    validate_private_file(source).map_err(|_| RuntimeError::AccessDenied)?;
    let mut input = File::open(source).map_err(|_| RuntimeError::Storage)?;
    let mut output = create_private_file(destination).map_err(|_| RuntimeError::AccessDenied)?;
    std::io::copy(&mut input, &mut output).map_err(|_| RuntimeError::Storage)?;
    output.sync_all().map_err(|_| RuntimeError::Storage)
}

fn acquire_runtime_lock(
    locations: &PrivateLocations,
    expected: WorkspaceId,
    timeout: Duration,
) -> Result<PrivateLock, RuntimeError> {
    PrivateLock::acquire(
        &locations
            .runtime()
            .join(format!("workspace-{expected}.runtime.lock")),
        timeout,
    )
    .map_err(|error| match error {
        relayterm_platform::LockError::Busy => RuntimeError::Busy,
        relayterm_platform::LockError::AccessDenied => RuntimeError::AccessDenied,
        relayterm_platform::LockError::Unavailable => RuntimeError::Transport,
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
) -> Result<std::process::Child, RuntimeError> {
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
