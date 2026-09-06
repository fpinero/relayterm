use crate::{MINIMUM_SQLITE_VERSION, StorageError};
use relayterm_platform::{
    create_private_file, secure_generated_file, validate_private_dir, validate_private_file,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{ConnectOptions, Connection, Executor, Row, SqliteConnection, SqlitePool};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseKind {
    Registry,
    Workspace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenMode {
    ExplicitNew,
    ResumeInitialization,
    Reopen,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolSettings {
    pub busy_timeout: Duration,
    pub max_connections: u32,
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self {
            busy_timeout: Duration::from_millis(5_000),
            max_connections: 4,
        }
    }
}

impl PoolSettings {
    pub fn validate(self) -> Result<Self, StorageError> {
        if !(Duration::from_millis(1)..=Duration::from_secs(30)).contains(&self.busy_timeout)
            || !(1..=16).contains(&self.max_connections)
        {
            return Err(StorageError::Unavailable);
        }
        Ok(self)
    }
}

pub struct Database {
    pool: SqlitePool,
    path: PathBuf,
    kind: DatabaseKind,
    mode: OpenMode,
}

impl Database {
    pub async fn open(
        path: &Path,
        kind: DatabaseKind,
        mode: OpenMode,
        settings: PoolSettings,
    ) -> Result<Self, StorageError> {
        let settings = settings.validate()?;
        match mode {
            OpenMode::ExplicitNew => {
                if path.exists() {
                    return Err(StorageError::AlreadyExists);
                }
                create_private_file(path).map_err(|_| StorageError::AccessDenied)?;
            }
            OpenMode::ResumeInitialization | OpenMode::Reopen | OpenMode::ReadOnly => {
                if !path.exists() {
                    return Err(StorageError::NotFound);
                }
                validate_private_file(path).map_err(|_| StorageError::AccessDenied)?;
            }
        }

        let migration_path = match kind {
            DatabaseKind::Registry => {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/registry")
            }
            DatabaseKind::Workspace => {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/workspace")
            }
        };
        let migrator = sqlx::migrate::Migrator::new(migration_path.as_path())
            .await
            .map_err(|_| StorageError::Migration)?;
        if matches!(mode, OpenMode::Reopen | OpenMode::ReadOnly) {
            preflight_existing(path, kind, &migrator, mode == OpenMode::ReadOnly).await?;
        } else if mode == OpenMode::ResumeInitialization {
            preflight_initializing(path, kind, &migrator).await?;
        }

        let options = SqliteConnectOptions::from_str("sqlite:")
            .map_err(|_| StorageError::Unavailable)?
            .filename(path)
            .create_if_missing(false)
            .read_only(mode == OpenMode::ReadOnly)
            .foreign_keys(true)
            .busy_timeout(settings.busy_timeout)
            .disable_statement_logging();
        let options = if mode == OpenMode::ReadOnly {
            options
        } else {
            options
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Full)
        };
        let pool = SqlitePoolOptions::new()
            .max_connections(settings.max_connections)
            .after_connect(|connection, _| {
                Box::pin(async move {
                    connection.execute("PRAGMA foreign_keys = ON").await?;
                    Ok(())
                })
            })
            .connect_with(options)
            .await
            .map_err(map_sqlx)?;

        verify_engine_and_pragmas(&pool, mode).await?;
        if mode != OpenMode::ReadOnly {
            migrator.run(&pool).await.map_err(map_migration)?;
        }
        let integrity: String = sqlx::query_scalar("PRAGMA quick_check(1)")
            .fetch_one(&pool)
            .await
            .map_err(map_sqlx)?;
        if integrity != "ok" {
            return Err(StorageError::Integrity);
        }
        Ok(Self {
            pool,
            path: path.to_path_buf(),
            kind,
            mode,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    pub fn mode(&self) -> OpenMode {
        self.mode
    }
    pub fn private_path(&self) -> &Path {
        &self.path
    }

    pub async fn snapshot_to(&self, destination: &Path) -> Result<(), StorageError> {
        if self.mode == OpenMode::ReadOnly {
            return Err(StorageError::ReadOnly);
        }
        if destination.exists() {
            return Err(StorageError::AlreadyExists);
        }
        validate_private_dir(destination.parent().ok_or(StorageError::AccessDenied)?)
            .map_err(|_| StorageError::AccessDenied)?;
        let destination_text = destination.to_str().ok_or(StorageError::Unavailable)?;
        sqlx::query("VACUUM INTO ?")
            .bind(destination_text)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        secure_generated_file(destination).map_err(|_| StorageError::AccessDenied)?;
        preflight_existing(
            destination,
            self.kind,
            &migration_for(self.kind).await?,
            true,
        )
        .await
    }
}

async fn migration_for(kind: DatabaseKind) -> Result<sqlx::migrate::Migrator, StorageError> {
    let path = match kind {
        DatabaseKind::Registry => Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/registry"),
        DatabaseKind::Workspace => {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/workspace")
        }
    };
    sqlx::migrate::Migrator::new(path.as_path())
        .await
        .map_err(|_| StorageError::Migration)
}

async fn preflight_existing(
    path: &Path,
    kind: DatabaseKind,
    migrator: &sqlx::migrate::Migrator,
    require_current: bool,
) -> Result<(), StorageError> {
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::from_str("sqlite:")
            .map_err(|_| StorageError::Unavailable)?
            .filename(path)
            .read_only(true)
            .create_if_missing(false)
            .disable_statement_logging(),
    )
    .await
    .map_err(map_sqlx)?;
    validate_schema_identity(&mut connection, kind, migrator, false, require_current).await
}

async fn preflight_initializing(
    path: &Path,
    kind: DatabaseKind,
    migrator: &sqlx::migrate::Migrator,
) -> Result<(), StorageError> {
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::from_str("sqlite:")
            .map_err(|_| StorageError::Unavailable)?
            .filename(path)
            .create_if_missing(false)
            .disable_statement_logging(),
    )
    .await
    .map_err(map_sqlx)?;
    validate_schema_identity(&mut connection, kind, migrator, true, false).await
}

async fn validate_schema_identity(
    connection: &mut SqliteConnection,
    kind: DatabaseKind,
    migrator: &sqlx::migrate::Migrator,
    allow_empty: bool,
    require_current: bool,
) -> Result<(), StorageError> {
    let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .fetch_all(&mut *connection).await.map_err(map_sqlx)?;
    if tables.is_empty() {
        return if allow_empty {
            Ok(())
        } else {
            Err(StorageError::Integrity)
        };
    }
    let identity = match kind {
        DatabaseKind::Registry => "registry_meta",
        DatabaseKind::Workspace => "workspace_meta",
    };
    if !tables.iter().any(|table| table == "_sqlx_migrations")
        || !tables.iter().any(|table| table == identity)
    {
        return Err(StorageError::Integrity);
    }
    let applied =
        sqlx::query("SELECT version,checksum,success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&mut *connection)
            .await
            .map_err(map_sqlx)?;
    if require_current && applied.len() != migrator.iter().count() {
        return Err(StorageError::IncompatibleVersion);
    }
    for row in applied {
        let version: i64 = row
            .try_get("version")
            .map_err(|_| StorageError::Integrity)?;
        let checksum: Vec<u8> = row
            .try_get("checksum")
            .map_err(|_| StorageError::Integrity)?;
        let success: bool = row
            .try_get("success")
            .map_err(|_| StorageError::Integrity)?;
        let Some(expected) = migrator
            .iter()
            .find(|migration| migration.version == version)
        else {
            return Err(StorageError::IncompatibleVersion);
        };
        if !success || checksum != expected.checksum.as_ref() {
            return Err(StorageError::Migration);
        }
    }
    Ok(())
}

async fn verify_engine_and_pragmas(pool: &SqlitePool, mode: OpenMode) -> Result<(), StorageError> {
    let row = sqlx::query("SELECT sqlite_version() AS version, (SELECT foreign_keys FROM pragma_foreign_keys) AS foreign_keys")
        .fetch_one(pool).await.map_err(map_sqlx)?;
    let version: String = row
        .try_get("version")
        .map_err(|_| StorageError::Integrity)?;
    let foreign_keys: i64 = row
        .try_get("foreign_keys")
        .map_err(|_| StorageError::Integrity)?;
    if parse_version(&version).is_none_or(|version| version < MINIMUM_SQLITE_VERSION) {
        return Err(StorageError::IncompatibleVersion);
    }
    if foreign_keys != 1 {
        return Err(StorageError::Integrity);
    }
    if mode != OpenMode::ReadOnly {
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(pool)
            .await
            .map_err(map_sqlx)?;
        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
            .fetch_one(pool)
            .await
            .map_err(map_sqlx)?;
        if !journal.eq_ignore_ascii_case("wal") || synchronous != 2 {
            return Err(StorageError::Integrity);
        }
    }
    Ok(())
}

fn parse_version(value: &str) -> Option<(u32, u32, u32)> {
    let mut components = value.split('.').map(str::parse::<u32>);
    Some((
        components.next()?.ok()?,
        components.next()?.ok()?,
        components.next()?.ok()?,
    ))
}

pub async fn inspect_sqlite_version() -> Result<String, StorageError> {
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(|_| StorageError::Unavailable)?
            .disable_statement_logging(),
    )
    .await
    .map_err(map_sqlx)?;
    sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&mut connection)
        .await
        .map_err(map_sqlx)
}

pub(crate) fn map_sqlx(error: sqlx::Error) -> StorageError {
    match error {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        sqlx::Error::Database(ref database) if database.code().as_deref() == Some("8") => {
            StorageError::ReadOnly
        }
        sqlx::Error::Database(ref database)
            if database.is_unique_violation() || database.is_foreign_key_violation() =>
        {
            StorageError::Integrity
        }
        sqlx::Error::Database(ref database)
            if database.message().contains("locked") || database.message().contains("busy") =>
        {
            StorageError::Busy
        }
        sqlx::Error::Migrate(_) => StorageError::Migration,
        _ => StorageError::Unavailable,
    }
}

fn map_migration(error: sqlx::migrate::MigrateError) -> StorageError {
    match error {
        sqlx::migrate::MigrateError::Execute(error)
        | sqlx::migrate::MigrateError::ExecuteMigration(error, _) => match map_sqlx(error) {
            StorageError::Busy => StorageError::Busy,
            _ => StorageError::Migration,
        },
        _ => StorageError::Migration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_engine_is_patched_and_workspace_pragmas_are_verified() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let version = inspect_sqlite_version().await.unwrap();
                assert!(parse_version(&version).unwrap() >= MINIMUM_SQLITE_VERSION);
                let temporary = tempfile::tempdir().unwrap();
                let private = temporary.path().join("private");
                relayterm_platform::create_private_dir(&private).unwrap();
                let path = private.join("workspace.sqlite3");
                let database = Database::open(
                    &path,
                    DatabaseKind::Workspace,
                    OpenMode::ExplicitNew,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
                assert_eq!(database.mode(), OpenMode::ExplicitNew);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
                        .fetch_one(database.pool())
                        .await
                        .unwrap(),
                    1
                );
                database.pool().close().await;
                Database::open(
                    &path,
                    DatabaseKind::Workspace,
                    OpenMode::Reopen,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
            });
    }

    #[test]
    fn reopen_never_creates_a_missing_database() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let temporary = tempfile::tempdir().unwrap();
                let path = temporary.path().join("missing.sqlite3");
                assert_eq!(
                    Database::open(
                        &path,
                        DatabaseKind::Workspace,
                        OpenMode::Reopen,
                        PoolSettings::default()
                    )
                    .await
                    .err(),
                    Some(StorageError::NotFound)
                );
                assert!(!path.exists());
            });
    }

    #[test]
    fn unknown_files_and_newer_or_altered_migrations_are_preserved() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let temporary = tempfile::tempdir().unwrap();
                let private = temporary.path().join("private");
                relayterm_platform::create_private_dir(&private).unwrap();
                let unknown = private.join("unknown.sqlite3");
                relayterm_platform::create_private_file(&unknown).unwrap();
                let mut raw = SqliteConnection::connect_with(
                    &SqliteConnectOptions::from_str("sqlite:")
                        .unwrap()
                        .filename(&unknown)
                        .create_if_missing(false),
                )
                .await
                .unwrap();
                raw.execute("CREATE TABLE retained_marker(value TEXT)")
                    .await
                    .unwrap();
                raw.close().await.unwrap();
                assert_eq!(
                    Database::open(
                        &unknown,
                        DatabaseKind::Workspace,
                        OpenMode::Reopen,
                        PoolSettings::default()
                    )
                    .await
                    .err(),
                    Some(StorageError::Integrity)
                );
                let mut inspect = SqliteConnection::connect_with(
                    &SqliteConnectOptions::from_str("sqlite:")
                        .unwrap()
                        .filename(&unknown)
                        .read_only(true),
                )
                .await
                .unwrap();
                let marker: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM sqlite_master WHERE name='retained_marker'",
                )
                .fetch_one(&mut inspect)
                .await
                .unwrap();
                assert_eq!(marker, 1);

                let newer = private.join("newer.sqlite3");
                let database = Database::open(
                    &newer,
                    DatabaseKind::Workspace,
                    OpenMode::ExplicitNew,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
                sqlx::query("UPDATE _sqlx_migrations SET version=99")
                    .execute(database.pool())
                    .await
                    .unwrap();
                database.pool().close().await;
                assert_eq!(
                    Database::open(
                        &newer,
                        DatabaseKind::Workspace,
                        OpenMode::Reopen,
                        PoolSettings::default()
                    )
                    .await
                    .err(),
                    Some(StorageError::IncompatibleVersion)
                );

                let altered = private.join("altered.sqlite3");
                let database = Database::open(
                    &altered,
                    DatabaseKind::Workspace,
                    OpenMode::ExplicitNew,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
                sqlx::query("UPDATE _sqlx_migrations SET checksum=zeroblob(48)")
                    .execute(database.pool())
                    .await
                    .unwrap();
                database.pool().close().await;
                assert_eq!(
                    Database::open(
                        &altered,
                        DatabaseKind::Workspace,
                        OpenMode::Reopen,
                        PoolSettings::default()
                    )
                    .await
                    .err(),
                    Some(StorageError::Migration)
                );
            });
    }

    #[test]
    fn read_only_handles_refuse_writes_and_vacuum_snapshot_is_reopenable() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let temporary = tempfile::tempdir().unwrap();
                let private = temporary.path().join("private");
                relayterm_platform::create_private_dir(&private).unwrap();
                let source = private.join("source.sqlite3");
                let database = Database::open(
                    &source,
                    DatabaseKind::Workspace,
                    OpenMode::ExplicitNew,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
                let backup = private.join("backup.sqlite3");
                database.snapshot_to(&backup).await.unwrap();
                relayterm_platform::validate_private_file(&backup).unwrap();
                let backup_db = Database::open(
                    &backup,
                    DatabaseKind::Workspace,
                    OpenMode::ReadOnly,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
                let error = sqlx::query("CREATE TABLE forbidden(value INTEGER)")
                    .execute(backup_db.pool())
                    .await
                    .unwrap_err();
                assert_eq!(map_sqlx(error), StorageError::ReadOnly);
            });
    }

    #[test]
    fn failed_transactional_schema_change_preserves_existing_rows() {
        tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(async {
            let temporary = tempfile::tempdir().unwrap();
            let private = temporary.path().join("private");
            relayterm_platform::create_private_dir(&private).unwrap();
            let database = Database::open(&private.join("rollback.sqlite3"), DatabaseKind::Workspace, OpenMode::ExplicitNew, PoolSettings::default()).await.unwrap();
            sqlx::query("CREATE TABLE migration_fixture(id INTEGER PRIMARY KEY, value TEXT NOT NULL)").execute(database.pool()).await.unwrap();
            sqlx::query("INSERT INTO migration_fixture VALUES(1,'retained')").execute(database.pool()).await.unwrap();
            let mut connection = database.pool().acquire().await.unwrap();
            sqlx::query("BEGIN IMMEDIATE").execute(&mut *connection).await.unwrap();
            sqlx::query("ALTER TABLE migration_fixture ADD COLUMN candidate TEXT").execute(&mut *connection).await.unwrap();
            assert!(sqlx::query("INSERT INTO missing_table VALUES(1)").execute(&mut *connection).await.is_err());
            sqlx::query("ROLLBACK").execute(&mut *connection).await.unwrap();
            let value: String = sqlx::query_scalar("SELECT value FROM migration_fixture WHERE id=1").fetch_one(database.pool()).await.unwrap();
            assert_eq!(value, "retained");
            let columns: i64 = sqlx::query_scalar("SELECT count(*) FROM pragma_table_info('migration_fixture') WHERE name='candidate'").fetch_one(database.pool()).await.unwrap();
            assert_eq!(columns, 0);
        });
    }

    #[test]
    fn cancelled_and_full_writes_leave_reusable_connections() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let temporary = tempfile::tempdir().unwrap();
                let private = temporary.path().join("private");
                relayterm_platform::create_private_dir(&private).unwrap();
                let database = Database::open(
                    &private.join("failures.sqlite3"),
                    DatabaseKind::Workspace,
                    OpenMode::ExplicitNew,
                    PoolSettings {
                        busy_timeout: Duration::from_millis(500),
                        max_connections: 2,
                    },
                )
                .await
                .unwrap();

                let mut locked = database.pool().acquire().await.unwrap();
                sqlx::query("BEGIN IMMEDIATE")
                    .execute(&mut *locked)
                    .await
                    .unwrap();
                let cancelled = tokio::time::timeout(
                    Duration::from_millis(20),
                    sqlx::query("CREATE TABLE cancelled_write(value INTEGER)")
                        .execute(database.pool()),
                )
                .await;
                assert!(cancelled.is_err());
                sqlx::query("ROLLBACK").execute(&mut *locked).await.unwrap();
                drop(locked);
                sqlx::query("CREATE TABLE recovered_write(value BLOB)")
                    .execute(database.pool())
                    .await
                    .unwrap();

                let mut connection = database.pool().acquire().await.unwrap();
                let pages: i64 = sqlx::query_scalar("PRAGMA page_count")
                    .fetch_one(&mut *connection)
                    .await
                    .unwrap();
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "PRAGMA max_page_count={pages}"
                )))
                .execute(&mut *connection)
                .await
                .unwrap();
                let failure = sqlx::query("INSERT INTO recovered_write VALUES(zeroblob(1048576))")
                    .execute(&mut *connection)
                    .await
                    .unwrap_err();
                assert_eq!(map_sqlx(failure), StorageError::Unavailable);
                sqlx::query("PRAGMA max_page_count=1073741823")
                    .execute(&mut *connection)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO recovered_write VALUES(x'01')")
                    .execute(&mut *connection)
                    .await
                    .unwrap();
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM recovered_write")
                    .fetch_one(&mut *connection)
                    .await
                    .unwrap();
                assert_eq!(count, 1);
            });
    }

    #[test]
    fn concurrent_initialization_converges_on_one_migration_history() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let temporary = tempfile::tempdir().unwrap();
                let private = temporary.path().join("private");
                relayterm_platform::create_private_dir(&private).unwrap();
                let path = private.join("concurrent.sqlite3");
                relayterm_platform::create_private_file(&path).unwrap();
                let first = Database::open(
                    &path,
                    DatabaseKind::Workspace,
                    OpenMode::ResumeInitialization,
                    PoolSettings::default(),
                );
                let second = Database::open(
                    &path,
                    DatabaseKind::Workspace,
                    OpenMode::ResumeInitialization,
                    PoolSettings::default(),
                );
                let (first, second) = tokio::join!(first, second);
                assert!(first.is_ok() || second.is_ok());
                for result in [first, second] {
                    match result {
                        Ok(database) => database.pool().close().await,
                        Err(error) => assert!(matches!(
                            error,
                            StorageError::Busy | StorageError::Migration
                        )),
                    }
                }
                let reopened = Database::open(
                    &path,
                    DatabaseKind::Workspace,
                    OpenMode::Reopen,
                    PoolSettings::default(),
                )
                .await
                .unwrap();
                let migrations: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success=1")
                        .fetch_one(reopened.pool())
                        .await
                        .unwrap();
                assert_eq!(migrations, 1);
            });
    }
}
