# Relayterm MVP technical specification

## Document status

- Status: Draft for implementation
- Audience: Coding agents, contributors, and maintainers
- Product direction: [Project vision and request for project](PROJECT_VISION.md)
- Requirement keywords: `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, and `MAY` are normative

## 1. Objective

Build Relayterm, a cross-platform, persistent, agent-neutral workspace that supervises real terminal sessions and maintains shared task, progress, and handover state for Claude Code, Codex, OpenCode, and future command-line agents.

The MVP MUST ship a terminal user interface, but the core MUST run independently from the TUI. The architecture MUST permit future desktop and web clients to connect without moving domain logic into those clients.

The MVP is complete when a single user can create or open a workspace, launch multiple real terminal sessions, associate agents with tasks, record structured handovers, disconnect and reconnect the TUI, and recover durable state safely on Linux, macOS, and Windows.

## 2. Design constraints

- The product is TUI-first, not TUI-only.
- The daemon owns workspace state, process supervision, and persistence.
- The TUI is a replaceable client of the daemon.
- Command-line agents run as ordinary child processes inside real pseudo-terminals.
- Provider authentication remains under the control of each provider's CLI.
- Core operation is local-first and MUST NOT require a hosted service.
- The domain model is provider-neutral and MUST NOT encode Claude, Codex, or OpenCode as privileged types.
- Cross-platform behavior is required for Linux, macOS, and Windows.
- Public repository content MUST be free of personal data and secrets.
- The MVP serves one local operating-system user and is not a remote multi-tenant service.

### 2.1 Product and command identity

- The product name for the initial release is **Relayterm**.
- The canonical user-facing executable MUST be **`rt`**.
- Running `rt` without a subcommand SHOULD open the TUI for the current directory or the selected workspace.
- Daemon and administrative operations SHOULD be exposed as subcommands such as `rt daemon` and `rt status`, keeping one concise installation entry point.
- Installers and documentation MUST detect or explain local executable conflicts. Initial Linux and macOS checks found no broadly established general-purpose `rt` command, but the project MUST NOT assume universal availability of the name.
- The reserved **`relayterm.com`** domain is intended for a future landing page. The MVP MUST NOT contact it or require it for startup, discovery, authentication, updates, telemetry, or any core workflow.

## 3. Recommended technology

### 3.1 Language: Rust

Rust is recommended for the core, daemon, TUI, and first-party command-line tools.

Rationale:

- Strong ownership and concurrency guarantees reduce risk in long-lived process and session management.
- Rust provides direct access to operating-system primitives while retaining memory safety.
- Its ecosystem includes mature terminal UI, asynchronous runtime, serialization, database, and pseudo-terminal libraries.
- A single compiled `rt` binary simplifies installation and use on remote hosts while allowing internal daemon and client roles to remain architecturally separate.
- Cross-platform abstractions can coexist with targeted platform-specific implementations when required.
- The type system is well suited to explicit task, process, protocol, and lifecycle state machines.

Go remains a credible alternative for network services and simpler operational tooling, but Rust is preferred because terminal emulation, pseudo-terminal control, lifecycle safety, and future native clients are central to the product rather than peripheral concerns.

### 3.2 Primary crates and frameworks

- **Ratatui:** TUI layout, widgets, rendering, and application structure.
- **Crossterm:** Cross-platform terminal input, raw mode, alternate screen handling, and event support.
- **portable-pty:** Cross-platform pseudo-terminal creation and child process control.
- **Tokio:** Asynchronous tasks, channels, timers, process-adjacent orchestration, and local IPC.
- **SQLite:** Durable embedded storage with transactional updates and no external database service.
- **sqlx:** Recommended SQLite access with migrations and compile-time query checking where practical.
- **Serde:** Protocol, configuration, and event serialization.
- **tracing:** Structured diagnostics with configurable redaction and output sinks.
- **thiserror** and **anyhow:** Typed library errors and application-level context respectively.
- **clap:** Command-line parsing for daemon, client, and administrative commands.
- **uuid:** Stable opaque identifiers for workspaces, tasks, sessions, and handovers.
- **time:** UTC timestamps and serialization.

Crate versions MUST be selected and locked during bootstrap after confirming current platform support and license compatibility. Optional dependencies should not be added before an implementation need exists.

### 3.3 Git integration

Git worktrees SHOULD be used when two tasks need isolated working directories in the same repository. Worktree use MUST be explicit in the MVP and MUST NOT be required for a single task or a non-Git workspace.

The core SHOULD invoke the installed Git executable through a narrow adapter. It SHOULD NOT implement Git object semantics or make a large Git library a foundational dependency for the MVP.

## 4. System architecture

```text
+-------------------+       local versioned IPC       +----------------------+
| rt TUI client     | <-----------------------------> | Relayterm daemon     |
| Ratatui/Crossterm |                                 | domain and services  |
+-------------------+                                 +----------+-----------+
                                                                  |
                          +---------------------------------------+------------------+
                          |                  |                    |                  |
                    +-----v------+     +-----v------+      +------v------+    +------v------+
                    | SQLite     |     | PTY/session|      | Git/worktree|    | Agent       |
                    | repository |     | supervisor |      | adapter     |    | adapters    |
                    +------------+     +-----+------+      +-------------+    +------^------+
                                               |                                  |
                                         Real shells                      Provider CLIs
                                         and processes               Claude, Codex, OpenCode
```

The dependency direction MUST point inward toward domain types and use cases. UI, SQLite, PTY, Git, and provider-specific code are adapters around the core.

### 4.1 Process model

The reference Relayterm deployment contains:

- One daemon process started and managed through `rt`, either per local user or scoped to a selected workspace in the earliest implementation.
- Zero or more TUI client processes.
- Zero or more shell or agent child processes, each attached to a pseudo-terminal.

The daemon MUST continue running when a TUI client disconnects. A client reconnect MUST retrieve a fresh snapshot followed by ordered events.

The MVP MAY initially require the daemon process to remain alive for PTY sessions to survive. Host restart recovery of live child processes is explicitly excluded. Durable metadata MUST still recover after a daemon restart, and previously live sessions MUST be marked with an honest terminal state such as `lost` or `exited`.

### 4.2 Layering

#### Domain layer

Pure Rust types and state transitions for workspaces, agents, tasks, sessions, handovers, and events. It MUST NOT depend on Ratatui, Crossterm, SQLite, portable-pty, or a specific provider.

#### Application layer

Use cases such as creating a task, claiming work, launching an agent session, recording progress, producing a handover, releasing a task, and reconnecting a client.

#### Ports

Traits for persistence, event publication, clocks, identifiers, PTY supervision, Git operations, and agent launch configuration.

#### Adapters

SQLite repositories, local IPC transport, PTY implementation, Git command adapter, configuration loader, agent command adapters, and TUI presentation.

## 5. Core domain model

### 5.1 Workspace

Required fields:

- `id`
- `display_name`
- `project_root`
- `created_at`
- `updated_at`
- `schema_version`

The stored path MAY be machine-specific local state but MUST NOT be written into a tracked project file by default.

### 5.2 Agent definition

Required fields:

- `id`
- `display_name`
- `command`
- `arguments`
- `environment_allowlist`
- `capabilities`
- `enabled`

Initial built-in templates MAY exist for Claude Code, Codex, and OpenCode, but templates MUST be editable and MUST use the same model as custom agents.

The environment policy MUST inherit only the environment required for normal child-process behavior, with documented overrides. Secret values MUST NOT be copied into SQLite, events, or logs.

### 5.3 Agent instance and terminal session

Required fields:

- `id`
- `session_id`
- `workspace_id`
- `agent_definition_id`, nullable for a generic shell
- `task_id`, nullable
- `working_directory`
- `status`
- `started_at`
- `last_observed_at`
- `ended_at`, nullable
- `exit_code`, nullable
- `terminal_size`
- `launch_definition_snapshot`, nullable for a generic shell and immutable after registration

Required lifecycle:

| Source | Allowed destinations |
| --- | --- |
| `starting` | `running`, `failed`, `terminated`, `lost` |
| `running` | `exited`, `failed`, `terminated`, `lost` |
| `exited`, `failed`, `terminated`, `lost` | None |

An instance and its terminal session share one lifecycle record, with distinct one-to-one `id` and `session_id` identifiers and explicit `workspace_id`. Configured launches capture the accepted definition ID, display name, command, arguments, environment names, capabilities, and enabled state in `launch_definition_snapshot` inside the registration transaction. Generic shells have no definition snapshot. Definition edits affect later attempts and never rewrite existing snapshots. `starting` records a launch attempt. Startup failure is `failed` without an invented exit code. `exited` is an observed exit, including nonzero codes; `terminated` requires confirmed termination. `lost` means the outcome cannot be reconstructed. Unknown exit codes are nullable; final states require `ended_at`. Identical final observations are no-ops; incompatible observations are rejected.

Finalizing an instance MUST atomically close its open claim and block the active task, if any. Exit never automatically completes work. Optional `task_id` is launch context, not current ownership. Pre-PTY simulation is restricted to tests, including M06; production cannot fabricate running processes.

### 5.4 Task

Required fields:

- `id`
- `workspace_id`
- `title`
- `description`
- `status`
- `priority`
- `scope_paths`
- `dependency_ids`
- `acceptance_notes`
- `claimed_by_instance_id`, nullable
- `worktree_id`, nullable
- `created_at`
- `updated_at`

Required lifecycle:

| Source | Allowed destinations |
| --- | --- |
| `backlog` | `ready`, `cancelled` |
| `ready` | `active`, `cancelled` |
| `active` | `blocked`, `handover_ready`, `done`, `cancelled` |
| `blocked` | `ready`, `cancelled` |
| `handover_ready` | `active`, `cancelled` |
| `done` | None |
| `cancelled` | None |

Tasks have explicit `workspace_id`. Dependencies are informational references to up to 128 distinct tasks in the same workspace. Self-references are rejected; cycles between distinct tasks are allowed. Dependencies never block claims or propagate state and do not introduce scheduling.

Transitions MUST be validated by the domain layer; same-state transitions are rejected. Activation requires a dedicated claim operation from `ready` or `handover_ready` and a `running` instance. Exactly one claim is open on an active task, and none on other states. An instance may hold at most one open claim. Repeated claims, including by the owner, are rejected. Closed claims remain immutable history.

Leaving `active` closes the claim atomically. Explicit release leaves the task `blocked`; availability requires an explicit `blocked` to `ready` transition. Handover preparation atomically creates the handover, closes the claim, and sets `handover_ready`; a successor must claim explicitly. Final tasks cannot reopen or be edited, but the user may append historical corrections.

Actors are `LocalUser` (without personal identity), `Instance(AgentInstanceId)`, and internal `System` for lifecycle observations/reconciliation. Clients cannot select `System`. The user may administer tasks, cancel any non-final task, and request claims for running instances with user attribution. Instances may act only on their own active work. Unowned-task cancellation requires the user. User intervention never impersonates an instance. See the [M02 contracts](docs/M02_details.md) for the permission table and validation limits.

### 5.5 Progress entry

Required fields:

- `id`
- `task_id`
- `agent_instance_id`, nullable
- `summary`
- `verification`
- `created_at`

Progress entries are append-only. Correction is represented by a newer entry rather than silent mutation.

### 5.6 Handover

Required fields:

- `id`
- `task_id`
- `from_agent_instance_id`, nullable
- `summary`
- `decisions`
- `changed_paths`
- `verification_performed`
- `open_questions`
- `recommended_next_action`
- `created_at`

A handover is structured project context, not a raw chat transcript. Secret-like content MUST be rejected or flagged before an explicit export to a project file.

### 5.7 Workspace event

Every material state transition MUST emit an ordered event containing:

- `sequence`
- `event_id`
- `workspace_id`
- `event_type`
- `entity_id`
- `timestamp`
- `payload_version`
- `payload`

Events support client synchronization and auditability. The relational state in SQLite remains the authoritative current state for the MVP. The MVP does not require full event sourcing.

## 6. Components

### 6.1 Relayterm daemon

Responsibilities:

- Open and migrate the SQLite database.
- Enforce domain transitions and task claims.
- Launch, resize, signal, and observe PTY sessions.
- Retain bounded terminal scrollback in memory.
- Publish ordered workspace events to connected clients.
- Reconcile persisted session metadata after restart.
- Invoke narrow Git worktree operations.
- Apply logging and redaction policy.

The daemon MUST reject unsupported protocol versions and malformed requests without crashing.

M05 implements one independently detached daemon per registered workspace. A finite internal bootstrap subprocess performs explicit initialization or non-creating registry lookup before a workspace endpoint exists. Public clients never open SQLite. Readiness requires an authenticated workspace handshake after exclusive endpoint ownership and restart reconciliation. Orderly shutdown rejects new mutations, retains daemon ownership of accepted transactions, closes client tasks, and releases the endpoint last.

### 6.2 Local IPC API

The MVP SHOULD use a framed, versioned request, response, and event protocol over:

- Unix domain sockets on Linux and macOS.
- Named pipes on Windows.

JSON is recommended for the first protocol because it is inspectable and easy for future clients to implement. Messages MUST include a protocol version and request identifier. Terminal byte streams MAY use a distinct binary frame type to avoid unsafe text conversions.

Protocol version 1 uses a seven-byte frame header with a big-endian payload length and version followed by a frame kind. Control JSON rejects duplicate and unknown fields, and all payloads have explicit byte limits. Request identifiers are nonzero decimal `u64` strings that increase per connection. The first request MUST be `protocol.hello` and MUST bind the connection to the server's fixed workspace.

The IPC endpoint MUST be accessible only to the current local user using operating-system permissions. TCP listening is excluded from the MVP.

Minimum operations:

- `workspace.get_snapshot`
- `agent.list_definitions`
- `session.create`
- `session.attach`
- `session.input`
- `session.resize`
- `session.terminate`
- `task.list`
- `task.create`
- `task.update`
- `task.claim`
- `task.release`
- `progress.append`
- `handover.create`
- `worktree.create`
- `worktree.list`
- `event.subscribe`

The initial protocol also provides `protocol.hello`, `protocol.ping`, `daemon.status`, `daemon.shutdown`, `agent.register_definition`, `agent.update_definition`, `agent.import_definitions`, `task.get`, `task.transition`, `handover.get`, `task.get_history`, `task.get_claim_history`, `session.list`, `event.list`, and `event.unsubscribe`. M07 activates session creation with bounded generation-local receipts, attachment, detach, input lease acquisition and release, sequenced bounded input, resize, termination, authoritative snapshots, and offset-checked binary output continuation. M08 adds `session.read_display`, a capability-negotiated bounded parsed viewport with compact neutral cells, revision checks, parsed scrollback offsets, and unchanged responses. These additions are backward-compatible protocol version 1 operations advertised during hello because no prior production server accepted these reserved or new operation names. Worktree operations retain versioned payload definitions but return `operation_unavailable` until M10.

Protocol version 1 permits these additive lifecycle and configuration operations. Clients inspect advertised operations and treat an older server's rejection as an unsupported capability. Shutdown targets an opaque runtime generation so a delayed request cannot stop a replacement daemon. Configuration import sends bounded contents, never a source path, and preserves revision-based atomic semantics.

Public IPC clients act only as `LocalUser`; lifecycle observations and the `System` actor remain internal service boundaries. Mutation requests that update observed state include an expected workspace revision. Claims retain domain-level atomic exclusivity and do not require a revision precondition. A client MUST NOT automatically repeat a mutation if writing began and no definitive response arrived. It MUST report the outcome as unknown and refresh authoritative state.

Snapshot collection pages carry a workspace revision, last event sequence, and retained event floor. Clients MUST stage all pages against one revision and install them together. Event subscription starts after the snapshot watermark, replays durable events in sequence, and reports invalid or expired cursors explicitly. Notification is an optimization; bounded durable polling remains the gap-free fallback.

The exact subcommand hierarchy MAY evolve before the first stable release, but `rt` MUST remain the canonical user-facing executable.

### 6.3 PTY and session supervisor

The supervisor MUST:

- Create a real pseudo-terminal through `portable-pty`.
- Start a configured shell or agent CLI in the selected working directory.
- Forward terminal input and output without interpreting provider content.
- Propagate terminal resize events.
- Record process exit status.
- Bound scrollback by a configurable byte or line limit.
- Apply backpressure so a slow client cannot exhaust daemon memory.
- Support client detach and reattach while the daemon and child process remain alive.

The MVP SHOULD treat terminal output as opaque bytes plus minimal terminal metadata. Full terminal emulation inside the daemon is not required unless needed to provide correct reattachment behavior. Any parser introduced MUST be provider-neutral.

### 6.4 Agent adapter registry

An adapter describes how to launch a CLI and what optional capabilities it exposes. The generic adapter MUST support any interactive command.

Provider-specific adapters MAY supply:

- Default executable discovery.
- Suggested arguments.
- Capability declarations.
- Health checks that do not access credentials.

Adapters MUST NOT:

- Capture provider credentials.
- Bypass interactive authentication.
- Depend on undocumented private APIs.
- Parse cosmetic terminal output as a source of authoritative task state.

### 6.5 Persistence

SQLite MUST use migrations checked into the repository. Updates that combine state changes and workspace events MUST be transactional.

The database MUST live in an operating-system-appropriate application data directory, not inside the Git repository, unless the user explicitly selects a different location.

Terminal scrollback persistence is optional for the MVP. If implemented, it MUST be disabled by default or have an explicit retention policy because terminal output may contain sensitive data.

### 6.6 TUI client

The TUI SHOULD provide these views:

- Workspace overview.
- Task board or list with status and claim owner.
- Terminal session tabs or panes.
- Agent definitions and active instances.
- Task detail with progress and handovers.
- Event or diagnostic view with redacted content.

Minimum interactions:

- Create, edit, claim, release, and transition a task.
- Launch a generic shell or configured agent for a task.
- Attach to and detach from a terminal session.
- Switch between concurrent sessions.
- Resize terminals correctly.
- Add a progress entry or structured handover.
- Reconnect to the daemon and refresh state.
- Display keyboard help and actionable errors.

The TUI MUST contain presentation logic only. It MUST NOT open SQLite directly or own child processes.

M08 implements these views and interactions in `relayterm-tui`, with `rt` providing only bootstrap and client composition. Stdin and stdout must both be terminals before bootstrap begins. Forms remain in bounded memory, use byte-aware limits, preserve rejected or uncertain submissions, and require explicit discard. The client uses independent control and event connections, bounded reconnect delays, and no automatic retry for uncertain mutations or input.

Terminal panes render only daemon-parsed replacement viewports. They map neutral colors, attributes, cursor and wide-cell state into Ratatui and sanitize unexpected controls. Page-based terminal history uses revision-scoped parsed scrollback offsets without changing another attachment's viewport. One generation-scoped input lease permits key, paste, and resize operations; `Ctrl-]` releases it. Client close and detach leave sessions running. The complete key map, limits, and evidence procedure are defined in [the TUI workflow guide](docs/tui-workflow.md).

### 6.7 Git worktree adapter

The adapter SHOULD support:

- Detecting whether the workspace is a Git repository.
- Listing project-owned worktrees.
- Creating a worktree and branch for a task.
- Associating the resulting path with the task.
- Detecting conflicts such as an existing branch or path.

Deletion of worktrees and branches is excluded from the MVP because it can destroy uncommitted work. The product may display safe manual cleanup guidance.

The adapter MUST validate all generated paths, avoid shell string interpolation, and pass arguments directly to the Git process.

## 7. Functional requirements

### FR-1: Workspace lifecycle

The user MUST be able to initialize or open a workspace for an existing directory. Reopening it MUST restore tasks, handovers, definitions, and terminal session metadata.

### FR-2: Agent-neutral configuration

The user MUST be able to register an arbitrary interactive command as an agent definition. Built-in provider templates MUST NOT have capabilities unavailable to a custom adapter through the same interfaces.

### FR-3: Real terminal sessions

The user MUST be able to launch at least three concurrent PTY sessions, interact with full-screen terminal applications, resize them, switch between them, and observe their exit status.

### FR-4: Detach and reattach

Closing or disconnecting the TUI MUST NOT terminate active sessions. Reconnecting while the daemon remains alive MUST restore session listings and allow the user to attach to a running session.

### FR-5: Task lifecycle

The user MUST be able to create tasks, change valid statuses, claim and release tasks, and view claim history. Invalid transitions and concurrent claims MUST be rejected consistently.

### FR-6: Progress and handover

The user MUST be able to append progress and create a structured handover. Another session MUST be able to read this state without access to the previous agent's private chat history.

### FR-7: Worktree isolation

In a Git repository, the user SHOULD be able to create an isolated worktree for a task. The task MUST record the worktree path and branch. Non-Git workspaces MUST continue to function.

### FR-8: Diagnostics

The user MUST receive actionable errors for missing executables, failed process starts, database failures, unsupported terminals, IPC disconnections, and invalid task transitions.

### FR-9: Clean shutdown and recovery

The daemon MUST flush transactional state on orderly shutdown. On restart it MUST reconcile sessions that were previously marked running and expose their loss honestly.

### FR-10: Public-safe export

If the MVP exports a handover or diagnostic artifact into the project, the user MUST preview and confirm the exact content and destination. Export is optional and MUST NOT happen automatically.

## 8. Non-functional requirements

### NFR-1: Platform support

Continuous integration MUST build and test on current stable Rust for Linux, macOS, and Windows. The supported shell and terminal matrix MUST be documented.

### NFR-2: Reliability

One failed child process or disconnected client MUST NOT crash the daemon or terminate unrelated sessions. State transitions and related events MUST be atomic.

### NFR-3: Performance

On a typical developer machine, the TUI SHOULD become interactive within two seconds of connecting to an existing local daemon. UI input SHOULD remain responsive during high-volume terminal output. Scrollback and event buffers MUST be bounded.

### NFR-4: Compatibility

The IPC protocol and database schema MUST be versioned from their first committed implementation. Incompatible versions MUST fail with an actionable message rather than undefined behavior.

### NFR-5: Observability

Diagnostics MUST use structured severity levels and stable event names. Logs MUST avoid terminal content, environment values, command arguments likely to contain secrets, and personal filesystem paths unless the user explicitly enables a diagnostic mode.

### NFR-6: Accessibility and usability

Essential TUI state MUST not depend only on color. Keyboard bindings MUST be discoverable. The interface SHOULD adapt to small terminal sizes and clearly report when a minimum size is required.

### NFR-7: Maintainability

Domain and application crates MUST be testable without a terminal, PTY, SQLite file, Git repository, or provider CLI. Platform-specific code MUST be isolated behind traits or narrow modules.

### NFR-8: Resource limits

The daemon MUST configure bounds for scrollback, queued terminal frames, event history returned to clients, concurrent sessions, and diagnostic log retention.

## 9. Security and privacy requirements

### 9.1 Trust boundaries

The MVP assumes the local operating-system user and explicitly configured agent commands are trusted. Project content and child process output are not assumed to be safe for logs or publication.

The daemon is not a security sandbox. A child process normally inherits the launching user's filesystem and network permissions. This limitation MUST be stated in user documentation.

### 9.2 Local access control

- IPC endpoints MUST be restricted to the current user.
- Runtime directories and SQLite files MUST use restrictive permissions where supported.
- The daemon MUST reject connections that fail local peer validation when the platform exposes that capability.
- The MVP MUST NOT listen on TCP or expose an Internet endpoint.

### 9.3 Secrets

- Secrets MUST NOT be stored in agent definitions, task descriptions, handovers, events, or logs.
- Environment variables MUST be filtered before diagnostic serialization.
- Command arguments MUST be treated as potentially sensitive.
- Provider authentication MUST occur within the provider CLI or operating-system facilities.
- Tests MUST use obvious fake values such as `test-token-not-secret`.

### 9.4 Personal data

- Examples MUST use fictional names and synthetic paths.
- Repository fixtures MUST NOT include real usernames, email addresses, hostnames, account IDs, or conversation content.
- Local absolute paths SHOULD be represented as aliases in normal diagnostics.
- Crash reports and telemetry MUST NOT be transmitted automatically.

### 9.5 Input validation

- IPC payload sizes and terminal frame sizes MUST be bounded.
- Task and handover text lengths MUST have documented limits.
- Worktree and working-directory paths MUST be canonicalized and validated against the allowed workspace roots.
- External commands MUST receive argument arrays, not interpolated shell command strings.
- ANSI terminal output MUST never be rendered into logs or non-terminal UI without escaping or sanitization.

### 9.6 Repository hygiene

The initial repository SHOULD include:

- A `.gitignore` covering runtime databases, logs, socket files, terminal captures, local configuration, coverage output, and build artifacts.
- A secret-scanning job or pre-commit option using a documented open-source scanner.
- Dependency audit and license checks in CI.
- A `SECURITY.md` with a private vulnerability reporting path before the first public release.
- Synthetic fixtures reviewed for personal data.

## 10. Suggested repository structure

```text
.
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
├── PROJECT_VISION.md
├── MVP_TECHNICAL_SPEC.md
├── rust-toolchain.toml
├── deny.toml
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── security.yml
├── crates/
│   ├── relayterm-domain/
│   ├── relayterm-application/
│   ├── relayterm-protocol/
│   ├── relayterm-persistence-sqlite/
│   ├── relayterm-pty/
│   ├── relayterm-git/
│   ├── relayterm-agent-adapters/
│   ├── relayterm-daemon/
│   ├── relayterm-tui/
│   └── relayterm-cli/
├── migrations/
├── docs/
│   ├── architecture/
│   ├── decisions/
│   ├── privacy.md
│   └── supported-platforms.md
├── fixtures/
│   └── synthetic/
└── tests/
    ├── integration/
    └── end-to-end/
```

The bootstrap phase MAY start with fewer crates to reduce overhead, but it MUST preserve the dependency boundaries. Splitting a crate should follow demonstrated coupling or build needs rather than the diagram alone.

## 11. Configuration and runtime locations

Configuration SHOULD follow operating-system conventions through an appropriate directories crate.

Suggested logical separation:

- User configuration: agent definitions and user preferences.
- Application data: SQLite databases and durable workspace registry.
- Runtime data: sockets or named-pipe metadata and process locks.
- Cache: disposable derived data.
- Project directory: source code and only explicitly exported coordination artifacts.

Configuration MUST support a documented override for tests and portable development environments. Unknown fields SHOULD produce warnings, and invalid security-sensitive fields MUST fail closed.

Relayterm uses `RELAYTERM_HOME` as its single bootstrap override, with `config`, `data`, `runtime`, and `cache` children. An explicitly supplied internal `LocationOptions` value takes precedence. Overrides must be absolute and are the only supported way to place private state inside a project. Default resolution never falls back to the current directory. Location resolution has no filesystem side effects.

Accepted definitions in SQLite are authoritative. TOML uses `format_version = 1`, requires stable definition IDs, and is imported only by an explicit operation. Imports compare a captured workspace revision, apply supplied creates and updates atomically, preserve omitted definitions, and never rewrite the source file. Unknown non-security fields produce bounded warnings. Unknown security-shaped settings, invalid security values, and recognized credential patterns are rejected without echoing input.

## 12. Implementation phases

### Phase 0: Repository and architectural skeleton

Deliverables:

- Rust workspace with the agreed crate boundaries.
- A user-facing `rt` binary with placeholder TUI and daemon subcommands wired to separate application boundaries.
- License, contribution, security, privacy, and supported-platform documents.
- CI for formatting, linting, tests, builds, dependency audit, and secret scanning.
- Domain types and architecture decision records for process model, IPC, persistence, and PTY handling.
- Synthetic test fixtures only.

Exit criteria:

- All supported platforms build in CI.
- Dependency direction is enforced by crate boundaries.
- Repository scans contain no known secrets or personal data.

### Phase 1: Domain, SQLite, and local IPC

Deliverables:

- Workspace, agent, task, progress, handover, session, and event models.
- Validated task state machine and exclusive claims.
- SQLite migrations and transactional repositories.
- Versioned local IPC server and a minimal administrative CLI client.
- Restart reconciliation for persisted session metadata.

Exit criteria:

- Domain tests cover every allowed and rejected task transition.
- Persistence tests survive process restart and migration from the initial schema.
- Two clients observe ordered events and consistent snapshots.

### Phase 2: PTY supervision

Deliverables:

- Generic shell and custom command launch.
- Concurrent PTY sessions with input, output, resize, exit, detach, and reattach.
- Bounded scrollback and backpressure.
- Platform-specific integration tests where CI permits.

Exit criteria:

- Three concurrent interactive sessions operate independently.
- Disconnecting a client leaves sessions running.
- A failed session does not affect the daemon or other sessions.

### Phase 3: TUI workflow

Deliverables:

- Workspace, task, agent, terminal, handover, and diagnostic views.
- Discoverable keyboard controls and small-terminal handling.
- Reconnection and snapshot refresh.
- Actionable error presentation.

Exit criteria:

- The primary end-to-end scenario can be completed without the administrative CLI.
- TUI disconnection and reconnection do not lose durable state or running sessions.
- Essential information remains understandable without color.

### Phase 4: Agent templates and Git worktrees

Deliverables:

- Editable templates for Claude Code, Codex, and OpenCode.
- Generic adapter documentation for future CLIs.
- Git repository detection and explicit worktree creation for a task.
- Task-to-worktree association and conflict reporting.

Exit criteria:

- Each available provider CLI can be launched through the same neutral session workflow when installed and already authenticated.
- A custom interactive CLI receives equivalent lifecycle handling.
- Two tasks can use separate worktrees without changing each other's working directory.

### Phase 5: Hardening and release candidate

Deliverables:

- Cross-platform manual test matrix.
- Failure injection for client disconnects, child crashes, corrupt requests, and daemon restart.
- Privacy review, threat model, resource limit tests, and sanitized diagnostics.
- Installation, upgrade, backup, recovery, and troubleshooting documentation.

Exit criteria:

- All MVP acceptance criteria pass on Linux, macOS, and Windows.
- No critical or high-severity dependency audit findings remain without a documented exception.
- A clean machine can install and complete the documented quick start.

## 13. MVP acceptance criteria

The MVP MUST satisfy all of the following:

1. A user initializes a workspace in an existing project without writing runtime-private state into the repository.
2. The user starts Relayterm through `rt`; its daemon runs independently and the TUI client connects through user-restricted local IPC.
3. The user launches three concurrent real PTY sessions, at least one generic shell and two configured agent commands or synthetic interactive test agents.
4. Full-screen terminal behavior, input, resize, session switching, exit reporting, and bounded output work on every supported platform.
5. The user creates a task, claims it from one agent instance, and a second claim is rejected.
6. The active instance appends progress and creates a structured handover containing verification and next action.
7. A different instance reads the task and handover through the neutral workspace model and resumes the task.
8. Closing and reopening the TUI leaves child sessions running while the daemon remains alive and permits reattachment.
9. Restarting the daemon restores durable entities and marks unrecoverable former sessions accurately.
10. In a Git test repository, the user creates a task worktree and launches its session in the isolated directory.
11. The same core and protocol tests pass without importing TUI modules.
12. Linux, macOS, and Windows CI builds pass with formatting, linting, unit, integration, audit, and secret-scanning checks.
13. Logs and generated repository artifacts contain no injected test secrets, raw environment dumps, personal paths, or terminal transcripts.
14. No network service, provider API key, hosted account, desktop environment, or graphical client is required.
15. An unknown custom interactive command can be configured without modifying the domain or TUI crates.
16. `rt` is the only required user-facing executable, and a local naming conflict produces actionable installation guidance.

## 14. Explicit exclusions

The following MUST NOT be implemented as part of the MVP unless this specification is formally revised:

- Desktop, browser, mobile, or IDE extension clients.
- Remote TCP access, public daemon endpoints, or multi-user tenancy.
- Cloud synchronization or a mandatory hosted control plane.
- Provider API integrations or storage of provider API keys.
- Authentication automation, credential scraping, or bypass of provider terms.
- Autonomous task planning, model selection, bidding, voting, or swarms.
- Automatic parsing of agent prose to mutate authoritative task state.
- Full chat transcript ingestion or permanent terminal recording.
- Semantic vector search, embeddings, or retrieval-augmented project memory.
- Automatic commits, merges, rebases, branch deletion, or worktree deletion.
- Containers, virtual machines, or operating-system sandboxing as a security boundary.
- Collaborative text editing, code review hosting, or source-control hosting.
- Notifications, mobile push, billing, usage metering, or marketplace features.
- Backward compatibility guarantees before the first stable protocol release.

## 15. Testing strategy

### Unit tests

- Domain state machines and invariants.
- Redaction and serialization rules.
- Configuration validation.
- Path and command argument validation.
- Protocol framing and version negotiation.

### Integration tests

- SQLite migrations, transactions, and restart recovery.
- IPC request, response, subscription, disconnect, and malformed-frame handling.
- PTY lifecycle using synthetic interactive child programs.
- Git worktree creation in disposable repositories.
- Resource bounds and backpressure.

### End-to-end tests

- Daemon plus client plus synthetic agent lifecycle.
- Task claim, progress, handover, release, and resume.
- TUI detach and reattach where terminal automation is reliable.
- Cross-platform smoke tests for shell launch and resizing.

Provider CLIs SHOULD NOT be required in automated CI. Manual release checks MAY cover already authenticated installations, but test success MUST not depend on external accounts or network availability.

## 16. Definition of done for each change

A change is done only when:

- Its behavior is covered by proportionate automated tests.
- Formatting and lint checks pass with warnings treated according to repository policy.
- Relevant Linux, macOS, and Windows behavior is implemented or an explicit platform limitation is documented.
- Public examples and fixtures contain no personal data or secrets.
- Logs introduced by the change comply with redaction policy.
- User-facing or protocol behavior is documented.
- A migration and compatibility note exists for persistence or protocol changes.
- The change preserves the separation between core, daemon, adapters, and clients.

## 17. Decisions to record during bootstrap

The implementation team MUST create concise architecture decision records for:

1. Daemon scope, one per user or one per workspace.
2. IPC framing, authentication, protocol versioning, and terminal stream multiplexing.
3. PTY scrollback ownership and reattachment semantics.
4. SQLite location, backup, migration, and corruption recovery.
5. Environment inheritance and redaction policy.
6. Agent adapter configuration format and executable discovery.
7. Git worktree naming, allowed roots, and lifecycle ownership.
8. Minimum supported Rust version and platform support policy.

These decisions may refine implementation details but MUST preserve the product principles and acceptance criteria in this specification.

## 18. First implementation task

Start with Phase 0 and Phase 1. Do not begin by building terminal widgets.

The first vertical slice should:

1. Initialize a workspace outside version-controlled runtime state.
2. Start the daemon through `rt daemon` or an equivalent internal subcommand.
3. Connect a minimal `rt` CLI client through versioned local IPC.
4. Create and claim a task.
5. Append progress and a handover.
6. Restart the daemon.
7. Read the same durable state and ordered event history.

This slice validates neutrality, persistence, protocol boundaries, and testability before PTY and TUI complexity is added.
