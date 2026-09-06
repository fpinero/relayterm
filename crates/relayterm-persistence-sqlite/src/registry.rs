use crate::{
    Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore, StorageError, encode_timestamp,
};
use relayterm_application::{Clock, EventNotifier, IdGenerator, Service, Store, Transaction};
use relayterm_domain::{Timestamp, WorkspaceId};
use relayterm_platform::{
    LocationAlias, PrivateLocations, PrivateLock, WorkspaceRootIdentity, create_private_dir,
    decode_native_path, encode_native_path,
};
use sqlx::Row;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrationState {
    Initializing,
    Ready,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Registration {
    pub workspace_id: WorkspaceId,
    pub state: RegistrationState,
    canonical_root: PathBuf,
    filesystem_guard: Option<Vec<u8>>,
}

impl Registration {
    pub fn canonical_root(&self) -> &Path {
        &self.canonical_root
    }
}

pub struct Registry {
    database: Database,
}

impl Registry {
    pub async fn open(path: &Path, settings: PoolSettings) -> Result<Self, StorageError> {
        let mode = if path.exists() {
            OpenMode::Reopen
        } else {
            OpenMode::ExplicitNew
        };
        Ok(Self {
            database: Database::open(path, DatabaseKind::Registry, mode, settings).await?,
        })
    }

    pub async fn reserve_or_get(
        &self,
        identity: &WorkspaceRootIdentity,
        reserved_id: WorkspaceId,
        timestamp: Timestamp,
    ) -> Result<Registration, StorageError> {
        let existing = sqlx::query("SELECT * FROM workspace_registrations WHERE root_lookup_key=?")
            .bind(identity.lookup_key().to_vec())
            .fetch_optional(self.database.pool())
            .await
            .map_err(crate::map_sqlx)?;
        if let Some(row) = existing {
            return decode_registration(row, identity);
        }
        let encoded =
            encode_native_path(identity.canonical_root()).map_err(|_| StorageError::Integrity)?;
        let (seconds, nanoseconds) = encode_timestamp(timestamp);
        let result = sqlx::query("INSERT INTO workspace_registrations VALUES(?,?,?,?,?,?,?,?,?,?)")
            .bind(id_bytes(reserved_id.as_uuid()))
            .bind(encoded.tag)
            .bind(encoded.bytes)
            .bind(identity.lookup_key().to_vec())
            .bind(identity.filesystem_guard().map(<[u8]>::to_vec))
            .bind("initializing")
            .bind(seconds)
            .bind(nanoseconds)
            .bind(seconds)
            .bind(nanoseconds)
            .execute(self.database.pool())
            .await;
        match result {
            Ok(_) => Ok(Registration {
                workspace_id: reserved_id,
                state: RegistrationState::Initializing,
                canonical_root: identity.canonical_root().to_path_buf(),
                filesystem_guard: identity.filesystem_guard().map(<[u8]>::to_vec),
            }),
            Err(_) => {
                let row =
                    sqlx::query("SELECT * FROM workspace_registrations WHERE root_lookup_key=?")
                        .bind(identity.lookup_key().to_vec())
                        .fetch_optional(self.database.pool())
                        .await
                        .map_err(crate::map_sqlx)?
                        .ok_or(StorageError::Busy)?;
                decode_registration(row, identity)
            }
        }
    }

    pub async fn mark_ready(
        &self,
        workspace_id: WorkspaceId,
        timestamp: Timestamp,
    ) -> Result<(), StorageError> {
        let (seconds, nanoseconds) = encode_timestamp(timestamp);
        let changed = sqlx::query("UPDATE workspace_registrations SET initialization_state='ready',updated_seconds=?,updated_nanoseconds=? WHERE workspace_id=? AND initialization_state='initializing'")
            .bind(seconds).bind(nanoseconds).bind(id_bytes(workspace_id.as_uuid())).execute(self.database.pool()).await.map_err(crate::map_sqlx)?.rows_affected();
        if changed == 1 {
            return Ok(());
        }
        let state: Option<String> = sqlx::query_scalar(
            "SELECT initialization_state FROM workspace_registrations WHERE workspace_id=?",
        )
        .bind(id_bytes(workspace_id.as_uuid()))
        .fetch_optional(self.database.pool())
        .await
        .map_err(crate::map_sqlx)?;
        if state.as_deref() == Some("ready") {
            Ok(())
        } else {
            Err(StorageError::Integrity)
        }
    }

    pub async fn lookup(
        &self,
        identity: &WorkspaceRootIdentity,
    ) -> Result<Option<Registration>, StorageError> {
        sqlx::query("SELECT * FROM workspace_registrations WHERE root_lookup_key=?")
            .bind(identity.lookup_key().to_vec())
            .fetch_optional(self.database.pool())
            .await
            .map_err(crate::map_sqlx)?
            .map(|row| decode_registration(row, identity))
            .transpose()
    }
}

fn decode_registration(
    row: sqlx::sqlite::SqliteRow,
    expected: &WorkspaceRootIdentity,
) -> Result<Registration, StorageError> {
    let root = decode_native_path(
        &row.try_get::<String, _>("root_codec")
            .map_err(|_| StorageError::Integrity)?,
        &row.try_get::<Vec<u8>, _>("canonical_root")
            .map_err(|_| StorageError::Integrity)?,
    )
    .map_err(|_| StorageError::Integrity)?;
    let guard: Option<Vec<u8>> = row
        .try_get("filesystem_guard")
        .map_err(|_| StorageError::Integrity)?;
    if root != expected.canonical_root() || guard.as_deref() != expected.filesystem_guard() {
        return Err(StorageError::Integrity);
    }
    let id = Uuid::from_slice(
        &row.try_get::<Vec<u8>, _>("workspace_id")
            .map_err(|_| StorageError::Integrity)?,
    )
    .map_err(|_| StorageError::Integrity)?;
    let state = match row
        .try_get::<String, _>("initialization_state")
        .map_err(|_| StorageError::Integrity)?
        .as_str()
    {
        "initializing" => RegistrationState::Initializing,
        "ready" => RegistrationState::Ready,
        _ => return Err(StorageError::Integrity),
    };
    Ok(Registration {
        workspace_id: WorkspaceId::from_uuid(id),
        state,
        canonical_root: root,
        filesystem_guard: guard,
    })
}

pub struct InitializedWorkspace {
    pub workspace_id: WorkspaceId,
    pub database: Database,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitializationError {
    Location,
    Identity,
    Busy,
    RecoveryRequired,
    Storage,
    Domain,
}

pub async fn initialize_workspace<C, I, N>(
    locations: &PrivateLocations,
    project_root: &Path,
    display_name: String,
    settings: PoolSettings,
    clock: C,
    ids: I,
    notifier: N,
) -> Result<InitializedWorkspace, InitializationError>
where
    C: Clock,
    I: IdGenerator,
    N: EventNotifier,
{
    let identity =
        WorkspaceRootIdentity::resolve(project_root).map_err(|_| InitializationError::Identity)?;
    for alias in [LocationAlias::Data, LocationAlias::Runtime] {
        create_private_dir(locations.path(alias)).map_err(|_| InitializationError::Location)?;
    }
    let workspaces = locations.data().join("workspaces");
    create_private_dir(&workspaces).map_err(|_| InitializationError::Location)?;
    let registry_path = locations.data().join("registry.sqlite3");
    let lock_path = locations.runtime().join("registry.lock");
    let registry_lock = PrivateLock::acquire(&lock_path, settings.busy_timeout)
        .map_err(|_| InitializationError::Busy)?;
    let registry = Registry::open(&registry_path, settings)
        .await
        .map_err(map_initialization_storage)?;
    let reserved_id = WorkspaceId::from_uuid(
        ids.next()
            .map_err(|_| InitializationError::Domain)?
            .as_uuid(),
    );
    let registration = registry
        .reserve_or_get(
            &identity,
            reserved_id,
            clock.now().map_err(|_| InitializationError::Domain)?,
        )
        .await
        .map_err(map_initialization_storage)?;
    let ready_at = clock.now().map_err(|_| InitializationError::Domain)?;
    drop(registry_lock);

    let workspace_directory = workspaces.join(registration.workspace_id.to_string());
    create_private_dir(&workspace_directory).map_err(|_| InitializationError::Location)?;
    let init_lock_path = locations
        .runtime()
        .join(format!("workspace-{}.init.lock", registration.workspace_id));
    let init_lock = PrivateLock::acquire(&init_lock_path, settings.busy_timeout)
        .map_err(|_| InitializationError::Busy)?;
    let database_path = workspace_directory.join("workspace.sqlite3");
    if registration.state == RegistrationState::Ready && !database_path.exists() {
        return Err(InitializationError::RecoveryRequired);
    }
    let mode = if database_path.exists() {
        if registration.state == RegistrationState::Initializing {
            OpenMode::ResumeInitialization
        } else {
            OpenMode::Reopen
        }
    } else {
        OpenMode::ExplicitNew
    };
    let database = Database::open(&database_path, DatabaseKind::Workspace, mode, settings)
        .await
        .map_err(map_initialization_storage)?;
    let store = SqliteStore::new(database.pool().clone());
    let transaction = store
        .begin(registration.workspace_id)
        .await
        .map_err(|_| InitializationError::Domain)?;
    if transaction.snapshot().revision() == 0 {
        Service::new(store, clock, ids, notifier)
            .create_workspace_reserved(
                registration.workspace_id,
                display_name,
                identity.canonical_root().to_path_buf(),
            )
            .await
            .map_err(|_| InitializationError::Domain)?;
    } else if transaction
        .snapshot()
        .state()
        .map_err(|_| InitializationError::RecoveryRequired)?
        .workspace()
        .record()
        .id
        != registration.workspace_id
    {
        return Err(InitializationError::RecoveryRequired);
    }
    drop(init_lock);
    let final_lock = PrivateLock::acquire(&lock_path, settings.busy_timeout)
        .map_err(|_| InitializationError::Busy)?;
    registry
        .mark_ready(registration.workspace_id, ready_at)
        .await
        .map_err(map_initialization_storage)?;
    drop(final_lock);
    Ok(InitializedWorkspace {
        workspace_id: registration.workspace_id,
        database,
    })
}

fn map_initialization_storage(error: StorageError) -> InitializationError {
    match error {
        StorageError::Busy => InitializationError::Busy,
        StorageError::Integrity | StorageError::IncompatibleVersion => {
            InitializationError::RecoveryRequired
        }
        _ => InitializationError::Storage,
    }
}

fn id_bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}
