use relayterm_application::{
    DurableReadStore, EventNotifier, EventPageRequest, IdGenerator, IdPageRequest, Request, Service,
};
use relayterm_domain::{
    Actor, Error, EventId, EventPayload, ProgressEntryId, TaskContent, TaskId, Timestamp,
    WorkspaceId,
};
use relayterm_persistence_sqlite::{
    Database, DatabaseKind, OpenMode, PoolSettings, SqliteStore, encode_counter,
};
use serde_json::to_vec;
use sqlx::QueryBuilder;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct FixedClock;

impl relayterm_application::Clock for FixedClock {
    fn now(&self) -> relayterm_domain::Result<Timestamp> {
        Timestamp::try_from(1)
    }
}

struct RandomIds;

impl IdGenerator for RandomIds {
    fn next(&self) -> relayterm_domain::Result<EventId> {
        Ok(EventId::from_uuid(Uuid::new_v4()))
    }
}

#[derive(Clone, Copy)]
struct NoopNotifier;

impl EventNotifier for NoopNotifier {
    async fn notify(&self, _: WorkspaceId, _: u64) -> relayterm_domain::Result<()> {
        Ok(())
    }
}

fn fixture_uuid(prefix: u64, ordinal: u64) -> Uuid {
    Uuid::from_u128((u128::from(prefix) << 64) | u128::from(ordinal))
}

#[tokio::test]
async fn large_durable_history_is_read_in_sql_bounded_pages() {
    const TASKS: u64 = 10_000;
    const PROGRESS: u64 = 100_000;
    const PAGE: u16 = 200;

    let temporary = tempfile::tempdir().unwrap();
    let private = temporary.path().join("private");
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    relayterm_platform::create_private_dir(&private).unwrap();
    let database = Database::open(
        &private.join("workspace.sqlite3"),
        DatabaseKind::Workspace,
        OpenMode::ExplicitNew,
        PoolSettings::default(),
    )
    .await
    .unwrap();
    let store = SqliteStore::new(database.pool().clone());
    let workspace_id = WorkspaceId::from_uuid(fixture_uuid(0x1100, 1));
    let service = Service::new(store.clone(), FixedClock, RandomIds, NoopNotifier);
    service
        .create_workspace_reserved(workspace_id, "Large history fixture".into(), project)
        .await
        .unwrap();
    let created = service
        .execute(
            workspace_id,
            Actor::LocalUser,
            Request::CreateTask(TaskContent {
                title: "History anchor".into(),
                description: "Synthetic bounded read fixture".into(),
                priority: relayterm_domain::Priority::Normal,
                scope_paths: Vec::new(),
                acceptance_notes: "Pages remain bounded".into(),
                dependency_ids: Vec::new(),
            }),
        )
        .await
        .unwrap();
    let anchor = match created.committed.events[0].record().payload {
        EventPayload::TaskCreated { id } => id,
        _ => panic!("expected task creation"),
    };
    let baseline = store.consistent_snapshot(workspace_id).await.unwrap();
    let mut next_sequence = baseline.last_sequence + 1;
    let workspace_bytes = workspace_id.as_uuid().as_bytes().to_vec();
    let mut transaction = database.pool().begin().await.unwrap();

    for first in (1..TASKS).step_by(400) {
        let end = (first + 400).min(TASKS);
        let mut tasks = QueryBuilder::new(
            "INSERT INTO tasks(workspace_id,task_id,title,description,priority,status,acceptance_notes,worktree_id,created_seconds,created_nanoseconds,updated_seconds,updated_nanoseconds) ",
        );
        tasks.push_values(first..end, |mut row, ordinal| {
            row.push_bind(workspace_bytes.clone())
                .push_bind(fixture_uuid(0x2200, ordinal).as_bytes().to_vec())
                .push_bind(format!("Scale task {ordinal:05}"))
                .push_bind("Synthetic bounded read fixture")
                .push_bind("normal")
                .push_bind("backlog")
                .push_bind("Pages remain bounded")
                .push_bind(Option::<Vec<u8>>::None)
                .push_bind(1_i64)
                .push_bind(0_i64)
                .push_bind(1_i64)
                .push_bind(0_i64);
        });
        tasks.build().execute(&mut *transaction).await.unwrap();

        let mut events = QueryBuilder::new(
            "INSERT INTO workspace_events(workspace_id,sequence,event_id,event_type,entity_kind,entity_id,timestamp_seconds,timestamp_nanoseconds,actor_kind,actor_instance_id,payload_version,payload_json) ",
        );
        events.push_values(first..end, |mut row, ordinal| {
            let task_id = TaskId::from_uuid(fixture_uuid(0x2200, ordinal));
            let sequence = next_sequence + ordinal - first;
            row.push_bind(workspace_bytes.clone())
                .push_bind(encode_counter(sequence).to_vec())
                .push_bind(fixture_uuid(0x3300, ordinal).as_bytes().to_vec())
                .push_bind("task_created")
                .push_bind("task")
                .push_bind(task_id.as_uuid().as_bytes().to_vec())
                .push_bind(1_i64)
                .push_bind(0_i64)
                .push_bind("local_user")
                .push_bind(Option::<Vec<u8>>::None)
                .push_bind(1_i64)
                .push_bind(to_vec(&EventPayload::TaskCreated { id: task_id }).unwrap());
        });
        events.build().execute(&mut *transaction).await.unwrap();
        next_sequence += end - first;
    }

    for first in (1..=PROGRESS).step_by(500) {
        let end = (first + 500).min(PROGRESS + 1);
        let batch_sequence = next_sequence;
        let mut progress = QueryBuilder::new(
            "INSERT INTO progress_entries(workspace_id,progress_id,task_id,instance_id,summary,verification,created_seconds,created_nanoseconds,creation_event_sequence) ",
        );
        progress.push_values(first..end, |mut row, ordinal| {
            row.push_bind(workspace_bytes.clone())
                .push_bind(fixture_uuid(0x4400, ordinal).as_bytes().to_vec())
                .push_bind(anchor.as_uuid().as_bytes().to_vec())
                .push_bind(Option::<Vec<u8>>::None)
                .push_bind(format!("Scale progress {ordinal:06}"))
                .push_bind("Synthetic scale verification")
                .push_bind(1_i64)
                .push_bind(0_i64)
                .push_bind(encode_counter(batch_sequence + ordinal - first).to_vec());
        });
        progress.build().execute(&mut *transaction).await.unwrap();

        let mut events = QueryBuilder::new(
            "INSERT INTO workspace_events(workspace_id,sequence,event_id,event_type,entity_kind,entity_id,timestamp_seconds,timestamp_nanoseconds,actor_kind,actor_instance_id,payload_version,payload_json) ",
        );
        events.push_values(first..end, |mut row, ordinal| {
            let progress_id = ProgressEntryId::from_uuid(fixture_uuid(0x4400, ordinal));
            let sequence = batch_sequence + ordinal - first;
            row.push_bind(workspace_bytes.clone())
                .push_bind(encode_counter(sequence).to_vec())
                .push_bind(fixture_uuid(0x5500, ordinal).as_bytes().to_vec())
                .push_bind("progress_added")
                .push_bind("progress")
                .push_bind(progress_id.as_uuid().as_bytes().to_vec())
                .push_bind(1_i64)
                .push_bind(0_i64)
                .push_bind("local_user")
                .push_bind(Option::<Vec<u8>>::None)
                .push_bind(1_i64)
                .push_bind(
                    to_vec(&EventPayload::ProgressAdded {
                        id: progress_id,
                        task_id: anchor,
                    })
                    .unwrap(),
                );
        });
        events.build().execute(&mut *transaction).await.unwrap();
        next_sequence += end - first;
    }

    let revision = baseline.snapshot.revision() + TASKS - 1 + PROGRESS;
    sqlx::query("UPDATE workspace_meta SET revision=?,last_event_sequence=? WHERE workspace_id=?")
        .bind(encode_counter(revision).to_vec())
        .bind(encode_counter(next_sequence - 1).to_vec())
        .bind(workspace_bytes)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let task_count: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let progress_count: i64 = sqlx::query_scalar("SELECT count(*) FROM progress_entries")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let event_count: i64 = sqlx::query_scalar("SELECT count(*) FROM workspace_events")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(task_count, TASKS as i64);
    assert_eq!(progress_count, PROGRESS as i64);
    assert_eq!(
        event_count,
        (baseline.last_sequence + TASKS - 1 + PROGRESS) as i64
    );

    let tasks = store
        .task_page(
            workspace_id,
            IdPageRequest::new(None, PAGE, Some(revision)).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tasks.items.len(), PAGE as usize);
    assert!(tasks.has_more);
    let task_cursor = tasks
        .items
        .last()
        .unwrap()
        .record()
        .id
        .as_uuid()
        .into_bytes();
    let task_tail = store
        .task_page(
            workspace_id,
            IdPageRequest::new(Some(task_cursor), PAGE, Some(revision)).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(task_tail.items.len(), PAGE as usize);
    assert!(
        task_tail.items[0].record().id.as_uuid()
            > tasks.items.last().unwrap().record().id.as_uuid()
    );
    let progress = store
        .progress_page(
            workspace_id,
            IdPageRequest::new(None, PAGE, Some(revision)).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(progress.items.len(), PAGE as usize);
    assert!(progress.has_more);
    let progress_cursor = progress
        .items
        .last()
        .unwrap()
        .record()
        .id
        .as_uuid()
        .into_bytes();
    let progress_tail = store
        .progress_page(
            workspace_id,
            IdPageRequest::new(Some(progress_cursor), PAGE, Some(revision)).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(progress_tail.items.len(), PAGE as usize);
    assert!(
        progress_tail.items[0].record().id.as_uuid()
            > progress.items.last().unwrap().record().id.as_uuid()
    );
    let events = store
        .event_page(workspace_id, EventPageRequest::new(0, PAGE).unwrap())
        .await
        .unwrap();
    assert_eq!(events.events.len(), PAGE as usize);
    assert!(events.events.last().unwrap().record().sequence < events.last_sequence);
    let event_tail = store
        .event_page(
            workspace_id,
            EventPageRequest::new(events.events.last().unwrap().record().sequence, PAGE).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(event_tail.events.len(), PAGE as usize);
    assert!(
        event_tail.events[0].record().sequence > events.events.last().unwrap().record().sequence
    );

    let mut samples = Vec::with_capacity(20);
    for _ in 0..20 {
        let started = Instant::now();
        let page = store
            .progress_page(
                workspace_id,
                IdPageRequest::new(None, PAGE, Some(revision)).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(page.items.len(), PAGE as usize);
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let median = samples[9];
    let p95 = samples[18];
    let maximum = samples[19];
    eprintln!(
        "M11 history query tasks={TASKS} progress={PROGRESS} page={PAGE} median_ms={} p95_ms={} max_ms={}",
        median.as_millis(),
        p95.as_millis(),
        maximum.as_millis()
    );
    assert!(p95 <= Duration::from_secs(2));

    let invalid = store
        .progress_page(
            workspace_id,
            IdPageRequest::new(None, PAGE, Some(revision - 1)).unwrap(),
        )
        .await;
    assert_eq!(invalid.err(), Some(Error::Conflict));
}
