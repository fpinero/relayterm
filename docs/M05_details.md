# M05: Daemon lifecycle and administrative CLI

## 1. Delivery and execution contract

This is the implementation contract for M05 in [TODO.md](../TODO.md). It refines the [technical specification](../MVP_TECHNICAL_SPEC.md), [project vision](../PROJECT_VISION.md), [architecture decisions](architecture/README.md), and the completed [M04 plan](M04_details.md). Read repository instructions and these sources before implementation. Resolve contradictions explicitly in the affected specification or ADR; never quietly weaken acceptance requirements.

The planning delivery contains documentation only. Publishing this document does not complete M05.01 or any other implementation task. Preserve M05.01 through M05.07 in the pending queue until their individual verification succeeds. Implementation must start on a new feature branch from updated `main`, after checking Git status and preserving existing changes.

The outcome is one independent daemon per initialized workspace, operated through the single `rt` executable. Durable state belongs to daemon-side application services and SQLite. Administrative clients use IPC and survive neither as owners nor prerequisites of that daemon. Linux, macOS, and Windows require native execution evidence.

### 1.1 Implementation workflow

1. Read `AGENTS.md`, applicable local instructions, `PROJECT_VISION.md`, `MVP_TECHNICAL_SPEC.md`, `README.md`, `TODO.md`, this document, and recent `avances.md` entries.
2. Inspect the actual code and current CI. Earlier logbook entries are evidence of previous checks, not a substitute for inspecting lifecycle behavior.
3. Add the atomic child tasks from section 10 to `TODO.md` before coding. Retain the original M05 IDs as their prefixes. Keep the plan link.
4. Implement in dependency order. Run narrow tests before broad checks. Keep commits coherent and English documentation and comments consistent with repository policy.
5. Remove only verified tasks and append their exact evidence to `avances.md` in the same change. Never rewrite existing logbook entries.
6. Record unresolved native evidence, access failures, and technical blockers as pending tasks. Cross-compilation cannot close a runtime gate.
7. Follow the actual implementation request for publishing authorization. This document itself does not authorize external actions, force-pushes, branch deletion, or bypassing checks.

### 1.2 Explicit exclusions

Do not implement production PTYs, real agent launches, terminal emulation, TUI flows, provider templates, worktrees, process reattachment, a scheduler, TCP, a per-user persistent manager, hosted dependencies, telemetry, public export, or transcript capture. Do not add a production flag, environment variable, or hidden RPC that fabricates a running instance. Existing reserved session and worktree operations remain unavailable. Ordinary invocation without a subcommand remains the honest TUI placeholder until M08; help and version remain side-effect free.

M05 establishes daemon independence and restart behavior. M06 still owns the complete durable claim/progress/handover/restart vertical slice. M07 establishes real child and terminal survival. Do not claim either later gate from an empty production supervisor.

## 2. Inspected starting point and integration risks

The planning baseline is the M04 follow-up merged into `main` at `52cee834133fc3b3502cadadaacf52bb18d3f509`. Recheck the starting revision if implementation begins later.

| Existing area | Reuse and required integration |
| --- | --- |
| `relayterm-domain` and `relayterm-application` | Validated entities, claims, loss observations, deterministic services, typed errors, revisioned transactions, and durable event contracts already exist. Reuse their invariants. |
| `relayterm-persistence-sqlite` | Registry, initialization recovery, private workspace databases, migrations, and transactional store already exist. Promote the adapter to a production daemon dependency. |
| `relayterm-platform` | Native root identity, private locations, OS locks, and filesystem protections already exist. Add narrow process-detachment support here. |
| `relayterm-config` | Bounded TOML parsing, neutral definitions, and explicit revision-based import already exist. Compose them on the daemon side. |
| `relayterm-protocol` and `relayterm-ipc` | Version 1 frames, typed DTOs, current-user transport, endpoint ownership, and existing operations are implemented. Preserve limits and security. |
| `relayterm-client` | Handshake, snapshots, events, reconnect, and mutation uncertainty exist. Reuse this library for administrative commands. |
| `relayterm-daemon/src/lib.rs` | `run()` is a placeholder. `WorkspaceServer` is reusable but not a complete detached runtime. Its current accept loop spawns connection tasks without joining them on shutdown. |
| `relayterm-cli/src/main.rs` | Clap exposes only the unavailable daemon placeholder; default invocation calls the unavailable TUI. Administrative dispatch remains unimplemented. |
| `crates/relayterm-cli/tests/architecture.rs` | Dependency allowlists and forbidden core dependency closure are enforced. Update only justified adapter/composition edges. |

The following are mandatory integration audits, not assertions that each suspected defect has been reproduced:

- Inspect ownership of the listener, workspace lock, connection tasks, writers, and in-flight service calls. Returning from the accept loop must not be mistaken for shutdown completion.
- Check whether selecting notifications or polling against a partially read frame cancels a read and discards framing progress. Reproduce fragmentation across a wakeup; use a persistent decoder or dedicated bounded reader if needed.
- Check client request-ID allocation relative to actual serialization, concurrent reconnect, and an event reader holding the connection lock while another request waits. Demonstrate progress with simultaneous calls and event delivery, or correct ownership without serializing unrelated connections.
- Existing fault-injection helpers must not become production CLI or RPC controls. Keep new lifecycle fault controls in test support; isolate any touched existing helpers appropriately.
- Registry `open` currently creates storage when missing. Status and lookup need a genuine non-creating path; calling the existing constructor and labeling it read-only is insufficient.
- Workspace initialization and endpoint binding use different locks. Document their acquisition order and never run restart reconciliation before exclusive daemon ownership.

## 3. Architecture and bootstrap decision

### 3.1 Responsibility boundaries

| Component | Responsibility |
| --- | --- |
| CLI | Parse arguments and bounded input files, render authorized results, invoke bootstrap client or workspace client, map errors to exit codes. No SQL, migrations, or domain mutation. |
| Shared client | Bootstrap IPC exchange and ordinary authenticated workspace RPC, deadlines, response validation, reconnect and explicit uncertainty. |
| Daemon bootstrap component | Resolve native roots and registry identities, initialize only on explicit request, start or discover the workspace daemon. Runs through an internal mode of the same executable. |
| Workspace runtime | Acquire exclusive ownership, open storage, reconcile before readiness, dispatch operations, coordinate shutdown, and own diagnostics. |
| Platform adapter | Private locations and process creation/detachment using safe platform APIs. No business state transitions. |
| Application | Existing use cases plus narrowly necessary configuration/recovery service operations. No Tokio, process handles, SQLx, or CLI dependency. |
| Protocol | Pure DTOs for bootstrap and additive lifecycle/configuration operations. No process, filesystem, or persistence dependency. |

Allow CLI dependencies on client, protocol, and platform only where needed, in addition to its existing daemon entrypoint and TUI composition. Allow daemon dependencies on persistence, platform, and config. Keep client independent of persistence and domain. Do not admit Tokio or any concrete adapter into the domain/application/protocol dependency closure. If shared configuration input DTOs are needed, place wire representations in protocol and perform config/domain conversion on the daemon side.

### 3.2 Bootstrap before a workspace endpoint exists

A workspace-scoped handshake requires an ID, while first initialization creates that ID. Resolve this explicitly with a finite bootstrap subprocess, not client-owned SQLite and not a permanent per-user daemon.

The public CLI launches the same executable in an internal bootstrap mode using an absolute executable path and argument arrays. It exchanges exactly one versioned, bounded request and response over inherited anonymous stdin/stdout pipes. This is a distinct bootstrap IPC contract, not an unauthenticated substitute for normal workspace RPC. Anonymous pipes connect only the created child and parent; there is no public bootstrap socket or TCP listener.

The daemon-side helper performs registry lookup or explicit initialization. For a start request it launches the detached workspace runtime using the platform adapter, waits for validated readiness, returns a compact result, closes its storage handles and exits. The long-lived runtime is a different process with null standard streams and no inherited bootstrap pipes. A lookup-only helper never creates private directories, locks, databases, registry entries, or a runtime process. Implement non-creating registry reads for this path.

Use these bootstrap actions:

| Action | Meaning |
| --- | --- |
| `initialize` | Resolve an existing directory, reserve or reuse its identity through M03 initialization, then ensure its daemon is ready. Never create the project directory or initialize Git. |
| `open` | Resolve an already registered root and ensure its daemon is ready. Unknown roots return `workspace_not_initialized`. |
| `locate` | Resolve an existing registration and return routing metadata only. Never start a daemon or initialize missing storage. |

Bootstrap schema version is 1, independent of workspace IPC version and event payload versions. Use an explicit action enum, native-path wire encoding already provided by M04, optional validated display name for initialization, and effective private-location selection. Reject unknown fields/actions/versions and extra frames. Cap each bootstrap request/response at 64 KiB including framing. Send one structured error, never a panic, SQL error, path value, or raw subprocess stderr. Do not print logs on its response stream.

The response contains workspace ID, endpoint routing information, and `initialized`, `already_initialized`, `started`, or `already_running` facts as applicable. It is not a domain snapshot. The public client then connects through ordinary authenticated IPC and verifies workspace identity and protocol. Initialization is not silently rolled back if startup fails; return the registered ID when known and explain that retrying open is safe.

Protect internal entrypoints with the same validation as public input. Being hidden in help is not authorization. A manually invoked internal runtime must still prove registry membership, root identity, private location access, and exclusive ownership. It must not accept arbitrary database or socket paths that bypass derivation. Do not expose the internal grammar as a stable public command.

### 3.3 Singleton and startup sequence

1. Resolve explicit location override, environment override, or native defaults using M03 precedence. Freeze effective locations for all child processes; do not let a later current directory change select different storage.
2. Canonicalize the supplied project root using the existing native identity adapter. Do not trim, lowercase, or lossy-convert paths. Require an existing accessible directory. Git is irrelevant.
3. For `initialize`, use registry reservation and initialization recovery with the existing lock order. Close the initialized store handle before spawning the long-lived runtime. Never recreate a missing database belonging to a ready registration.
4. For `open` or `locate`, use non-creating lookup. Preserve the distinction between unknown workspace, inaccessible storage, incompatible schema, and recovery required.
5. Derive endpoint and lock names from the opaque workspace ID and existing private-location contract. No client deletes or replaces an endpoint.
6. Try authenticated hello/status on an existing endpoint. A live compatible runtime is reused. A live incompatible or inaccessible endpoint fails closed. A failed ping alone is not evidence of staleness.
7. If startup is necessary, launch a candidate runtime. It acquires the same per-workspace lifetime ownership used by the endpoint, before opening the workspace for recovery or accepting requests. Do not accidentally acquire a second conflicting copy of the same lock.
8. Only the lock owner may apply existing ownership-safe stale cleanup, open storage, and reconcile. Never hold the registry lock during readiness waiting, SQLite reconciliation, or IPC service.
9. Bind/listen and advertise readiness only after storage and reconciliation succeed. If binding is required to acquire ownership, retain the listener without dispatching public work until recovery completes.
10. Competing candidates that lose ownership close their own resources and exit without touching the winner's endpoint. Bootstrap callers connect to the winner within their startup deadline.
11. Readiness means a successful authenticated hello and `daemon.status` reporting `ready` for the intended workspace and runtime generation. A spawned PID, existing socket, open pipe, or registry row is insufficient.
12. Startup failure releases only resources owned by that candidate. Keep initialized durable state and safe diagnostics; never remove a live winner's endpoint during error cleanup.

Each runtime receives a fresh opaque generation ID. It is diagnostic lifecycle identity, not authorization and not a durable event sequence. PID metadata may aid diagnostics but cannot decide lock ownership, readiness, or shutdown target.

## 4. Detached lifetime and orderly shutdown

### 4.1 Platform process contract

Use the current executable resolved through the operating system, not a bare `rt` PATH lookup. Pass argument arrays without shell interpolation. Pass workspace IDs and effective locations using the validated internal protocol or bounded arguments; never put task prose, definition arguments, credentials, or raw configuration on a daemon command line.

Implement the narrow platform adapter with safe, MSRV-compatible APIs. Before choosing a new crate, inspect its actual detachment behavior, license, target support, and handle inheritance. Do not add unchecked `unsafe` or bypass repository lints. Record the selected API and native evidence in the relevant ADR during implementation.

- Unix: create a detached session/process arrangement independent of the launching terminal and process group. Null stdin/stdout/stderr, inherit no client descriptors or runtime locks, and reap the intermediate helper. Demonstrate terminal hangup and starter exit do not stop the runtime. A plain spawn without session separation is insufficient.
- Windows: create a process independent of the launching console using verified creation flags and handle inheritance. Null standard handles and prevent inherited IPC/lock handles. Inspect job-object behavior: do not claim survival of a killing job unless demonstrated. Do not request unsupported breakaway flags unconditionally. A native launcher-exit test and a console-lifetime test are required.
- Do not install systemd, launchd, or Windows services, elevate privileges, or auto-start at login. Host shutdown, user logout policies, administrator termination, and destruction of an enclosing execution container/job are not session reattachment guarantees.

Closing a normal CLI, starter, or terminal client while the host and user session remain available must not stop the daemon. Do not retry daemon creation indefinitely when detachment is unsupported; return a typed, actionable failure.

### 4.2 Lifecycle and deadlines

Runtime states are `starting`, `ready`, `draining`, and `stopped`. A failed startup is a process result, not a fake running session. `daemon.status` reports workspace ID, generation, lifecycle state, protocol version, schema version, and bounded counts. Never include configuration contents, raw paths, commands, or environment values.

Use monotonic deadlines for process/IPC waiting and the injected UTC clock for durable records. Initial bounds:

| Resource | Bound and behavior |
| --- | --- |
| Bootstrap request/response | 64 KiB each, one exchange |
| Startup readiness | 15 seconds default, configurable CLI timeout from 1 to 120 seconds |
| One status/connect attempt | At most 2 seconds and never beyond the remaining caller deadline |
| Readiness polling | Bounded backoff from 25 to 250 milliseconds, no busy loop |
| Shutdown drain target | 10 seconds before reporting delayed shutdown; never abort an accepted write to meet this target |
| Stop waiting | 15 seconds default, same 1 to 120 second CLI range |
| Future child termination grace | 5 seconds inside the supervisor policy, only operational in M07 |
| Existing IPC queues, frame sizes, pages | Preserve M04 values, including page default 50 and maximum 200 |

A startup timeout can leave a daemon that later becomes ready. Say so and direct the user to status; never kill a PID or delete a socket to enforce a client deadline. A stop timeout similarly means shutdown is pending or uncertain, not successful or rolled back.

### 4.3 Admission and transaction ownership

Implement one atomic admission boundary shared by shutdown and mutation dispatch. After entering `draining`, no new business mutation may begin. Previously admitted service operations remain daemon-owned even if their connection disappears. Registry/bootstrap helpers cannot bypass that boundary to edit a live workspace.

Track accepted operations separately from reader/writer connection tasks. Use explicit task ownership and joins. Aborting a connection reader, cancelling an event subscription, dropping a response receiver, or killing a CLI must not cancel an accepted transaction. A request not admitted before shutdown is rejected without mutation; a request admitted before the boundary commits or rolls back normally.

Shutdown sequence:

1. Authenticate and validate `daemon.shutdown`; atomically enter `draining`. Repeated shutdown is an idempotent lifecycle request for the same generation.
2. Return an acknowledgement containing generation and `draining`, not a claim that the process has exited. Stop accepting new connections after making bounded delivery of that acknowledgement possible.
3. Reject new mutations on existing connections. Permit bounded status responses during drain; close event subscriptions and idle readers without waiting forever for clients.
4. Join all admitted service operations. Preserve successful commits and event ordering even when response delivery fails. Do not retry a mutation automatically.
5. Invoke the supervisor shutdown policy, persist confirmed observations, and join supervisor tasks. M05's production implementation owns zero children and does not invent observations.
6. Drain or close bounded writer queues, close store/pool handles, join background tasks, and release listener/endpoint and lifetime lock last. Identity-safe cleanup must not remove a replacement runtime's endpoint.
7. Exit only when owned state resources are released. The stop client verifies the acknowledged generation has stopped; a new generation appearing during its wait must not be stopped by a repeated request.

If an admitted store operation exceeds the drain target, retain `draining`, report a bounded diagnostic, and continue awaiting completion. A user timeout must not trigger forced termination. Document this deliberate distinction between bounded client wait and guaranteed transaction ownership. Abrupt OS termination remains a crash path handled by SQLite and subsequent reconciliation, not orderly shutdown.

Route Unix termination signals and available Windows cooperative shutdown notifications to the same coordinator. Do not rely on terminal/TUI teardown. Do not use `process::exit` while accepted work or resources remain owned.

### 4.4 Supervision boundary for M07

Define only the lifecycle interface required now: enumerate owned runtime handles, request shutdown, await bounded termination observations, and report unresolved outcomes. Keep OS handles in the runtime adapter, outside the domain. Use the real empty implementation in M05 production. Test implementations may produce synthetic observations in integration support.

A termination request is not confirmation. Future confirmed exits preserve actual exit details; unconfirmed outcomes remain recoverable as lost. Never signal a PID recovered from SQLite because it might identify an unrelated process. Do not add PTY crates or process launching use cases until M07.

## 5. Recovery contract

Reconcile before readiness, under exclusive workspace ownership, after schema validation and before new public mutations. Read a coherent persisted snapshot. For every instance in `starting` or `running`, submit the existing internal system observation marking it `lost`, with unknown exit code and a valid ended timestamp. Never expose `System` as a caller-selectable actor.

For each affected instance, its final state, open claim closure with the existing instance-ended reason, task transition to `blocked`, and related events must commit atomically. Reuse application invariants: no claim means no task change; no automatic task completion; historical claims remain intact. Do not reconstruct a claim from the optional launch-context task ID.

Use one application transaction per affected instance unless an existing validated batch operation already supplies the same semantics. The runtime becomes ready only after all candidates are processed. A crash between instance transactions may leave partial overall progress, but never a partially reconciled instance/claim/task group. On the next start, skip final instances and finish the remaining candidates. Do not require a new all-workspace transaction or SQLite migration merely to make startup recovery work.

Required cases:

- A `starting` instance without a claim becomes lost, with no invented exit code.
- A `running` owner becomes lost; its claim closes and task becomes blocked in the same revision.
- Final `exited`, `failed`, `terminated`, and `lost` records retain timestamps, exit codes, and history unchanged.
- A second restart produces no duplicate recovery events or revisions for already final instances.
- Invalid references, malformed persisted records, or database compatibility failures fail closed. Do not repair by deleting rows.
- If the clock would violate a timestamp invariant, return the existing typed time error and remain unready. Do not silently clamp durable timestamps.
- An injected precommit failure leaves the whole affected group unchanged; a crash after commit preserves the whole group.
- Another workspace remains independent throughout recovery or failure.

Recovering a daemon is not recovering a live process. Document that distinction in CLI help and the runtime guide.

## 6. Public CLI and wire contracts

### 6.1 Selection and side effects

Use `rt [global options] <group> <command>`. Global selection is `--workspace <path>` with the current directory as its default. Do not search ancestor directories implicitly. Use `--home <absolute-path>` for the existing explicit private-location override, `--format human|json` (human default), and bounded `--timeout <seconds>` for lifecycle waits. Paths use native OS strings; a non-UTF-8 root must not be lost through Clap string conversion.

Only `workspace init` creates a registration. `workspace open` and `daemon start` may start an already initialized workspace. Other commands require an existing ready daemon and return safe guidance when stopped. This avoids hidden process starts from status or inspection. `workspace status` and `daemon status` use non-creating locate followed by bounded IPC when an endpoint exists. `daemon stop` does not start a stopped daemon.

Do not persist an active workspace pointer or change the caller's working directory. `workspace open` returns authoritative identity/status and ensures readiness; it does not open a TUI in M05. Repeated initialization preserves the stable ID and existing workspace name; a differing supplied name must not silently rename the workspace.

### 6.2 Command inventory

Implement the following public surface. Final flag spellings may be refined for Clap consistency only if the contract table and tests are updated together. Do not omit a row.

| Command | Required input and behavior | Transport |
| --- | --- | --- |
| `workspace init` | Optional `--name`; use a fixed safe default if omitted rather than lossy root basename conversion. Initialize and ensure ready. | Bootstrap initialize, then snapshot/status |
| `workspace open` | Existing workspace selection; ensure ready and report identity. | Bootstrap open, then snapshot/status |
| `workspace status` | Report initialized/stopped/ready/draining or safe failure, without creation. | Bootstrap locate, then status/snapshot when ready |
| `daemon start` | Existing workspace only; return started/already running. | Bootstrap open, then status |
| `daemon status` | Report lifecycle or stopped; no side effects. | Bootstrap locate, then status |
| `daemon stop` | Request and await same-generation shutdown; already stopped is success. | Locate, shutdown, bounded termination observation |
| `agent list` | Paginated accepted definitions. | `agent.list_definitions` |
| `agent register` | Complete neutral definition via `--file` or `--stdin`. | `agent.register_definition` |
| `agent update <definition-id>` | Complete editable fields plus required `--expected-revision`. | `agent.update_definition` |
| `agent import` | Explicit TOML input via `--file` or `--stdin`; apply an atomic validated import. | New `agent.import_definitions` |
| `task create` | Complete task content via `--file` or `--stdin`. | `task.create` |
| `task list` | Cursor/limit and applicable revision. | `task.list` |
| `task get <task-id>` | Optional expected revision. | `task.get` |
| `task update <task-id>` | Complete editable content and required expected revision. | `task.update` |
| `task transition <task-id> <state>` | Required expected revision; ordinary administrative transitions only. | `task.transition` |
| `task claim <task-id> --instance <id>` | Request as LocalUser on behalf of an existing running instance. | `task.claim` |
| `task release <task-id>` | Required expected revision; close claim and block. | `task.release` |
| `progress append <task-id>` | Summary and verification via bounded input. | `progress.append` |
| `handover create <task-id>` | Complete structured content and required expected revision. | `handover.create` |
| `handover get <handover-id>` | Read authorized structured content. | `handover.get` |
| `task history <task-id>` | Cursor, limit, expected revision as required by M04. | `task.get_history` |
| `task claims <task-id>` | Paginated claim history. | `task.get_claim_history` |
| `session list` | Persisted instance/session metadata only. | `session.list` |
| `event list` | Exclusive sequence cursor and limit. | `event.list` |
| `event watch` | Exclusive cursor; stream bounded JSON Lines or escaped human events until interrupted. | Subscribe/unsubscribe |

JSON input files contain exactly the existing operation-specific content DTO, excluding envelope IDs, actor, workspace, and expected revision, which come from validated command arguments. TOML import is the one format exception. Reject mixed `--file`/`--stdin`, extra JSON values, unknown fields, invalid UTF-8 narrative text, and over-limit data before sending. Read at most the applicable cap plus one byte, never `read_to_end` on unbounded input. Bound JSON by the existing protocol control-frame ceiling including envelope overhead; TOML remains at the config parser's 1 MiB limit. Validate domain field limits again on the server.

Use input files/stdin for narrative text and definition argument arrays; never introduce shell command parsing. Document complete synthetic JSON/TOML examples matching current DTOs, including priority, scope, informational dependencies, acceptance notes, verification, decisions, open questions, and next action where applicable. Do not silently default required handover verification to empty text.

Production M05 has no way to manufacture a running instance. Therefore claim and handover commands are fully implemented and tested with daemon fixtures but ordinarily reject absent/non-running instances until M07 can launch them. Public documentation must state this limitation rather than suggesting fake session creation. LocalUser can still create tasks and append permitted historical annotations.

### 6.3 Additive wire changes

Preserve workspace IPC version 1 and existing frame/envelope semantics. Add explicit operation variants, strict request/response DTOs, advertised support, dispatch mapping, and round-trip/negative tests for:

- `daemon.status`: empty parameters; lifecycle, workspace ID, generation, schema/protocol versions, safe bounded counts.
- `daemon.shutdown`: target generation required; acknowledgement identifies that generation and `draining`. A different generation is a conflict, never permission to stop the replacement.
- `agent.import_definitions`: bounded TOML document, optional expected revision, bounded structural warnings and compact mutation receipt. No source path is needed by the daemon because the client sends the bounded input contents.

Do not confuse a static known-operation enum with implemented capability. Older version-1 servers may reject new operations normally; clients must report unsupported capability without killing or replacing the server. Preserve unknown-operation behavior and reserved PTY/worktree unavailability. Add a stable `daemon_draining` rejection if the existing error vocabulary cannot express admission refusal; specify its retry guidance explicitly and test older unknown-code handling. Document all additions in the protocol reference/specification during implementation.

All public mutations remain LocalUser operations. There is no actor-selection argument. Runtime status and shutdown are control operations, not fabricated domain events. Actual task/claim/recovery/import mutations keep existing durable event semantics.

### 6.4 Configuration import

Parse and validate the entire TOML candidate on the daemon before reading its baseline revision. Construct validated definitions for the bound workspace. If an explicit expected revision is supplied, compare it in the write transaction; otherwise capture the current revision only after complete candidate validation. Then invoke the existing atomic application import path. Do not read the revision first and perform arbitrarily slow input loading afterward.

Preserve ADR 0006: stale baseline conflicts even for identical values; current identical import is a no-op; omitted definitions remain; imports neither delete definitions nor rewrite the input file. Invalid entries roll back the entire candidate. Unknown non-security fields yield bounded structural warnings; sensitive patterns and security-shaped unknowns fail closed. Store environment variable names only, never assignments or values.

Definition import does not hot-apply storage pool settings or terminal preferences to a running daemon. Preserve existing startup settings semantics and clearly report the scope of agent import. Source edits do nothing until explicit import. Enabled/disabled changes use definition updates, not a second configuration authority. Executable availability is checked at real launch in M07; M05 must not execute or reject an arbitrary registered command merely because it is absent today.

For runtime settings, use the private config directory's `config.toml` as the single optional startup source. An absent file uses the existing `StorageSettings` defaults; an inaccessible or invalid file fails startup safely. Load and validate it before opening storage, using the existing 1 MiB parser bound, busy-timeout range, and pool-size range. Its definition entries are validated but never automatically imported. Restart may apply storage settings, but cannot apply definition edits. Lookup-only status does not need to load this file. Help/version do not read it. Document this filename and settings-only startup behavior in the specification and config guide, since M03 provided the parser rather than a complete runtime loading policy.

### 6.5 Output, errors, and uncertain mutations

Define CLI output schema version 1 independently of IPC. Finite `--format json` commands emit exactly one JSON object on stdout, containing `schema_version`, `command`, `ok`, and exactly one of `result` or `error`. Preserve decimal-string revisions, sequences, request IDs, and existing native-path encodings. Include pagination cursor and revision explicitly. Do not automatically collect an unbounded history.

In JSON mode, errors use the same envelope on stdout; stderr is reserved for bounded diagnostic notices. In human mode, results go to stdout and safe errors to stderr. Help/version retain conventional Clap output and no private state creation. Parser errors must not echo secret-like user argument values; customize raw Clap diagnostics where necessary. Disable color in JSON and respect `NO_COLOR` for human output.

`event watch --format json` uses JSON Lines with a declared record type for event, caught-up, resnapshot-required, and terminal error/control records. Bound buffering, flush incrementally, and terminate cleanly on broken stdout. Ctrl+C cancels the subscription and leaves the daemon running. Do not turn a resnapshot-required signal into silent event loss or automatic mutation replay.

Human renderers escape ANSI, carriage returns, control sequences, and untrusted field delimiters outside intentional narrative line breaks. JSON serializers must escape control bytes correctly. Authorized task/definition content may appear in an explicitly requested result; that does not authorize copying it into diagnostic logs. No automatic export or file creation is added.

| Exit code | Contract |
| --- | --- |
| 0 | Successful result, including already running/already stopped and successful status reporting a stopped daemon |
| 1 | Unexpected internal failure with safe diagnostic code |
| 2 | CLI usage or invalid local input |
| 3 | Workspace/entity not found or workspace not initialized |
| 4 | Daemon unavailable, startup/stop deadline, or transport failure with no uncertain mutation |
| 5 | Conflict, invalid state, invalid reference, or actor rejection from authoritative state |
| 6 | Access/security, protocol/schema incompatibility, or recovery-required failure |
| 7 | Mutation result unknown after possible dispatch |
| 8 | Storage/service failure reported definitively without an uncertain outcome |
| 130 | User interruption, except a possibly dispatched mutation which returns 7 |

Return stable machine codes and actionable safe messages; never pass raw OS/SQL/parser errors through. A stopped workspace is a status result, whereas an uninitialized root is code 3. Stop on an initialized already stopped workspace is code 0. A live inaccessible endpoint is not stopped.

Once a mutation may have been dispatched, loss of its response produces code 7, including cancellation. Return any known workspace, request, and entity identifiers without inventing unknown ones; guide the user to refresh state/history/events. Never repeat create, progress, claim, handover, or import automatically. Read-only retry is bounded. Read/modify/write commands require the advertised revision and do not silently fetch a new revision to retry a conflict.

## 7. Diagnostics and private resource bounds

Use structured severity levels (`error`, `warn`, `info`, `debug`) and stable event names. Initial names include `daemon.starting`, `daemon.ready`, `daemon.start_failed`, `daemon.draining`, `daemon.drain_delayed`, `daemon.stopped`, `daemon.recovery_failed`, `ipc.peer_rejected`, `ipc.request_failed`, and `storage.unavailable`. Record only allowlisted fields: safe error code, lifecycle, workspace/generation IDs where needed, aggregate counts, and bounded durations. Do not derive Debug or log entire DTOs, requests, domain entities, configuration, arguments, paths, or error source chains.

Use the existing location aliases for guidance, such as the private data/runtime/config locations. Diagnostic logging is distinct from durable workspace events: event sequence comes from committed storage, not log timestamps. Missing daemon executable, failed spawn, unavailable DB, incompatible schema/protocol, denied endpoint, and full disk all need distinct safe messages. Missing configured agent executables remain launch-time M07 failures; registration must not pretend to launch them.

Place daemon logs under the private per-workspace data area. Default to info severity, no terminal captures and no request/response bodies. Bound each encoded record to 4 KiB, each file to 1 MiB, and retention to one active plus three rotated files (at most 4 MiB of retained record content per workspace). Use a bounded asynchronous queue of at most 256 records if logging is asynchronous. Drop excess diagnostic records with a saturating count and a later compact notice; never block admitted storage work indefinitely on logging.

Rotation may remove only the oldest application-owned diagnostic file under the verified private log directory. Check path type/ownership, reject symlink/reparse redirection using existing platform protections, and never recursively clean a directory. Test startup with preexisting rotated files and oversized files. A bounded record larger than the per-record limit is structurally truncated without exposing rejected payload content.

A log write/rotation failure must not undo a committed operation, create a retry loop, or cause unbounded stderr output. Fail closed if a safe private log destination cannot be established before startup; once running, degrade diagnostics with a bounded safe notice while preserving durable state. Document loss of diagnostic records honestly. Do not persist raw panic payloads or backtraces from potentially sensitive inputs.

## 8. Verification scenarios and required evidence

Tests must use disposable native temporary directories, real SQLite, actual local IPC, and the built `rt` executable where process behavior is under test. Set an isolated explicit private home. Never touch the developer's registry, logs, socket, project data, or installed agents. Native Unix socket paths must remain within existing length limits; do not hardcode a macOS temporary path into Linux tests.

Use barriers, channels, process readiness records, and monotonic deadlines for races. Sleeps alone are not proof of ordering. Parent harness cleanup must know which child it owns; do not kill a PID discovered from untrusted metadata. Killing an explicitly owned disposable test process is a crash fixture, not production stop behavior.

### 8.1 Startup and detachment matrix

| Scenario | Required assertion |
| --- | --- |
| Two simultaneous initializers | One registry identity and valid database; both resolve the same authoritative workspace. |
| Two simultaneous starters | One runtime generation owns the endpoint; loser cleanup cannot unlink or stop it. |
| Starter exits after readiness | A separate client reads and mutates the same workspace. |
| Starter exits while candidate starts | No pipe-induced daemon termination; later status/open converges within bounded behavior. |
| Launching terminal closes | Runtime remains reachable on native Unix and Windows under the declared normal-session boundary. |
| Existing compatible runtime | Start is idempotent; no reconciliation, definition reload, or second runtime. |
| Existing incompatible/inaccessible endpoint | Safe failure; no replacement or cleanup. |
| Owner process crashes | Subsequent owner performs safe stale recovery; prior durable state survives. |
| Wrong-type/symlink endpoint or lock | Fail closed; unrelated paths and processes untouched. |
| Two different workspaces | Independent IDs, endpoints, state, stop, and failure boundaries. |
| Spaces, Unicode, and supported native path forms | Identity stable through bootstrap and reconnect; no shell or lossy path conversion. |
| Unknown status/help/version | No registry, runtime, database, or log created. |
| Missing ready database | Recovery-required failure; no empty replacement. |

### 8.2 Shutdown and cancellation matrix

Pause an accepted mutation before commit, request shutdown, disconnect its client, and release the barrier. Assert exactly one complete durable transaction and event set, or the injected complete rollback. The runtime must not release ownership while that operation remains pending. Reopen storage after actual shutdown to inspect the result independently.

Also prove:

- A request arriving after the drain boundary is rejected without state or event changes.
- An idle connection, partial frame, slow subscriber, and blocked response consumer cannot keep shutdown alive after admitted work finishes.
- A partial frame survives notification/poll selection or is rejected safely without consuming later frames incorrectly.
- Two simultaneous client calls and a waiting event consumer make progress with correctly correlated, monotonic request IDs.
- Lost shutdown acknowledgement yields bounded uncertainty/status guidance; repeating against the same generation is safe.
- A replacement generation is never stopped by retrying a stale shutdown request.
- Expiry of the client's timeout does not abort a committed or accepted mutation or kill the daemon.
- Repeated shutdown requests do not duplicate domain observations or leak tasks, handles, subscriptions, or locks.
- Signal-triggered shutdown uses the same drain path. Abrupt crash leaves SQLite reopenable with all-or-nothing writes.

### 8.3 Recovery fixtures

Use test-only setup to persist starting/running instances and active claims before launching the real runtime. No production fake-supervisor switch is allowed. Verify every case in section 5, including a failure between two reconciled instances and a second restart. Check entities, open/closed claim history, task state, workspace revision, and ordered events together. Keep historical nonzero exits and unknown exit codes intact. Verify no cross-workspace effects.

This targeted lifecycle suite is required in M05. The full multi-instance handover continuity scenario across these layers remains M06, with its queue entries preserved.

### 8.4 CLI and privacy matrix

- Execute every command row against the real server or an appropriate test fixture; include invalid IDs, wrong-workspace IDs, stale revisions, impossible state transitions, second claims, and missing instance failures.
- Test 0, default, maximum, and excessive pagination limits; preserve cursor exclusivity and coherent revision paging.
- Import valid TOML, edit its source without import, repeat identical import, submit stale identical import, race a conflicting edit, and inject one invalid definition among valid ones. Check source bytes remain unchanged.
- Exercise oversized stdin/files, malformed JSON/TOML, duplicate options, unsafe argument values, multibyte boundaries, and non-UTF-8 native roots where supported.
- Verify JSON output parses, has stable types, contains no ANSI, uses the declared exit codes, and includes required pagination metadata. Verify broken stdout and Ctrl+C do not stop the daemon.
- Inject synthetic sensitive markers into configuration, tasks, paths, malformed frames, and parser errors. Assert absence from stderr diagnostics and private logs. Authorized requested content remains readable through result output only.
- Fill the log queue, exceed every rotation bound, precreate unsafe log paths, and inject disk/write failures. Verify retention bounds and no effect on already committed mutations.
- Snapshot the disposable source tree before and after Git and non-Git workflows. No private artifacts are added there under default locations. Preserve deliberately explicit private-home overrides already allowed by M03; do not silently change that contract.

### 8.5 Native evidence

Run process detachment, endpoint security regressions, shutdown, recovery, and CLI process tests on `ubuntu-24.04`, `macos-14`, and `windows-2022` or explicitly reviewed successor runners. Run the pinned-toolchain Linux job as well. Existing CI workspace tests must actually discover the new tests; do not leave acceptance tests ignored without an invoking parent.

A native process-exit test does not alone establish terminal-close behavior. Add an automated console/session harness when the runner permits it; otherwise retain that specific gate and obtain a sanitized native manual record. Record OS, terminal/shell or harness, candidate revision, procedure, observed separate-client reconnect, and cleanup. Do not publish raw terminal transcripts or personal paths. Never label cross-compilation as native evidence or skip Windows failures to merge.

## 9. Documentation and compatibility deliverables

During implementation update the README to show only actually available commands and create a focused daemon/CLI guide in `docs` with complete runnable synthetic examples. Cover initialization versus open, selection and private locations, daemon lifetime, normal stop versus crash, restart loss semantics, JSON/exit codes, explicit config import, unknown mutation outcomes, and recovery guidance that preserves original data.

Update ADR 0001 with verified bootstrap/detachment/shutdown decisions, ADR 0002 with any relevant additive IPC contract, and ADR 0006 only if its clarification is necessary. Preserve the eight-ADR repository check unless a separately justified architecture decision truly requires changing that contract. Update the specification for the administrative and lifecycle additions. Keep the default TUI and real-session limitations visible.

Do not introduce a schema migration unless an actual persistent field is necessary and justified. Runtime generation, transient startup state, and stop coordination do not require durable domain rows. Keep schema version and wire version independent. New dependencies must respect pinned Rust/MSRV, lockfile, license and advisory policy; add no unused production dependencies or test execution flags to the public binary.

## 10. Atomic execution queue

### M05.01: Compose the runtime and bootstrap

1. M05.01a: Reinspect the baseline and record exact startup, lock-order, capability, bootstrap, and admission contracts. Add child tasks before implementation.
2. M05.01b: Implement typed bootstrap request/response DTOs, bounded anonymous-pipe exchange, and non-creating lookup. Test malformed/oversized/extra input and missing registry without filesystem changes.
3. M05.01c: Compose real clock/IDs/notifier, config settings, private locations, registry, SQLite, application service, and workspace server in daemon production code. Remove the runtime placeholder only when composition works.
4. M05.01d: Integrate exclusive lock ownership, candidate cleanup, readiness generation and discovery. Test duplicate init/start, live endpoint preservation, failed initialization recovery, and independent workspaces.
5. M05.01e: Update architecture allowlists narrowly and test forbidden core closure. Keep public client state access entirely through IPC.

Gate: a real runtime can be discovered and reached from two clients; lookup is side-effect free; no competing runtime performs recovery or destructive cleanup.

### M05.02: Detach and control the daemon

1. M05.02a: Select and prototype safe native detachment APIs on all three platforms, documenting inherited handles and terminal/job boundaries.
2. M05.02b: Implement bootstrap helper orchestration and detached runtime launch through the same `rt` executable, without shell lookup or inherited client pipes.
3. M05.02c: Add strict status/shutdown protocol DTOs, generation checks, capability handling, and bounded readiness logic.
4. M05.02d: Implement daemon start/status/stop parsing and renderer skeleton with explicit side effects and timeout results.
5. M05.02e: Run starter-exit, terminal-close, startup race, timeout, and live-incompatible-endpoint tests natively. Preserve unresolved platform evidence as pending.

Gate: a separate client reconnects after the launching client/terminal closes; same-generation stop behavior is testable and no PID-only ownership exists.

### M05.03: Drain and release resources

1. M05.03a: Refactor server resource ownership into joined connection/writer/background tasks and independently owned admitted service work.
2. M05.03b: Implement atomic mutation admission and the shutdown coordinator, including repeated requests and delayed drains.
3. M05.03c: Correct reproduced fragmented-reader/client concurrency defects required by lifecycle integration; retain M04 regression coverage.
4. M05.03d: Add empty production supervision and test-only observations through a narrow interface. Wire cooperative signals to the coordinator.
5. M05.03e: Test barriers before/after commit, disconnects, slow peers, stale-generation stop, resource release, abrupt crash, and durable reopen.

Gate: shutdown never loses accepted work or waits on an idle client indefinitely; lock release follows operation/store cleanup; production contains no fake session launch path.

### M05.04: Reconcile lost sessions

1. M05.04a: Enumerate nonfinal instances after acquiring ownership and before readiness; reuse validated internal application observations.
2. M05.04b: Preserve atomic instance/claim/task/events per instance and explicit failure/unready behavior.
3. M05.04c: Add test-only persisted fixtures for starting, running, claimed, unclaimed, and historical final cases.
4. M05.04d: Verify repeated restart, failure between instances, temporal validation, precommit rollback, and cross-workspace isolation.

Gate: every affected group is consistent, history is preserved, and repeated startup produces no duplicate loss effects.

### M05.05: Deliver administrative commands

1. M05.05a: Implement global native-path selection, format/timeout parsing, bounded file/stdin input, and safe parser errors.
2. M05.05b: Implement workspace init/open/status using bootstrap plus ordinary IPC, including no implicit initialization.
3. M05.05c: Implement agent list/register/update/import and daemon-side atomic config conversion with baseline semantics.
4. M05.05d: Implement task create/list/get/update/transition/claim/release using existing actor and revision contracts.
5. M05.05e: Implement progress, handovers, task/claim history, session listing, and event list/watch with bounded paging/streaming.
6. M05.05f: Finalize schema-versioned JSON, escaped human output, stable exits, cancellation, and unknown-mutation guidance.
7. M05.05g: Run command-by-command success/failure tests, including claim/handover fixture success and honest production missing-instance failures.

Gate: every row of section 6.2 is implemented and verified; CLI never opens SQLite; no command bypasses the daemon's ownership or authorization rules.

### M05.06: Bound and sanitize diagnostics

1. M05.06a: Define allowlisted diagnostic events, fields, severity, error mapping, and safe location aliases.
2. M05.06b: Implement private logging, record/file/retention/queue limits, safe rotation, and bounded failure handling.
3. M05.06c: Remove touched raw error/Debug/panic leakage and keep result rendering distinct from logging.
4. M05.06d: Verify missing runtime executable, storage/IPC/version failures, marker injection, rotation, full queues, and unsafe log destinations.

Gate: diagnostics remain actionable without sensitive payloads, and logging failure cannot reverse or duplicate durable operations.

### M05.07: Verify and document the lifecycle gate

1. M05.07a: Write the runtime/CLI guide and update README/specification/ADRs with implemented behavior and exact examples.
2. M05.07b: Execute the guide in disposable Git and non-Git roots, spaces/Unicode paths, and an isolated private home; verify source-tree cleanliness and two-client authority.
3. M05.07c: Run all local checks below, inspect actual test discovery, and fix failures without weakening requirements.
4. M05.07d: Obtain all native CI and terminal-lifetime evidence, record candidate revisions/results, and preserve missing evidence as an open gate.
5. M05.07e: Reconcile TODO/avances and acceptance coverage. Prepare implementation handoff with PR/merge and postmerge CI evidence when publishing is explicitly authorized.

Gate: all M05 contracts pass; M06 and M07 remain pending; no claim of full MVP, real PTY survival, or TUI completion is made.

## 11. Checks and completion criteria

Use narrow checks during iteration, adapted to touched crates:

```text
cargo test -p relayterm-platform -p relayterm-protocol --locked
cargo test -p relayterm-persistence-sqlite -p relayterm-application --locked
cargo test -p relayterm-client -p relayterm-daemon -p relayterm-cli --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
```

Before implementation closure run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny --locked check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner .
git diff --check
```

These commands are required future implementation evidence, not claims that they were executed for this planning document. Use the repository's platform-appropriate Python command on CI. Keep lockfile and dependency checks reproducible. Do not repeatedly push known failures merely to obtain another CI run; diagnose available local and remote logs first.

| Completion item | Minimum evidence |
| --- | --- |
| Bootstrap and identity | Concurrent init/start, non-creating locate, protected live endpoint, missing-ready-DB failure |
| Detached runtime | Native separate-client reconnect after starter and terminal closure |
| Orderly shutdown | Admission race, accepted transaction completion, drained task/handle ownership, generation-safe stop |
| Restart | Atomic loss/claim/task/events, idempotence, historical exit preservation |
| CLI | Every command mapped to IPC, strict input/output, exit codes, explicit config import |
| Observability | Stable safe diagnostics and tested record/queue/retention bounds |
| Security | Existing current-user Unix/Windows gates retained; bootstrap has no public endpoint |
| Architecture | Core/protocol tests independent from TUI; no persistence access in client |
| Native quality | All required Linux/macOS/Windows jobs and additional terminal evidence at reviewed revision |
| Scope and logbook | Only verified M05 work removed, append-only evidence, M06 onward preserved |

If any row is missing, M05 remains open. When delivery includes a PR, inspect required checks and reviews, merge only after this gate passes, synchronize local and remote `main`, and inspect postmerge CI. Report a concrete repository approval/access blocker rather than bypassing it.
