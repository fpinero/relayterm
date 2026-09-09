use crate::{decode_counter, decode_timestamp, encode_counter, encode_timestamp, map_sqlx};
use relayterm_application::{
    Committed, DurableReadStore, EventPage, EventPageRequest, IdPage, IdPageRequest, Snapshot,
    Store, TaskHistoryEntry, TaskHistoryItem, TaskHistoryPage, TaskHistoryPageRequest,
    Transaction as ApplicationTransaction, WatermarkedSnapshot, WriteBatch,
};
use relayterm_domain::*;
use relayterm_platform::{decode_native_path, encode_native_path};
use sqlx::{AssertSqlSafe, Row, SqliteConnection, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

pub struct SqliteTransaction {
    pool: SqlitePool,
    workspace_id: WorkspaceId,
    snapshot: Snapshot,
}

impl Store for SqliteStore {
    type Transaction = SqliteTransaction;

    async fn begin(&self, workspace_id: WorkspaceId) -> Result<SqliteTransaction> {
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        let snapshot = load_snapshot(&mut connection, workspace_id).await?;
        Ok(SqliteTransaction {
            pool: self.pool.clone(),
            workspace_id,
            snapshot,
        })
    }
}

impl DurableReadStore for SqliteStore {
    async fn consistent_snapshot(&self, workspace_id: WorkspaceId) -> Result<WatermarkedSnapshot> {
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        sqlx::query("BEGIN")
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        let result = async {
            let snapshot = load_snapshot(&mut connection, workspace_id).await?;
            let (last_sequence, retained_from_sequence) =
                load_watermarks(&mut connection, workspace_id).await?;
            Ok(WatermarkedSnapshot {
                snapshot,
                last_sequence,
                retained_from_sequence,
            })
        }
        .await;
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
        result
    }

    async fn event_page(
        &self,
        workspace_id: WorkspaceId,
        request: EventPageRequest,
    ) -> Result<EventPage> {
        if request.limit == 0 || request.limit > 200 {
            return Err(Error::Validation("page_limit"));
        }
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        sqlx::query("BEGIN")
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        let result = async {
            let (last_sequence, retained_from_sequence) =
                load_watermarks(&mut connection, workspace_id).await?;
            if request.after_sequence > last_sequence {
                return Err(Error::InvalidCursor);
            }
            if request.after_sequence.saturating_add(1) < retained_from_sequence {
                return Err(Error::ResnapshotRequired);
            }
            let rows = sqlx::query("SELECT * FROM workspace_events WHERE workspace_id=? AND sequence>? ORDER BY sequence LIMIT ?")
                .bind(id_bytes(workspace_id.as_uuid())).bind(encode_counter(request.after_sequence).to_vec()).bind(i64::from(request.limit))
                .fetch_all(&mut *connection).await.map_err(domain_storage)?;
            let events = rows
                .into_iter()
                .map(decode_event)
                .collect::<Result<Vec<_>>>()?;
            Ok(EventPage {
                events,
                last_sequence,
                retained_from_sequence,
            })
        }
        .await;
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
        result
    }

    async fn task_history_page(
        &self,
        workspace_id: WorkspaceId,
        task_id: TaskId,
        request: TaskHistoryPageRequest,
    ) -> Result<TaskHistoryPage> {
        if request.limit == 0 || request.limit > 200 || request.expected_revision == 0 {
            return Err(Error::Validation("page_limit"));
        }
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        sqlx::query("BEGIN")
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        let result = async {
            let revision_bytes: Vec<u8> = sqlx::query_scalar(
                "SELECT revision FROM workspace_meta WHERE workspace_id=?",
            )
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_optional(&mut *connection)
            .await
            .map_err(domain_storage)?
            .ok_or(Error::Reference)?;
            let revision = decode_counter(&revision_bytes).map_err(|_| Error::Integrity)?;
            if revision != request.expected_revision {
                return Err(Error::Conflict);
            }
            let task_exists: i64 = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE workspace_id=? AND task_id=?)",
            )
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(task_id.as_uuid()))
            .fetch_one(&mut *connection)
            .await
            .map_err(domain_storage)?;
            if task_exists == 0 {
                return Err(Error::Reference);
            }
            let rows = sqlx::query(
                "SELECT sequence,kind,item_id FROM (\
                 SELECT opening_event_sequence AS sequence,'claim' AS kind,claim_id AS item_id FROM claims WHERE workspace_id=? AND task_id=? \
                 UNION ALL SELECT creation_event_sequence,'progress',progress_id FROM progress_entries WHERE workspace_id=? AND task_id=? \
                 UNION ALL SELECT creation_event_sequence,'handover',handover_id FROM handovers WHERE workspace_id=? AND task_id=?\
                 ) WHERE sequence>? ORDER BY sequence LIMIT ?",
            )
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(task_id.as_uuid()))
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(task_id.as_uuid()))
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(task_id.as_uuid()))
            .bind(encode_counter(request.after_sequence).to_vec())
            .bind(i64::from(request.limit))
            .fetch_all(&mut *connection)
            .await
            .map_err(domain_storage)?;
            let mut entries = Vec::with_capacity(rows.len());
            for row in rows {
                let sequence = decode_counter(
                    &row.try_get::<Vec<u8>, _>("sequence")
                        .map_err(|_| Error::Integrity)?,
                )
                .map_err(|_| Error::Integrity)?;
                let id = row
                    .try_get::<Vec<u8>, _>("item_id")
                    .map_err(|_| Error::Integrity)?;
                let item = match row
                    .try_get::<String, _>("kind")
                    .map_err(|_| Error::Integrity)?
                    .as_str()
                {
                    "claim" => TaskHistoryItem::Claim(
                        load_claim(&mut connection, workspace_id, &id).await?,
                    ),
                    "progress" => TaskHistoryItem::Progress(
                        load_progress_entry(&mut connection, workspace_id, &id).await?,
                    ),
                    "handover" => TaskHistoryItem::Handover(
                        load_handover(&mut connection, workspace_id, &id).await?,
                    ),
                    _ => return Err(Error::Integrity),
                };
                entries.push(TaskHistoryEntry { sequence, item });
            }
            Ok(TaskHistoryPage { revision, entries })
        }
        .await;
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
        result
    }

    async fn progress_page(
        &self,
        workspace_id: WorkspaceId,
        request: IdPageRequest,
    ) -> Result<IdPage<ProgressEntry>> {
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        sqlx::query("BEGIN")
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        let result = load_progress_page(&mut connection, workspace_id, request).await;
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
        result
    }

    async fn handover_page(
        &self,
        workspace_id: WorkspaceId,
        request: IdPageRequest,
    ) -> Result<IdPage<Handover>> {
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        sqlx::query("BEGIN")
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        let result = load_handover_page(&mut connection, workspace_id, request).await;
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
        result
    }
}

async fn page_metadata(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    expected_revision: Option<u64>,
) -> Result<(u64, u64, u64)> {
    let revision_bytes: Vec<u8> =
        sqlx::query_scalar("SELECT revision FROM workspace_meta WHERE workspace_id=?")
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_optional(&mut *connection)
            .await
            .map_err(domain_storage)?
            .ok_or(Error::Reference)?;
    let revision = decode_counter(&revision_bytes).map_err(|_| Error::Integrity)?;
    if expected_revision.is_some_and(|expected| expected != revision) {
        return Err(Error::Conflict);
    }
    let (last_sequence, retained_from_sequence) = load_watermarks(connection, workspace_id).await?;
    Ok((revision, last_sequence, retained_from_sequence))
}

async fn load_progress_page(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    request: IdPageRequest,
) -> Result<IdPage<ProgressEntry>> {
    let (revision, last_sequence, retained_from_sequence) =
        page_metadata(connection, workspace_id, request.expected_revision).await?;
    let rows = if let Some(after) = request.after_id {
        sqlx::query(
            "SELECT * FROM progress_entries WHERE workspace_id=? AND progress_id>? ORDER BY progress_id LIMIT ?",
        )
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(after.to_vec())
        .bind(i64::from(request.limit) + 1)
        .fetch_all(&mut *connection)
        .await
        .map_err(domain_storage)?
    } else {
        sqlx::query(
            "SELECT * FROM progress_entries WHERE workspace_id=? ORDER BY progress_id LIMIT ?",
        )
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(i64::from(request.limit) + 1)
        .fetch_all(&mut *connection)
        .await
        .map_err(domain_storage)?
    };
    let has_more = rows.len() > usize::from(request.limit);
    let items = rows
        .into_iter()
        .take(usize::from(request.limit))
        .map(|row| decode_progress(row, workspace_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(IdPage {
        revision,
        last_sequence,
        retained_from_sequence,
        items,
        has_more,
    })
}

async fn load_handover_page(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    request: IdPageRequest,
) -> Result<IdPage<Handover>> {
    let (revision, last_sequence, retained_from_sequence) =
        page_metadata(connection, workspace_id, request.expected_revision).await?;
    let rows = if let Some(after) = request.after_id {
        sqlx::query(
            "SELECT * FROM handovers WHERE workspace_id=? AND handover_id>? ORDER BY handover_id LIMIT ?",
        )
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(after.to_vec())
        .bind(i64::from(request.limit) + 1)
        .fetch_all(&mut *connection)
        .await
        .map_err(domain_storage)?
    } else {
        sqlx::query("SELECT * FROM handovers WHERE workspace_id=? ORDER BY handover_id LIMIT ?")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(i64::from(request.limit) + 1)
            .fetch_all(&mut *connection)
            .await
            .map_err(domain_storage)?
    };
    let has_more = rows.len() > usize::from(request.limit);
    let mut items = Vec::with_capacity(rows.len().min(usize::from(request.limit)));
    for row in rows.into_iter().take(usize::from(request.limit)) {
        items.push(decode_handover(connection, row, workspace_id).await?);
    }
    Ok(IdPage {
        revision,
        last_sequence,
        retained_from_sequence,
        items,
        has_more,
    })
}

async fn load_watermarks(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<(u64, u64)> {
    let row = sqlx::query("SELECT last_event_sequence,retained_from_sequence FROM workspace_meta WHERE workspace_id=?")
        .bind(id_bytes(workspace_id.as_uuid())).fetch_optional(&mut *connection).await.map_err(domain_storage)?.ok_or(Error::Reference)?;
    Ok((
        decode_counter(
            &row.try_get::<Vec<u8>, _>("last_event_sequence")
                .map_err(|_| Error::Integrity)?,
        )
        .map_err(|_| Error::Integrity)?,
        decode_counter(
            &row.try_get::<Vec<u8>, _>("retained_from_sequence")
                .map_err(|_| Error::Integrity)?,
        )
        .map_err(|_| Error::Integrity)?,
    ))
}

fn decode_event(row: sqlx::sqlite::SqliteRow) -> Result<WorkspaceEvent> {
    let workspace_id = workspace_id(
        row.try_get::<Vec<u8>, _>("workspace_id")
            .map_err(|_| Error::Integrity)?,
    )?;
    let sequence = decode_counter(
        &row.try_get::<Vec<u8>, _>("sequence")
            .map_err(|_| Error::Integrity)?,
    )
    .map_err(|_| Error::Integrity)?;
    let event_id = event_id(
        row.try_get::<Vec<u8>, _>("event_id")
            .map_err(|_| Error::Integrity)?,
    )?;
    let event_type = parse_event_type(
        &row.try_get::<String, _>("event_type")
            .map_err(|_| Error::Integrity)?,
    )?;
    let entity_id = parse_entity(
        &row.try_get::<String, _>("entity_kind")
            .map_err(|_| Error::Integrity)?,
        row.try_get::<Vec<u8>, _>("entity_id")
            .map_err(|_| Error::Integrity)?,
    )?;
    let actor = parse_actor(
        &row.try_get::<String, _>("actor_kind")
            .map_err(|_| Error::Integrity)?,
        row.try_get::<Option<Vec<u8>>, _>("actor_instance_id")
            .map_err(|_| Error::Integrity)?,
    )?;
    let payload_bytes: Vec<u8> = row.try_get("payload_json").map_err(|_| Error::Integrity)?;
    if payload_bytes.len() > 16384 {
        return Err(Error::Integrity);
    }
    let payload: EventPayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| Error::Version)?;
    WorkspaceEvent::try_from(EventRecord {
        sequence,
        event_id,
        workspace_id,
        event_type,
        entity_id,
        timestamp: timestamp_row(&row, "timestamp_seconds", "timestamp_nanoseconds")?,
        actor,
        payload_version: row
            .try_get::<i64, _>("payload_version")
            .ok()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(Error::Version)?,
        payload,
    })
}

impl ApplicationTransaction for SqliteTransaction {
    fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    async fn commit(self, batch: WriteBatch) -> Result<Committed> {
        let mut connection = self.pool.acquire().await.map_err(domain_storage)?;
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        let result = commit_locked(&mut connection, self.workspace_id, &batch).await;
        match result {
            Ok(committed) => {
                sqlx::query("COMMIT")
                    .execute(&mut *connection)
                    .await
                    .map_err(|_| Error::Uncertain)?;
                Ok(committed)
            }
            Err(error) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
                Err(error)
            }
        }
    }
}

async fn commit_locked(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    batch: &WriteBatch,
) -> Result<Committed> {
    let current = load_snapshot(connection, workspace_id).await?;
    batch.validate(&current)?;
    if batch.events().is_empty() {
        return Ok(Committed {
            snapshot: current,
            events: Vec::new(),
        });
    }
    let revision = current.revision().checked_add(1).ok_or(Error::Storage)?;
    let last_sequence = load_last_sequence(connection, workspace_id).await?;
    let mut confirmed = Vec::with_capacity(batch.events().len());
    for (offset, event) in batch.events().iter().enumerate() {
        let sequence = last_sequence
            .checked_add(u64::try_from(offset).map_err(|_| Error::Storage)? + 1)
            .ok_or(Error::Storage)?;
        confirmed.push(WorkspaceEvent::confirm(event.clone(), sequence)?);
    }
    persist_state(
        connection,
        current.state().ok(),
        batch.state(),
        &confirmed,
        revision,
    )
    .await?;
    let snapshot = Snapshot::restore(revision, Some(batch.state().clone()))?;
    Ok(Committed {
        snapshot,
        events: confirmed,
    })
}

async fn load_last_sequence(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<u64> {
    let value: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT last_event_sequence FROM workspace_meta WHERE workspace_id = ?")
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_optional(&mut *connection)
            .await
            .map_err(domain_storage)?;
    value.map_or(Ok(0), |bytes| {
        decode_counter(&bytes).map_err(|_| Error::Storage)
    })
}

async fn load_snapshot(
    connection: &mut SqliteConnection,
    expected: WorkspaceId,
) -> Result<Snapshot> {
    let meta = sqlx::query("SELECT workspace_id, revision FROM workspace_meta WHERE singleton = 1")
        .fetch_optional(&mut *connection)
        .await
        .map_err(domain_storage)?;
    let Some(meta) = meta else {
        return Snapshot::restore(0, None);
    };
    let workspace_id = workspace_id(
        meta.try_get::<Vec<u8>, _>("workspace_id")
            .map_err(|_| Error::Storage)?,
    )?;
    if workspace_id != expected {
        return Err(Error::Reference);
    }
    let revision = decode_counter(
        &meta
            .try_get::<Vec<u8>, _>("revision")
            .map_err(|_| Error::Storage)?,
    )
    .map_err(|_| Error::Storage)?;
    let row = sqlx::query("SELECT display_name, root_codec, project_root, created_seconds, created_nanoseconds, updated_seconds, updated_nanoseconds, schema_version FROM workspaces WHERE workspace_id = ?")
        .bind(id_bytes(expected.as_uuid())).fetch_one(&mut *connection).await.map_err(domain_storage)?;
    let workspace = Workspace::restore(WorkspaceRecord {
        id: expected,
        display_name: row.try_get("display_name").map_err(|_| Error::Storage)?,
        project_root: decode_native_path(
            row.try_get::<String, _>("root_codec")
                .map_err(|_| Error::Storage)?
                .as_str(),
            &row.try_get::<Vec<u8>, _>("project_root")
                .map_err(|_| Error::Storage)?,
        )
        .map_err(|_| Error::Storage)?,
        created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
        updated_at: timestamp_row(&row, "updated_seconds", "updated_nanoseconds")?,
        schema_version: row
            .try_get::<i64, _>("schema_version")
            .ok()
            .and_then(|x| u32::try_from(x).ok())
            .ok_or(Error::Storage)?,
    })?;
    let definitions = load_definitions(connection, expected).await?;
    let claims = load_claims(connection, expected).await?;
    let owners: HashMap<TaskId, AgentInstanceId> = claims
        .iter()
        .filter_map(|claim| {
            let record = claim.record();
            record
                .closed_at
                .is_none()
                .then_some((record.task_id, record.instance_id))
        })
        .collect();
    let tasks = load_tasks(connection, expected, &owners).await?;
    let instances = load_instances(connection, expected).await?;
    let progress = load_progress(connection, expected).await?;
    let handovers = load_handovers(connection, expected).await?;
    let (approved_roots, worktree_intents, worktrees) =
        load_worktree_state(connection, expected).await?;
    let state = WorkspaceState::restore(
        workspace,
        WorkspaceRows {
            definitions,
            tasks,
            instances,
            claims,
            progress,
            handovers,
            approved_roots,
            worktree_intents,
            worktrees,
        },
    )?;
    Snapshot::restore(revision, Some(state))
}

async fn load_worktree_state(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<(Vec<ApprovedRoot>, Vec<WorktreeIntent>, Vec<Worktree>)> {
    let mut roots = Vec::new();
    for row in
        sqlx::query("SELECT * FROM approved_worktree_roots WHERE workspace_id=? ORDER BY root_id")
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_all(&mut *connection)
            .await
            .map_err(domain_storage)?
    {
        roots.push(ApprovedRoot::restore(ApprovedRootRecord {
            id: approved_root_id(row.try_get("root_id").map_err(|_| Error::Storage)?)?,
            workspace_id,
            canonical_parent: decode_native_path(
                &row.try_get::<String, _>("path_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("canonical_parent")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            filesystem_identity: row
                .try_get("filesystem_identity")
                .map_err(|_| Error::Storage)?,
            private_default: int_bool(row.try_get("private_default").map_err(|_| Error::Storage)?)?,
            created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
        })?);
    }
    let mut intents = Vec::new();
    for row in
        sqlx::query("SELECT * FROM worktree_intents WHERE workspace_id=? ORDER BY operation_id")
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_all(&mut *connection)
            .await
            .map_err(domain_storage)?
    {
        let fingerprint: Vec<u8> = row
            .try_get("request_fingerprint")
            .map_err(|_| Error::Storage)?;
        let fingerprint: [u8; 32] = fingerprint.try_into().map_err(|_| Error::Storage)?;
        intents.push(WorktreeIntent::restore(WorktreeIntentRecord {
            id: worktree_operation_id(row.try_get("operation_id").map_err(|_| Error::Storage)?)?,
            workspace_id,
            task_id: task_id(row.try_get("task_id").map_err(|_| Error::Storage)?)?,
            worktree_id: worktree_id(row.try_get("worktree_id").map_err(|_| Error::Storage)?)?,
            root_id: approved_root_id(row.try_get("root_id").map_err(|_| Error::Storage)?)?,
            actor: Actor::LocalUser,
            schema_version: u32::try_from(
                row.try_get::<i64, _>("schema_version")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            expected_revision: decode_counter(
                &row.try_get::<Vec<u8>, _>("expected_revision")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            repository_identity: decode_native_path(
                &row.try_get::<String, _>("repository_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("repository_identity")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            common_directory_identity: decode_native_path(
                &row.try_get::<String, _>("common_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("common_directory_identity")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            destination: decode_native_path(
                &row.try_get::<String, _>("destination_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("destination")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            branch: row.try_get("branch").map_err(|_| Error::Storage)?,
            base_expression: row.try_get("base_expression").map_err(|_| Error::Storage)?,
            resolved_commit: row.try_get("resolved_commit").map_err(|_| Error::Storage)?,
            request_fingerprint: fingerprint,
            phase: parse_worktree_phase(
                &row.try_get::<String, _>("phase")
                    .map_err(|_| Error::Storage)?,
            )?,
            reason: row
                .try_get::<Option<String>, _>("reason")
                .map_err(|_| Error::Storage)?
                .as_deref()
                .map(parse_worktree_reason)
                .transpose()?,
            created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
            updated_at: timestamp_row(&row, "updated_seconds", "updated_nanoseconds")?,
        })?);
    }
    let mut worktrees = Vec::new();
    for row in sqlx::query("SELECT * FROM worktrees WHERE workspace_id=? ORDER BY worktree_id")
        .bind(id_bytes(workspace_id.as_uuid()))
        .fetch_all(&mut *connection)
        .await
        .map_err(domain_storage)?
    {
        worktrees.push(Worktree::restore(WorktreeRecord {
            id: worktree_id(row.try_get("worktree_id").map_err(|_| Error::Storage)?)?,
            workspace_id,
            task_id: task_id(row.try_get("task_id").map_err(|_| Error::Storage)?)?,
            operation_id: worktree_operation_id(
                row.try_get("operation_id").map_err(|_| Error::Storage)?,
            )?,
            root_id: approved_root_id(row.try_get("root_id").map_err(|_| Error::Storage)?)?,
            checkout_path: decode_native_path(
                &row.try_get::<String, _>("checkout_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("checkout_path")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            common_directory_identity: decode_native_path(
                &row.try_get::<String, _>("common_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("common_directory_identity")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            branch_ref: row.try_get("branch_ref").map_err(|_| Error::Storage)?,
            initial_base_commit: row
                .try_get("initial_base_commit")
                .map_err(|_| Error::Storage)?,
            health: parse_worktree_health(
                &row.try_get::<String, _>("health")
                    .map_err(|_| Error::Storage)?,
            )?,
            created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
            updated_at: timestamp_row(&row, "updated_seconds", "updated_nanoseconds")?,
        })?);
    }
    Ok((roots, intents, worktrees))
}

async fn load_definitions(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<Vec<AgentDefinition>> {
    let rows = sqlx::query("SELECT definition_id, display_name, command, enabled FROM agent_definitions WHERE workspace_id = ? ORDER BY definition_id")
        .bind(id_bytes(workspace_id.as_uuid())).fetch_all(&mut *connection).await.map_err(domain_storage)?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let id = definition_id(
            row.try_get::<Vec<u8>, _>("definition_id")
                .map_err(|_| Error::Storage)?,
        )?;
        values.push(AgentDefinition::restore(AgentDefinitionRecord {
            id,
            workspace_id,
            display_name: row.try_get("display_name").map_err(|_| Error::Storage)?,
            command: row.try_get("command").map_err(|_| Error::Storage)?,
            arguments: load_strings(
                connection,
                "definition_arguments",
                "definition_id",
                id.as_uuid(),
                workspace_id,
            )
            .await?,
            environment_allowlist: load_strings(
                connection,
                "definition_environment",
                "definition_id",
                id.as_uuid(),
                workspace_id,
            )
            .await?,
            capabilities: load_strings(
                connection,
                "definition_capabilities",
                "definition_id",
                id.as_uuid(),
                workspace_id,
            )
            .await?,
            enabled: int_bool(row.try_get("enabled").map_err(|_| Error::Storage)?)?,
        })?);
    }
    Ok(values)
}

async fn load_tasks(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    owners: &HashMap<TaskId, AgentInstanceId>,
) -> Result<Vec<Task>> {
    let rows = sqlx::query("SELECT task_id,title,description,priority,status,acceptance_notes,worktree_id,created_seconds,created_nanoseconds,updated_seconds,updated_nanoseconds FROM tasks WHERE workspace_id=? ORDER BY task_id")
        .bind(id_bytes(workspace_id.as_uuid())).fetch_all(&mut *connection).await.map_err(domain_storage)?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let id = task_id(
            row.try_get::<Vec<u8>, _>("task_id")
                .map_err(|_| Error::Storage)?,
        )?;
        let dependency_rows = sqlx::query_scalar::<_, Vec<u8>>("SELECT dependency_id FROM task_dependencies WHERE workspace_id=? AND task_id=? ORDER BY ordinal")
            .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(id.as_uuid())).fetch_all(&mut *connection).await.map_err(domain_storage)?;
        let dependency_ids = dependency_rows
            .into_iter()
            .map(task_id)
            .collect::<Result<Vec<_>>>()?;
        let worktree_id = row
            .try_get::<Option<Vec<u8>>, _>("worktree_id")
            .map_err(|_| Error::Storage)?
            .map(worktree_id)
            .transpose()?;
        values.push(Task::restore(TaskRecord {
            id,
            workspace_id,
            content: TaskContent {
                title: row.try_get("title").map_err(|_| Error::Storage)?,
                description: row.try_get("description").map_err(|_| Error::Storage)?,
                priority: parse_priority(
                    &row.try_get::<String, _>("priority")
                        .map_err(|_| Error::Storage)?,
                )?,
                scope_paths: load_strings(
                    connection,
                    "task_scope_paths",
                    "task_id",
                    id.as_uuid(),
                    workspace_id,
                )
                .await?,
                acceptance_notes: row
                    .try_get("acceptance_notes")
                    .map_err(|_| Error::Storage)?,
                dependency_ids,
            },
            status: parse_task_status(
                &row.try_get::<String, _>("status")
                    .map_err(|_| Error::Storage)?,
            )?,
            claimed_by_instance_id: owners.get(&id).copied(),
            worktree_id,
            created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
            updated_at: timestamp_row(&row, "updated_seconds", "updated_nanoseconds")?,
        })?);
    }
    Ok(values)
}

async fn load_instances(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<Vec<AgentInstance>> {
    let rows =
        sqlx::query("SELECT * FROM agent_instances WHERE workspace_id=? ORDER BY instance_id")
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_all(&mut *connection)
            .await
            .map_err(domain_storage)?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let id = instance_id(
            row.try_get::<Vec<u8>, _>("instance_id")
                .map_err(|_| Error::Storage)?,
        )?;
        let definition = row
            .try_get::<Option<Vec<u8>>, _>("definition_id")
            .map_err(|_| Error::Storage)?
            .map(definition_id)
            .transpose()?;
        let launch_definition = load_launch_snapshot(connection, workspace_id, id).await?;
        values.push(AgentInstance::restore(AgentInstanceRecord {
            id,
            session_id: session_id(
                row.try_get::<Vec<u8>, _>("session_id")
                    .map_err(|_| Error::Storage)?,
            )?,
            workspace_id,
            agent_definition_id: definition,
            task_id: row
                .try_get::<Option<Vec<u8>>, _>("task_id")
                .map_err(|_| Error::Storage)?
                .map(task_id)
                .transpose()?,
            launch_definition,
            worktree_id: row
                .try_get::<Option<Vec<u8>>, _>("worktree_id")
                .map_err(|_| Error::Storage)?
                .map(worktree_id)
                .transpose()?,
            working_directory: decode_native_path(
                &row.try_get::<String, _>("working_directory_codec")
                    .map_err(|_| Error::Storage)?,
                &row.try_get::<Vec<u8>, _>("working_directory")
                    .map_err(|_| Error::Storage)?,
            )
            .map_err(|_| Error::Storage)?,
            status: parse_instance_status(
                &row.try_get::<String, _>("status")
                    .map_err(|_| Error::Storage)?,
            )?,
            started_at: timestamp_row(&row, "started_seconds", "started_nanoseconds")?,
            last_observed_at: timestamp_row(&row, "observed_seconds", "observed_nanoseconds")?,
            ended_at: optional_timestamp_row(&row, "ended_seconds", "ended_nanoseconds")?,
            exit_code: row.try_get("exit_code").map_err(|_| Error::Storage)?,
            terminal_size: TerminalSize::new(
                u16::try_from(
                    row.try_get::<i64, _>("terminal_rows")
                        .map_err(|_| Error::Storage)?,
                )
                .map_err(|_| Error::Storage)?,
                u16::try_from(
                    row.try_get::<i64, _>("terminal_columns")
                        .map_err(|_| Error::Storage)?,
                )
                .map_err(|_| Error::Storage)?,
            )?,
        })?);
    }
    Ok(values)
}

async fn load_launch_snapshot(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    instance: AgentInstanceId,
) -> Result<Option<LaunchDefinitionSnapshot>> {
    let row = sqlx::query("SELECT definition_id,display_name,command,enabled FROM instance_launch_definitions WHERE workspace_id=? AND instance_id=?")
        .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(instance.as_uuid())).fetch_optional(&mut *connection).await.map_err(domain_storage)?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(LaunchDefinitionSnapshot {
        definition_id: definition_id(
            row.try_get::<Vec<u8>, _>("definition_id")
                .map_err(|_| Error::Storage)?,
        )?,
        display_name: row.try_get("display_name").map_err(|_| Error::Storage)?,
        command: row.try_get("command").map_err(|_| Error::Storage)?,
        arguments: load_strings(
            connection,
            "instance_launch_arguments",
            "instance_id",
            instance.as_uuid(),
            workspace_id,
        )
        .await?,
        environment_allowlist: load_strings(
            connection,
            "instance_launch_environment",
            "instance_id",
            instance.as_uuid(),
            workspace_id,
        )
        .await?,
        capabilities: load_strings(
            connection,
            "instance_launch_capabilities",
            "instance_id",
            instance.as_uuid(),
            workspace_id,
        )
        .await?,
        enabled: int_bool(row.try_get("enabled").map_err(|_| Error::Storage)?)?,
    }))
}

async fn load_claims(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<Vec<Claim>> {
    let rows =
        sqlx::query("SELECT * FROM claims WHERE workspace_id=? ORDER BY opening_event_sequence")
            .bind(id_bytes(workspace_id.as_uuid()))
            .fetch_all(&mut *connection)
            .await
            .map_err(domain_storage)?;
    rows.into_iter()
        .map(|row| decode_claim(row, workspace_id))
        .collect()
}

fn decode_claim(row: sqlx::sqlite::SqliteRow, workspace_id: WorkspaceId) -> Result<Claim> {
    Claim::restore(ClaimRecord {
        id: claim_id(
            row.try_get::<Vec<u8>, _>("claim_id")
                .map_err(|_| Error::Storage)?,
        )?,
        workspace_id,
        task_id: task_id(
            row.try_get::<Vec<u8>, _>("task_id")
                .map_err(|_| Error::Storage)?,
        )?,
        instance_id: instance_id(
            row.try_get::<Vec<u8>, _>("instance_id")
                .map_err(|_| Error::Storage)?,
        )?,
        requested_by: parse_actor(
            &row.try_get::<String, _>("requested_by_kind")
                .map_err(|_| Error::Storage)?,
            row.try_get::<Option<Vec<u8>>, _>("requested_by_instance_id")
                .map_err(|_| Error::Storage)?,
        )?,
        opened_at: timestamp_row(&row, "opened_seconds", "opened_nanoseconds")?,
        closed_at: optional_timestamp_row(&row, "closed_seconds", "closed_nanoseconds")?,
        close_reason: row
            .try_get::<Option<String>, _>("close_reason")
            .map_err(|_| Error::Storage)?
            .map(|x| parse_close_reason(&x))
            .transpose()?,
        closed_by: match row
            .try_get::<Option<String>, _>("closed_by_kind")
            .map_err(|_| Error::Storage)?
        {
            Some(kind) => Some(parse_actor(
                &kind,
                row.try_get::<Option<Vec<u8>>, _>("closed_by_instance_id")
                    .map_err(|_| Error::Storage)?,
            )?),
            None => None,
        },
    })
}

async fn load_claim(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    id: &[u8],
) -> Result<Claim> {
    let row = sqlx::query("SELECT * FROM claims WHERE workspace_id=? AND claim_id=?")
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(id)
        .fetch_optional(&mut *connection)
        .await
        .map_err(domain_storage)?
        .ok_or(Error::Integrity)?;
    decode_claim(row, workspace_id)
}

async fn load_progress(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<Vec<ProgressEntry>> {
    let rows = sqlx::query(
        "SELECT * FROM progress_entries WHERE workspace_id=? ORDER BY creation_event_sequence",
    )
    .bind(id_bytes(workspace_id.as_uuid()))
    .fetch_all(&mut *connection)
    .await
    .map_err(domain_storage)?;
    rows.into_iter()
        .map(|row| decode_progress(row, workspace_id))
        .collect()
}

fn decode_progress(
    row: sqlx::sqlite::SqliteRow,
    workspace_id: WorkspaceId,
) -> Result<ProgressEntry> {
    ProgressEntry::restore(ProgressEntryRecord {
        id: progress_id(
            row.try_get::<Vec<u8>, _>("progress_id")
                .map_err(|_| Error::Storage)?,
        )?,
        workspace_id,
        task_id: task_id(
            row.try_get::<Vec<u8>, _>("task_id")
                .map_err(|_| Error::Storage)?,
        )?,
        agent_instance_id: row
            .try_get::<Option<Vec<u8>>, _>("instance_id")
            .map_err(|_| Error::Storage)?
            .map(instance_id)
            .transpose()?,
        summary: row.try_get("summary").map_err(|_| Error::Storage)?,
        verification: row.try_get("verification").map_err(|_| Error::Storage)?,
        created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
    })
}

async fn load_progress_entry(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    id: &[u8],
) -> Result<ProgressEntry> {
    let row = sqlx::query("SELECT * FROM progress_entries WHERE workspace_id=? AND progress_id=?")
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(id)
        .fetch_optional(&mut *connection)
        .await
        .map_err(domain_storage)?
        .ok_or(Error::Integrity)?;
    decode_progress(row, workspace_id)
}

async fn load_handovers(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
) -> Result<Vec<Handover>> {
    let rows = sqlx::query(
        "SELECT * FROM handovers WHERE workspace_id=? ORDER BY creation_event_sequence",
    )
    .bind(id_bytes(workspace_id.as_uuid()))
    .fetch_all(&mut *connection)
    .await
    .map_err(domain_storage)?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        values.push(decode_handover(connection, row, workspace_id).await?);
    }
    Ok(values)
}

async fn decode_handover(
    connection: &mut SqliteConnection,
    row: sqlx::sqlite::SqliteRow,
    workspace_id: WorkspaceId,
) -> Result<Handover> {
    let id = handover_id(
        row.try_get::<Vec<u8>, _>("handover_id")
            .map_err(|_| Error::Storage)?,
    )?;
    Handover::restore(HandoverRecord {
        id,
        workspace_id,
        task_id: task_id(
            row.try_get::<Vec<u8>, _>("task_id")
                .map_err(|_| Error::Storage)?,
        )?,
        from_agent_instance_id: row
            .try_get::<Option<Vec<u8>>, _>("from_instance_id")
            .map_err(|_| Error::Storage)?
            .map(instance_id)
            .transpose()?,
        content: HandoverContent {
            summary: row.try_get("summary").map_err(|_| Error::Storage)?,
            decisions: row.try_get("decisions").map_err(|_| Error::Storage)?,
            changed_paths: load_strings(
                connection,
                "handover_changed_paths",
                "handover_id",
                id.as_uuid(),
                workspace_id,
            )
            .await?,
            verification_performed: row
                .try_get("verification_performed")
                .map_err(|_| Error::Storage)?,
            open_questions: row.try_get("open_questions").map_err(|_| Error::Storage)?,
            recommended_next_action: row
                .try_get("recommended_next_action")
                .map_err(|_| Error::Storage)?,
        },
        created_at: timestamp_row(&row, "created_seconds", "created_nanoseconds")?,
    })
}

async fn load_handover(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    id: &[u8],
) -> Result<Handover> {
    let row = sqlx::query("SELECT * FROM handovers WHERE workspace_id=? AND handover_id=?")
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(id)
        .fetch_optional(&mut *connection)
        .await
        .map_err(domain_storage)?
        .ok_or(Error::Integrity)?;
    decode_handover(connection, row, workspace_id).await
}

async fn load_strings(
    connection: &mut SqliteConnection,
    table: &str,
    id_column: &str,
    id: Uuid,
    workspace_id: WorkspaceId,
) -> Result<Vec<String>> {
    let allowed = [
        "definition_arguments",
        "definition_environment",
        "definition_capabilities",
        "task_scope_paths",
        "instance_launch_arguments",
        "instance_launch_environment",
        "instance_launch_capabilities",
        "handover_changed_paths",
    ];
    if !allowed.contains(&table) {
        return Err(Error::Storage);
    }
    let query = format!(
        "SELECT value FROM {table} WHERE workspace_id=? AND {id_column}=? ORDER BY ordinal"
    );
    sqlx::query_scalar(AssertSqlSafe(query))
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(id_bytes(id))
        .fetch_all(&mut *connection)
        .await
        .map_err(domain_storage)
}

async fn persist_state(
    connection: &mut SqliteConnection,
    before: Option<&WorkspaceState>,
    after: &WorkspaceState,
    events: &[WorkspaceEvent],
    revision: u64,
) -> Result<()> {
    let workspace_id = after.workspace().record().id;
    if before.is_none() {
        let workspace = after.workspace().record();
        let root = encode_native_path(&workspace.project_root).map_err(|_| Error::Storage)?;
        let (created_s, created_ns) = encode_timestamp(workspace.created_at);
        let (updated_s, updated_ns) = encode_timestamp(workspace.updated_at);
        sqlx::query("INSERT INTO workspaces VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(workspace.display_name.as_str())
            .bind(root.tag)
            .bind(root.bytes)
            .bind(created_s)
            .bind(created_ns)
            .bind(updated_s)
            .bind(updated_ns)
            .bind(i64::from(workspace.schema_version))
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        sqlx::query("INSERT INTO workspace_meta VALUES(1,?,?,?,?,?)")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(encode_counter(revision).to_vec())
            .bind(encode_counter(u64::try_from(events.len()).map_err(|_| Error::Storage)?).to_vec())
            .bind(encode_counter(1).to_vec())
            .bind(1_i64)
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
    } else {
        let workspace = after.workspace().record();
        let (updated_s, updated_ns) = encode_timestamp(workspace.updated_at);
        sqlx::query("UPDATE workspaces SET display_name=?,updated_seconds=?,updated_nanoseconds=? WHERE workspace_id=?")
            .bind(workspace.display_name.as_str()).bind(updated_s).bind(updated_ns).bind(id_bytes(workspace_id.as_uuid()))
            .execute(&mut *connection).await.map_err(domain_storage)?;
    }
    persist_definitions(
        connection,
        before.map(WorkspaceState::definitions).unwrap_or(&[]),
        after.definitions(),
        workspace_id,
    )
    .await?;
    persist_tasks(
        connection,
        before.map(WorkspaceState::tasks).unwrap_or(&[]),
        after.tasks(),
        workspace_id,
    )
    .await?;
    persist_worktree_state(connection, before, after, workspace_id).await?;
    persist_instances(
        connection,
        before.map(WorkspaceState::instances).unwrap_or(&[]),
        after.instances(),
        workspace_id,
    )
    .await?;
    persist_claims(
        connection,
        before.map(WorkspaceState::claims).unwrap_or(&[]),
        after.claims(),
        workspace_id,
        events,
    )
    .await?;
    persist_progress(
        connection,
        before.map(WorkspaceState::progress).unwrap_or(&[]),
        after.progress(),
        workspace_id,
        events,
    )
    .await?;
    persist_handovers(
        connection,
        before.map(WorkspaceState::handovers).unwrap_or(&[]),
        after.handovers(),
        workspace_id,
        events,
    )
    .await?;
    for event in events {
        insert_event(connection, event).await?;
    }
    let last = events.last().ok_or(Error::Storage)?.record().sequence;
    sqlx::query("UPDATE workspace_meta SET revision=?,last_event_sequence=? WHERE workspace_id=?")
        .bind(encode_counter(revision).to_vec())
        .bind(encode_counter(last).to_vec())
        .bind(id_bytes(workspace_id.as_uuid()))
        .execute(&mut *connection)
        .await
        .map_err(domain_storage)?;
    Ok(())
}

async fn persist_worktree_state(
    connection: &mut SqliteConnection,
    before: Option<&WorkspaceState>,
    after: &WorkspaceState,
    workspace_id: WorkspaceId,
) -> Result<()> {
    let previous_roots = before.map(WorkspaceState::approved_roots).unwrap_or(&[]);
    for root in after.approved_roots() {
        if previous_roots.contains(root) {
            continue;
        }
        let record = root.record();
        let path = encode_native_path(&record.canonical_parent).map_err(|_| Error::Storage)?;
        let (seconds, nanos) = encode_timestamp(record.created_at);
        sqlx::query("INSERT INTO approved_worktree_roots VALUES(?,?,?,?,?,?,?,?)")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(record.id.as_uuid()))
            .bind(path.tag)
            .bind(path.bytes)
            .bind(record.filesystem_identity.as_deref())
            .bind(i64::from(record.private_default))
            .bind(seconds)
            .bind(nanos)
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
    }
    let previous_intents = before.map(WorkspaceState::worktree_intents).unwrap_or(&[]);
    for intent in after.worktree_intents() {
        let record = intent.record();
        if previous_intents.iter().any(|candidate| candidate == intent) {
            continue;
        }
        if previous_intents
            .iter()
            .any(|candidate| candidate.record().id == record.id)
        {
            let (updated_s, updated_ns) = encode_timestamp(record.updated_at);
            sqlx::query("UPDATE worktree_intents SET phase=?,reason=?,updated_seconds=?,updated_nanoseconds=? WHERE workspace_id=? AND operation_id=?")
                .bind(worktree_phase_name(record.phase)).bind(record.reason.map(worktree_reason_name)).bind(updated_s).bind(updated_ns)
                .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid())).execute(&mut *connection).await.map_err(domain_storage)?;
        } else {
            let repository =
                encode_native_path(&record.repository_identity).map_err(|_| Error::Storage)?;
            let common = encode_native_path(&record.common_directory_identity)
                .map_err(|_| Error::Storage)?;
            let destination =
                encode_native_path(&record.destination).map_err(|_| Error::Storage)?;
            let (created_s, created_ns) = encode_timestamp(record.created_at);
            let (updated_s, updated_ns) = encode_timestamp(record.updated_at);
            sqlx::query("INSERT INTO worktree_intents VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid())).bind(id_bytes(record.task_id.as_uuid())).bind(id_bytes(record.worktree_id.as_uuid())).bind(id_bytes(record.root_id.as_uuid()))
                .bind(i64::from(record.schema_version)).bind(encode_counter(record.expected_revision).to_vec())
                .bind(repository.tag).bind(repository.bytes).bind(common.tag).bind(common.bytes).bind(destination.tag).bind(destination.bytes)
                .bind(record.branch.as_str()).bind(record.base_expression.as_str()).bind(record.resolved_commit.as_str()).bind(record.request_fingerprint.as_slice())
                .bind(worktree_phase_name(record.phase)).bind(record.reason.map(worktree_reason_name)).bind(created_s).bind(created_ns).bind(updated_s).bind(updated_ns)
                .execute(&mut *connection).await.map_err(domain_storage)?;
        }
    }
    let previous_worktrees = before.map(WorkspaceState::worktrees).unwrap_or(&[]);
    for worktree in after.worktrees() {
        let record = worktree.record();
        if previous_worktrees
            .iter()
            .any(|candidate| candidate == worktree)
        {
            continue;
        }
        if previous_worktrees
            .iter()
            .any(|candidate| candidate.record().id == record.id)
        {
            let (updated_s, updated_ns) = encode_timestamp(record.updated_at);
            sqlx::query("UPDATE worktrees SET health=?,updated_seconds=?,updated_nanoseconds=? WHERE workspace_id=? AND worktree_id=?")
                .bind(worktree_health_name(record.health)).bind(updated_s).bind(updated_ns).bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid()))
                .execute(&mut *connection).await.map_err(domain_storage)?;
        } else {
            let checkout = encode_native_path(&record.checkout_path).map_err(|_| Error::Storage)?;
            let common = encode_native_path(&record.common_directory_identity)
                .map_err(|_| Error::Storage)?;
            let (created_s, created_ns) = encode_timestamp(record.created_at);
            let (updated_s, updated_ns) = encode_timestamp(record.updated_at);
            sqlx::query("INSERT INTO worktrees VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(id_bytes(workspace_id.as_uuid()))
                .bind(id_bytes(record.id.as_uuid()))
                .bind(id_bytes(record.task_id.as_uuid()))
                .bind(id_bytes(record.operation_id.as_uuid()))
                .bind(id_bytes(record.root_id.as_uuid()))
                .bind(checkout.tag)
                .bind(checkout.bytes)
                .bind(common.tag)
                .bind(common.bytes)
                .bind(record.branch_ref.as_str())
                .bind(record.initial_base_commit.as_str())
                .bind(worktree_health_name(record.health))
                .bind(created_s)
                .bind(created_ns)
                .bind(updated_s)
                .bind(updated_ns)
                .execute(&mut *connection)
                .await
                .map_err(domain_storage)?;
        }
    }
    Ok(())
}

async fn persist_definitions(
    connection: &mut SqliteConnection,
    before: &[AgentDefinition],
    after: &[AgentDefinition],
    workspace_id: WorkspaceId,
) -> Result<()> {
    for definition in after {
        let record = definition.record();
        let changed = before
            .iter()
            .find(|x| x.record().id == record.id)
            .is_none_or(|x| x != definition);
        if !changed {
            continue;
        }
        sqlx::query("INSERT INTO agent_definitions VALUES(?,?,?,?,?) ON CONFLICT(workspace_id,definition_id) DO UPDATE SET display_name=excluded.display_name,command=excluded.command,enabled=excluded.enabled")
            .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid())).bind(record.display_name.as_str()).bind(record.command.as_str()).bind(i64::from(record.enabled))
            .execute(&mut *connection).await.map_err(domain_storage)?;
        replace_strings(
            connection,
            "definition_arguments",
            "definition_id",
            record.id.as_uuid(),
            workspace_id,
            &record.arguments,
        )
        .await?;
        replace_strings(
            connection,
            "definition_environment",
            "definition_id",
            record.id.as_uuid(),
            workspace_id,
            &record.environment_allowlist,
        )
        .await?;
        replace_strings(
            connection,
            "definition_capabilities",
            "definition_id",
            record.id.as_uuid(),
            workspace_id,
            &record.capabilities,
        )
        .await?;
    }
    Ok(())
}

async fn persist_tasks(
    connection: &mut SqliteConnection,
    before: &[Task],
    after: &[Task],
    workspace_id: WorkspaceId,
) -> Result<()> {
    for task in after {
        let record = task.record();
        let changed = before
            .iter()
            .find(|x| x.record().id == record.id)
            .is_none_or(|x| x != task);
        if !changed {
            continue;
        }
        let (created_s, created_ns) = encode_timestamp(record.created_at);
        let (updated_s, updated_ns) = encode_timestamp(record.updated_at);
        sqlx::query("INSERT INTO tasks VALUES(?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(workspace_id,task_id) DO UPDATE SET title=excluded.title,description=excluded.description,priority=excluded.priority,status=excluded.status,acceptance_notes=excluded.acceptance_notes,worktree_id=excluded.worktree_id,updated_seconds=excluded.updated_seconds,updated_nanoseconds=excluded.updated_nanoseconds")
            .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid())).bind(record.content.title.as_str()).bind(record.content.description.as_str())
            .bind(priority_name(record.content.priority)).bind(task_status_name(record.status)).bind(record.content.acceptance_notes.as_str())
            .bind(record.worktree_id.map(|x| id_bytes(x.as_uuid()))).bind(created_s).bind(created_ns).bind(updated_s).bind(updated_ns)
            .execute(&mut *connection).await.map_err(domain_storage)?;
        replace_strings(
            connection,
            "task_scope_paths",
            "task_id",
            record.id.as_uuid(),
            workspace_id,
            &record.content.scope_paths,
        )
        .await?;
        sqlx::query("DELETE FROM task_dependencies WHERE workspace_id=? AND task_id=?")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(record.id.as_uuid()))
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        for (ordinal, dependency) in record.content.dependency_ids.iter().enumerate() {
            sqlx::query("INSERT INTO task_dependencies VALUES(?,?,?,?)")
                .bind(id_bytes(workspace_id.as_uuid()))
                .bind(id_bytes(record.id.as_uuid()))
                .bind(i64::try_from(ordinal).map_err(|_| Error::Storage)?)
                .bind(id_bytes(dependency.as_uuid()))
                .execute(&mut *connection)
                .await
                .map_err(domain_storage)?;
        }
    }
    Ok(())
}

async fn persist_instances(
    connection: &mut SqliteConnection,
    before: &[AgentInstance],
    after: &[AgentInstance],
    workspace_id: WorkspaceId,
) -> Result<()> {
    for instance in after {
        let record = instance.record();
        match before.iter().find(|x| x.record().id == record.id) {
            Some(old) if old == instance => continue,
            Some(_) => {
                let (observed_s, observed_ns) = encode_timestamp(record.last_observed_at);
                let ended = record.ended_at.map(encode_timestamp);
                sqlx::query("UPDATE agent_instances SET status=?,observed_seconds=?,observed_nanoseconds=?,ended_seconds=?,ended_nanoseconds=?,exit_code=? WHERE workspace_id=? AND instance_id=?")
                    .bind(instance_status_name(record.status)).bind(observed_s).bind(observed_ns).bind(ended.map(|x| x.0)).bind(ended.map(|x| x.1)).bind(record.exit_code)
                    .bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid())).execute(&mut *connection).await.map_err(domain_storage)?;
            }
            None => {
                let path =
                    encode_native_path(&record.working_directory).map_err(|_| Error::Storage)?;
                let (started_s, started_ns) = encode_timestamp(record.started_at);
                let (observed_s, observed_ns) = encode_timestamp(record.last_observed_at);
                let ended = record.ended_at.map(encode_timestamp);
                sqlx::query("INSERT INTO agent_instances(workspace_id,instance_id,session_id,definition_id,task_id,working_directory_codec,working_directory,status,started_seconds,started_nanoseconds,observed_seconds,observed_nanoseconds,ended_seconds,ended_nanoseconds,exit_code,terminal_rows,terminal_columns,worktree_id) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(id_bytes(workspace_id.as_uuid()))
                .bind(id_bytes(record.id.as_uuid()))
                .bind(id_bytes(record.session_id.as_uuid()))
                .bind(record.agent_definition_id.map(|x| id_bytes(x.as_uuid())))
                .bind(record.task_id.map(|x| id_bytes(x.as_uuid())))
                .bind(path.tag)
                .bind(path.bytes)
                .bind(instance_status_name(record.status))
                .bind(started_s)
                .bind(started_ns)
                .bind(observed_s)
                .bind(observed_ns)
                .bind(ended.map(|x| x.0))
                .bind(ended.map(|x| x.1))
                .bind(record.exit_code)
                .bind(i64::from(record.terminal_size.rows()))
                .bind(i64::from(record.terminal_size.columns()))
                .bind(record.worktree_id.map(|value| id_bytes(value.as_uuid())))
                .execute(&mut *connection)
                .await
                .map_err(domain_storage)?;
                if let Some(snapshot) = &record.launch_definition {
                    insert_launch_snapshot(connection, workspace_id, record.id, snapshot).await?;
                }
            }
        }
    }
    Ok(())
}

async fn insert_launch_snapshot(
    connection: &mut SqliteConnection,
    workspace_id: WorkspaceId,
    instance_id: AgentInstanceId,
    snapshot: &LaunchDefinitionSnapshot,
) -> Result<()> {
    sqlx::query("INSERT INTO instance_launch_definitions VALUES(?,?,?,?,?,?)")
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(id_bytes(instance_id.as_uuid()))
        .bind(id_bytes(snapshot.definition_id.as_uuid()))
        .bind(snapshot.display_name.as_str())
        .bind(snapshot.command.as_str())
        .bind(i64::from(snapshot.enabled))
        .execute(&mut *connection)
        .await
        .map_err(domain_storage)?;
    append_strings(
        connection,
        "instance_launch_arguments",
        "instance_id",
        instance_id.as_uuid(),
        workspace_id,
        &snapshot.arguments,
    )
    .await?;
    append_strings(
        connection,
        "instance_launch_environment",
        "instance_id",
        instance_id.as_uuid(),
        workspace_id,
        &snapshot.environment_allowlist,
    )
    .await?;
    append_strings(
        connection,
        "instance_launch_capabilities",
        "instance_id",
        instance_id.as_uuid(),
        workspace_id,
        &snapshot.capabilities,
    )
    .await
}

async fn persist_claims(
    connection: &mut SqliteConnection,
    before: &[Claim],
    after: &[Claim],
    workspace_id: WorkspaceId,
    events: &[WorkspaceEvent],
) -> Result<()> {
    for claim in after {
        let record = claim.record();
        match before.iter().find(|x| x.record().id == record.id) {
            None => {
                let sequence = event_sequence(
                    events,
                    |payload| matches!(payload, EventPayload::ClaimOpened { id, .. } if *id == record.id),
                )?;
                let (opened_s, opened_ns) = encode_timestamp(record.opened_at);
                let (requested_kind, requested_id) = actor_parts(record.requested_by);
                sqlx::query("INSERT INTO claims VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                    .bind(id_bytes(workspace_id.as_uuid()))
                    .bind(id_bytes(record.id.as_uuid()))
                    .bind(id_bytes(record.task_id.as_uuid()))
                    .bind(id_bytes(record.instance_id.as_uuid()))
                    .bind(requested_kind)
                    .bind(requested_id)
                    .bind(opened_s)
                    .bind(opened_ns)
                    .bind(Option::<i64>::None)
                    .bind(Option::<i64>::None)
                    .bind(Option::<String>::None)
                    .bind(Option::<String>::None)
                    .bind(Option::<Vec<u8>>::None)
                    .bind(encode_counter(sequence).to_vec())
                    .execute(&mut *connection)
                    .await
                    .map_err(domain_storage)?;
            }
            Some(old) if old == claim => {}
            Some(old) if old.record().closed_at.is_none() && record.closed_at.is_some() => {
                let (closed_s, closed_ns) = encode_timestamp(record.closed_at.ok_or(Error::State)?);
                let (closed_kind, closed_id) = actor_parts(record.closed_by.ok_or(Error::State)?);
                sqlx::query("UPDATE claims SET closed_seconds=?,closed_nanoseconds=?,close_reason=?,closed_by_kind=?,closed_by_instance_id=? WHERE workspace_id=? AND claim_id=? AND closed_seconds IS NULL")
                    .bind(closed_s).bind(closed_ns).bind(close_reason_name(record.close_reason.ok_or(Error::State)?)).bind(closed_kind).bind(closed_id).bind(id_bytes(workspace_id.as_uuid())).bind(id_bytes(record.id.as_uuid()))
                    .execute(&mut *connection).await.map_err(domain_storage)?;
            }
            Some(_) => return Err(Error::State),
        }
    }
    Ok(())
}

async fn persist_progress(
    connection: &mut SqliteConnection,
    before: &[ProgressEntry],
    after: &[ProgressEntry],
    workspace_id: WorkspaceId,
    events: &[WorkspaceEvent],
) -> Result<()> {
    for entry in &after[before.len()..] {
        let record = entry.record();
        let sequence = event_sequence(
            events,
            |payload| matches!(payload, EventPayload::ProgressAdded { id, .. } if *id == record.id),
        )?;
        let (created_s, created_ns) = encode_timestamp(record.created_at);
        sqlx::query("INSERT INTO progress_entries VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(record.id.as_uuid()))
            .bind(id_bytes(record.task_id.as_uuid()))
            .bind(record.agent_instance_id.map(|x| id_bytes(x.as_uuid())))
            .bind(record.summary.as_str())
            .bind(record.verification.as_str())
            .bind(created_s)
            .bind(created_ns)
            .bind(encode_counter(sequence).to_vec())
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
    }
    Ok(())
}

async fn persist_handovers(
    connection: &mut SqliteConnection,
    before: &[Handover],
    after: &[Handover],
    workspace_id: WorkspaceId,
    events: &[WorkspaceEvent],
) -> Result<()> {
    for handover in &after[before.len()..] {
        let record = handover.record();
        let sequence = event_sequence(
            events,
            |payload| matches!(payload, EventPayload::HandoverPrepared { id, .. } if *id == record.id),
        )?;
        let (created_s, created_ns) = encode_timestamp(record.created_at);
        let content = &record.content;
        sqlx::query("INSERT INTO handovers VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(record.id.as_uuid()))
            .bind(id_bytes(record.task_id.as_uuid()))
            .bind(record.from_agent_instance_id.map(|x| id_bytes(x.as_uuid())))
            .bind(content.summary.as_str())
            .bind(content.decisions.as_str())
            .bind(content.verification_performed.as_str())
            .bind(content.open_questions.as_str())
            .bind(content.recommended_next_action.as_str())
            .bind(created_s)
            .bind(created_ns)
            .bind(encode_counter(sequence).to_vec())
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
        append_strings(
            connection,
            "handover_changed_paths",
            "handover_id",
            record.id.as_uuid(),
            workspace_id,
            &content.changed_paths,
        )
        .await?;
    }
    Ok(())
}

async fn insert_event(connection: &mut SqliteConnection, event: &WorkspaceEvent) -> Result<()> {
    let record = event.record();
    let (seconds, nanoseconds) = encode_timestamp(record.timestamp);
    let (actor_kind, actor_id) = actor_parts(record.actor);
    let (entity_kind, entity_id) = entity_parts(record.entity_id);
    let payload = serde_json::to_vec(&record.payload).map_err(|_| Error::Storage)?;
    if payload.len() > 16384 {
        return Err(Error::Storage);
    }
    sqlx::query("INSERT INTO workspace_events VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(id_bytes(record.workspace_id.as_uuid()))
        .bind(encode_counter(record.sequence).to_vec())
        .bind(id_bytes(record.event_id.as_uuid()))
        .bind(event_type_name(record.event_type))
        .bind(entity_kind)
        .bind(id_bytes(entity_id))
        .bind(seconds)
        .bind(nanoseconds)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(i64::from(record.payload_version))
        .bind(payload)
        .execute(&mut *connection)
        .await
        .map_err(domain_storage)?;
    Ok(())
}

async fn replace_strings(
    connection: &mut SqliteConnection,
    table: &str,
    id_column: &str,
    id: Uuid,
    workspace_id: WorkspaceId,
    values: &[String],
) -> Result<()> {
    let query = format!("DELETE FROM {table} WHERE workspace_id=? AND {id_column}=?");
    sqlx::query(AssertSqlSafe(query))
        .bind(id_bytes(workspace_id.as_uuid()))
        .bind(id_bytes(id))
        .execute(&mut *connection)
        .await
        .map_err(domain_storage)?;
    append_strings(connection, table, id_column, id, workspace_id, values).await
}

async fn append_strings(
    connection: &mut SqliteConnection,
    table: &str,
    id_column: &str,
    id: Uuid,
    workspace_id: WorkspaceId,
    values: &[String],
) -> Result<()> {
    let allowed = [
        "definition_arguments",
        "definition_environment",
        "definition_capabilities",
        "task_scope_paths",
        "instance_launch_arguments",
        "instance_launch_environment",
        "instance_launch_capabilities",
        "handover_changed_paths",
    ];
    if !allowed.contains(&table) {
        return Err(Error::Storage);
    }
    let query =
        format!("INSERT INTO {table}(workspace_id,{id_column},ordinal,value) VALUES(?,?,?,?)");
    for (ordinal, value) in values.iter().enumerate() {
        sqlx::query(AssertSqlSafe(query.clone()))
            .bind(id_bytes(workspace_id.as_uuid()))
            .bind(id_bytes(id))
            .bind(i64::try_from(ordinal).map_err(|_| Error::Storage)?)
            .bind(value)
            .execute(&mut *connection)
            .await
            .map_err(domain_storage)?;
    }
    Ok(())
}

fn event_sequence(
    events: &[WorkspaceEvent],
    predicate: impl Fn(&EventPayload) -> bool,
) -> Result<u64> {
    events
        .iter()
        .find(|event| predicate(&event.record().payload))
        .map(|event| event.record().sequence)
        .ok_or(Error::State)
}
fn id_bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}
fn decode_uuid(bytes: Vec<u8>) -> Result<Uuid> {
    Uuid::from_slice(&bytes).map_err(|_| Error::Storage)
}
macro_rules! typed_id {
    ($name:ident, $ty:ty) => {
        fn $name(bytes: Vec<u8>) -> Result<$ty> {
            Ok(<$ty>::from_uuid(decode_uuid(bytes)?))
        }
    };
}
typed_id!(workspace_id, WorkspaceId);
typed_id!(definition_id, AgentDefinitionId);
typed_id!(task_id, TaskId);
typed_id!(instance_id, AgentInstanceId);
typed_id!(session_id, TerminalSessionId);
typed_id!(claim_id, ClaimId);
typed_id!(progress_id, ProgressEntryId);
typed_id!(handover_id, HandoverId);
typed_id!(worktree_id, WorktreeId);
typed_id!(worktree_operation_id, WorktreeOperationId);
typed_id!(approved_root_id, ApprovedRootId);
typed_id!(event_id, EventId);
fn worktree_phase_name(value: WorktreePhase) -> &'static str {
    match value {
        WorktreePhase::Prepared => "prepared",
        WorktreePhase::Applying => "applying",
        WorktreePhase::Ready => "ready",
        WorktreePhase::Failed => "failed",
        WorktreePhase::NeedsAttention => "needs_attention",
    }
}
fn parse_worktree_phase(value: &str) -> Result<WorktreePhase> {
    match value {
        "prepared" => Ok(WorktreePhase::Prepared),
        "applying" => Ok(WorktreePhase::Applying),
        "ready" => Ok(WorktreePhase::Ready),
        "failed" => Ok(WorktreePhase::Failed),
        "needs_attention" => Ok(WorktreePhase::NeedsAttention),
        _ => Err(Error::Storage),
    }
}
fn worktree_health_name(value: WorktreeHealth) -> &'static str {
    match value {
        WorktreeHealth::Ready => "ready",
        WorktreeHealth::Missing => "missing",
        WorktreeHealth::Mismatch => "mismatch",
        WorktreeHealth::Unavailable => "unavailable",
    }
}
fn parse_worktree_health(value: &str) -> Result<WorktreeHealth> {
    match value {
        "ready" => Ok(WorktreeHealth::Ready),
        "missing" => Ok(WorktreeHealth::Missing),
        "mismatch" => Ok(WorktreeHealth::Mismatch),
        "unavailable" => Ok(WorktreeHealth::Unavailable),
        _ => Err(Error::Storage),
    }
}
fn worktree_reason_name(value: WorktreeReason) -> &'static str {
    match value {
        WorktreeReason::GitMissing => "git_missing",
        WorktreeReason::GitUnsupported => "git_unsupported",
        WorktreeReason::NotRepository => "not_repository",
        WorktreeReason::UnsupportedRoot => "unsupported_root",
        WorktreeReason::InvalidReference => "invalid_reference",
        WorktreeReason::BranchConflict => "branch_conflict",
        WorktreeReason::DestinationConflict => "destination_conflict",
        WorktreeReason::PathRejected => "path_rejected",
        WorktreeReason::Busy => "busy",
        WorktreeReason::UnsupportedCheckoutFilter => "unsupported_checkout_filter",
        WorktreeReason::StorageUnavailable => "storage_unavailable",
        WorktreeReason::OutcomeUncertain => "outcome_uncertain",
        WorktreeReason::CancelledTask => "cancelled_task",
    }
}
fn parse_worktree_reason(value: &str) -> Result<WorktreeReason> {
    match value {
        "git_missing" => Ok(WorktreeReason::GitMissing),
        "git_unsupported" => Ok(WorktreeReason::GitUnsupported),
        "not_repository" => Ok(WorktreeReason::NotRepository),
        "unsupported_root" => Ok(WorktreeReason::UnsupportedRoot),
        "invalid_reference" => Ok(WorktreeReason::InvalidReference),
        "branch_conflict" => Ok(WorktreeReason::BranchConflict),
        "destination_conflict" => Ok(WorktreeReason::DestinationConflict),
        "path_rejected" => Ok(WorktreeReason::PathRejected),
        "busy" => Ok(WorktreeReason::Busy),
        "unsupported_checkout_filter" => Ok(WorktreeReason::UnsupportedCheckoutFilter),
        "storage_unavailable" => Ok(WorktreeReason::StorageUnavailable),
        "outcome_uncertain" => Ok(WorktreeReason::OutcomeUncertain),
        "cancelled_task" => Ok(WorktreeReason::CancelledTask),
        _ => Err(Error::Storage),
    }
}
fn timestamp_row(row: &sqlx::sqlite::SqliteRow, seconds: &str, nanos: &str) -> Result<Timestamp> {
    decode_timestamp(
        row.try_get(seconds).map_err(|_| Error::Storage)?,
        row.try_get(nanos).map_err(|_| Error::Storage)?,
    )
    .map_err(|_| Error::Storage)
}
fn optional_timestamp_row(
    row: &sqlx::sqlite::SqliteRow,
    seconds: &str,
    nanos: &str,
) -> Result<Option<Timestamp>> {
    let seconds: Option<i64> = row.try_get(seconds).map_err(|_| Error::Storage)?;
    let nanos: Option<i64> = row.try_get(nanos).map_err(|_| Error::Storage)?;
    match (seconds, nanos) {
        (None, None) => Ok(None),
        (Some(s), Some(n)) => decode_timestamp(s, n).map(Some).map_err(|_| Error::Storage),
        _ => Err(Error::Storage),
    }
}
fn int_bool(value: i64) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Storage),
    }
}
fn domain_storage(error: sqlx::Error) -> Error {
    match map_sqlx(error) {
        crate::StorageError::NotFound => Error::Reference,
        crate::StorageError::Busy => Error::StorageBusy,
        crate::StorageError::ReadOnly => Error::ReadOnly,
        crate::StorageError::IncompatibleVersion => Error::Version,
        crate::StorageError::Integrity => Error::Integrity,
        crate::StorageError::Migration => Error::Migration,
        _ => Error::Storage,
    }
}
fn parse_priority(value: &str) -> Result<Priority> {
    match value {
        "low" => Ok(Priority::Low),
        "normal" => Ok(Priority::Normal),
        "high" => Ok(Priority::High),
        "urgent" => Ok(Priority::Urgent),
        _ => Err(Error::Storage),
    }
}
fn priority_name(value: Priority) -> &'static str {
    match value {
        Priority::Low => "low",
        Priority::Normal => "normal",
        Priority::High => "high",
        Priority::Urgent => "urgent",
    }
}
fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value {
        "backlog" => Ok(TaskStatus::Backlog),
        "ready" => Ok(TaskStatus::Ready),
        "active" => Ok(TaskStatus::Active),
        "blocked" => Ok(TaskStatus::Blocked),
        "handover_ready" => Ok(TaskStatus::HandoverReady),
        "done" => Ok(TaskStatus::Done),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(Error::Storage),
    }
}
fn task_status_name(value: TaskStatus) -> &'static str {
    match value {
        TaskStatus::Backlog => "backlog",
        TaskStatus::Ready => "ready",
        TaskStatus::Active => "active",
        TaskStatus::Blocked => "blocked",
        TaskStatus::HandoverReady => "handover_ready",
        TaskStatus::Done => "done",
        TaskStatus::Cancelled => "cancelled",
    }
}
fn parse_instance_status(value: &str) -> Result<InstanceStatus> {
    match value {
        "starting" => Ok(InstanceStatus::Starting),
        "running" => Ok(InstanceStatus::Running),
        "exited" => Ok(InstanceStatus::Exited),
        "failed" => Ok(InstanceStatus::Failed),
        "terminated" => Ok(InstanceStatus::Terminated),
        "lost" => Ok(InstanceStatus::Lost),
        _ => Err(Error::Storage),
    }
}
fn instance_status_name(value: InstanceStatus) -> &'static str {
    match value {
        InstanceStatus::Starting => "starting",
        InstanceStatus::Running => "running",
        InstanceStatus::Exited => "exited",
        InstanceStatus::Failed => "failed",
        InstanceStatus::Terminated => "terminated",
        InstanceStatus::Lost => "lost",
    }
}
fn parse_close_reason(value: &str) -> Result<CloseReason> {
    match value {
        "explicit_release" => Ok(CloseReason::ExplicitRelease),
        "blocking" => Ok(CloseReason::Blocking),
        "handover" => Ok(CloseReason::Handover),
        "completion" => Ok(CloseReason::Completion),
        "cancellation" => Ok(CloseReason::Cancellation),
        "instance_end" => Ok(CloseReason::InstanceEnd),
        _ => Err(Error::Storage),
    }
}
fn close_reason_name(value: CloseReason) -> &'static str {
    match value {
        CloseReason::ExplicitRelease => "explicit_release",
        CloseReason::Blocking => "blocking",
        CloseReason::Handover => "handover",
        CloseReason::Completion => "completion",
        CloseReason::Cancellation => "cancellation",
        CloseReason::InstanceEnd => "instance_end",
    }
}
fn actor_parts(actor: Actor) -> (&'static str, Option<Vec<u8>>) {
    match actor {
        Actor::LocalUser => ("local_user", None),
        Actor::System => ("system", None),
        Actor::Instance(id) => ("instance", Some(id_bytes(id.as_uuid()))),
    }
}
fn parse_actor(kind: &str, id: Option<Vec<u8>>) -> Result<Actor> {
    match (kind, id) {
        ("local_user", None) => Ok(Actor::LocalUser),
        ("system", None) => Ok(Actor::System),
        ("instance", Some(id)) => Ok(Actor::Instance(instance_id(id)?)),
        _ => Err(Error::Storage),
    }
}
fn entity_parts(entity: EntityId) -> (&'static str, Uuid) {
    match entity {
        EntityId::Workspace(id) => ("workspace", id.as_uuid()),
        EntityId::Definition(id) => ("definition", id.as_uuid()),
        EntityId::Task(id) => ("task", id.as_uuid()),
        EntityId::Instance(id) => ("instance", id.as_uuid()),
        EntityId::Claim(id) => ("claim", id.as_uuid()),
        EntityId::Progress(id) => ("progress", id.as_uuid()),
        EntityId::Handover(id) => ("handover", id.as_uuid()),
        EntityId::Worktree(id) => ("worktree", id.as_uuid()),
        EntityId::WorktreeOperation(id) => ("worktree_operation", id.as_uuid()),
        EntityId::ApprovedRoot(id) => ("approved_root", id.as_uuid()),
    }
}
fn event_type_name(value: EventType) -> &'static str {
    match value {
        EventType::WorkspaceCreated => "workspace_created",
        EventType::DefinitionCreated => "definition_created",
        EventType::DefinitionUpdated => "definition_updated",
        EventType::TaskCreated => "task_created",
        EventType::TaskEdited => "task_edited",
        EventType::TaskTransitioned => "task_transitioned",
        EventType::InstanceRegistered => "instance_registered",
        EventType::InstanceObserved => "instance_observed",
        EventType::ClaimOpened => "claim_opened",
        EventType::ClaimClosed => "claim_closed",
        EventType::ProgressAdded => "progress_added",
        EventType::HandoverPrepared => "handover_prepared",
        EventType::WorktreeIntentCreated => "worktree_intent_created",
        EventType::WorktreeIntentChanged => "worktree_intent_changed",
        EventType::WorktreeRegistered => "worktree_registered",
        EventType::TaskWorktreeSelected => "task_worktree_selected",
    }
}
fn parse_event_type(value: &str) -> Result<EventType> {
    match value {
        "workspace_created" => Ok(EventType::WorkspaceCreated),
        "definition_created" => Ok(EventType::DefinitionCreated),
        "definition_updated" => Ok(EventType::DefinitionUpdated),
        "task_created" => Ok(EventType::TaskCreated),
        "task_edited" => Ok(EventType::TaskEdited),
        "task_transitioned" => Ok(EventType::TaskTransitioned),
        "instance_registered" => Ok(EventType::InstanceRegistered),
        "instance_observed" => Ok(EventType::InstanceObserved),
        "claim_opened" => Ok(EventType::ClaimOpened),
        "claim_closed" => Ok(EventType::ClaimClosed),
        "progress_added" => Ok(EventType::ProgressAdded),
        "handover_prepared" => Ok(EventType::HandoverPrepared),
        "worktree_intent_created" => Ok(EventType::WorktreeIntentCreated),
        "worktree_intent_changed" => Ok(EventType::WorktreeIntentChanged),
        "worktree_registered" => Ok(EventType::WorktreeRegistered),
        "task_worktree_selected" => Ok(EventType::TaskWorktreeSelected),
        _ => Err(Error::Version),
    }
}
fn parse_entity(kind: &str, bytes: Vec<u8>) -> Result<EntityId> {
    Ok(match kind {
        "workspace" => EntityId::Workspace(workspace_id(bytes)?),
        "definition" => EntityId::Definition(definition_id(bytes)?),
        "task" => EntityId::Task(task_id(bytes)?),
        "instance" => EntityId::Instance(instance_id(bytes)?),
        "claim" => EntityId::Claim(claim_id(bytes)?),
        "progress" => EntityId::Progress(progress_id(bytes)?),
        "handover" => EntityId::Handover(handover_id(bytes)?),
        "worktree" => EntityId::Worktree(worktree_id(bytes)?),
        "worktree_operation" => EntityId::WorktreeOperation(worktree_operation_id(bytes)?),
        "approved_root" => EntityId::ApprovedRoot(approved_root_id(bytes)?),
        _ => return Err(Error::Version),
    })
}
