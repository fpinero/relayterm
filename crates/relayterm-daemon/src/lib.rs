//! Workspace protocol server, runtime composition, recovery, and lifecycle control.

mod diagnostics;
mod runtime;
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

use relayterm_application::{
    Clock, DurableReadStore, EventNotifier, EventPageRequest, IdGenerator, Request, Service, Store,
    TaskHistoryItem, TaskHistoryPageRequest,
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
    fmt,
    str::FromStr,
    sync::{
        Arc,
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
static CONNECTION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

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

#[derive(Clone, Copy)]
struct Subscription {
    after_sequence: u64,
    id: wire::SubscriptionId,
    request_id: wire::DecimalU64,
    caught_up_watermark: Option<u64>,
}

enum WriterControl {
    Frame(Vec<u8>),
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
    mutation_paused: Arc<Notify>,
    release_mutation_response: Arc<Notify>,
    event_wakeup_count: Arc<AtomicU64>,
    event_wakeup_observed: Arc<Notify>,
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
    pub fn fault_injector(&self) -> ServerFaults {
        self.faults.clone()
    }
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) -> Result<(), ServerError> {
        let shared = Arc::new(self);
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {biased;
                changed=shutdown.changed()=>{if changed.is_err()||*shutdown.borrow(){break}}
                joined=connections.join_next(),if !connections.is_empty()=>{let _=joined;}
                accepted=shared.listener.accept()=>{let Ok(stream)=accepted else{continue};let Ok(permit)=shared.connections.clone().try_acquire_owned()else{drop(stream);continue};let server=shared.clone();let child_shutdown=shutdown.clone();connections.spawn(async move{let _permit=permit;let _=server.connection(stream,child_shutdown).await;});}
            }
        }
        while connections.join_next().await.is_some() {}
        if let Some(control) = &shared.lifecycle {
            control.state.store(2, Ordering::Release);
        }
        Ok(())
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
        let hello = wire::HelloResult::new(connection_id, self.wire_workspace_id, operations);
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
        let writer_task = tokio::spawn(async move {
            if writer_loop(writer, control_rx, event_rx).await.is_err() {
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
                    let _operation=self.operations.clone().acquire_owned().await.map_err(|_|ServerError::ResourceLimit)?;
                    let response=match self.dispatch(&request).await{Ok(value)=>wire::ResponseEnvelope::success(request.request_id,self.wire_workspace_id,value),Err(error)=>wire::ResponseEnvelope::failure(request.request_id,self.wire_workspace_id,error)};
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

    async fn dispatch(&self, request: &wire::RequestEnvelope) -> Result<Value, wire::ErrorBody> {
        use wire::Operation as O;
        if request.operation == O::Unknown {
            return Err(wire::ErrorBody::not_applied(
                wire::ErrorCode::UnknownOperation,
                wire::Recovery::None,
            ));
        }
        if request.operation.is_reserved() {
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
                let snapshot = self
                    .reads
                    .consistent_snapshot(self.workspace_id)
                    .await
                    .map_err(map_domain_error)?;
                let state = snapshot.snapshot.state().map_err(map_domain_error)?;
                serde_json::to_value(wire::DaemonStatusResult {
                    workspace_id: wire::WorkspaceId::from_uuid(self.workspace_id.as_uuid()),
                    generation: control.generation().to_owned(),
                    lifecycle: control.lifecycle().to_owned(),
                    protocol_version: wire::PROTOCOL_VERSION,
                    schema_version: state.workspace().record().schema_version,
                    revision: wire::DecimalU64::new(snapshot.snapshot.revision())
                        .map_err(|_| invalid())?,
                    definitions: state.definitions().len(),
                    tasks: state.tasks().len(),
                    instances: state.instances().len(),
                })
                .map_err(|_| invalid())
            }
            O::DaemonShutdown => {
                let params: wire::DaemonShutdownParams = parameters(&request.params)?;
                let control = self.lifecycle.as_ref().ok_or_else(unavailable)?;
                control.begin_shutdown(&params.generation)?;
                serde_json::to_value(wire::DaemonShutdownResult {
                    generation: control.generation().to_owned(),
                    lifecycle: "draining".to_owned(),
                })
                .map_err(|_| invalid())
            }
            O::WorkspaceGetSnapshot => self.snapshot_page(&request.params, None).await,
            O::AgentListDefinitions => {
                self.snapshot_page(&request.params, Some(Collection::Definitions))
                    .await
            }
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
                    .consistent_snapshot(self.workspace_id)
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
                let snapshot = self
                    .reads
                    .consistent_snapshot(self.workspace_id)
                    .await
                    .map_err(map_domain_error)?;
                let item = snapshot
                    .snapshot
                    .state()
                    .map_err(map_domain_error)?
                    .handovers()
                    .iter()
                    .find(|x| x.record().id == id)
                    .ok_or_else(|| map_domain_error(domain::Error::Reference))?;
                Ok(handover_dto(item))
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
            O::ProtocolHello | O::EventSubscribe | O::EventUnsubscribe => Err(invalid()),
            _ => Err(unavailable()),
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
        let snap = self
            .reads
            .consistent_snapshot(self.workspace_id)
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
            .import_definitions(self.workspace_id, expected, vec![definition])
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
                .consistent_snapshot(self.workspace_id)
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
) -> Result<(), ServerError> {
    loop {
        tokio::select! {biased;
            control=controls.recv()=>{
                match control {
                    Some(WriterControl::Frame(bytes))=>write_frame(&mut writer,wire::FrameKind::Json,&bytes).await.map_err(|_|ServerError::Transport)?,
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
                    Some(event)=>write_frame(&mut writer,wire::FrameKind::Json,&event.bytes).await.map_err(|_|ServerError::Transport)?,
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
            if params.base_ref.is_empty()
                || params.branch_name.is_empty()
                || params.relative_destination.is_empty()
            {
                return Err(invalid());
            }
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
fn entity_uuid(id: domain::EntityId) -> uuid::Uuid {
    match id {
        domain::EntityId::Workspace(v) => v.as_uuid(),
        domain::EntityId::Definition(v) => v.as_uuid(),
        domain::EntityId::Task(v) => v.as_uuid(),
        domain::EntityId::Instance(v) => v.as_uuid(),
        domain::EntityId::Claim(v) => v.as_uuid(),
        domain::EntityId::Progress(v) => v.as_uuid(),
        domain::EntityId::Handover(v) => v.as_uuid(),
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
    json!({"id":r.id.to_string(),"session_id":r.session_id.to_string(),"workspace_id":r.workspace_id.to_string(),"agent_definition_id":r.agent_definition_id.map(|x|x.to_string()),"task_id":r.task_id.map(|x|x.to_string()),"working_directory":path,"status":r.status,"started_at":timestamp(r.started_at),"last_observed_at":timestamp(r.last_observed_at),"ended_at":r.ended_at.map(timestamp),"exit_code":r.exit_code,"terminal_size":{"rows":r.terminal_size.rows(),"columns":r.terminal_size.columns()}})
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
        let writer = tokio::spawn(writer_loop(server, control_rx, event_rx));
        let frame = read_frame(&mut client, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(frame.payload, vec![42]);
        assert_eq!(budget.available_permits(), 64);
        writer.abort();
    }
}
