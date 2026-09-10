//! Workspace protocol server, runtime composition, recovery, and lifecycle control.

mod diagnostics;
mod runtime;
mod supervisor;
pub use runtime::*;

#[doc(hidden)]
pub use relayterm_application::{Clock as RuntimeClock, LaunchContext as RuntimeLaunchContext};
#[doc(hidden)]
pub use relayterm_domain::{
    AgentDefinitionId as RuntimeAgentDefinitionId, InstanceStatus as RuntimeInstanceStatus,
    Observation as RuntimeObservation, TerminalSize as RuntimeTerminalSize,
};
#[doc(hidden)]
pub use relayterm_platform::SystemClock as RuntimeSystemClock;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use relayterm_application::{
    Clock, DurableReadStore, EventNotifier, EventPageRequest, IdGenerator, MutationScope, Request,
    Service, Store, TaskHistoryItem, TaskHistoryPageRequest,
};
use relayterm_domain as domain;
use relayterm_ipc::{
    Endpoint, IpcError, LocalListener, LocalStream, MAX_CONNECTIONS, PARTIAL_FRAME_TIMEOUT,
    read_frame, write_frame,
};
use relayterm_protocol as wire;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fmt,
    future::Future,
    path::PathBuf,
    pin::Pin,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncWrite,
    sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc, watch},
    task::JoinSet,
};

pub const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(250);
pub const EVENT_QUEUE_ITEMS: usize = 256;
pub const EVENT_QUEUE_BYTES: usize = 1024 * 1024;
pub const TERMINAL_SCROLLBACK_BYTES: usize = relayterm_terminal::DEFAULT_SCROLLBACK_BYTES;
static CONNECTION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

type BackupFuture = Pin<Box<dyn Future<Output = Result<BackupReport, RuntimeError>> + Send>>;
type BackupHandler = Arc<dyn Fn(PathBuf) -> BackupFuture + Send + Sync>;

#[derive(Clone)]
pub struct DaemonControl {
    generation: Arc<str>,
    state: Arc<AtomicU64>,
    shutdown: watch::Sender<bool>,
}

impl DaemonControl {
    pub fn new(generation: String) -> (Self, watch::Receiver<bool>) {
        let (shutdown, receiver) = watch::channel(false);
        (
            Self {
                generation: generation.into(),
                state: Arc::new(AtomicU64::new(0)),
                shutdown,
            },
            receiver,
        )
    }

    pub fn generation(&self) -> &str {
        &self.generation
    }

    fn lifecycle(&self) -> &'static str {
        match self.state.load(Ordering::Acquire) {
            0 => "ready",
            1 => "draining",
            _ => "stopped",
        }
    }

    fn begin_shutdown(&self, generation: &str) -> Result<(), wire::ErrorBody> {
        if generation != self.generation() {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        self.state.store(1, Ordering::Release);
        Ok(())
    }

    pub fn request_shutdown(&self) {
        self.state.store(1, Ordering::Release);
        let _ = self.shutdown.send(true);
    }
}

struct AbortTask(Option<tokio::task::JoinHandle<()>>);
impl AbortTask {
    fn new(task: tokio::task::JoinHandle<()>) -> Self {
        Self(Some(task))
    }
    async fn finish(mut self) {
        if let Some(task) = self.0.take() {
            let _ = task.await;
        }
    }
}
impl Drop for AbortTask {
    fn drop(&mut self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }
}

struct TerminalConnectionGuard {
    supervisor: Option<Arc<supervisor::SessionSupervisor>>,
    connection_id: uuid::Uuid,
}

impl Drop for TerminalConnectionGuard {
    fn drop(&mut self) {
        if let Some(supervisor) = &self.supervisor {
            supervisor.release_connection(self.connection_id);
        }
    }
}

#[derive(Clone, Copy)]
struct Subscription {
    after_sequence: u64,
    id: wire::SubscriptionId,
    request_id: wire::DecimalU64,
    caught_up_watermark: Option<u64>,
}

enum WriterControl {
    Frame(Vec<u8>),
    FrameThenTerminal(Vec<u8>, Vec<u8>),
    FrameAndFlush(Vec<u8>, tokio::sync::oneshot::Sender<()>),
    ResetSubscription(Vec<u8>),
}

struct QueuedEvent {
    bytes: Vec<u8>,
    _permit: OwnedSemaphorePermit,
}

#[derive(Clone, Default)]
pub struct ServerFaults {
    mutation_response: Arc<AtomicBool>,
    any_response: Arc<AtomicBool>,
    pause_mutation_response: Arc<AtomicBool>,
    fail_worktree_after_git: Arc<AtomicBool>,
    pause_worktree_after_git: Arc<AtomicBool>,
    worktree_after_git_paused: Arc<Notify>,
    release_worktree_after_git: Arc<Notify>,
    mutation_paused: Arc<Notify>,
    release_mutation_response: Arc<Notify>,
    event_wakeup_count: Arc<AtomicU64>,
    event_wakeup_observed: Arc<Notify>,
    connection_failure_count: Arc<AtomicU64>,
    connection_failure_observed: Arc<Notify>,
    connection_accept_count: Arc<AtomicU64>,
    connection_accepted: Arc<Notify>,
    completed_connections: Arc<Mutex<BTreeSet<u64>>>,
    connection_completed: Arc<Notify>,
    pause_event_writer: Arc<AtomicBool>,
    event_writer_paused: Arc<Notify>,
    release_event_writer: Arc<Notify>,
    slow_subscriber_count: Arc<AtomicU64>,
    slow_subscriber_observed: Arc<Notify>,
}
impl ServerFaults {
    pub fn drop_next_mutation_response(&self) {
        self.mutation_response.store(true, Ordering::Release)
    }
    pub fn drop_next_response(&self) {
        self.any_response.store(true, Ordering::Release)
    }
    pub fn pause_next_mutation_response(&self) {
        self.pause_mutation_response.store(true, Ordering::Release)
    }
    /// Stops one worktree operation after Git succeeds and before final storage commit.
    /// This deterministic fault is exposed only through an in-process test handle.
    pub fn fail_next_worktree_after_git(&self) {
        self.fail_worktree_after_git.store(true, Ordering::Release)
    }
    /// Pauses one worktree operation after Git succeeds and before final storage commit.
    pub fn pause_next_worktree_after_git(&self) {
        self.pause_worktree_after_git.store(true, Ordering::Release)
    }
    pub async fn wait_until_worktree_after_git_paused(&self) {
        self.worktree_after_git_paused.notified().await;
    }
    pub fn release_worktree_after_git(&self) {
        self.release_worktree_after_git.notify_one();
    }
    pub async fn wait_until_mutation_response_paused(&self) {
        self.mutation_paused.notified().await;
    }
    pub fn release_paused_mutation_response(&self) {
        self.release_mutation_response.notify_one();
    }
    pub fn event_wakeup_count(&self) -> u64 {
        self.event_wakeup_count.load(Ordering::Acquire)
    }
    pub async fn wait_for_event_wakeup_after(&self, previous: u64) {
        while self.event_wakeup_count() <= previous {
            self.event_wakeup_observed.notified().await;
        }
    }
    pub fn connection_failure_count(&self) -> u64 {
        self.connection_failure_count.load(Ordering::Acquire)
    }
    pub async fn wait_for_connection_failure_after(&self, previous: u64) {
        while self.connection_failure_count() <= previous {
            self.connection_failure_observed.notified().await;
        }
    }
    pub fn connection_accept_count(&self) -> u64 {
        self.connection_accept_count.load(Ordering::Acquire)
    }
    pub async fn wait_for_connection_accepted_after(&self, previous: u64) -> u64 {
        while self.connection_accept_count() <= previous {
            self.connection_accepted.notified().await;
        }
        self.connection_accept_count()
    }
    pub async fn wait_for_connection_completed(&self, connection: u64) {
        while !self
            .completed_connections
            .lock()
            .expect("connection completion lock poisoned")
            .contains(&connection)
        {
            self.connection_completed.notified().await;
        }
    }
    pub fn pause_next_event_write(&self) {
        self.pause_event_writer.store(true, Ordering::Release)
    }
    pub async fn wait_until_event_writer_paused(&self) {
        self.event_writer_paused.notified().await;
    }
    pub fn release_paused_event_writer(&self) {
        self.release_event_writer.notify_one();
    }
    pub fn slow_subscriber_count(&self) -> u64 {
        self.slow_subscriber_count.load(Ordering::Acquire)
    }
    pub async fn wait_for_slow_subscriber_after(&self, previous: u64) {
        while self.slow_subscriber_count() <= previous {
            self.slow_subscriber_observed.notified().await;
        }
    }
    async fn pause_if_requested(&self, mutation: bool) {
        if mutation && self.pause_mutation_response.swap(false, Ordering::AcqRel) {
            self.mutation_paused.notify_one();
            self.release_mutation_response.notified().await;
        }
    }
    fn record_event_wakeup(&self) {
        self.event_wakeup_count.fetch_add(1, Ordering::AcqRel);
        self.event_wakeup_observed.notify_waiters();
    }
    fn record_connection_accepted(&self) -> u64 {
        let connection = self.connection_accept_count.fetch_add(1, Ordering::AcqRel) + 1;
        self.connection_accepted.notify_one();
        connection
    }
    fn record_connection_completed(&self, connection: u64, failed: bool) {
        if failed {
            self.connection_failure_count.fetch_add(1, Ordering::AcqRel);
            self.connection_failure_observed.notify_waiters();
        }
        self.completed_connections
            .lock()
            .expect("connection completion lock poisoned")
            .insert(connection);
        self.connection_completed.notify_one();
    }
    async fn pause_event_write_if_requested(&self) {
        if self.pause_event_writer.swap(false, Ordering::AcqRel) {
            self.event_writer_paused.notify_one();
            self.release_event_writer.notified().await;
        }
    }
    fn record_slow_subscriber(&self) {
        self.slow_subscriber_count.fetch_add(1, Ordering::AcqRel);
        self.slow_subscriber_observed.notify_waiters();
    }
    fn take_drop(&self, mutation: bool) -> bool {
        self.any_response.swap(false, Ordering::AcqRel)
            || (mutation && self.mutation_response.swap(false, Ordering::AcqRel))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerError {
    Transport,
    Storage,
    Protocol,
    ResourceLimit,
}
impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The workspace server stopped.")
    }
}
impl std::error::Error for ServerError {}

pub struct WorkspaceServer<S, C, I, N> {
    workspace_id: domain::WorkspaceId,
    wire_workspace_id: wire::WorkspaceId,
    listener: LocalListener,
    service: Arc<Service<S, C, I, N>>,
    reads: S,
    connections: Arc<Semaphore>,
    operations: Arc<Semaphore>,
    faults: ServerFaults,
    event_wakeups: Option<watch::Receiver<u64>>,
    lifecycle: Option<DaemonControl>,
    supervisor: Option<Arc<supervisor::SessionSupervisor>>,
    resolver_admissions: Arc<Semaphore>,
    resolver_jobs: Arc<Semaphore>,
    worktree_parent: Option<PathBuf>,
    git_admissions: Arc<Semaphore>,
    git: relayterm_git::Git,
    backup_admission: Arc<Semaphore>,
    backup: Option<BackupHandler>,
}
impl<S, C, I, N> WorkspaceServer<S, C, I, N>
where
    S: Store + DurableReadStore + Clone + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    I: IdGenerator + Send + Sync + 'static,
    N: EventNotifier + Send + Sync + 'static,
    S::Transaction: 'static,
{
    pub async fn bind(
        workspace_id: domain::WorkspaceId,
        endpoint: &Endpoint,
        service: Arc<Service<S, C, I, N>>,
        reads: S,
    ) -> Result<Self, ServerError> {
        let listener = LocalListener::bind(endpoint)
            .await
            .map_err(|_| ServerError::Transport)?;
        Ok(Self::from_listener(workspace_id, listener, service, reads))
    }

    pub(crate) fn from_listener(
        workspace_id: domain::WorkspaceId,
        listener: LocalListener,
        service: Arc<Service<S, C, I, N>>,
        reads: S,
    ) -> Self {
        Self {
            workspace_id,
            wire_workspace_id: wire::WorkspaceId::from_uuid(workspace_id.as_uuid()),
            listener,
            service,
            reads,
            connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            operations: Arc::new(Semaphore::new(relayterm_ipc::MAX_OUTSTANDING_GLOBAL)),
            faults: ServerFaults::default(),
            event_wakeups: None,
            lifecycle: None,
            supervisor: None,
            resolver_admissions: Arc::new(Semaphore::new(20)),
            resolver_jobs: Arc::new(Semaphore::new(4)),
            worktree_parent: None,
            git_admissions: Arc::new(Semaphore::new(2)),
            git: relayterm_git::Git::default(),
            backup_admission: Arc::new(Semaphore::new(1)),
            backup: None,
        }
    }
    pub fn with_event_wakeups(mut self, receiver: watch::Receiver<u64>) -> Self {
        self.event_wakeups = Some(receiver);
        self
    }
    pub fn with_lifecycle(mut self, control: DaemonControl) -> Self {
        self.lifecycle = Some(control);
        self
    }
    pub(crate) fn with_supervisor(
        mut self,
        supervisor: Arc<supervisor::SessionSupervisor>,
    ) -> Self {
        self.supervisor = Some(supervisor);
        self
    }
    pub fn with_worktrees(mut self, parent: PathBuf) -> Self {
        self.worktree_parent = Some(parent);
        self
    }
    pub(crate) fn with_backup(mut self, backup: BackupHandler) -> Self {
        self.backup = Some(backup);
        self
    }
    /// Replaces the Git adapter for deterministic native integration tests.
    #[cfg(feature = "test-hooks")]
    pub fn with_git(mut self, git: relayterm_git::Git) -> Self {
        self.git = git;
        self
    }
    pub fn fault_injector(&self) -> ServerFaults {
        self.faults.clone()
    }
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) -> Result<(), ServerError> {
        self.run_retaining_listener(&mut shutdown).await.map(drop)
    }
    pub(crate) async fn run_retaining_listener(
        self,
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<Self, ServerError> {
        let shared = Arc::new(self);
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {biased;
                changed=shutdown.changed()=>{if changed.is_err()||*shutdown.borrow(){break}}
                joined=connections.join_next(),if !connections.is_empty()=>{let _=joined;}
                accepted=shared.listener.accept()=>{
                    let Ok(stream)=accepted else{continue};
                    let connection=shared.faults.record_connection_accepted();
                    let Ok(permit)=shared.connections.clone().try_acquire_owned()else{
                        drop(stream);
                        shared.faults.record_connection_completed(connection,false);
                        continue
                    };
                    let server=shared.clone();
                    let child_shutdown=shutdown.clone();
                    connections.spawn(async move{
                        let failed=server.clone().connection(stream,child_shutdown).await.is_err();
                        drop(permit);
                        server.faults.record_connection_completed(connection,failed);
                    });
                }
            }
        }
        while connections.join_next().await.is_some() {}
        if let Some(control) = &shared.lifecycle {
            control.state.store(2, Ordering::Release);
        }
        Arc::try_unwrap(shared).map_err(|_| ServerError::ResourceLimit)
    }
    async fn connection(
        self: Arc<Self>,
        mut stream: LocalStream,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), ServerError> {
        let frame = read_frame(&mut stream, relayterm_ipc::HANDSHAKE_TIMEOUT)
            .await
            .map_err(|_| ServerError::Protocol)?;
        if frame.kind != wire::FrameKind::Json {
            return Err(ServerError::Protocol);
        }
        let request: wire::RequestEnvelope =
            wire::decode_json(&frame.payload, true).map_err(|_| ServerError::Protocol)?;
        if request.protocol_version != wire::PROTOCOL_VERSION {
            send_response(
                &mut stream,
                wire::ResponseEnvelope::failure(
                    request.request_id,
                    self.wire_workspace_id,
                    wire::ErrorBody::not_applied(
                        wire::ErrorCode::UnsupportedVersion,
                        wire::Recovery::UseMatchingVersion,
                    ),
                ),
            )
            .await?;
            return Err(ServerError::Protocol);
        }
        if request.workspace_id != self.wire_workspace_id {
            send_response(
                &mut stream,
                wire::ResponseEnvelope::failure(
                    request.request_id,
                    self.wire_workspace_id,
                    wire::ErrorBody::not_applied(
                        wire::ErrorCode::WorkspaceMismatch,
                        wire::Recovery::None,
                    ),
                ),
            )
            .await?;
            return Err(ServerError::Protocol);
        }
        if request.operation != wire::Operation::ProtocolHello || empty(&request.params).is_err() {
            return Err(ServerError::Protocol);
        }
        let connection_id = connection_id();
        let _terminal_connection = TerminalConnectionGuard {
            supervisor: self.supervisor.clone(),
            connection_id: connection_id.as_uuid(),
        };
        let operations = wire::Operation::ALL
            .into_iter()
            .filter(|operation| {
                self.lifecycle.is_some()
                    || !matches!(
                        operation,
                        wire::Operation::DaemonStatus | wire::Operation::DaemonShutdown
                    )
            })
            .collect();
        let hello = wire::HelloResult::new(connection_id, self.wire_workspace_id, operations)
            .with_terminal(self.supervisor.is_some())
            .with_worktrees(self.worktree_parent.is_some());
        send_response(
            &mut stream,
            wire::ResponseEnvelope::success(
                request.request_id,
                self.wire_workspace_id,
                serde_json::to_value(hello).map_err(|_| ServerError::Protocol)?,
            ),
        )
        .await?;
        let mut last_request = request.request_id.get();
        let mut subscription: Option<Subscription> = None;
        let mut interval = tokio::time::interval(EVENT_POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut wakeups = self.event_wakeups.clone();
        let (mut reader, writer) = stream.split();
        let (incoming_tx, mut incoming_rx) = mpsc::channel(2);
        let reader_task = tokio::spawn(async move {
            loop {
                let frame: Result<wire::Frame, IpcError> =
                    read_frame(&mut reader, PARTIAL_FRAME_TIMEOUT).await;
                let terminal = frame.is_err();
                if incoming_tx.send(frame).await.is_err() || terminal {
                    return;
                }
            }
        });
        let _reader_guard = AbortTask::new(reader_task);
        let (control_tx, control_rx) = mpsc::channel(2);
        let (event_tx, event_rx) = mpsc::channel(EVENT_QUEUE_ITEMS);
        let event_budget = Arc::new(Semaphore::new(EVENT_QUEUE_BYTES));
        let (writer_failed_tx, mut writer_failed_rx) = mpsc::channel(1);
        let writer_faults = self.faults.clone();
        let writer_task = tokio::spawn(async move {
            if writer_loop(writer, control_rx, event_rx, writer_faults)
                .await
                .is_err()
            {
                let _ = writer_failed_tx.send(()).await;
            }
        });
        let writer_guard = AbortTask::new(writer_task);
        loop {
            tokio::select! {biased;
                changed=shutdown.changed()=>{if changed.is_err()||*shutdown.borrow(){break}}
                failed=writer_failed_rx.recv()=>{let _=failed;return Err(ServerError::Transport)}
                incoming=incoming_rx.recv()=>{
                    let frame=incoming.ok_or(ServerError::Transport)?.map_err(|_|ServerError::Transport)?;
                    if frame.kind!=wire::FrameKind::Json{return Err(ServerError::Protocol)}
                    let request:wire::RequestEnvelope=wire::decode_json(&frame.payload,false).map_err(|_|ServerError::Protocol)?;
                    if request.request_id.get()<=last_request{return Err(ServerError::Protocol)}
                    last_request=request.request_id.get();
                    if request.protocol_version!=wire::PROTOCOL_VERSION{
                        queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,wire::ErrorBody::not_applied(wire::ErrorCode::UnsupportedVersion,wire::Recovery::UseMatchingVersion)),false).await?;
                        return Err(ServerError::Protocol)
                    }else if request.workspace_id!=self.wire_workspace_id{
                        queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,wire::ErrorBody::not_applied(wire::ErrorCode::WorkspaceMismatch,wire::Recovery::None)),false).await?;
                        return Err(ServerError::Protocol)
                    }
                    if request.operation==wire::Operation::ProtocolHello{return Err(ServerError::Protocol)}
                    if request.operation==wire::Operation::EventSubscribe{
                        if subscription.is_some(){queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,map_domain_error(domain::Error::State)),false).await?;continue}
                        let Ok(after)=parse_decimal_field(&request.params,"after_sequence",true) else {queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,invalid()),false).await?;continue};
                        let Ok(page_request)=EventPageRequest::new(after,1) else {queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,invalid()),false).await?;continue};
                        match self.reads.event_page(self.workspace_id,page_request).await{
                            Ok(page)=>{
                                let id=subscription_id();
                                queue_response(&control_tx,wire::ResponseEnvelope::success(request.request_id,self.wire_workspace_id,json!({"subscription_id":id.to_string(),"request_id":request.request_id.get().to_string(),"accepted_after_sequence":after.to_string(),"last_sequence":page.last_sequence.to_string(),"retained_from_sequence":page.retained_from_sequence.to_string()})),false).await?;
                                subscription=Some(Subscription{after_sequence:after,id,request_id:request.request_id,caught_up_watermark:None});
                            },
                            Err(error)=>queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,map_domain_error(error)),false).await?,
                        }
                        continue
                    }
                    if request.operation==wire::Operation::EventUnsubscribe{
                        let Ok(supplied)=parse_id::<wire::SubscriptionId>(&request.params,"subscription_id") else {queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,invalid()),false).await?;continue};
                        if subscription.is_none_or(|active|active.id!=supplied){queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,map_domain_error(domain::Error::Reference)),false).await?;continue}
                        subscription=None;
                        queue_response(&control_tx,wire::ResponseEnvelope::success(request.request_id,self.wire_workspace_id,json!({"unsubscribed":true})),true).await?;
                        continue
                    }
                    if request.operation.is_mutation()
                        && request.operation != wire::Operation::DaemonShutdown
                        && self.lifecycle.as_ref().is_some_and(|value| value.lifecycle() != "ready") {
                        queue_response(&control_tx,wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,unavailable()),false).await?;
                        continue
                    }
                    if request.operation == wire::Operation::SessionReadOutput {
                        let (response, terminal) = match self.session_read_output(&request.params, connection_id) {
                            Ok((value, terminal)) => (wire::ResponseEnvelope::success(request.request_id, self.wire_workspace_id, value), terminal),
                            Err(error) => (wire::ResponseEnvelope::failure(request.request_id, self.wire_workspace_id, error), None),
                        };
                        queue_terminal_response(&control_tx, response, terminal).await?;
                        continue
                    }
                    let _operation=self.operations.clone().acquire_owned().await.map_err(|_|ServerError::ResourceLimit)?;
                    let response=match self.dispatch(&request,connection_id).await{Ok(value)=>wire::ResponseEnvelope::success(request.request_id,self.wire_workspace_id,value),Err(error)=>wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,error)};
                    let shutdown_accepted = request.operation == wire::Operation::DaemonShutdown && response.error.is_none();
                    self.faults.pause_if_requested(request.operation.is_mutation()).await;
                    if self.faults.take_drop(request.operation.is_mutation()){return Err(ServerError::Transport)}
                    if shutdown_accepted {
                        queue_response_confirmed(&control_tx, response).await?;
                        if let Some(control) = &self.lifecycle { control.request_shutdown(); }
                    } else {
                        queue_response(&control_tx,response,false).await?;
                    }
                }
                _=wait_for_wakeup(&mut wakeups),if subscription.is_some()=>{
                    self.faults.record_event_wakeup();
                    self.queue_events(&mut subscription,&event_tx,&control_tx,&event_budget).await?;
                }
                _=interval.tick(),if subscription.is_some()=>{
                    self.queue_events(&mut subscription,&event_tx,&control_tx,&event_budget).await?;
                }
            }
        }
        drop(control_tx);
        drop(event_tx);
        writer_guard.finish().await;
        Ok(())
    }

    async fn queue_events(
        &self,
        subscription: &mut Option<Subscription>,
        event_tx: &mpsc::Sender<QueuedEvent>,
        control_tx: &mpsc::Sender<WriterControl>,
        budget: &Arc<Semaphore>,
    ) -> Result<(), ServerError> {
        let active = subscription.ok_or(ServerError::Protocol)?;
        let page = match self
            .reads
            .event_page(
                self.workspace_id,
                EventPageRequest::new(active.after_sequence, 200)
                    .map_err(|_| ServerError::Protocol)?,
            )
            .await
        {
            Ok(page) => page,
            Err(domain::Error::ResnapshotRequired) => {
                queue_synchronization(
                    control_tx,
                    self.wire_workspace_id,
                    active,
                    wire::SynchronizationReason::CursorExpired,
                )
                .await?;
                *subscription = None;
                return Ok(());
            }
            Err(_) => return Err(ServerError::Storage),
        };
        let mut after_sequence = active.after_sequence;
        for event in page.events {
            let sequence = event.record().sequence;
            if sequence != after_sequence.saturating_add(1) {
                return Err(ServerError::Protocol);
            }
            let envelope = wire::EventEnvelope {
                message_type: wire::EventMessageType::Event,
                protocol_version: wire::PROTOCOL_VERSION,
                workspace_id: self.wire_workspace_id,
                subscription_id: active.id,
                request_id: active.request_id,
                sequence: wire::DecimalU64::new(sequence).map_err(|_| ServerError::Protocol)?,
                event: event_dto(&event),
            };
            let bytes = wire::encode_json(&envelope).map_err(|_| ServerError::Protocol)?;
            let permit_count =
                u32::try_from(bytes.len()).map_err(|_| ServerError::ResourceLimit)?;
            if !try_queue_event(event_tx, budget, bytes, permit_count) {
                self.faults.record_slow_subscriber();
                queue_synchronization(
                    control_tx,
                    self.wire_workspace_id,
                    active,
                    wire::SynchronizationReason::SlowSubscriber,
                )
                .await?;
                *subscription = None;
                return Ok(());
            }
            subscription
                .as_mut()
                .ok_or(ServerError::Protocol)?
                .after_sequence = sequence;
            after_sequence = sequence;
        }
        if page.last_sequence != 0
            && after_sequence == page.last_sequence
            && active.caught_up_watermark != Some(page.last_sequence)
        {
            let envelope = wire::SynchronizationEnvelope {
                message_type: wire::SynchronizationMessageType::Event,
                protocol_version: wire::PROTOCOL_VERSION,
                workspace_id: self.wire_workspace_id,
                subscription_id: active.id,
                request_id: active.request_id,
                control: wire::SynchronizationControl::CaughtUp,
                reason: None,
                watermark: Some(
                    wire::DecimalU64::new(page.last_sequence).map_err(|_| ServerError::Protocol)?,
                ),
            };
            let bytes = wire::encode_json(&envelope).map_err(|_| ServerError::Protocol)?;
            let permit_count =
                u32::try_from(bytes.len()).map_err(|_| ServerError::ResourceLimit)?;
            if !try_queue_event(event_tx, budget, bytes, permit_count) {
                queue_synchronization(
                    control_tx,
                    self.wire_workspace_id,
                    active,
                    wire::SynchronizationReason::SlowSubscriber,
                )
                .await?;
                *subscription = None;
                return Ok(());
            }
            subscription
                .as_mut()
                .ok_or(ServerError::Protocol)?
                .caught_up_watermark = Some(page.last_sequence);
        }
        Ok(())
    }

    async fn dispatch(
        &self,
        request: &wire::RequestEnvelope,
        connection_id: wire::ConnectionId,
    ) -> Result<Value, wire::ErrorBody> {
        use wire::Operation as O;
        self.observe_terminal_exits().await;
        if request.operation == O::Unknown {
            return Err(wire::ErrorBody::not_applied(
                wire::ErrorCode::UnknownOperation,
                wire::Recovery::None,
            ));
        }
        if request.operation.is_reserved()
            && !matches!(
                request.operation,
                O::SessionCreate
                    | O::SessionAttach
                    | O::SessionReadDisplay
                    | O::SessionInput
                    | O::SessionResize
                    | O::SessionTerminate
                    | O::SessionDetach
                    | O::SessionAcquireInput
                    | O::SessionReleaseInput
                    | O::SessionReadOutput
                    | O::WorktreeCreate
                    | O::WorktreeList
                    | O::WorktreeInspectRepository
                    | O::WorktreeGetOperation
                    | O::WorktreeSelect
                    | O::WorktreeReconcile
                    | O::BackupCreate
            )
        {
            validate_reserved(request.operation, &request.params)?;
            return Err(unavailable());
        }
        match request.operation {
            O::ProtocolPing => {
                empty(&request.params)?;
                Ok(json!({"ok":true}))
            }
            O::DaemonStatus => {
                let _: wire::DaemonStatusParams = parameters(&request.params)?;
                let control = self.lifecycle.as_ref().ok_or_else(unavailable)?;
                let overview = self
                    .reads
                    .workspace_overview(self.workspace_id)
                    .await
                    .map_err(map_domain_error)?;
                serde_json::to_value(wire::DaemonStatusResult {
                    workspace_id: wire::WorkspaceId::from_uuid(self.workspace_id.as_uuid()),
                    generation: control.generation().to_owned(),
                    lifecycle: control.lifecycle().to_owned(),
                    protocol_version: wire::PROTOCOL_VERSION,
                    schema_version: overview.schema_version,
                    revision: wire::DecimalU64::new(overview.revision).map_err(|_| invalid())?,
                    definitions: overview.definitions,
                    tasks: overview.tasks,
                    instances: overview.instances,
                })
                .map_err(|_| invalid())
            }
            O::DaemonShutdown => {
                let params: wire::DaemonShutdownParams = parameters(&request.params)?;
                let control = self.lifecycle.as_ref().ok_or_else(unavailable)?;
                if self
                    .supervisor
                    .as_ref()
                    .is_some_and(|supervisor| supervisor.live_count() != 0)
                {
                    if !params.terminate_sessions {
                        return Err(map_domain_error(domain::Error::Conflict));
                    }
                    if let Some(supervisor) = &self.supervisor {
                        supervisor.terminate_all();
                    }
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    while self
                        .supervisor
                        .as_ref()
                        .is_some_and(|supervisor| supervisor.live_count() != 0)
                    {
                        supervisor::wait_for_output().await;
                        self.observe_terminal_exits().await;
                        if let Some(supervisor) = &self.supervisor {
                            supervisor.terminate_all();
                        }
                        if tokio::time::Instant::now() >= deadline {
                            return Err(unavailable());
                        }
                    }
                }
                control.begin_shutdown(&params.generation)?;
                serde_json::to_value(wire::DaemonShutdownResult {
                    generation: control.generation().to_owned(),
                    lifecycle: "draining".to_owned(),
                })
                .map_err(|_| invalid())
            }
            O::SessionCreate => self.session_create(&request.params).await,
            O::BackupCreate => self.backup_create(&request.params).await,
            O::SessionAttach => self.session_attach(&request.params, connection_id).await,
            O::SessionReadDisplay => {
                self.session_read_display(&request.params, connection_id)
                    .await
            }
            O::SessionDetach => {
                let params: wire::SessionDetachParams = parameters(&request.params)?;
                self.supervisor
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .detach(params.session_id.as_uuid(), connection_id.as_uuid())
                    .map_err(map_supervisor)?;
                Ok(json!({"detached":true}))
            }
            O::SessionAcquireInput => {
                let params: wire::SessionAcquireInputParams = parameters(&request.params)?;
                let lease = self
                    .supervisor
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .acquire_input(params.session_id.as_uuid(), connection_id.as_uuid())
                    .map_err(map_supervisor)?;
                Ok(json!({"lease_id":lease.to_string(),"next_input_sequence":"1"}))
            }
            O::SessionReleaseInput => {
                let params: wire::SessionReleaseInputParams = parameters(&request.params)?;
                self.supervisor
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .release_input(
                        params.session_id.as_uuid(),
                        connection_id.as_uuid(),
                        params.lease_id.get(),
                    )
                    .map_err(map_supervisor)?;
                Ok(json!({"released":true}))
            }
            O::SessionInput => {
                let params: wire::SessionInputParams = parameters(&request.params)?;
                let data = STANDARD
                    .decode(params.data)
                    .map_err(|_| invalid_field(wire::ErrorField::TerminalData))?;
                self.supervisor
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .input(
                        params.session_id.as_uuid(),
                        connection_id.as_uuid(),
                        params.lease_id.get(),
                        params.sequence.get(),
                        data,
                    )
                    .map_err(map_supervisor)?;
                Ok(json!({"accepted":true}))
            }
            O::SessionResize => {
                let params: wire::SessionResizeParams = parameters(&request.params)?;
                let revision = self
                    .supervisor
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .resize(
                        params.session_id.as_uuid(),
                        connection_id.as_uuid(),
                        params.lease_id.get(),
                        params.rows,
                        params.columns,
                    )
                    .map_err(map_supervisor)?;
                Ok(json!({"revision":revision.to_string()}))
            }
            O::SessionTerminate => {
                let params: wire::SessionTerminateParams = parameters(&request.params)?;
                self.supervisor
                    .as_ref()
                    .ok_or_else(unavailable)?
                    .terminate(params.session_id.as_uuid())
                    .map_err(map_supervisor)?;
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while self
                    .supervisor
                    .as_ref()
                    .is_some_and(|supervisor| supervisor.is_live(params.session_id.as_uuid()))
                {
                    supervisor::wait_for_output().await;
                    self.observe_terminal_exits().await;
                    if tokio::time::Instant::now() >= deadline {
                        return Err(unavailable());
                    }
                }
                Ok(json!({"terminated":true}))
            }
            O::WorkspaceGetSnapshot => self.snapshot_page(&request.params, None).await,
            O::AgentListDefinitions => {
                self.snapshot_page(&request.params, Some(Collection::Definitions))
                    .await
            }
            O::AgentListTemplates => self.agent_templates(&request.params),
            O::AgentCheckDefinition => self.check_definition(&request.params).await,
            O::TaskList => {
                self.snapshot_page(&request.params, Some(Collection::Tasks))
                    .await
            }
            O::SessionList => {
                self.snapshot_page(&request.params, Some(Collection::Instances))
                    .await
            }
            O::TaskGet => {
                let id = parse_id::<domain::TaskId>(&request.params, "task_id")?;
                let snapshot = self
                    .reads
                    .consistent_projection(
                        self.workspace_id,
                        MutationScope {
                            task_ids: vec![id],
                            ..MutationScope::default()
                        },
                    )
                    .await
                    .map_err(map_domain_error)?;
                let task = snapshot
                    .snapshot
                    .state()
                    .map_err(map_domain_error)?
                    .task(id)
                    .map_err(map_domain_error)?;
                Ok(task_dto(task))
            }
            O::HandoverGet => {
                let id = parse_id::<domain::HandoverId>(&request.params, "handover_id")?;
                let item = self
                    .reads
                    .handover_by_id(self.workspace_id, id)
                    .await
                    .map_err(map_domain_error)?;
                Ok(handover_dto(&item))
            }
            O::TaskGetHistory | O::TaskGetClaimHistory => self.history(request).await,
            O::EventList => self.event_list(&request.params).await,
            O::AgentRegisterDefinition => self.register_definition(&request.params).await,
            O::AgentUpdateDefinition => self.update_definition(&request.params).await,
            O::AgentImportDefinitions => self.import_definitions(&request.params).await,
            O::TaskCreate => self.mutate_task_create(&request.params).await,
            O::TaskUpdate => self.mutate_task_update(&request.params).await,
            O::TaskTransition => self.mutate_transition(&request.params).await,
            O::TaskClaim => self.mutate_claim(&request.params).await,
            O::TaskRelease => self.mutate_release(&request.params).await,
            O::ProgressAppend => self.mutate_progress(&request.params).await,
            O::HandoverCreate => self.mutate_handover(&request.params).await,
            O::WorktreeInspectRepository => self.worktree_inspect(&request.params).await,
            O::WorktreeCreate => self.worktree_create(&request.params).await,
            O::WorktreeGetOperation => self.worktree_operation(&request.params).await,
            O::WorktreeList => self.worktree_list(&request.params).await,
            O::WorktreeSelect => self.worktree_select(&request.params).await,
            O::WorktreeReconcile => self.worktree_reconcile(&request.params).await,
            O::ProtocolHello | O::EventSubscribe | O::EventUnsubscribe => Err(invalid()),
            _ => Err(unavailable()),
        }
    }

    async fn worktree_inspect(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::WorktreeInspectParams = parameters(value)?;
        let _permit = self
            .git_admissions
            .clone()
            .try_acquire_owned()
            .map_err(|_| unavailable())?;
        let snapshot = self
            .reads
            .consistent_projection(self.workspace_id, MutationScope::default())
            .await
            .map_err(map_domain_error)?;
        if params
            .expected_revision
            .is_some_and(|expected| expected.get() != snapshot.snapshot.revision())
        {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        let root = snapshot
            .snapshot
            .state()
            .map_err(map_domain_error)?
            .workspace()
            .record()
            .project_root
            .clone();
        let git = self.git.clone();
        let result = tokio::task::spawn_blocking(move || git.inspect(&root))
            .await
            .map_err(|_| unavailable())?;
        match result {
            Ok(repository) => Ok(
                json!({"status":"ready","head_commit":repository.head_commit,"default_parent_display":self.worktree_parent.as_ref().map(|path|path.to_string_lossy().into_owned())}),
            ),
            Err(error) => Ok(json!({"status":git_status(error.kind())})),
        }
    }

    async fn worktree_create(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::WorktreeCreateParams = parameters(value)?;
        validate_reserved(wire::Operation::WorktreeCreate, value)?;
        if params.payload_version != 1 {
            return Err(invalid());
        }
        relayterm_git::validate_branch(&params.branch_name).map_err(|_| invalid())?;
        relayterm_git::validate_base(&params.base_ref).map_err(|_| invalid())?;
        relayterm_git::validate_destination_leaf(&params.destination_leaf)
            .map_err(|_| invalid())?;
        let operation_id = domain::WorktreeOperationId::from_uuid(params.operation_id.as_uuid());
        let task_id = domain::TaskId::from_uuid(params.task_id.as_uuid());
        let expected_revision = params.expected_revision.get();
        let snapshot = self
            .reads
            .consistent_projection(
                self.workspace_id,
                MutationScope {
                    task_ids: vec![task_id],
                    ..MutationScope::default()
                },
            )
            .await
            .map_err(map_domain_error)?;
        let state = snapshot.snapshot.state().map_err(map_domain_error)?;
        let project_root = state.workspace().record().project_root.clone();
        let custom_parent = params.parent.is_some();
        let parent = match params.parent {
            Some(path) => {
                let bytes = path.decode().map_err(|_| invalid())?;
                relayterm_platform::decode_native_path(&path.encoding, &bytes)
                    .map_err(|_| invalid())?
            }
            None => self.worktree_parent.clone().ok_or_else(unavailable)?,
        };
        let parent = parent.canonicalize().map_err(|_| invalid())?;
        let destination = parent.join(&params.destination_leaf);
        let mut fingerprint = blake3::Hasher::new();
        for bytes in [
            task_id.to_string().as_bytes(),
            params.base_ref.as_bytes(),
            params.branch_name.as_bytes(),
            params.destination_leaf.as_bytes(),
        ] {
            fingerprint.update(bytes);
            fingerprint.update(&[0]);
        }
        let encoded_parent =
            relayterm_platform::encode_native_path(&parent).map_err(|_| invalid())?;
        fingerprint.update(encoded_parent.tag.as_bytes());
        fingerprint.update(&[0]);
        fingerprint.update(&encoded_parent.bytes);
        fingerprint.update(&[0]);
        let fingerprint = *fingerprint.finalize().as_bytes();
        if let Some(existing) = state
            .worktree_intents()
            .iter()
            .find(|intent| intent.record().id == operation_id)
        {
            let record = existing.record();
            if record.task_id != task_id
                || record.base_expression != params.base_ref
                || record.branch != params.branch_name
                || record.destination != destination
            {
                return Err(map_domain_error(domain::Error::Conflict));
            }
            existing
                .matches_fingerprint(fingerprint)
                .map_err(map_domain_error)?;
            return Ok(worktree_operation_dto(existing, state));
        }
        if snapshot.snapshot.revision() != expected_revision {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        let _permit = self
            .git_admissions
            .clone()
            .try_acquire_owned()
            .map_err(|_| unavailable())?;
        let discovery_base = params.base_ref.clone();
        let discovery_branch = params.branch_name.clone();
        let repository = tokio::task::spawn_blocking({
            let root = project_root.clone();
            let git = self.git.clone();
            move || {
                let repository = git.inspect(&root)?;
                let commit = git.resolve_commit(&root, &discovery_base)?;
                git.branch_available(&root, &discovery_branch)?;
                Ok::<_, relayterm_git::Error>((repository, commit))
            }
        })
        .await
        .map_err(|_| unavailable())?
        .map_err(map_git_error)?;
        let now = relayterm_platform::SystemClock
            .now()
            .map_err(map_domain_error)?;
        let root_id = state
            .approved_roots()
            .iter()
            .find(|root| root.record().canonical_parent == parent)
            .map(|root| root.record().id)
            .unwrap_or_else(|| domain::ApprovedRootId::from_uuid(uuid::Uuid::new_v4()));
        let worktree_id = domain::WorktreeId::from_uuid(uuid::Uuid::new_v4());
        let root = domain::ApprovedRoot::restore(domain::ApprovedRootRecord {
            id: root_id,
            workspace_id: self.workspace_id,
            canonical_parent: parent,
            filesystem_identity: None,
            private_default: !custom_parent,
            created_at: now,
        })
        .map_err(map_domain_error)?;
        let intent = domain::WorktreeIntent::restore(domain::WorktreeIntentRecord {
            id: operation_id,
            workspace_id: self.workspace_id,
            task_id,
            worktree_id,
            root_id,
            actor: domain::Actor::LocalUser,
            schema_version: 1,
            expected_revision,
            repository_identity: repository.0.checkout_top.clone(),
            common_directory_identity: repository.0.common_directory.clone(),
            destination: destination.clone(),
            branch: params.branch_name.clone(),
            base_expression: params.base_ref.clone(),
            resolved_commit: repository.1.clone(),
            request_fingerprint: fingerprint,
            phase: domain::WorktreePhase::Prepared,
            reason: None,
            created_at: now,
            updated_at: now,
        })
        .map_err(map_domain_error)?;
        let accepted = match self
            .service
            .execute_domain_command_at_revision(
                self.workspace_id,
                domain::Command::AddWorktreeIntent {
                    root,
                    intent: Box::new(intent),
                },
                expected_revision,
            )
            .await
        {
            Ok(outcome) => outcome,
            Err(domain::Error::Conflict) => {
                let current = self
                    .reads
                    .consistent_projection(
                        self.workspace_id,
                        MutationScope {
                            task_ids: vec![task_id],
                            ..MutationScope::default()
                        },
                    )
                    .await
                    .map_err(map_domain_error)?;
                let state = current.snapshot.state().map_err(map_domain_error)?;
                let existing = state
                    .worktree_intents()
                    .iter()
                    .find(|candidate| candidate.record().id == operation_id)
                    .ok_or_else(|| map_domain_error(domain::Error::Conflict))?;
                let record = existing.record();
                if record.task_id != task_id
                    || record.base_expression != params.base_ref
                    || record.branch != params.branch_name
                    || record.destination != destination
                {
                    return Err(map_domain_error(domain::Error::Conflict));
                }
                existing
                    .matches_fingerprint(fingerprint)
                    .map_err(map_domain_error)?;
                return Ok(worktree_operation_dto(existing, state));
            }
            Err(error) => return Err(map_domain_error(error)),
        };
        let revision = accepted.committed.snapshot.revision();
        let applying = self
            .service
            .execute_domain_command_at_revision(
                self.workspace_id,
                domain::Command::MarkWorktreeApplying { operation_id },
                revision,
            )
            .await
            .map_err(map_domain_error)?;
        let add_branch = params.branch_name.clone();
        let add_commit = repository.1.clone();
        let common_for_add = repository.0.common_directory.clone();
        let add_result = tokio::task::spawn_blocking({
            let root = project_root;
            let destination = destination.clone();
            let git = self.git.clone();
            move || {
                git.add(&root, &destination, &add_branch, &add_commit)?;
                git.verify_created_worktree(
                    &root,
                    &destination,
                    &add_branch,
                    &common_for_add,
                    &add_commit,
                )
            }
        })
        .await
        .map_err(|_| unavailable())?;
        if let Err(error) = add_result {
            let current = applying.committed.snapshot.revision();
            // Once dispatch reached Git, even an I/O or malformed-output error
            // cannot prove that no external worktree effect occurred.
            let needs_attention = true;
            let _ = self
                .service
                .execute_domain_command_at_revision(
                    self.workspace_id,
                    domain::Command::FailWorktree {
                        operation_id,
                        reason: git_reason(error.kind()),
                        needs_attention,
                    },
                    current,
                )
                .await;
            return Err(map_git_error(error));
        }
        if self
            .faults
            .pause_worktree_after_git
            .swap(false, Ordering::AcqRel)
        {
            self.faults.worktree_after_git_paused.notify_one();
            self.faults.release_worktree_after_git.notified().await;
        }
        if self
            .faults
            .fail_worktree_after_git
            .swap(false, Ordering::AcqRel)
        {
            return Err(map_domain_error(domain::Error::Uncertain));
        }
        let worktree = domain::Worktree::restore(domain::WorktreeRecord {
            id: worktree_id,
            workspace_id: self.workspace_id,
            task_id,
            operation_id,
            root_id,
            checkout_path: destination,
            common_directory_identity: repository.0.common_directory,
            branch_ref: format!("refs/heads/{}", params.branch_name),
            initial_base_commit: repository.1,
            health: domain::WorktreeHealth::Ready,
            created_at: now,
            updated_at: relayterm_platform::SystemClock
                .now()
                .map_err(map_domain_error)?,
        })
        .map_err(map_domain_error)?;
        self.finalize_verified_worktree(operation_id, worktree)
            .await?;
        self.worktree_operation(
            &serde_json::to_value(wire::WorktreeOperationParams {
                operation_id: params.operation_id,
            })
            .map_err(|_| invalid())?,
        )
        .await
    }

    async fn worktree_operation(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::WorktreeOperationParams = parameters(value)?;
        let id = domain::WorktreeOperationId::from_uuid(params.operation_id.as_uuid());
        let snapshot = self
            .reads
            .consistent_projection(
                self.workspace_id,
                MutationScope {
                    ..MutationScope::default()
                },
            )
            .await
            .map_err(map_domain_error)?;
        let state = snapshot.snapshot.state().map_err(map_domain_error)?;
        let intent = state
            .worktree_intents()
            .iter()
            .find(|intent| intent.record().id == id)
            .ok_or_else(|| map_domain_error(domain::Error::Reference))?;
        Ok(worktree_operation_dto(intent, state))
    }

    async fn worktree_list(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::WorktreeListParams = parameters(value)?;
        if params.limit == 0 || params.limit > wire::MAX_PAGE_SIZE {
            return Err(invalid_field(wire::ErrorField::PageLimit));
        }
        let task = params
            .task_id
            .map(|id| domain::TaskId::from_uuid(id.as_uuid()));
        let request = relayterm_application::IdPageRequest::new(
            params.after_id.map(|id| id.as_uuid().into_bytes()),
            params.limit,
            params.expected_revision.map(|revision| revision.get()),
        )
        .map_err(map_domain_error)?;
        let page = self
            .reads
            .worktree_page(self.workspace_id, task, request)
            .await
            .map_err(map_domain_error)?;
        let items = page.items.iter().map(worktree_dto).collect::<Vec<_>>();
        let next_after_id = page
            .has_more
            .then(|| page.items.last().map(|item| item.record().id.to_string()))
            .flatten();
        Ok(
            json!({"revision":page.revision.to_string(),"last_sequence":page.last_sequence.to_string(),"items":items,"next_after_id":next_after_id}),
        )
    }

    async fn worktree_select(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::WorktreeSelectParams = parameters(value)?;
        let task_id = domain::TaskId::from_uuid(params.task_id.as_uuid());
        let worktree_id = params
            .worktree_id
            .map(|id| domain::WorktreeId::from_uuid(id.as_uuid()));
        let outcome = self
            .service
            .execute_domain_command_at_revision(
                self.workspace_id,
                domain::Command::SelectWorktree {
                    task_id,
                    worktree_id,
                },
                params.expected_revision.get(),
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }

    async fn worktree_reconcile(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::WorktreeReconcileParams = parameters(value)?;
        let _permit = self
            .git_admissions
            .clone()
            .try_acquire_owned()
            .map_err(|_| unavailable())?;
        let operation_id = domain::WorktreeOperationId::from_uuid(params.operation_id.as_uuid());
        let snapshot = self
            .reads
            .consistent_projection(
                self.workspace_id,
                MutationScope {
                    ..MutationScope::default()
                },
            )
            .await
            .map_err(map_domain_error)?;
        if snapshot.snapshot.revision() != params.expected_revision.get() {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        let state = snapshot.snapshot.state().map_err(map_domain_error)?;
        let intent = state
            .worktree_intents()
            .iter()
            .find(|intent| intent.record().id == operation_id)
            .ok_or_else(|| map_domain_error(domain::Error::Reference))?;
        if intent.record().phase == domain::WorktreePhase::Ready {
            return Ok(worktree_operation_dto(intent, state));
        }
        if !matches!(
            intent.record().phase,
            domain::WorktreePhase::Applying | domain::WorktreePhase::NeedsAttention
        ) {
            return Err(map_domain_error(domain::Error::State));
        }
        let record = intent.record().clone();
        let git = self.git.clone();
        tokio::task::spawn_blocking(move || {
            git.verify_worktree(
                &record.repository_identity,
                &record.destination,
                &record.branch,
                &record.common_directory_identity,
                &record.resolved_commit,
            )
        })
        .await
        .map_err(|_| unavailable())?
        .map_err(map_git_error)?;
        let now = relayterm_platform::SystemClock
            .now()
            .map_err(map_domain_error)?;
        let record = intent.record();
        let worktree = domain::Worktree::restore(domain::WorktreeRecord {
            id: record.worktree_id,
            workspace_id: record.workspace_id,
            task_id: record.task_id,
            operation_id,
            root_id: record.root_id,
            checkout_path: record.destination.clone(),
            common_directory_identity: record.common_directory_identity.clone(),
            branch_ref: format!("refs/heads/{}", record.branch),
            initial_base_commit: record.resolved_commit.clone(),
            health: domain::WorktreeHealth::Ready,
            created_at: now,
            updated_at: now,
        })
        .map_err(map_domain_error)?;
        self.finalize_verified_worktree(operation_id, worktree)
            .await?;
        self.worktree_operation(
            &serde_json::to_value(wire::WorktreeOperationParams {
                operation_id: params.operation_id,
            })
            .map_err(|_| invalid())?,
        )
        .await
    }

    async fn finalize_verified_worktree(
        &self,
        operation_id: domain::WorktreeOperationId,
        worktree: domain::Worktree,
    ) -> Result<(), wire::ErrorBody> {
        for _ in 0..8 {
            let snapshot = self
                .reads
                .consistent_projection(
                    self.workspace_id,
                    MutationScope {
                        task_ids: vec![worktree.record().task_id],
                        ..MutationScope::default()
                    },
                )
                .await
                .map_err(map_domain_error)?;
            let state = snapshot.snapshot.state().map_err(map_domain_error)?;
            let task_id = worktree.record().task_id;
            let task = state.task(task_id).map_err(map_domain_error)?;
            let select = !task.record().status.is_final()
                && task.record().status != domain::TaskStatus::Active
                && state.current_claim(task_id).is_none()
                && !state.instances().iter().any(|instance| {
                    instance.record().task_id == Some(task_id)
                        && !instance.record().status.is_final()
                });
            match self
                .service
                .execute_domain_command_at_revision(
                    self.workspace_id,
                    domain::Command::FinalizeWorktree {
                        operation_id,
                        worktree: worktree.clone(),
                        select,
                    },
                    snapshot.snapshot.revision(),
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(domain::Error::Conflict) => continue,
                Err(error) => return Err(map_domain_error(error)),
            }
        }
        Err(map_domain_error(domain::Error::Conflict))
    }

    async fn backup_create(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::BackupCreateParams = parameters(value)?;
        let bytes = params.destination.decode().map_err(|_| invalid())?;
        let destination =
            relayterm_platform::decode_native_path(&params.destination.encoding, &bytes)
                .map_err(|_| invalid())?;
        let _permit = admit_backup(&self.backup_admission)?;
        let backup = self.backup.as_ref().ok_or_else(unavailable)?.clone();
        let report = backup(destination).await.map_err(map_backup_error)?;
        serde_json::to_value(report).map_err(|_| invalid())
    }

    async fn session_create(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::SessionCreateParams = parameters(value)?;
        validate_reserved(wire::Operation::SessionCreate, value)?;
        let supervisor = self.supervisor.as_ref().ok_or_else(unavailable)?;
        let definition_id = params
            .definition_id
            .map(|id| domain::AgentDefinitionId::from_uuid(id.as_uuid()));
        let task_id = params
            .task_id
            .map(|id| domain::TaskId::from_uuid(id.as_uuid()));
        let before = self
            .reads
            .consistent_projection(
                self.workspace_id,
                MutationScope {
                    task_ids: task_id.into_iter().collect(),
                    definition_ids: definition_id.into_iter().collect(),
                    ..MutationScope::default()
                },
            )
            .await
            .map_err(map_domain_error)?;
        let state = before.snapshot.state().map_err(map_domain_error)?;
        let root = match task_id {
            Some(task_id) => {
                if state.worktree_intents().iter().any(|intent| {
                    intent.record().task_id == task_id && !intent.record().phase.is_terminal()
                }) {
                    return Err(map_domain_error(domain::Error::State));
                }
                let task = state.task(task_id).map_err(map_domain_error)?;
                match task.record().worktree_id {
                    Some(id) => {
                        let worktree = state
                            .worktrees()
                            .iter()
                            .find(|candidate| candidate.record().id == id)
                            .ok_or_else(|| map_domain_error(domain::Error::Integrity))?;
                        if worktree.record().health != domain::WorktreeHealth::Ready {
                            return Err(unavailable());
                        }
                        let record = worktree.record().clone();
                        let git = self.git.clone();
                        tokio::task::spawn_blocking(move || {
                            let path = record.checkout_path.canonicalize().map_err(|_| ())?;
                            let repository = git.inspect(&path).map_err(|_| ())?;
                            if repository.common_directory != record.common_directory_identity {
                                return Err(());
                            }
                            let branch = git
                                .list(&path)
                                .map_err(|_| ())?
                                .into_iter()
                                .find(|entry| {
                                    entry.path.canonicalize().ok().as_ref() == Some(&path)
                                })
                                .and_then(|entry| entry.branch);
                            if branch.as_deref() != Some(record.branch_ref.as_str()) {
                                return Err(());
                            }
                            Ok(path)
                        })
                        .await
                        .map_err(|_| unavailable())?
                        .map_err(|_| unavailable())?
                    }
                    None => state.workspace().record().project_root.clone(),
                }
            }
            None => state.workspace().record().project_root.clone(),
        };
        let requested = match params.working_directory {
            Some(path) => {
                let bytes = path.decode().map_err(|_| invalid())?;
                relayterm_platform::decode_native_path(&path.encoding, &bytes)
                    .map_err(|_| invalid())?
            }
            None => root.clone(),
        };
        let working_directory =
            relayterm_pty::canonical_working_directory(&root, &requested).map_err(|_| invalid())?;
        let (program, arguments, allowed) = match params.launch_kind {
            wire::SessionLaunchKind::DefaultShell => {
                (relayterm_pty::default_shell(), Vec::new(), Vec::new())
            }
            wire::SessionLaunchKind::Definition => {
                let id = definition_id.ok_or_else(invalid)?;
                let definition = state
                    .definitions()
                    .iter()
                    .find(|definition| definition.record().id == id)
                    .ok_or_else(|| map_domain_error(domain::Error::Reference))?;
                let record = definition.record();
                if !record.enabled {
                    return Err(unavailable());
                }
                let resolved = self
                    .resolve_program(
                        record.command.clone(),
                        root.clone(),
                        working_directory.clone(),
                    )
                    .await?;
                (
                    resolved.into(),
                    record.arguments.iter().map(Into::into).collect(),
                    record.environment_allowlist.clone(),
                )
            }
        };
        let receipt_id = params.receipt_id.as_uuid();
        let launch_key = supervisor::LaunchKey {
            definition_id: definition_id.map(|id| id.as_uuid()),
            task_id: task_id.map(|id| id.as_uuid()),
            program: program.clone(),
            arguments: arguments.clone(),
            working_directory: working_directory.clone(),
            rows: params.rows,
            columns: params.columns,
        };
        match supervisor
            .admit_launch(receipt_id, launch_key)
            .map_err(map_supervisor)?
        {
            supervisor::LaunchAdmission::New => {}
            supervisor::LaunchAdmission::Existing(outcome) if outcome.succeeded => {
                return Ok(json!({
                    "receipt_id": receipt_id.to_string(),
                    "instance_id": outcome.instance_id.to_string(),
                    "session_id": outcome.session_id.to_string(),
                    "status": "running"
                }));
            }
            supervisor::LaunchAdmission::Existing(_) => {
                return Err(map_supervisor(supervisor::SupervisorError::Io));
            }
        }
        let registered = self
            .service
            .register_instance_at_revision(
                self.workspace_id,
                relayterm_application::LaunchContext {
                    agent_definition_id: definition_id,
                    task_id,
                    working_directory: working_directory.clone(),
                    terminal_size: domain::TerminalSize::new(params.rows, params.columns)
                        .map_err(map_domain_error)?,
                },
                Some(before.snapshot.revision()),
            )
            .await;
        let registered = match registered {
            Ok(value) => value,
            Err(error) => {
                supervisor.cancel_launch(receipt_id);
                return Err(map_domain_error(error));
            }
        };
        let instance_id = registered.instance_id;
        let session_id = registered.session_id;
        if let Some(snapshot) = &registered.launch_definition {
            debug_assert_eq!(snapshot.arguments.len(), arguments.len());
            debug_assert_eq!(snapshot.environment_allowlist, allowed);
        }
        let environment = relayterm_pty::checked_terminal_environment(
            relayterm_pty::approved_environment(&allowed, std::env::vars_os()),
        )
        .map_err(|_| resource())?;
        let spawned = supervisor.launch(
            session_id.as_uuid(),
            instance_id.as_uuid(),
            relayterm_pty::SpawnRequest {
                program,
                arguments,
                working_directory,
                environment,
                rows: params.rows,
                columns: params.columns,
            },
            cfg!(windows) && matches!(params.launch_kind, wire::SessionLaunchKind::DefaultShell),
        );
        let now = relayterm_platform::SystemClock
            .now()
            .map_err(map_domain_error)?;
        let status = if spawned.is_ok() {
            domain::InstanceStatus::Running
        } else {
            domain::InstanceStatus::Failed
        };
        let observed = self
            .service
            .observe(
                self.workspace_id,
                domain::Observation::Status {
                    id: instance_id,
                    status,
                    observed_at: now,
                    exit_code: None,
                },
            )
            .await;
        if observed.is_err() && spawned.is_ok() {
            let _ = supervisor.terminate(session_id.as_uuid());
        }
        let succeeded = spawned.is_ok() && observed.is_ok();
        supervisor
            .complete_launch(
                receipt_id,
                supervisor::LaunchOutcome {
                    instance_id: instance_id.as_uuid(),
                    session_id: session_id.as_uuid(),
                    succeeded,
                },
            )
            .map_err(map_supervisor)?;
        observed.map_err(map_domain_error)?;
        spawned.map_err(map_supervisor)?;
        Ok(json!({
            "receipt_id": receipt_id.to_string(),
            "instance_id": instance_id.to_string(),
            "session_id": session_id.to_string(),
            "status": "running"
        }))
    }

    fn agent_templates(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let _: wire::AgentListTemplatesParams = parameters(value)?;
        let templates = relayterm_config::agent_templates()
            .map_err(|_| unavailable())?
            .into_iter()
            .map(|template| wire::AgentTemplateDto {
                key: template.key.to_owned(),
                display_name: template.display_name,
                command: template.command,
                arguments: template.arguments,
                environment_allowlist: template.environment_allowlist,
                capabilities: template.capabilities,
                enabled: template.enabled,
            })
            .collect();
        serde_json::to_value(wire::AgentTemplateCatalogDto {
            catalog_version: relayterm_config::TEMPLATE_CATALOG_VERSION,
            templates,
        })
        .map_err(|_| invalid())
    }

    async fn check_definition(&self, value: &Value) -> Result<Value, wire::ErrorBody> {
        let params: wire::AgentCheckDefinitionParams = parameters(value)?;
        let snapshot = self
            .reads
            .consistent_projection(
                self.workspace_id,
                MutationScope {
                    definition_ids: vec![domain::AgentDefinitionId::from_uuid(
                        params.definition_id.as_uuid(),
                    )],
                    ..MutationScope::default()
                },
            )
            .await
            .map_err(map_domain_error)?;
        if snapshot.snapshot.revision() != params.expected_revision.get() {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        let id = domain::AgentDefinitionId::from_uuid(params.definition_id.as_uuid());
        let state = snapshot.snapshot.state().map_err(map_domain_error)?;
        let definition = state
            .definitions()
            .iter()
            .find(|candidate| candidate.record().id == id)
            .ok_or_else(|| map_domain_error(domain::Error::Reference))?;
        let record = definition.record();
        let root = state.workspace().record().project_root.clone();
        let resolution = self
            .resolve_executable(record.command.clone(), root.clone(), root)
            .await?;
        let status = availability_status(resolution.status);
        let guidance_code = match status {
            wire::AgentAvailabilityStatus::Available => "available",
            wire::AgentAvailabilityStatus::NotFound => "check_daemon_path_or_command",
            wire::AgentAvailabilityStatus::NotExecutable => "select_executable_file",
            wire::AgentAvailabilityStatus::UnsupportedLauncher => {
                "configure_native_executable_or_interpreter"
            }
            wire::AgentAvailabilityStatus::InvalidCommand => "edit_invalid_command",
            wire::AgentAvailabilityStatus::Unavailable => "check_resource_limit",
        };
        serde_json::to_value(wire::AgentCheckDefinitionResult {
            definition_id: params.definition_id,
            observed_revision: params.expected_revision,
            enabled: record.enabled,
            status,
            guidance_code: guidance_code.to_owned(),
        })
        .map_err(|_| invalid())
    }

    async fn resolve_program(
        &self,
        command: String,
        workspace_root: PathBuf,
        working_directory: PathBuf,
    ) -> Result<PathBuf, wire::ErrorBody> {
        let resolution = self
            .resolve_executable(command, workspace_root, working_directory)
            .await?;
        resolution
            .executable
            .map(|value| value.path)
            .ok_or_else(unavailable)
    }

    async fn resolve_executable(
        &self,
        command: String,
        workspace_root: PathBuf,
        working_directory: PathBuf,
    ) -> Result<relayterm_platform::ExecutableResolution, wire::ErrorBody> {
        let admission = self
            .resolver_admissions
            .clone()
            .try_acquire_owned()
            .map_err(|_| resource())?;
        let jobs = self.resolver_jobs.clone();
        let search_path = std::env::var_os("PATH");
        let task = async move {
            let job = jobs.acquire_owned().await.map_err(|_| unavailable())?;
            tokio::task::spawn_blocking(move || {
                let _admission = admission;
                let _job = job;
                relayterm_platform::resolve_executable(
                    &command,
                    &workspace_root,
                    &working_directory,
                    search_path.as_deref(),
                )
            })
            .await
            .map_err(|_| unavailable())
        };
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .map_err(|_| unavailable())?
    }

    async fn session_attach(
        &self,
        value: &Value,
        connection_id: wire::ConnectionId,
    ) -> Result<Value, wire::ErrorBody> {
        let params: wire::SessionAttachParams = parameters(value)?;
        let (attachment_id, stream_id, snapshot) = self
            .supervisor
            .as_ref()
            .ok_or_else(unavailable)?
            .attach(params.session_id.as_uuid(), connection_id.as_uuid())
            .map_err(map_supervisor)?;
        let generation = self
            .lifecycle
            .as_ref()
            .ok_or_else(unavailable)?
            .generation();
        let value = json!({
            "daemon_generation": generation,
            "attachment_id": attachment_id.to_string(),
            "stream_id": stream_id.to_string(),
            "snapshot": snapshot,
        });
        if serde_json::to_vec(&value).map_err(|_| resource())?.len() > wire::JSON_FRAME_LIMIT / 2 {
            return Err(resource());
        }
        Ok(value)
    }

    async fn session_read_display(
        &self,
        value: &Value,
        connection_id: wire::ConnectionId,
    ) -> Result<Value, wire::ErrorBody> {
        let params: wire::SessionReadDisplayParams = parameters(value)?;
        let requested = usize::from(params.rows)
            .checked_mul(usize::from(params.columns))
            .ok_or_else(resource)?;
        if params.rows == 0 || params.columns == 0 || requested > wire::MAX_DISPLAY_CELLS {
            return Err(resource());
        }
        let (attachment_id, stream_id, mut snapshot, scrollback_offset, retained_scrollback_rows) =
            self.supervisor
                .as_ref()
                .ok_or_else(unavailable)?
                .attach_at(
                    params.session_id.as_uuid(),
                    connection_id.as_uuid(),
                    usize::from(params.scrollback_rows),
                )
                .map_err(map_supervisor)?;
        if params
            .attachment_id
            .is_some_and(|expected| expected.as_uuid() != attachment_id)
        {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        let source_rows = snapshot.rows;
        let source_columns = snapshot.columns;
        let top = params.top.min(source_rows);
        let left = params.left.min(source_columns);
        let top_end = top.saturating_add(params.rows).min(source_rows);
        let left_end = left.saturating_add(params.columns).min(source_columns);
        let rows = top_end.saturating_sub(top);
        let columns = left_end.saturating_sub(left);
        let unchanged = params.after_revision == Some(snapshot.revision)
            && params.after_scrollback_offset == u16::try_from(scrollback_offset).ok();
        if !unchanged {
            let mut cells = Vec::with_capacity(usize::from(rows) * usize::from(columns));
            for row in top..top_end {
                let start = usize::from(row) * usize::from(source_columns) + usize::from(left);
                let end = start + usize::from(columns);
                cells.extend_from_slice(&snapshot.cells[start..end]);
            }
            let cursor_visible = snapshot.cursor_row >= top
                && snapshot.cursor_row < top_end
                && snapshot.cursor_column >= left
                && snapshot.cursor_column < left_end;
            snapshot.cursor_hidden |= !cursor_visible;
            snapshot.cursor_row = snapshot.cursor_row.saturating_sub(top);
            snapshot.cursor_column = snapshot.cursor_column.saturating_sub(left);
            snapshot.rows = rows;
            snapshot.columns = columns;
            snapshot.cells = cells;
        }
        let generation = self
            .lifecycle
            .as_ref()
            .ok_or_else(unavailable)?
            .generation();
        let snapshot = (!unchanged).then(|| display_snapshot(snapshot));
        serde_json::to_value(wire::TerminalDisplayDto {
            daemon_generation: generation.to_owned(),
            attachment_id: wire::AttachmentId::from_uuid(attachment_id),
            stream_id: wire::DecimalU64::new(stream_id).map_err(|_| resource())?,
            source_rows,
            source_columns,
            top,
            left,
            scrollback_offset: u16::try_from(scrollback_offset).map_err(|_| resource())?,
            retained_scrollback_rows: u16::try_from(retained_scrollback_rows)
                .map_err(|_| resource())?,
            unchanged,
            snapshot,
        })
        .map_err(|_| resource())
    }

    fn session_read_output(
        &self,
        value: &Value,
        connection_id: wire::ConnectionId,
    ) -> Result<(Value, Option<Vec<u8>>), wire::ErrorBody> {
        let params: wire::SessionReadOutputParams = parameters(value)?;
        let supervisor = self.supervisor.as_ref().ok_or_else(unavailable)?;
        match supervisor.read_output(
            params.session_id.as_uuid(),
            connection_id.as_uuid(),
            params.attachment_id.as_uuid(),
            params.after_offset.get(),
        ) {
            Ok((stream_id, next_offset, data)) => {
                let terminal = if data.is_empty() {
                    None
                } else {
                    Some(
                        wire::TerminalFrame {
                            session_id: params.session_id,
                            stream_id,
                            offset: params.after_offset.get(),
                            data,
                        }
                        .encode()
                        .map_err(|_| resource())?,
                    )
                };
                Ok((
                    json!({
                        "stream_id": stream_id.to_string(),
                        "data_follows": terminal.is_some(),
                        "resnapshot_required": false,
                        "next_offset": next_offset.to_string()
                    }),
                    terminal,
                ))
            }
            Err(supervisor::SupervisorError::ResnapshotRequired(stream_id)) => Ok((
                json!({
                    "stream_id": stream_id.to_string(),
                    "data_follows": false,
                    "resnapshot_required": true,
                    "next_offset": params.after_offset.get().to_string()
                }),
                None,
            )),
            Err(error) => Err(map_supervisor(error)),
        }
    }

    async fn observe_terminal_exits(&self) {
        let Some(supervisor) = &self.supervisor else {
            return;
        };
        for exit in supervisor.poll_exits() {
            let Ok(at) = relayterm_platform::SystemClock.now() else {
                continue;
            };
            let _ = self
                .service
                .observe(
                    self.workspace_id,
                    domain::Observation::Status {
                        id: domain::AgentInstanceId::from_uuid(exit.instance_id),
                        status: if exit.terminated {
                            domain::InstanceStatus::Terminated
                        } else {
                            domain::InstanceStatus::Exited
                        },
                        observed_at: at,
                        exit_code: Some(exit.code),
                    },
                )
                .await;
        }
    }

    async fn snapshot_page(
        &self,
        params: &Value,
        forced: Option<Collection>,
    ) -> Result<Value, wire::ErrorBody> {
        let p: SnapshotParams = parameters(params)?;
        if p.limit == 0 || p.limit > wire::MAX_PAGE_SIZE {
            return Err(invalid_field(wire::ErrorField::PageLimit));
        }
        let collection = forced.unwrap_or(p.collection.unwrap_or(Collection::Workspace));
        if matches!(
            collection,
            Collection::Definitions
                | Collection::Tasks
                | Collection::Instances
                | Collection::Claims
                | Collection::Progress
                | Collection::Handovers
        ) {
            return self.id_collection_page(collection, p).await;
        }
        let snap = self
            .reads
            .consistent_projection(self.workspace_id, MutationScope::default())
            .await
            .map_err(map_domain_error)?;
        let revision = snap.snapshot.revision();
        if p.expected_revision
            .as_deref()
            .is_some_and(|x| x != revision.to_string())
        {
            return Err(map_domain_error(domain::Error::Conflict));
        }
        let state = snap.snapshot.state().map_err(map_domain_error)?;
        let mut items = match collection {
            Collection::Workspace => vec![workspace_dto(state.workspace())],
            Collection::Definitions => state.definitions().iter().map(definition_dto).collect(),
            Collection::Tasks => state.tasks().iter().map(task_dto).collect(),
            Collection::Instances => state.instances().iter().map(instance_dto).collect(),
            Collection::Claims => state.claims().iter().map(claim_dto).collect(),
            Collection::Progress => state.progress().iter().map(progress_dto).collect(),
            Collection::Handovers => state.handovers().iter().map(handover_dto).collect(),
        };
        items.sort_by_key(entity_sort_key);
        if let Some(after) = p.after_id {
            items.retain(|v| entity_sort_key(v) > after)
        }
        let available = items.len();
        let mut page = Vec::new();
        let mut encoded_bytes = 2_usize;
        for item in items.into_iter().take(usize::from(p.limit)) {
            let item_bytes = serde_json::to_vec(&item).map_err(|_| invalid())?.len();
            let separator = usize::from(!page.is_empty());
            let next_bytes = encoded_bytes
                .checked_add(separator)
                .and_then(|value| value.checked_add(item_bytes))
                .ok_or_else(resource)?;
            if next_bytes > wire::COLLECTION_PAGE_BYTES {
                break;
            }
            encoded_bytes = next_bytes;
            page.push(item);
        }
        if page.is_empty() && available != 0 {
            return Err(resource());
        }
        let has_more = page.len() < available;
        let next_after_id = has_more.then(|| page.last().map(entity_sort_key)).flatten();
        Ok(
            json!({"revision":revision.to_string(),"last_sequence":snap.last_sequence.to_string(),"retained_from_sequence":snap.retained_from_sequence.to_string(),"collection":collection,"items":page,"next_after_id":next_after_id}),
        )
    }

    async fn id_collection_page(
        &self,
        collection: Collection,
        params: SnapshotParams,
    ) -> Result<Value, wire::ErrorBody> {
        let after_id = params
            .after_id
            .as_deref()
            .map(str::parse::<uuid::Uuid>)
            .transpose()
            .map_err(|_| invalid())?
            .map(uuid::Uuid::into_bytes);
        let expected_revision = params
            .expected_revision
            .as_deref()
            .map(|value| decimal(value, false))
            .transpose()?;
        let request =
            relayterm_application::IdPageRequest::new(after_id, params.limit, expected_revision)
                .map_err(map_domain_error)?;
        let (revision, last_sequence, retained_from_sequence, items, storage_has_more) =
            match collection {
                Collection::Definitions => {
                    let page = self
                        .reads
                        .definition_page(self.workspace_id, request)
                        .await
                        .map_err(map_domain_error)?;
                    (
                        page.revision,
                        page.last_sequence,
                        page.retained_from_sequence,
                        page.items.iter().map(definition_dto).collect::<Vec<_>>(),
                        page.has_more,
                    )
                }
                Collection::Tasks => {
                    let page = self
                        .reads
                        .task_page(self.workspace_id, request)
                        .await
                        .map_err(map_domain_error)?;
                    (
                        page.revision,
                        page.last_sequence,
                        page.retained_from_sequence,
                        page.items.iter().map(task_dto).collect::<Vec<_>>(),
                        page.has_more,
                    )
                }
                Collection::Instances => {
                    let page = self
                        .reads
                        .instance_page(self.workspace_id, request)
                        .await
                        .map_err(map_domain_error)?;
                    (
                        page.revision,
                        page.last_sequence,
                        page.retained_from_sequence,
                        page.items.iter().map(instance_dto).collect::<Vec<_>>(),
                        page.has_more,
                    )
                }
                Collection::Claims => {
                    let page = self
                        .reads
                        .claim_page(self.workspace_id, request)
                        .await
                        .map_err(map_domain_error)?;
                    (
                        page.revision,
                        page.last_sequence,
                        page.retained_from_sequence,
                        page.items.iter().map(claim_dto).collect::<Vec<_>>(),
                        page.has_more,
                    )
                }
                Collection::Progress => {
                    let page = self
                        .reads
                        .progress_page(self.workspace_id, request)
                        .await
                        .map_err(map_domain_error)?;
                    (
                        page.revision,
                        page.last_sequence,
                        page.retained_from_sequence,
                        page.items.iter().map(progress_dto).collect::<Vec<_>>(),
                        page.has_more,
                    )
                }
                Collection::Handovers => {
                    let page = self
                        .reads
                        .handover_page(self.workspace_id, request)
                        .await
                        .map_err(map_domain_error)?;
                    (
                        page.revision,
                        page.last_sequence,
                        page.retained_from_sequence,
                        page.items.iter().map(handover_dto).collect::<Vec<_>>(),
                        page.has_more,
                    )
                }
                _ => return Err(invalid()),
            };
        let available = items.len();
        let mut page = Vec::with_capacity(available);
        let mut encoded_bytes = 2_usize;
        for item in items {
            let item_bytes = serde_json::to_vec(&item).map_err(|_| invalid())?.len();
            let separator = usize::from(!page.is_empty());
            let next_bytes = encoded_bytes
                .checked_add(separator)
                .and_then(|value| value.checked_add(item_bytes))
                .ok_or_else(resource)?;
            if next_bytes > wire::COLLECTION_PAGE_BYTES {
                break;
            }
            encoded_bytes = next_bytes;
            page.push(item);
        }
        if page.is_empty() && available != 0 {
            return Err(resource());
        }
        let has_more = storage_has_more || page.len() < available;
        let next_after_id = has_more.then(|| page.last().map(entity_sort_key)).flatten();
        Ok(json!({
            "revision": revision.to_string(),
            "last_sequence": last_sequence.to_string(),
            "retained_from_sequence": retained_from_sequence.to_string(),
            "collection": collection,
            "items": page,
            "next_after_id": next_after_id
        }))
    }

    async fn event_list(&self, params: &Value) -> Result<Value, wire::ErrorBody> {
        let p: EventParams = parameters(params)?;
        if p.limit == 0 || p.limit > wire::MAX_PAGE_SIZE {
            return Err(invalid_field(wire::ErrorField::PageLimit));
        }
        let after = decimal(&p.after_sequence, true)?;
        let page = self
            .reads
            .event_page(
                self.workspace_id,
                EventPageRequest::new(after, p.limit).map_err(map_domain_error)?,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(
            json!({"events":page.events.iter().map(event_dto).collect::<Vec<_>>(),"last_sequence":page.last_sequence.to_string(),"retained_from_sequence":page.retained_from_sequence.to_string()}),
        )
    }
    async fn history(&self, request: &wire::RequestEnvelope) -> Result<Value, wire::ErrorBody> {
        let p: HistoryParams = parameters(&request.params)?;
        let task = p.task_id.parse().map_err(|_| invalid())?;
        let after = decimal(&p.after_sequence, true)?;
        let revision = decimal(&p.expected_revision, false)?;
        let page = self
            .reads
            .task_history_page(
                self.workspace_id,
                task,
                TaskHistoryPageRequest::new(after, p.limit, revision).map_err(map_domain_error)?,
            )
            .await
            .map_err(map_domain_error)?;
        let claims_only = request.operation == wire::Operation::TaskGetClaimHistory;
        let entries = page
            .entries
            .into_iter()
            .filter_map(|entry| {
                let value = match entry.item {
                    TaskHistoryItem::Claim(v) => claim_dto(&v),
                    TaskHistoryItem::Progress(v) if !claims_only => progress_dto(&v),
                    TaskHistoryItem::Handover(v) if !claims_only => handover_dto(&v),
                    _ => return None,
                };
                Some(json!({"sequence":entry.sequence.to_string(),"item":value}))
            })
            .collect::<Vec<_>>();
        Ok(json!({"revision":page.revision.to_string(),"entries":entries}))
    }

    async fn register_definition(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: DefinitionMutation = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::AddDefinition {
                    display_name: p.display_name,
                    command: p.command,
                    arguments: p.arguments,
                    environment_allowlist: p.environment_allowlist,
                    capabilities: p.capabilities,
                    enabled: p.enabled,
                },
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn update_definition(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: DefinitionUpdate = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let definition = domain::AgentDefinition::restore(domain::AgentDefinitionRecord {
            id: p.definition_id.parse().map_err(|_| invalid())?,
            workspace_id: self.workspace_id,
            display_name: p.display_name,
            command: p.command,
            arguments: p.arguments,
            environment_allowlist: p.environment_allowlist,
            capabilities: p.capabilities,
            enabled: p.enabled,
        })
        .map_err(map_domain_error)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::UpdateDefinition(definition),
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn import_definitions(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: wire::AgentImportDefinitionsParams = parameters(v)?;
        let parsed = relayterm_config::parse(p.document.as_bytes()).map_err(|_| invalid())?;
        let definitions = parsed
            .definitions_for(self.workspace_id)
            .map_err(|_| invalid())?;
        let baseline = match p.expected_revision {
            Some(value) => value.get(),
            None => self
                .reads
                .consistent_projection(self.workspace_id, MutationScope::default())
                .await
                .map_err(map_domain_error)?
                .snapshot
                .revision(),
        };
        let outcome = self
            .service
            .import_definitions(self.workspace_id, baseline, definitions)
            .await
            .map_err(map_domain_error)?;
        Ok(json!({
            "revision": outcome.committed.snapshot.revision().to_string(),
            "event_sequences": outcome.committed.events.iter().map(|event| event.record().sequence.to_string()).collect::<Vec<_>>(),
            "warnings": parsed.warnings.iter().map(|warning| format!("{:?}", warning.code).to_ascii_lowercase()).collect::<Vec<_>>()
        }))
    }
    async fn mutate_task_create(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: TaskCreate = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::CreateTask(p.content.try_into()?),
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn mutate_task_update(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: TaskUpdate = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::EditTask {
                    id: p.task_id.parse().map_err(|_| invalid())?,
                    content: p.content.try_into()?,
                },
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn mutate_transition(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: Transition = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::Transition {
                    id: p.task_id.parse().map_err(|_| invalid())?,
                    to: p.status,
                },
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn mutate_claim(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: Claim = parameters(v)?;
        let outcome = self
            .service
            .execute(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::Claim {
                    task_id: p.task_id.parse().map_err(|_| invalid())?,
                    instance_id: p.instance_id.parse().map_err(|_| invalid())?,
                },
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn mutate_release(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: TaskRef = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::Release {
                    task_id: p.task_id.parse().map_err(|_| invalid())?,
                },
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn mutate_progress(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: Progress = parameters(v)?;
        let outcome = self
            .service
            .execute(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::Progress {
                    task_id: p.task_id.parse().map_err(|_| invalid())?,
                    summary: p.summary,
                    verification: p.verification,
                },
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
    async fn mutate_handover(&self, v: &Value) -> Result<Value, wire::ErrorBody> {
        let p: Handover = parameters(v)?;
        let expected = decimal(&p.expected_revision, false)?;
        let outcome = self
            .service
            .execute_at_revision(
                self.workspace_id,
                domain::Actor::LocalUser,
                Request::Handover {
                    task_id: p.task_id.parse().map_err(|_| invalid())?,
                    content: domain::HandoverContent {
                        summary: p.summary,
                        decisions: p.decisions,
                        changed_paths: p.changed_paths,
                        verification_performed: p.verification_performed,
                        open_questions: p.open_questions,
                        recommended_next_action: p.recommended_next_action,
                    },
                },
                expected,
            )
            .await
            .map_err(map_domain_error)?;
        Ok(receipt(&outcome))
    }
}

fn admit_backup(admission: &Arc<Semaphore>) -> Result<OwnedSemaphorePermit, wire::ErrorBody> {
    admission
        .clone()
        .try_acquire_owned()
        .map_err(|_| unavailable())
}

fn try_queue_event(
    sender: &mpsc::Sender<QueuedEvent>,
    budget: &Arc<Semaphore>,
    bytes: Vec<u8>,
    permit_count: u32,
) -> bool {
    let Ok(permit) = budget.clone().try_acquire_many_owned(permit_count) else {
        return false;
    };
    sender
        .try_send(QueuedEvent {
            bytes,
            _permit: permit,
        })
        .is_ok()
}

async fn wait_for_wakeup(receiver: &mut Option<watch::Receiver<u64>>) {
    if let Some(active) = receiver {
        if active.changed().await.is_ok() {
            return;
        }
        *receiver = None;
    }
    std::future::pending().await
}

async fn writer_loop<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut controls: mpsc::Receiver<WriterControl>,
    mut events: mpsc::Receiver<QueuedEvent>,
    faults: ServerFaults,
) -> Result<(), ServerError> {
    loop {
        tokio::select! {biased;
            control=controls.recv()=>{
                match control {
                    Some(WriterControl::Frame(bytes))=>write_frame(&mut writer,wire::FrameKind::Json,&bytes).await.map_err(|_|ServerError::Transport)?,
                    Some(WriterControl::FrameThenTerminal(bytes, terminal))=>{
                        write_frame(&mut writer,wire::FrameKind::Json,&bytes).await.map_err(|_|ServerError::Transport)?;
                        write_frame(&mut writer,wire::FrameKind::Terminal,&terminal).await.map_err(|_|ServerError::Transport)?;
                    }
                    Some(WriterControl::FrameAndFlush(bytes, completed))=>{
                        write_frame(&mut writer,wire::FrameKind::Json,&bytes).await.map_err(|_|ServerError::Transport)?;
                        let _ = completed.send(());
                    }
                    Some(WriterControl::ResetSubscription(bytes))=>{
                        while events.try_recv().is_ok() {}
                        write_frame(&mut writer,wire::FrameKind::Json,&bytes).await.map_err(|_|ServerError::Transport)?;
                    }
                    None=>return Ok(()),
                }
            }
            event=events.recv()=>{
                match event {
                    Some(event)=>{
                        faults.pause_event_write_if_requested().await;
                        write_frame(&mut writer,wire::FrameKind::Json,&event.bytes).await.map_err(|_|ServerError::Transport)?
                    },
                    None=>return Ok(()),
                }
            }
        }
    }
}

async fn queue_response_confirmed(
    sender: &mpsc::Sender<WriterControl>,
    response: wire::ResponseEnvelope,
) -> Result<(), ServerError> {
    let bytes = wire::encode_json(&response).map_err(|_| ServerError::Protocol)?;
    if bytes.len() > wire::JSON_FRAME_LIMIT {
        return Err(ServerError::ResourceLimit);
    }
    let (completed, confirmation) = tokio::sync::oneshot::channel();
    tokio::time::timeout(
        relayterm_ipc::WRITER_TIMEOUT,
        sender.send(WriterControl::FrameAndFlush(bytes, completed)),
    )
    .await
    .map_err(|_| ServerError::Transport)?
    .map_err(|_| ServerError::Transport)?;
    tokio::time::timeout(relayterm_ipc::WRITER_TIMEOUT, confirmation)
        .await
        .map_err(|_| ServerError::Transport)?
        .map_err(|_| ServerError::Transport)
}

async fn queue_response(
    sender: &mpsc::Sender<WriterControl>,
    response: wire::ResponseEnvelope,
    reset_subscription: bool,
) -> Result<(), ServerError> {
    let bytes = wire::encode_json(&response).map_err(|_| ServerError::Protocol)?;
    if bytes.len() > wire::JSON_FRAME_LIMIT {
        return Err(ServerError::ResourceLimit);
    }
    let command = if reset_subscription {
        WriterControl::ResetSubscription(bytes)
    } else {
        WriterControl::Frame(bytes)
    };
    tokio::time::timeout(relayterm_ipc::WRITER_TIMEOUT, sender.send(command))
        .await
        .map_err(|_| ServerError::Transport)?
        .map_err(|_| ServerError::Transport)
}

async fn queue_terminal_response(
    sender: &mpsc::Sender<WriterControl>,
    response: wire::ResponseEnvelope,
    terminal: Option<Vec<u8>>,
) -> Result<(), ServerError> {
    let bytes = wire::encode_json(&response).map_err(|_| ServerError::Protocol)?;
    if bytes.len() > wire::JSON_FRAME_LIMIT {
        return Err(ServerError::ResourceLimit);
    }
    let command = match terminal {
        Some(terminal) => WriterControl::FrameThenTerminal(bytes, terminal),
        None => WriterControl::Frame(bytes),
    };
    tokio::time::timeout(relayterm_ipc::WRITER_TIMEOUT, sender.send(command))
        .await
        .map_err(|_| ServerError::Transport)?
        .map_err(|_| ServerError::Transport)
}

async fn queue_synchronization(
    sender: &mpsc::Sender<WriterControl>,
    workspace_id: wire::WorkspaceId,
    subscription: Subscription,
    reason: wire::SynchronizationReason,
) -> Result<(), ServerError> {
    let envelope = wire::SynchronizationEnvelope {
        message_type: wire::SynchronizationMessageType::Event,
        protocol_version: wire::PROTOCOL_VERSION,
        workspace_id,
        subscription_id: subscription.id,
        request_id: subscription.request_id,
        control: wire::SynchronizationControl::ResnapshotRequired,
        reason: Some(reason),
        watermark: None,
    };
    let bytes = wire::encode_json(&envelope).map_err(|_| ServerError::Protocol)?;
    tokio::time::timeout(
        relayterm_ipc::WRITER_TIMEOUT,
        sender.send(WriterControl::ResetSubscription(bytes)),
    )
    .await
    .map_err(|_| ServerError::Transport)?
    .map_err(|_| ServerError::Transport)
}

async fn send_response<W: AsyncWrite + Unpin>(
    stream: &mut W,
    response: wire::ResponseEnvelope,
) -> Result<(), ServerError> {
    let bytes = wire::encode_json(&response).map_err(|_| ServerError::Protocol)?;
    if bytes.len() > wire::JSON_FRAME_LIMIT {
        return Err(ServerError::ResourceLimit);
    }
    write_frame(stream, wire::FrameKind::Json, &bytes)
        .await
        .map_err(|_| ServerError::Transport)
}
fn connection_id() -> wire::ConnectionId {
    wire::ConnectionId::from_uuid(opaque_uuid())
}
fn subscription_id() -> wire::SubscriptionId {
    wire::SubscriptionId::from_uuid(opaque_uuid())
}
fn opaque_uuid() -> uuid::Uuid {
    let n = CONNECTION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x40;
    bytes[8..].copy_from_slice(&(n | 0x8000_0000_0000_0000).to_be_bytes());
    uuid::Uuid::from_bytes(bytes)
}

#[derive(Clone, Copy, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Collection {
    Workspace,
    Definitions,
    Tasks,
    Instances,
    Claims,
    Progress,
    Handovers,
}
fn default_limit() -> u16 {
    wire::DEFAULT_PAGE_SIZE
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotParams {
    #[serde(default)]
    collection: Option<Collection>,
    #[serde(default)]
    after_id: Option<String>,
    #[serde(default = "default_limit")]
    limit: u16,
    #[serde(default)]
    expected_revision: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventParams {
    after_sequence: String,
    #[serde(default = "default_limit")]
    limit: u16,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryParams {
    task_id: String,
    after_sequence: String,
    expected_revision: String,
    #[serde(default = "default_limit")]
    limit: u16,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionMutation {
    expected_revision: String,
    display_name: String,
    command: String,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    environment_allowlist: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionUpdate {
    definition_id: String,
    expected_revision: String,
    display_name: String,
    command: String,
    arguments: Vec<String>,
    environment_allowlist: Vec<String>,
    capabilities: Vec<String>,
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskContentInput {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    priority: domain::Priority,
    #[serde(default)]
    scope_paths: Vec<String>,
    #[serde(default)]
    acceptance_notes: String,
    #[serde(default)]
    dependency_ids: Vec<String>,
}
impl TryFrom<TaskContentInput> for domain::TaskContent {
    type Error = wire::ErrorBody;
    fn try_from(v: TaskContentInput) -> Result<Self, Self::Error> {
        let content = Self {
            title: v.title,
            description: v.description,
            priority: v.priority,
            scope_paths: v.scope_paths,
            acceptance_notes: v.acceptance_notes,
            dependency_ids: v
                .dependency_ids
                .into_iter()
                .map(|x| x.parse().map_err(|_| invalid()))
                .collect::<Result<_, _>>()?,
        };
        content.validate().map_err(map_domain_error)?;
        Ok(content)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskCreate {
    expected_revision: String,
    #[serde(flatten)]
    content: TaskContentInput,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskUpdate {
    task_id: String,
    expected_revision: String,
    #[serde(flatten)]
    content: TaskContentInput,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    task_id: String,
    expected_revision: String,
    status: domain::TaskStatus,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    task_id: String,
    instance_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskRef {
    task_id: String,
    expected_revision: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    task_id: String,
    summary: String,
    verification: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Handover {
    task_id: String,
    expected_revision: String,
    summary: String,
    #[serde(default)]
    decisions: String,
    #[serde(default)]
    changed_paths: Vec<String>,
    verification_performed: String,
    #[serde(default)]
    open_questions: String,
    recommended_next_action: String,
}

fn parameters<T: for<'de> Deserialize<'de>>(value: &Value) -> Result<T, wire::ErrorBody> {
    serde_json::from_value(value.clone()).map_err(|_| invalid())
}

fn availability_status(
    status: relayterm_platform::ExecutableStatus,
) -> wire::AgentAvailabilityStatus {
    match status {
        relayterm_platform::ExecutableStatus::Available => wire::AgentAvailabilityStatus::Available,
        relayterm_platform::ExecutableStatus::NotFound => wire::AgentAvailabilityStatus::NotFound,
        relayterm_platform::ExecutableStatus::NotExecutable => {
            wire::AgentAvailabilityStatus::NotExecutable
        }
        relayterm_platform::ExecutableStatus::UnsupportedLauncher => {
            wire::AgentAvailabilityStatus::UnsupportedLauncher
        }
        relayterm_platform::ExecutableStatus::InvalidCommand => {
            wire::AgentAvailabilityStatus::InvalidCommand
        }
        relayterm_platform::ExecutableStatus::Unavailable => {
            wire::AgentAvailabilityStatus::Unavailable
        }
    }
}
fn empty(value: &Value) -> Result<(), wire::ErrorBody> {
    let map = value.as_object().ok_or_else(invalid)?;
    if map.is_empty() {
        Ok(())
    } else {
        Err(invalid())
    }
}
fn validate_reserved(operation: wire::Operation, value: &Value) -> Result<(), wire::ErrorBody> {
    use wire::Operation as O;
    match operation {
        O::SessionCreate => {
            let params: wire::SessionCreateParams = parameters(value)?;
            let definition_matches = match params.launch_kind {
                wire::SessionLaunchKind::DefaultShell => params.definition_id.is_none(),
                wire::SessionLaunchKind::Definition => params.definition_id.is_some(),
            };
            if !definition_matches
                || !(1..=1000).contains(&params.rows)
                || !(1..=1000).contains(&params.columns)
                || params
                    .working_directory
                    .as_ref()
                    .is_some_and(|path| path.decode().is_err())
            {
                return Err(invalid());
            }
        }
        O::SessionAttach => {
            let _: wire::SessionAttachParams = parameters(value)?;
        }
        O::SessionReadDisplay => {
            let params: wire::SessionReadDisplayParams = parameters(value)?;
            if params.rows == 0
                || params.columns == 0
                || usize::from(params.rows)
                    .checked_mul(usize::from(params.columns))
                    .is_none_or(|cells| cells > wire::MAX_DISPLAY_CELLS)
            {
                return Err(resource());
            }
        }
        O::SessionReadOutput => {
            let _: wire::SessionReadOutputParams = parameters(value)?;
        }
        O::SessionInput => {
            let _: wire::SessionInputParams = parameters(value)?;
        }
        O::SessionResize => {
            let params: wire::SessionResizeParams = parameters(value)?;
            if !(1..=1000).contains(&params.rows) || !(1..=1000).contains(&params.columns) {
                return Err(invalid());
            }
        }
        O::SessionTerminate => {
            let _: wire::SessionTerminateParams = parameters(value)?;
        }
        O::WorktreeCreate => {
            let params: wire::WorktreeCreateParams = parameters(value)?;
            if params.payload_version != 1
                || params.base_ref.is_empty()
                || params.branch_name.is_empty()
                || params.destination_leaf.is_empty()
                || params
                    .parent
                    .as_ref()
                    .is_some_and(|path| path.decode().is_err())
            {
                return Err(invalid());
            }
        }
        O::WorktreeInspectRepository => {
            let _: wire::WorktreeInspectParams = parameters(value)?;
        }
        O::WorktreeGetOperation => {
            let _: wire::WorktreeOperationParams = parameters(value)?;
        }
        O::WorktreeReconcile => {
            let _: wire::WorktreeReconcileParams = parameters(value)?;
        }
        O::WorktreeSelect => {
            let _: wire::WorktreeSelectParams = parameters(value)?;
        }
        O::BackupCreate => {
            let _: wire::BackupCreateParams = parameters(value)?;
        }
        O::WorktreeList => {
            let params: wire::WorktreeListParams = parameters(value)?;
            if params.limit == 0 || params.limit > wire::MAX_PAGE_SIZE {
                return Err(invalid_field(wire::ErrorField::PageLimit));
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}
fn parse_decimal_field(value: &Value, key: &str, allow_zero: bool) -> Result<u64, wire::ErrorBody> {
    let text = value
        .as_object()
        .and_then(|x| x.get(key))
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    decimal(text, allow_zero)
}
fn decimal(text: &str, allow_zero: bool) -> Result<u64, wire::ErrorBody> {
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|x| x.is_ascii_digit())
    {
        return Err(invalid());
    }
    let value = text.parse().map_err(|_| invalid())?;
    if value == 0 && !allow_zero {
        Err(invalid())
    } else {
        Ok(value)
    }
}
fn parse_id<T: FromStr>(value: &Value, key: &str) -> Result<T, wire::ErrorBody> {
    value
        .as_object()
        .and_then(|x| x.get(key))
        .and_then(Value::as_str)
        .ok_or_else(invalid)?
        .parse()
        .map_err(|_| invalid())
}
fn invalid() -> wire::ErrorBody {
    wire::ErrorBody::not_applied(wire::ErrorCode::InvalidParams, wire::Recovery::None)
}
fn invalid_field(field: wire::ErrorField) -> wire::ErrorBody {
    let mut e = invalid();
    e.field = Some(field);
    e
}
fn unavailable() -> wire::ErrorBody {
    wire::ErrorBody::not_applied(wire::ErrorCode::OperationUnavailable, wire::Recovery::None)
}
fn resource() -> wire::ErrorBody {
    wire::ErrorBody::not_applied(wire::ErrorCode::ResourceLimit, wire::Recovery::None)
}
fn map_backup_error(error: RuntimeError) -> wire::ErrorBody {
    use wire::{ErrorCode as C, Recovery as R};
    match error {
        RuntimeError::InvalidLocation | RuntimeError::InvalidWorkspace => {
            wire::ErrorBody::not_applied(C::InvalidParams, R::None)
        }
        RuntimeError::AccessDenied => wire::ErrorBody::not_applied(C::Unauthorized, R::None),
        RuntimeError::Busy | RuntimeError::Timeout => {
            wire::ErrorBody::not_applied(C::StorageBusy, R::None)
        }
        RuntimeError::RecoveryRequired => {
            wire::ErrorBody::not_applied(C::IntegrityError, R::InspectState)
        }
        RuntimeError::Storage => wire::ErrorBody::not_applied(C::StorageError, R::InspectState),
        RuntimeError::WorkspaceNotInitialized
        | RuntimeError::Transport
        | RuntimeError::Protocol
        | RuntimeError::Spawn => unavailable(),
    }
}
fn map_supervisor(error: supervisor::SupervisorError) -> wire::ErrorBody {
    match error {
        supervisor::SupervisorError::Reference => map_domain_error(domain::Error::Reference),
        supervisor::SupervisorError::Conflict => map_domain_error(domain::Error::Conflict),
        supervisor::SupervisorError::ResourceLimit => resource(),
        supervisor::SupervisorError::Final => map_domain_error(domain::Error::State),
        supervisor::SupervisorError::Io => unavailable(),
        supervisor::SupervisorError::ResnapshotRequired(_) => resource(),
    }
}
fn map_domain_error(error: domain::Error) -> wire::ErrorBody {
    use domain::Error as D;
    use wire::{ErrorCode as C, Recovery as R};
    let (code, recovery, effect) = match error {
        D::Validation(_) => (C::InvalidParams, R::None, wire::ErrorEffect::NotApplied),
        D::State => (C::InvalidState, R::Refresh, wire::ErrorEffect::NotApplied),
        D::Unauthorized => (C::Unauthorized, R::None, wire::ErrorEffect::NotApplied),
        D::Reference => (
            C::InvalidReference,
            R::Refresh,
            wire::ErrorEffect::NotApplied,
        ),
        D::Conflict => (C::Conflict, R::Refresh, wire::ErrorEffect::NotApplied),
        D::Time => (C::InvalidTime, R::Refresh, wire::ErrorEffect::NotApplied),
        D::StorageBusy => (C::StorageBusy, R::None, wire::ErrorEffect::NotApplied),
        D::ReadOnly => (C::ReadOnly, R::None, wire::ErrorEffect::NotApplied),
        D::Integrity => (
            C::IntegrityError,
            R::InspectState,
            wire::ErrorEffect::NotApplied,
        ),
        D::InvalidCursor => (C::InvalidCursor, R::Refresh, wire::ErrorEffect::NotApplied),
        D::ResnapshotRequired => (
            C::ResnapshotRequired,
            R::Resnapshot,
            wire::ErrorEffect::NotApplied,
        ),
        D::Uncertain => (
            C::ResultUnknown,
            R::InspectState,
            wire::ErrorEffect::Unknown,
        ),
        D::Version => (
            C::UnsupportedVersion,
            R::UseMatchingVersion,
            wire::ErrorEffect::NotApplied,
        ),
        D::Storage | D::Migration | D::Unavailable => {
            (C::StorageError, R::InspectState, wire::ErrorEffect::Unknown)
        }
    };
    wire::ErrorBody {
        code,
        field: None,
        effect,
        recovery,
        message: None,
    }
}

fn receipt(outcome: &relayterm_application::Outcome) -> Value {
    let events = &outcome.committed.events;
    let ids = events
        .iter()
        .map(|event| entity_uuid(event.record().entity_id).to_string())
        .collect::<Vec<_>>();
    json!({"entity_ids":ids,"revision":outcome.committed.snapshot.revision().to_string(),"changed":!events.is_empty(),"first_sequence":events.first().map(|x|x.record().sequence.to_string()),"last_sequence":events.last().map(|x|x.record().sequence.to_string()),"notification_delivered":outcome.notification_delivered})
}
fn worktree_operation_dto(
    intent: &domain::WorktreeIntent,
    state: &domain::WorkspaceState,
) -> Value {
    let record = intent.record();
    let selected = state
        .task(record.task_id)
        .ok()
        .is_some_and(|task| task.record().worktree_id == Some(record.worktree_id));
    json!({"operation_id":record.id.to_string(),"worktree_id":record.worktree_id.to_string(),"task_id":record.task_id.to_string(),"phase":record.phase,"reason":record.reason,"selected":selected})
}
fn worktree_dto(worktree: &domain::Worktree) -> Value {
    let record = worktree.record();
    let path = relayterm_ipc::encode_native_path(&record.checkout_path)
        .ok()
        .and_then(|value| serde_json::to_value(value).ok())
        .unwrap_or(Value::Null);
    json!({"id":record.id.to_string(),"task_id":record.task_id.to_string(),"operation_id":record.operation_id.to_string(),"checkout_path":path,"checkout_display":record.checkout_path.to_string_lossy(),"branch_ref":record.branch_ref,"initial_base_commit":record.initial_base_commit,"health":record.health})
}
fn git_status(kind: relayterm_git::ErrorKind) -> &'static str {
    match kind {
        relayterm_git::ErrorKind::GitMissing => "git_missing",
        relayterm_git::ErrorKind::NotRepository => "not_repository",
        relayterm_git::ErrorKind::UnsupportedRoot => "unsupported_root",
        relayterm_git::ErrorKind::InvalidReference => "invalid_reference",
        relayterm_git::ErrorKind::BranchConflict => "branch_conflict",
        relayterm_git::ErrorKind::DestinationConflict => "destination_conflict",
        relayterm_git::ErrorKind::UnsupportedCheckoutFilter => "unsupported_checkout_filter",
        relayterm_git::ErrorKind::OutputLimit => "resource_limit",
        relayterm_git::ErrorKind::Timeout => "needs_attention",
        relayterm_git::ErrorKind::CommandFailed
        | relayterm_git::ErrorKind::MalformedOutput
        | relayterm_git::ErrorKind::Io => "unavailable",
    }
}
fn git_reason(kind: relayterm_git::ErrorKind) -> domain::WorktreeReason {
    match kind {
        relayterm_git::ErrorKind::GitMissing => domain::WorktreeReason::GitMissing,
        relayterm_git::ErrorKind::NotRepository => domain::WorktreeReason::NotRepository,
        relayterm_git::ErrorKind::UnsupportedRoot => domain::WorktreeReason::UnsupportedRoot,
        relayterm_git::ErrorKind::InvalidReference => domain::WorktreeReason::InvalidReference,
        relayterm_git::ErrorKind::BranchConflict => domain::WorktreeReason::BranchConflict,
        relayterm_git::ErrorKind::DestinationConflict => {
            domain::WorktreeReason::DestinationConflict
        }
        relayterm_git::ErrorKind::UnsupportedCheckoutFilter => {
            domain::WorktreeReason::UnsupportedCheckoutFilter
        }
        relayterm_git::ErrorKind::OutputLimit
        | relayterm_git::ErrorKind::Timeout
        | relayterm_git::ErrorKind::CommandFailed
        | relayterm_git::ErrorKind::MalformedOutput
        | relayterm_git::ErrorKind::Io => domain::WorktreeReason::OutcomeUncertain,
    }
}
fn map_git_error(error: relayterm_git::Error) -> wire::ErrorBody {
    match error.kind() {
        relayterm_git::ErrorKind::OutputLimit => resource(),
        relayterm_git::ErrorKind::GitMissing => unavailable(),
        relayterm_git::ErrorKind::Timeout | relayterm_git::ErrorKind::CommandFailed => {
            wire::ErrorBody {
                code: wire::ErrorCode::ResultUnknown,
                field: None,
                effect: wire::ErrorEffect::Unknown,
                recovery: wire::Recovery::InspectState,
                message: None,
            }
        }
        relayterm_git::ErrorKind::BranchConflict
        | relayterm_git::ErrorKind::DestinationConflict => {
            map_domain_error(domain::Error::Conflict)
        }
        _ => invalid(),
    }
}
fn entity_uuid(id: domain::EntityId) -> uuid::Uuid {
    match id {
        domain::EntityId::Workspace(v) => v.as_uuid(),
        domain::EntityId::Definition(v) => v.as_uuid(),
        domain::EntityId::Task(v) => v.as_uuid(),
        domain::EntityId::Instance(v) => v.as_uuid(),
        domain::EntityId::Claim(v) => v.as_uuid(),
        domain::EntityId::Progress(v) => v.as_uuid(),
        domain::EntityId::Handover(v) => v.as_uuid(),
        domain::EntityId::Worktree(v) => v.as_uuid(),
        domain::EntityId::WorktreeOperation(v) => v.as_uuid(),
        domain::EntityId::ApprovedRoot(v) => v.as_uuid(),
    }
}
fn timestamp(value: domain::Timestamp) -> Value {
    let time = value.value();
    json!({"unix_seconds":time.unix_timestamp().to_string(),"nanoseconds":time.nanosecond()})
}
fn actor(value: domain::Actor) -> Value {
    match value {
        domain::Actor::LocalUser => json!({"kind":"local_user"}),
        domain::Actor::System => json!({"kind":"system"}),
        domain::Actor::Instance(id) => json!({"kind":"instance","instance_id":id.to_string()}),
    }
}
fn display_snapshot(snapshot: relayterm_terminal::TerminalSnapshot) -> wire::TerminalViewportDto {
    let cells = snapshot
        .cells
        .into_iter()
        .map(|cell| {
            let mut flags = 0_u8;
            for (enabled, flag) in [
                (cell.bold, wire::TERMINAL_CELL_BOLD),
                (cell.dim, wire::TERMINAL_CELL_DIM),
                (cell.italic, wire::TERMINAL_CELL_ITALIC),
                (cell.underline, wire::TERMINAL_CELL_UNDERLINE),
                (cell.inverse, wire::TERMINAL_CELL_INVERSE),
                (cell.wide, wire::TERMINAL_CELL_WIDE),
                (
                    cell.wide_continuation,
                    wire::TERMINAL_CELL_WIDE_CONTINUATION,
                ),
            ] {
                if enabled {
                    flags |= flag;
                }
            }
            wire::TerminalViewportCellDto(
                cell.contents,
                display_color(cell.foreground),
                display_color(cell.background),
                flags,
            )
        })
        .collect();
    wire::TerminalViewportDto {
        schema_version: snapshot.schema_version,
        revision: snapshot.revision,
        raw_offset: snapshot.raw_offset,
        retained_from_offset: snapshot.retained_from_offset,
        rows: snapshot.rows,
        columns: snapshot.columns,
        cursor_row: snapshot.cursor_row,
        cursor_column: snapshot.cursor_column,
        cursor_hidden: snapshot.cursor_hidden,
        alternate_screen: snapshot.alternate_screen,
        application_cursor: snapshot.application_cursor,
        application_keypad: snapshot.application_keypad,
        bracketed_paste: snapshot.bracketed_paste,
        cells,
    }
}

fn display_color(color: relayterm_terminal::Color) -> wire::TerminalColorDto {
    match color {
        relayterm_terminal::Color::Default => wire::TerminalColorDto::Default,
        relayterm_terminal::Color::Indexed(value) => wire::TerminalColorDto::Indexed(value),
        relayterm_terminal::Color::Rgb(value) => wire::TerminalColorDto::Rgb(value),
    }
}
fn event_dto(event: &domain::WorkspaceEvent) -> Value {
    let r = event.record();
    json!({"sequence":r.sequence.to_string(),"event_id":r.event_id.to_string(),"event_type":r.event_type,"entity_id":entity_uuid(r.entity_id).to_string(),"timestamp":timestamp(r.timestamp),"actor":actor(r.actor),"payload_version":r.payload_version,"payload":r.payload})
}
fn workspace_dto(value: &domain::Workspace) -> Value {
    let r = value.record();
    let path = relayterm_ipc::encode_native_path(&r.project_root)
        .ok()
        .and_then(|x| serde_json::to_value(x).ok())
        .unwrap_or(Value::Null);
    json!({"id":r.id.to_string(),"display_name":r.display_name,"project_root":path,"created_at":timestamp(r.created_at),"updated_at":timestamp(r.updated_at),"schema_version":r.schema_version})
}
fn definition_dto(value: &domain::AgentDefinition) -> Value {
    let r = value.record();
    json!({"id":r.id.to_string(),"workspace_id":r.workspace_id.to_string(),"display_name":r.display_name,"command":r.command,"arguments":r.arguments,"environment_allowlist":r.environment_allowlist,"capabilities":r.capabilities,"enabled":r.enabled})
}
fn task_content(value: &domain::TaskContent) -> Value {
    json!({"title":value.title,"description":value.description,"priority":value.priority,"scope_paths":value.scope_paths,"acceptance_notes":value.acceptance_notes,"dependency_ids":value.dependency_ids.iter().map(ToString::to_string).collect::<Vec<_>>()})
}
fn task_dto(value: &domain::Task) -> Value {
    let r = value.record();
    json!({"id":r.id.to_string(),"workspace_id":r.workspace_id.to_string(),"content":task_content(&r.content),"status":r.status,"claimed_by_instance_id":r.claimed_by_instance_id.map(|x|x.to_string()),"worktree_id":r.worktree_id.map(|x|x.to_string()),"created_at":timestamp(r.created_at),"updated_at":timestamp(r.updated_at)})
}
fn instance_dto(value: &domain::AgentInstance) -> Value {
    let r = value.record();
    let path = relayterm_ipc::encode_native_path(&r.working_directory)
        .ok()
        .and_then(|x| serde_json::to_value(x).ok())
        .unwrap_or(Value::Null);
    let launch_definition = r.launch_definition.as_ref().map(|snapshot| {
        json!({
            "definition_id":snapshot.definition_id.to_string(),
            "display_name":snapshot.display_name,
            "command":snapshot.command,
            "arguments":snapshot.arguments,
            "environment_allowlist":snapshot.environment_allowlist,
            "capabilities":snapshot.capabilities,
            "enabled":snapshot.enabled
        })
    });
    json!({"id":r.id.to_string(),"session_id":r.session_id.to_string(),"workspace_id":r.workspace_id.to_string(),"agent_definition_id":r.agent_definition_id.map(|x|x.to_string()),"task_id":r.task_id.map(|x|x.to_string()),"worktree_id":r.worktree_id.map(|x|x.to_string()),"launch_definition":launch_definition,"working_directory":path,"status":r.status,"started_at":timestamp(r.started_at),"last_observed_at":timestamp(r.last_observed_at),"ended_at":r.ended_at.map(timestamp),"exit_code":r.exit_code,"terminal_size":{"rows":r.terminal_size.rows(),"columns":r.terminal_size.columns()}})
}
fn claim_dto(value: &domain::Claim) -> Value {
    let r = value.record();
    json!({"id":r.id.to_string(),"workspace_id":r.workspace_id.to_string(),"task_id":r.task_id.to_string(),"instance_id":r.instance_id.to_string(),"requested_by":actor(r.requested_by),"opened_at":timestamp(r.opened_at),"closed_at":r.closed_at.map(timestamp),"close_reason":r.close_reason,"closed_by":r.closed_by.map(actor)})
}
fn progress_dto(value: &domain::ProgressEntry) -> Value {
    let r = value.record();
    json!({"id":r.id.to_string(),"workspace_id":r.workspace_id.to_string(),"task_id":r.task_id.to_string(),"agent_instance_id":r.agent_instance_id.map(|x|x.to_string()),"summary":r.summary,"verification":r.verification,"created_at":timestamp(r.created_at)})
}
fn handover_dto(value: &domain::Handover) -> Value {
    let r = value.record();
    json!({"id":r.id.to_string(),"workspace_id":r.workspace_id.to_string(),"task_id":r.task_id.to_string(),"from_agent_instance_id":r.from_agent_instance_id.map(|x|x.to_string()),"content":{"summary":r.content.summary,"decisions":r.content.decisions,"changed_paths":r.content.changed_paths,"verification_performed":r.content.verification_performed,"open_questions":r.content.open_questions,"recommended_next_action":r.content.recommended_next_action},"created_at":timestamp(r.created_at)})
}
fn entity_sort_key(value: &Value) -> String {
    value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_admission_is_immediate_and_exclusive() {
        let admission = Arc::new(Semaphore::new(1));
        let Ok(first) = admit_backup(&admission) else {
            panic!("the first backup must be admitted");
        };
        let Err(error) = admit_backup(&admission) else {
            panic!("a concurrent backup must be rejected");
        };
        assert_eq!(error.code, wire::ErrorCode::OperationUnavailable);
        drop(first);
        assert!(admit_backup(&admission).is_ok());
    }

    #[tokio::test]
    async fn event_queue_enforces_item_and_byte_limits_and_releases_permits() {
        let (sender, mut receiver) = mpsc::channel(1);
        let budget = Arc::new(Semaphore::new(5));
        assert!(try_queue_event(&sender, &budget, vec![1; 3], 3));
        assert!(!try_queue_event(&sender, &budget, vec![2], 1));
        assert!(!try_queue_event(&sender, &budget, vec![3; 6], 6));
        assert_eq!(budget.available_permits(), 2);
        drop(receiver.recv().await.unwrap());
        assert_eq!(budget.available_permits(), 5);
    }

    #[tokio::test]
    async fn reset_control_discards_queued_subscription_events_before_writing() {
        let (server, mut client) = tokio::io::duplex(4096);
        let (control_tx, control_rx) = mpsc::channel(2);
        let (event_tx, event_rx) = mpsc::channel(2);
        let budget = Arc::new(Semaphore::new(64));
        assert!(try_queue_event(&event_tx, &budget, vec![99], 1));
        control_tx
            .send(WriterControl::ResetSubscription(vec![42]))
            .await
            .unwrap();
        let writer = tokio::spawn(writer_loop(
            server,
            control_rx,
            event_rx,
            ServerFaults::default(),
        ));
        let frame = read_frame(&mut client, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(frame.payload, vec![42]);
        assert_eq!(budget.available_permits(), 64);
        writer.abort();
    }
}
