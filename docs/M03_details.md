# M03: Private configuration and transactional SQLite

## 1. Status, deliverable, and scope

This is the implementation plan for M03 in the [pending queue](../TODO.md). Preparing this document completes only M03-DOC. It does not implement, verify, or close M03.01-M03.09. The specification and accepted ADRs remain authoritative; implementation must update affected contracts when introducing the explicitly identified application extensions below.

Prerequisites and references:

- [Project vision](../PROJECT_VISION.md) and [MVP technical specification](../MVP_TECHNICAL_SPEC.md), especially sections 5, 6.5, 9, 11, and phase 1.
- [M02 implementation plan](M02_details.md), [storage/application contract](M02_storage_contract.md), and [verification log](../avances.md).
- [ADR 0001](decisions/0001-daemon-scope.md): canonical workspace identity and short-lived registry OS lock.
- [ADR 0002](decisions/0002-local-ipc.md): consistent snapshot watermarks and explicit resynchronization.
- [ADR 0004](decisions/0004-sqlite-recovery.md): one private database per workspace, separate registry, transactional migrations, and non-destructive recovery.
- [ADR 0005](decisions/0005-environment-privacy.md): environment names only and bounded safe diagnostics.
- [ADR 0006](decisions/0006-agent-configuration.md): explicit TOML import/reload and SQLite as the accepted active configuration.
- [ADR 0007](decisions/0007-worktree-ownership.md): authorized roots; worktree creation remains M10.

M02 is complete. Its core already validates task/instance states, claims, progress, handovers, and typed events with deterministic services. M03 turns these contracts into durable storage, with private location resolution, a recoverable workspace registry, validated configuration imports, database constraints, and bounded durable queries.

M03 must prove that a separate process can reopen committed state and continue a handover or recover lost ownership. It does not implement daemon startup, production IPC, process launching, PTYs, provider integrations, a TUI, worktrees, automatic project export, or a production test-session mode. M04 supplies transport, M05 assembles the daemon/CLI, M06 verifies the administrative slice, and M07 connects real supervision. `rt` remains the only required user-facing executable.

## 2. Decisions and contracts to preserve

| Topic | M03 decision |
| --- | --- |
| Workspace storage | One private SQLite database per workspace; separate private SQLite registry |
| Identity | Opaque stable workspace ID, canonical root lookup, physical-directory identity checks where supported; no lowercasing arbitrary paths |
| Configuration | TOML is an explicit import candidate; accepted definitions in SQLite are authoritative |
| Reload conflict | Compare the workspace revision captured for the candidate; reject stale candidates without overwriting newer edits |
| Reload scope | Atomically upsert the explicitly supplied stable definition IDs; omission does not delete or disable definitions |
| Existing instances | Retain an immutable snapshot of the definition accepted for their launch attempt |
| SQLite writes | Short write transaction, revision comparison, complete batch validation, atomic row/history/event commit |
| SQLite reads | Consistent short read transaction; indexed bounded queries for history and events |
| Event retention | Retain all committed events in M03; no automatic pruning, renumbering, or retention job |
| Event identity | Durable uniqueness within the workspace database, plus collision-resistant generated IDs; cross-file global uniqueness is not a distributed transaction guarantee |
| Recovery | Preserve existing files on incompatibility/corruption; resume only recognizable interrupted initialization |
| Live-state reconciliation | Explicit internal operation after exclusive daemon ownership in M05, never a side effect of ordinary database reads |
| Backup direction | SQLite-consistent snapshot to a fresh private destination; full backup/restore user workflow remains M11 |

Keep M02 actor attribution, transition rules, informational dependencies, exclusivity, append-only history, and notification semantics unchanged. No SQL operation may bypass a domain guard because an adapter finds it convenient.

### 2.1 Extensions required by the actual M02 API

The current [application service](../crates/relayterm-application/src/lib.rs) has important boundaries that must be extended deliberately:

1. `Service::create_workspace` always generates an ID. A registry reservation needs a creation path that accepts its reserved identity through an internal, validated initialization request. It must not create another ID on retry.
2. `Request::AddDefinition` generates a new ID and cannot update an existing definition. Add stable-ID import/update commands and revision-aware application entry points; do not call `AddDefinition` repeatedly to simulate reload.
3. `AgentInstanceRecord` references a definition but has no immutable launch-definition snapshot. Add that snapshot before testing edits against existing instances.
4. `Service::history` slices a complete in-memory workspace snapshot. It does not provide bounded SQLite reads. Add a read port and query service for durable history/events and consistent watermarks, preserving the existing pure in-memory API where useful.
5. Existing storage errors are broad. Add safe, typed adapter failure categories and map them into application outcomes without forwarding SQLx, TOML, OS, or raw SQLite messages.
6. The architecture allowlist recognizes only six crates. Add the actual adapter crates and explicit dependency edges while retaining the forbidden core dependency checks.
7. The M02 test store scans all its workspaces for duplicate event IDs. Separate workspace databases cannot promise the same cross-file atomic reservation. Document per-workspace durable event-ID uniqueness explicitly, align the double and adapter contract tests to that scope, and preserve duplicate rejection within each workspace. Do not introduce a cross-database event ledger just to imply stronger atomicity.

These are M03 integration requirements, not a claim that M02 persistence already exists. Update M02's storage contract, relevant specification fields, in-memory doubles, and tests with these additions during implementation. Keep IPC version 1. Add explicit definition-update event variants with stable names; document that older pre-release decoders reject unknown variants rather than silently ignoring them.

## 3. Private locations and workspace identity

### 3.1 Location resolution

Introduce a platform adapter with injectable location inputs. Resolve operating-system directories through a reviewed directories library, then append stable Relayterm-specific components. Do not read or dump the entire environment. The selected library's platform mappings must be checked in native tests; [ProjectDirs](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html) is the candidate abstraction.

| Class | Default policy | Contents |
| --- | --- | --- |
| Configuration | OS user configuration location | Optional `config.toml`, explicit definition-import sources |
| Durable data | OS local application data location | `registry.sqlite3`, `workspaces/<workspace-id>/workspace.sqlite3` |
| Runtime | Validated owner-private runtime location; private local fallback when unavailable | Registry coordination lock; reserved space for M05 workspace locks/endpoints |
| Cache | OS user cache location | Disposable derived data; never authoritative state |

Use XDG conventions on Linux, appropriate Library locations on macOS, and known user folders on Windows. On Windows, workspace databases belong in local application data, not a roaming/synchronized location. On macOS, separate `config` and `data` subdirectories if the directory library maps both logical classes to the same Application Support root.

Define one explicit bootstrap override, `RELAYTERM_HOME`, with `config`, `data`, `runtime`, and `cache` children. An explicit `LocationOptions` value takes precedence over that environment variable; absent both, use OS defaults. Do not introduce a CLI flag before M05. Reject empty or relative override roots. Resolve an explicit override once, validate ownership/access, and pass the resulting locations to all adapters instead of reading environment variables repeatedly.

Default location resolution must not place private state inside the project root. An explicit `LocationOptions` or `RELAYTERM_HOME` override is the user-selected location exception allowed by the specification, including a deliberately selected project-contained portable directory. Document that exception and preserve private permissions; do not infer it from the current working directory. Tests use isolated temporary roots outside the synthetic project. No fallback writes into the current directory when a home/config/runtime directory is unavailable.

Resolving locations is side-effect free. Creation occurs only in initialization/open-for-write paths. Merely parsing configuration, checking an unknown workspace, or asking for locations must not create a database or directories.

### 3.2 Permissions and native paths

- Create Unix private directories with mode `0700` and files with mode `0600`, independent of a permissive umask; verify actual ownership and mode before use.
- On Windows, create and verify an explicit protected DACL permitting the current user and necessary administrative/system access, without ordinary-user or broad inherited access. Directory inheritance must cover SQLite sidecars and newly created files.
- Treat registry, workspace database, WAL, SHM, locks, temporary migration files, and backup destinations as sensitive.
- Inspect existing app-owned locations. Reject ownership mismatches, unexpected symlinks/reparse points in managed children, permissive access, and unexpected file types. Do not automatically chmod/chown or replace existing user data.
- Validate the explicitly selected parent root before creating managed children. A legitimate resolved user-directory alias is distinct from an unexpected link replacing a managed database or lock file.
- Use safe native APIs or reviewed safe wrappers. Preserve workspace `unsafe_code = "forbid"`; do not silently relax it or invoke a shell to manipulate ACLs. The dependency spike in M03.01 must demonstrate secure creation and access checks on Windows before this subtask closes.
- Keep paths as native `PathBuf`/`OsString` values. Persist roots and working directories through a tagged, reversible OS-native codec, not `to_string_lossy` or a connection URL built by interpolation.
- Decode Unix path bytes and Windows UTF-16 units only on the matching platform. Reject malformed or unsupported encodings with safe recovery guidance. Database portability does not imply that machine-specific paths can be used unchanged on another OS.

Use structured SQLx connection options with a filename and disabled statement logging. Do not log their `Debug` representation. [SQLx connection options](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html) expose filename, busy-timeout, journal, synchronization, and logging controls; verify the exact API against the version selected during implementation.

### 3.3 Root identity

Canonicalize an existing directory, resolving symlinks/junctions. Reject a missing root or a regular file. Derive an opaque lookup key from the native canonical path and retain a private filesystem identity guard when available, such as volume/device plus directory file ID. Do not use display names, Git remotes, usernames, or lowercased strings as identity.

Aliases resolving to the same directory must converge on one workspace. Native tests must cover case-sensitive and case-insensitive filesystem behavior rather than assuming that all macOS or Windows volumes behave identically.

A renamed/moved registered directory, a deleted-and-recreated directory at the same path, or a registry/path identity mismatch requires an explicit safe outcome. Preserve the existing ID and data; return a relocation/recovery requirement instead of silently reassigning old state or creating duplicate identity. Automatic relocation and registry repair commands are outside M03.

## 4. Registry and interrupted initialization

The registry has its own schema and migration history. Minimum row fields are `workspace_id`, native canonical-root encoding, root lookup key, optional filesystem identity guard, initialization state, and creation/update timestamps. Database location is derived from the workspace ID under the private data root, not supplied by untrusted project content.

Use a short-lived registry OS lock as required by ADR 0001, plus SQLite uniqueness constraints. Implement lock ownership through a safe native/file-lock abstraction, not PID existence or lock-file contents. A process crash releases the OS lock. Do not unlink a lock file while another process may hold it. Set a bounded acquisition deadline; never hold the registry lock while waiting for a daemon to become ready or a workspace initialization lock. All initialization paths must use that same lock order.

### 4.1 Initialization protocol

1. Resolve and validate the project root and private locations.
2. Acquire the registry lock and open/migrate the registry using non-destructive rules.
3. Look up the canonical root and identity guard. Return a validated existing ready registration if found.
4. Otherwise reserve one workspace ID and insert a durable `initializing` registry row. Concurrent callers use that same reservation.
5. Release the registry lock before lengthy workspace migration/initialization. Coordinate competing initializers through the workspace database and a narrowly scoped initialization lock, without inventing M05 daemon ownership.
6. Create the workspace database only for that reservation, with secure permissions and a recognizable initialization state.
7. Apply migrations, then atomically persist the workspace row, revision 1, and its single `WorkspaceCreated` event through application services using the reserved ID.
8. Reacquire the registry lock, validate the database identity and committed workspace, and mark the registration `ready`.
9. Return the same workspace ID on later opens without new creation events or definition re-imports.

Registry and workspace writes are not a single transaction. Do not use `ATTACH` plus WAL to claim cross-database atomicity. SQLite documents that WAL transactions are atomic per attached database, not across the set. [WAL constraints](https://www.sqlite.org/wal.html).

### 4.2 Crash states and recovery

| Observed state | Required response |
| --- | --- |
| No reservation and no managed database | Normal initialization |
| Reserved ID, database absent | Resume creation for that ID under initialization coordination |
| Reserved ID, recognizable empty/migrated database, no workspace row | Resume initial workspace transaction; do not generate another ID |
| Reserved ID, matching committed workspace and creation event | Mark ready without replaying creation |
| Ready registration and valid matching database | Reopen without mutation |
| Ready registration but missing database | Recovery-required error; never create a replacement |
| Existing file with unknown contents, wrong workspace ID, newer schema, or corruption | Preserve files and fail closed |
| Orphan database without a registry reservation | Preserve it; no automatic adoption, deletion, or scanning of unrelated directories |

For failures between steps, test both immediate retry and retry in a new process. Competing initializers must converge or return a bounded busy/recovery outcome, never two ready IDs for one root. Keep partial artifacts available for diagnosis without exposing their contents in logs.

## 5. Configuration and explicit import

### 5.1 Candidate format

Use versioned TOML with `format_version = 1`. Definition entries contain stable IDs, display names, command, argument arrays, environment allowlist names, neutral capabilities, and enabled state. Require IDs on imported definitions; keep the existing generated-ID creation use case for programmatic additions.

Parse into dedicated DTOs, then invoke domain validators. Do not derive unrestricted deserialization on domain entities. Read a bounded file before parsing and reject duplicate TOML keys, wrong types, unsupported format versions, duplicate definition IDs, invalid identifiers, and M02 text/list violations.

Proposed initial parser and storage settings:

| Input | Default or bound |
| --- | --- |
| Configuration file | At most 1 MiB before parsing |
| Definitions per import | At most 128 |
| Definition fields | Existing M02 byte/list/control/secret limits |
| `storage.busy_timeout_ms` | Default 5,000; permitted 1 through 30,000 |
| `storage.max_connections` | Default 4; permitted 1 through 16 per opened database pool |
| History/event page size | Default 50; maximum 200, independent of parser input |
| Serialized event payload | Maximum 16 KiB per payload; reject before unbounded decoding |
| `preferences.color_mode` | `auto`, `always`, or `never`; presentation consumer belongs to M08 |

Storage settings are immutable for a constructed adapter/pool. A file edit does not alter an existing pool. M03 parses settings into a validated startup snapshot; M05 defines application restart/reload presentation. Do not implement unused PTY, remote endpoint, telemetry, or provider-credential settings.

Unknown non-security fields generate bounded warnings with a stable warning code and safe structural location, not arbitrary key/value echoes. Recognized credential-shaped content is rejected even in ignored fields. Unknown fields purporting to provide environment values, credentials, arbitrary SQL/PRAGMAs, extensions, or permission overrides are rejected rather than silently accepted. Invalid known security-sensitive fields always fail closed. TOML parser diagnostics must not include source excerpts, secrets, private paths, or raw input keys.

### 5.2 Active-state and conflict behavior

1. Read and validate the entire candidate without touching active definitions.
2. Resolve the target workspace and capture its current revision.
3. Produce a proposed create/update set keyed by stable IDs and a baseline revision. A future client can review this candidate; M03 tests the application contract without adding UI.
4. On explicit apply, begin a transaction and compare that baseline revision with current durable state.
5. Reject stale candidates as `Conflict`; do not silently rebase or overwrite. An unrelated workspace mutation may conservatively require re-reading the candidate because M02 uses a workspace-wide revision.
6. Validate every entry, then apply all supplied creations/updates and their typed events in one commit. One invalid entry aborts the entire import.
7. Leave omitted definitions intact. No implicit deletion, disabling, or source-file rewriting.
8. An identical validated import with a matching baseline is a no-op. A stale baseline still conflicts, even if the candidate happens to match the latest fields.

Opening a workspace loads accepted active definitions from SQLite. It does not automatically import the configuration file, watch it, or synchronize it bidirectionally. An explicit reload may replace accepted fields with the validated candidate, including changes made through future IPC, only against a current baseline.

Commands remain command-plus-argument arrays. Empty commands fail registration; a syntactically valid executable that is not installed may remain configured. Real executable discovery and launch-time failure belong to M07/M09, so M03 tests must not require a provider installation or execute imported commands.

### 5.3 Existing-instance configuration

Add a neutral immutable `LaunchDefinitionSnapshot` to configured instance launch metadata. Capture it from the accepted definition inside the same transaction that registers the launch attempt. Do not accept a caller-supplied snapshot that can disagree with the active definition.

The snapshot preserves stable definition ID, command, arguments, environment names, capabilities, display name, and enabled state at acceptance. It contains no environment values or resolved credentials and has no content-bearing `Debug`. Generic-shell instances have no definition snapshot; their actual shell resolution remains M07.

Definition edits affect future launch attempts only. Existing instances keep their captured data and attribution through reload, disablement, exit, and reopen. Persist the snapshot in immutable instance-owned rows with ordered children. Extend reconstruction validation and the M02 in-memory tests accordingly; no SQLx dependency enters the core.

## 6. Crates, dependencies, and production adapters

| Component | Responsibility and allowed dependencies |
| --- | --- |
| `relayterm-domain` | Existing rules plus neutral definition updates and launch snapshots; no filesystem, SQLx, Tokio, config parser, or UI |
| `relayterm-application` | Initialization/import/query contracts and existing transaction services; depends on domain only among project crates |
| `relayterm-platform` | Private locations, native path/identity/permission checks, registry/init OS locks, production clock and IDs; narrow platform dependencies |
| `relayterm-config` | Bounded TOML parsing, warnings, validated candidates; domain/application contracts, no SQLite or UI |
| `relayterm-persistence-sqlite` | Registry/workspace migrations, SQLx adapters, codecs, bounded queries, error mapping; application/domain and necessary platform file-opening helpers |
| Existing daemon/client crates | Remain placeholders until their milestones; no direct SQLite access from TUI or CLI |

Add these adapter crates only when implementing their first real functionality. Update the architecture test allowlist with exact edges and add negative tests for core-to-adapter, TUI-to-SQLite, renamed, optional, target-specific, and transitive dependency leaks.

Dependency selection is an implementation task, not an instruction to install packages during this documentation delivery. Evaluate and lock SQLx with only SQLite/migration and the chosen Tokio runtime features, a TOML parser, an OS-directory resolver, and safe native permission/lock helpers against Rust 1.98.1, licenses, advisories, and all native platforms. Use bundled SQLite for a reproducible engine unless a documented build constraint requires otherwise. Do not enable PostgreSQL, MySQL, TLS, runtime extension loading, or SQL statement logging accidentally.

Require an SQLite engine containing the WAL-reset fix. The SQLite project identifies 3.51.3 and later as fixed, with specific earlier backports; for this new implementation, select a supported patched release at least 3.51.3 and verify `sqlite_version()` in CI rather than trusting the host executable. This is relevant to the required concurrent-writer/checkpoint tests. [SQLite WAL-reset advisory](https://www.sqlite.org/wal.html#walresetbug).

The dependency spike must prove: exact feature graph, engine version, safe Windows ACL creation/inspection, Tokio confined to adapters/tests, cancellation-safe SQL transaction cleanup, native filenames without lossy conversion, and migrations from checked-in files. If a candidate library cannot satisfy a contract, replace it or record the specific technical blocker before coding around the limitation. Do not relax a security or architectural invariant silently.

## 7. Relational schema and codecs

### 7.1 Version boundaries

Registry schema version, workspace SQL migration version, `WorkspaceRecord::schema_version`, configuration format version, event payload version, and IPC version are separate concepts. Document their mapping. The initial M03 workspace model/schema begins at 1; SQLx's ordered migration ledger records individual migration versions and checksums. Do not treat a domain version of 1 as permission to ignore a newer migration ledger.

Use separate checked-in registry and workspace migration directories under `migrations/`. Apply SQL from these files without generating it from user content. Prefer bound runtime queries initially; use compile-time SQLx macros only if their offline metadata can be generated from synthetic databases reproducibly, without private connection strings or live-database requirements during builds.

### 7.2 Encoding rules

| Value | Storage representation and validation |
| --- | --- |
| Typed UUID | 16-byte BLOB with a length check; restore the correct wrapper type |
| `u64` revision, sequence, append ordinal | Fixed-width 8-byte big-endian BLOB, compared as BLOBs; increment in checked Rust arithmetic |
| Timestamp | Signed Unix seconds INTEGER plus nanoseconds INTEGER in 0 through 999,999,999; use Euclidean split for negative timestamps |
| Native filesystem path | Platform/codec tag plus lossless BLOB; never a lossy diagnostic string |
| Narrative text and relative paths | UTF-8 TEXT with domain byte limits; SQL byte checks use BLOB length, not character count |
| Boolean | INTEGER restricted to 0 or 1 |
| Status, priority, closure reason, actor kind | Stable `snake_case` TEXT restricted to the supported variants |
| Actor identity | Actor kind plus nullable instance ID with a matching nullability constraint |
| Event payload | Bounded JSON for the typed payload only, with explicit payload version and envelope columns |

The counter encoding deliberately preserves the full M02 `u64` range despite SQLite's signed INTEGER range. Never cast silently to `i64`, order decimal strings lexically, or use `AUTOINCREMENT` as the workspace event counter. Tests must cover zero, signed-boundary crossings, and `u64::MAX` exhaustion. Sequences and material revisions begin at 1; the absent-workspace snapshot uses revision 0 without a workspace row.

Timestamp codecs must preserve nanosecond precision, including dates before the Unix epoch and the full supported `Timestamp` range. Enforce paired nullability for optional timestamps; compare second/nanosecond tuples for temporal constraints. Decode through the validated domain constructor.

Native-path storage must apply M02 limits to the decoded native path, not incorrectly impose a UTF-8 byte bound on UTF-16 storage. Bound the raw native payload separately (at most 8 KiB for the supported codecs), validate tags/unit alignment, and test the maximum-length Windows case. SQLx migration ledger identifiers retain their own library-defined representation; they are not workspace `u64` counters.

### 7.3 Workspace tables

Every entity/reference table includes the workspace ID even though the database belongs to one workspace. Keep a singleton metadata/identity row and reject an open handle whose expected ID disagrees with it. Use composite foreign keys to prevent cross-workspace references inside malformed or test databases.

| Table or family | Main contents and constraints |
| --- | --- |
| `workspace_meta` | Expected workspace ID, current revision, last event sequence, event retention floor, model schema version |
| `workspaces` | M02 workspace fields and native-root codec; exactly one workspace identity per file |
| `agent_definitions` | Current stable ID, workspace ID, display name, command, enabled state |
| Definition arguments | Ordered ordinal/value rows; repeated argument values allowed |
| Definition environment/capabilities | Validated sets; preserve names and enforce native environment-name equality rules |
| `tasks` | Title, description, priority, status, acceptance notes, creation/update timestamps; null worktree reference until M10 |
| Task scope/dependency rows | Ordered scope paths and distinct dependency references; dependency self-reference forbidden, cycles permitted |
| `agent_instances` | IDs, one-to-one session ID, optional definition/task context, native working directory, status, all lifecycle timestamps, exit code, terminal dimensions |
| Instance launch-definition rows | Immutable captured definition plus ordered arguments and sets, one snapshot per configured instance |
| `claims` | Immutable opening identity/actor/time, one-time nullable closure actor/time/reason, opening event sequence |
| `progress_entries` | All M02 fields, immutable, plus creation event sequence for append order |
| `handovers` and changed paths | All M02 fields and ordered paths, immutable, plus creation event sequence |
| `workspace_events` | Complete validated event envelope, unique event ID and workspace sequence |

Derive `TaskRecord::claimed_by_instance_id` from the current open claim during loading instead of maintaining an independent owner column that can drift. If implementation chooses a cached owner column for measured reasons, it must add and test an explicit equality invariant before adopting it.

Use partial unique indexes for one open claim per task and one open claim per instance. Add uniqueness for session IDs, child ordinals, dependency pairs, set values, and event IDs. Use foreign keys for both sides of dependencies, all historical attributions, and event-linked creation ordinals. Enforce foreign keys on every pooled connection and verify the setting before opening transactions. [SQLite foreign-key enforcement](https://www.sqlite.org/foreignkeys.html#fk_enable).

Do not pretend that an ordinary SQLite `CHECK` can validate an arbitrary cross-table aggregate. Indexes and foreign keys enforce identity, reference integrity, and claim exclusivity. Active-task/open-claim equivalence, valid running ownership, complete handover bundles, temporal/history relationships, and import semantics are additionally checked under the write lock by domain reconstruction and batch validation before commit. Intermediate row states may exist only inside that transaction.

Append-only tables reject UPDATE/DELETE through normal runtime access, with defensive triggers for progress, handovers, captured launch configuration, and retained events. Claims permit exactly one update of closure fields, leaving opening identity and attribution unchanged; reject deletion and changes after closure. Avoid `INSERT OR REPLACE`, cascaded history deletion, or whole-snapshot delete/reinsert persistence.

Editable child sets may be replaced within the parent mutation transaction. Bound counts and byte totals before writing, use row/ordinal checks where expressible, and validate decoded sets on reopening. Domain dependencies remain informational; add no scheduler, topological sort, or automatic status propagation.

### 7.4 Field coverage checklist

Use the [M02 record inventory](M02_storage_contract.md) as the mandatory row-codec checklist. A field may be derived from a constrained relation, but it must never disappear on round-trip:

| Entity | Required domain fields to reconstruct |
| --- | --- |
| Workspace | `id`, `display_name`, `project_root`, `created_at`, `updated_at`, `schema_version` |
| Definition | `id`, `workspace_id`, `display_name`, `command`, `arguments`, `environment_allowlist`, `capabilities`, `enabled` |
| Task | `id`, `workspace_id`, `title`, `description`, `priority`, `scope_paths`, `acceptance_notes`, `dependency_ids`, `status`, derived `claimed_by_instance_id`, null `worktree_id`, `created_at`, `updated_at` |
| Instance/session | `id`, `session_id`, `workspace_id`, `agent_definition_id`, `task_id`, `working_directory`, `status`, `started_at`, `last_observed_at`, `ended_at`, `exit_code`, `terminal_size`, plus the new immutable launch-definition snapshot |
| Claim | `id`, `workspace_id`, `task_id`, `instance_id`, `requested_by`, `opened_at`, `closed_at`, `close_reason`, `closed_by` |
| Progress | `id`, `workspace_id`, `task_id`, `agent_instance_id`, `summary`, `verification`, `created_at` |
| Handover | `id`, `workspace_id`, `task_id`, `from_agent_instance_id`, `summary`, `decisions`, `changed_paths`, `verification_performed`, `open_questions`, `recommended_next_action`, `created_at` |
| Event | `sequence`, `event_id`, `workspace_id`, `event_type`, `entity_id`, `timestamp`, `actor`, `payload_version`, `payload` |

Test non-default values and both sides of every nullable field. Test reordered child rows, equal timestamps, all enum variants, disabled definitions, native path encodings, and historical user attribution. Database adapter metadata, such as append ordinals and initialization states, must be checked in addition to these domain fields.

## 8. SQLite connection and commit protocol

### 8.1 Connection policy

Use WAL and `synchronous = FULL` for workspace and registry databases, with foreign keys enabled and verified on every connection. Set the bounded busy timeout explicitly and use a small bounded pool. Parameterize all user values. Static PRAGMA names/values come from validated code settings, not raw TOML.

Verify the actual journal mode rather than assuming a request succeeded. Private database storage must be on a filesystem that supports the required local locking/shared-memory behavior. WAL is not a network-filesystem coordination mechanism; report unsupported storage rather than silently changing durability or locking behavior. [SQLite WAL limitations](https://www.sqlite.org/wal.html).

Keep read transactions short. Use passive/automatic checkpointing initially; do not equate a checkpoint with application commit, or delete sidecars manually. A blocked checkpoint must not report an already committed mutation as failed. M11 owns stress/resource tuning and broader storage-failure qualification.

### 8.2 Preserve optimistic M02 transactions

`Store::begin(workspace_id)` loads one consistent validated `Snapshot` and revision in a short read transaction, then releases that SQL read transaction. The application transaction object retains the snapshot and store handle, not a write lock while domain work runs.

At commit:

1. Acquire a connection and start a short write transaction with `BEGIN IMMEDIATE` through a cancellation-safe SQLx transaction wrapper.
2. Reload current revision and the state needed for `WriteBatch::validate` inside that write transaction.
3. Compare the expected revision. A stale transaction returns `Conflict`, including a stale no-op.
4. Validate the before/after relationship, all references, actors, state/history invariants, and pending event IDs.
5. Compute the row delta from current state to `WriteBatch::state()`. Update mutable records, close open claims once, and append new history rows. Never rewrite old progress, handovers, or closed claims.
6. Assign consecutive event sequences from the stored counter using checked arithmetic; build `WorkspaceEvent::confirm` records. Relate new history rows to their corresponding event sequence, not a timestamp-derived order.
7. Persist row deltas, events, revision, and last sequence in the same transaction. Check database constraints and final decoded invariants before committing.
8. Commit and only then return `Committed`. A material mutation increments revision once, even if it emits several events.
9. Call the notifier after commit. Notification failure leaves a successful durable result with `notification_delivered = false`.

SQLite permits one writer at a time; `BEGIN IMMEDIATE` acquires the write transaction before the validation read and can return busy. This avoids promoting the original stale read snapshot into a writer. [SQLite transaction semantics](https://www.sqlite.org/lang_transaction.html).

An unchanged exact final observation or identical current-revision import writes no rows/events and leaves revision/counters unchanged. Do not reserve sequences outside the transaction or create holes when a mutation rolls back.

### 8.3 Failures and cancellation

- Busy while acquiring the write transaction is a bounded storage-busy result, not proof that a domain claim conflict occurred. Once the lock is acquired, revision comparison determines `Conflict`.
- Never automatically replay a mutation after an uncertain commit response. M04 defines duplicate-safe client request semantics; M03 must preserve the distinction between known rollback and unknown outcome.
- Use explicit rollback on ordinary failure. Dropped/cancelled futures must roll back or discard their connection before it is returned to the pool; test cancellation both before and after row writes.
- A database or I/O error may affect the whole transaction, so do not rely on a failed statement alone undoing preceding statements. Return a safe category and verify the next connection observes either the whole old commit or the whole new commit, never a mixture.
- A process killed after successful commit but before notification must leave exactly one durable mutation and its events. The next process discovers them without replaying the original command.

## 9. Durable queries, watermarks, and retention

Add an application read port with operations for workspace snapshots, definitions/tasks, task history, and event pages. Pure contracts carry domain types and bounded requests; SQLx row types stay in the adapter.

A consistent snapshot result includes `workspace_id`, `revision`, `last_sequence`, and `retained_from_sequence`, obtained with relational rows in the same SQL read transaction. Event queries accept `after_sequence` and a validated page limit, order by sequence, and use an index range plus SQL LIMIT. No client-visible query may first load an unbounded event/history collection and truncate it in Rust.

History uses creation/opening event sequences as stable append order, with per-task indexes. Preserve M02's offset-page compatibility when needed, but introduce an explicit keyset cursor for durable reads. A closed-claim view can change after a previous page was read, so carry revision and define whether a subsequent page requires that same revision; use explicit `SnapshotChanged` rather than pretending pages form a single historical snapshot.

| Cursor situation | Response |
| --- | --- |
| Initial `after_sequence = 0` and retained floor 1 | First event page |
| Cursor at last committed sequence | Empty page with current watermark |
| Cursor between retained floor minus one and last sequence | Subsequent ordered page |
| Cursor older than retained coverage | Explicit `ResnapshotRequired` |
| Cursor ahead of the database watermark | Explicit invalid-cursor error, no silent rewind |
| Workspace missing or identity mismatched | Not-found/reference or storage-identity error |
| Unsupported payload/version or corrupt row | Explicit safe compatibility/integrity error; no silent skipping |

M03 retains all events, so normal retention floor remains 1. Implement and test the expired-cursor contract using a controlled retention fixture, not a production pruning feature. Events remain private durable data; retaining them does not authorize project export.

New definition-update payloads remain content-free. Validate event envelope type/identity against payload, version, byte bound, sequence, and workspace on decode. Unknown events never disappear from a stream without an error.

Keep M02's whole-workspace mutation snapshot initially; document its scaling limit. Indexed history/event reads must be bounded independently. Do not claim that M03 solves arbitrarily large workspace mutation cost or all M11 resource limits.

## 10. Migrations, compatibility, and recovery errors

### 10.1 Safe open sequence

1. Validate private location, permissions, expected file type, and requested access mode.
2. Distinguish explicit creation of a reserved workspace from opening an existing one. Open-for-read or reopen must not use `create_if_missing`.
3. Inspect database identity and migration compatibility before issuing persistent configuration changes or migrations. Reject newer schemas and unrecognized nonempty files without migration or replacement.
4. Verify migration ordering/checksums and acquire the migration/init coordination lock.
5. Apply pending supported migrations transactionally, validating constraints and version metadata before commit.
6. Configure the validated writable connection policy and load rows through checked codecs and entity reconstruction.
7. Return a usable handle only after identity, revision, state, and event metadata agree.

SQLite may perform necessary engine recovery on open; do not promise byte-for-byte immutability of active WAL files during recovery. Application-level refusal must avoid schema rewrites, deletion, truncation, automatic re-creation, or destructive repair. Test preservation using quiescent copies and compare logical contents as appropriate.

An empty explicit-new database is schema version 0 and migrates to the initial schema. An arbitrary existing empty file without a matching initialization reservation is not permission to create a workspace.

### 10.2 Migration test policy

Test both registry and workspace migrations from empty state and reopening populated initial-schema databases. Add a test-only follow-up migration or failpoint to demonstrate rollback after partial migration work and preservation of existing rows; do not invent an unnecessary production schema version solely to make a migration test nontrivial.

Once a production migration is published, do not edit it in place. Add subsequent migrations and fixture-based compatibility tests. Cover unknown future versions, altered checksums, interrupted upgrades, concurrent migrators, newer event payloads, and inconsistent domain/SQL version metadata. Never downgrade or reset a migration ledger automatically.

### 10.3 Error contract

| Failure | Safe caller outcome |
| --- | --- |
| Missing workspace/registration | Not found, without creating files |
| Registry/root/database identity mismatch | Recovery required; preserve IDs and originals |
| Permission or ACL rejection | Access denied with a location alias |
| Lock timeout / SQLite busy | Bounded busy outcome; no mutation replay |
| Stale revision / duplicate claim | Conflict |
| Invalid TOML/domain/reference | Typed validation/reference error and safe structural field |
| Read-only database on mutation | Read-only failure, without replacement or fallback |
| Newer schema or unsupported payload | Incompatible version with supported-version metadata |
| Corrupt rows, failed integrity check, wrong database | Storage integrity/recovery required |
| Migration checksum or execution failure | Migration failure preserving prior committed data |
| Disk full / I/O failure / cancellation | Typed storage failure; known rollback or explicit uncertain outcome |
| Notifier failure after successful commit | Successful durable result plus notification status |

Permit only stable codes, allowlisted field names, opaque IDs, revision/version numbers, and approved path aliases in normal diagnostics. Do not forward raw SQL text, bound values, parser excerpts, connection strings, filesystem paths, or source chains containing them. Use a safe recovery hint such as checking permissions or opening a private backup, not automatic destructive recovery commands.

### 10.4 Backup and read-only handling

Select `VACUUM INTO` as the initial documented SQLite-consistent snapshot mechanism, subject to the chosen SQLx adapter's capability test. It writes to a fresh private destination, uses a bound path, and must be validated before being considered a backup. It cannot be wrapped in an already active transaction. A interrupted destination is not a valid backup; preserve the source. [SQLite VACUUM INTO](https://www.sqlite.org/lang_vacuum.html#vacuum_with_an_into_clause).

M03 demonstrates that the selected mechanism reads committed WAL-backed state in an isolated test and documents restore into a fresh destination with the daemon stopped. Full backup/restore commands, scheduling, retention, and destructive replacement remain M11 or an explicit future user action. Never copy only a live database's main file.

Do not enable SQLite immutable mode on an ordinary live/read-only database to suppress locking errors. A read-only open must use correct read-only semantics and preserve WAL recovery requirements. Failed writable open does not silently switch to a read-only success result.

## 11. Atomic task breakdown

Each child ID is a reviewable pending outcome under its existing M03 parent. Complete and verify children in dependency order; keep a parent pending until all its children pass.

### M03.01: Private locations and canonical identity

1. M03.01a: Specify location inputs, default mappings, override precedence, side-effect-free resolution, and public-safe aliases.
2. M03.01b: Select and demonstrate safe Unix/Windows directory, file, ACL, and OS-lock APIs while preserving forbidden unsafe code.
3. M03.01c: Implement secure managed directory creation and checks for existing ownership, permissions, file types, and links/reparse points.
4. M03.01d: Implement native path codecs, canonical root resolution, filesystem identity guards, and case/alias behavior.
5. M03.01e: Implement registry/init lock primitives and bounded acquisition, without M05 daemon lifecycle.
6. M03.01f: Document default locations and `RELAYTERM_HOME`, including unavailable-directory and project-contained-override behavior.

Verification: no write on resolution; secure creation under permissive umask; actual Windows DACL inspection; aliases, case-sensitive volumes, Unicode/native paths, absent directories, wrong owner, rejected links, portable override isolation, and lock release after process exit. Add a negative access test with another security context where the CI environment permits; record a limitation rather than replacing ACL inspection with a mocked Boolean.

### M03.02: Configuration parsing and import contracts

1. M03.02a: Implement versioned bounded TOML DTOs for stable-ID definitions, preferences, and supported storage settings.
2. M03.02b: Map values through domain validators and produce bounded safe warnings/errors for unknown or invalid fields.
3. M03.02c: Add neutral definition-update/import commands, typed update fields/events, and expected-workspace-revision checks.
4. M03.02d: Add immutable launch-definition snapshots and capture them transactionally for configured instances.
5. M03.02e: Implement candidate validation, all-or-nothing apply semantics, identical-import no-op, and no implicit deletion/source rewrite.
6. M03.02f: Update ADR 0006's reload conflict details, affected specification fields, and M02 contracts/tests.

Verification: exact file/entry/field bounds; malformed and duplicate TOML keys/IDs; unknown-field warnings without input echo; secret-bearing ignored fields; first import; edit-without-reload; explicit reload; stale candidate; batch rejection; disablement; unchanged running-instance snapshots; missing installed executable accepted as configuration; no command execution.

### M03.03: SQLite adapter and initial schema

1. M03.03a: Pin compatible dependencies, verify native bundled engine version and the patched WAL requirement, and update architecture allowlists with negative tests.
2. M03.03b: Implement connection factories for explicit-new, reopen, and read-only modes with safe logging and verified PRAGMAs.
3. M03.03c: Add separate registry/workspace migration files and compatible version/checksum handling.
4. M03.03d: Implement typed UUID, unsigned-counter, timestamp, actor, enum, native-path, and event codecs.
5. M03.03e: Create relational tables, ordered children, foreign keys, partial unique indexes, append-only protections, and query indexes.
6. M03.03f: Implement full validated row reconstruction and before/after row-delta computation without historical rewrites.

Verification: fresh migrations; every field in the updated inventory round-trips; invalid encodings/statuses/versions fail safely; foreign keys checked on every pool connection; direct SQL attempts at duplicate open claims and illegal historical writes fail; full `u64` and timestamp boundaries; cross-workspace malformed rows rejected; no core adapter dependencies.

### M03.04: Initialization, reopening, and active definitions

1. M03.04a: Implement registry reservation states and one identity per canonical root under the registry OS lock.
2. M03.04b: Add the internal reserved-ID workspace creation path and idempotent initialization coordination.
3. M03.04c: Implement interruption recovery for each registry/database boundary in section 4.
4. M03.04d: Implement durable definition create/update/import using the M03.02 candidate contract.
5. M03.04e: Reopen existing workspaces and definitions through validated application/query ports without imports or lifecycle mutations.
6. M03.04f: Expose safe registry lookup results for M05 without exposing private paths in normal diagnostics.

Verification: simultaneous initialization in independent processes; repeated open returns one ID/creation event; two roots remain isolated; crash after reservation/migration/workspace commit/ready marking; read of an unknown workspace creates nothing; ready-but-missing/corrupt storage never recreated; accepted definitions survive source-file edits and reopening.

### M03.05: Transactional tasks and claims

1. M03.05a: Implement M02 `Store` and `Transaction` using short snapshot reads and write-lock revision validation.
2. M03.05b: Persist task create/edit/transition/dependency changes with their exact typed events.
3. M03.05c: Persist claim acquisition, one-time closure, and ownership reconstruction with uniqueness constraints.
4. M03.05d: Implement atomic revision/event-counter updates and event-ID collision rejection.
5. M03.05e: Add rollback/cancellation handling and distinguish busy, stale conflict, and uncertain outcomes.
6. M03.05f: Exercise real database races using two independently opened clients against the same revision.

Verification: all M02 state/actor guards retain their behavior with durable reopen; one claim winner; one `TaskTransitioned` and one `ClaimOpened` event for that winner, not one event total; losing writer leaves no rows/history/events; one instance cannot claim two tasks; release/reopen/reclaim preserves immutable history; informational dependency cycles remain non-blocking; failure at each write stage rolls back the whole batch.

### M03.06: Durable progress and handovers

1. M03.06a: Persist progress and user corrections as new append-only rows.
2. M03.06b: Atomically persist handover content, changed paths, task state, claim closure, and all three M02 handover events.
3. M03.06c: Add bounded indexed task-history query methods with append-order cursors and revision semantics.
4. M03.06d: Verify actor attribution and old-context reads after a new process opens the workspace.
5. M03.06e: Enforce reconstruction consistency between handover records and claim-closure history.

Verification: required verification/next action; exact aggregate size limits; immutable prior entries; user annotations after completion; foreign/non-owner rejection; paginated boundaries and equal timestamps; injected failure between each part of a handover; complete context continuity across two independent instances/processes.

### M03.07: Session metadata, events, and watermarks

1. M03.07a: Persist complete instance/session metadata and immutable accepted launch snapshots.
2. M03.07b: Persist final observations and lost-instance reconciliation with atomic claim closure/task blocking.
3. M03.07c: Implement consistent snapshot/revision/event watermarks from one read transaction.
4. M03.07d: Implement bounded indexed event pages, strict decoders, cursor validation, and retention-floor metadata.
5. M03.07e: Preserve no-op observations without sequence/revision/notification changes.
6. M03.07f: Document the M04 handoff for subscribe-after-watermark, resnapshot, uncertain mutation outcomes, and no pruning in M03.

Verification: startup failure without invented exit code; observed nonzero exit; repeated final observations; one workspace's reconciliation leaves other workspaces untouched; ordinary reopen does not mark instances lost; writer/snapshot interleavings yield matching rows and watermarks; no gaps/duplicates after the watermark; expired/future/unknown-version cursor fixtures; notifier failure after durable commit.

### M03.08: Migrations and non-destructive errors

1. M03.08a: Validate supported schema histories, model versions, migration checksums, and expected database identity before mutation.
2. M03.08b: Implement safe categorized open/migration/permission/busy/read-only/corruption outcomes and recovery hints.
3. M03.08c: Test transactional migration rollback with populated initial-schema fixtures and a test-only failing successor.
4. M03.08d: Test concurrent migration, cancelled SQL futures, interrupted initialization, disk-full/write-failure injection, and connection reuse after rollback.
5. M03.08e: Validate the chosen SQLite-consistent backup mechanism and document fresh-destination restore constraints without adding production restore commands.
6. M03.08f: Scan diagnostic paths and serialization for injected synthetic secrets, private path markers, and raw parser/SQL context.

Verification: newer/altered/unknown schema refuses safely; old rows survive failed migration; originals remain available; write attempts against read-only storage fail without fallback; lock deadlines are bounded; a killed writer leaves valid old-or-new state; consistent snapshot includes committed WAL data; no destructive automatic recovery.

### M03.09: Persistence gate and native evidence

1. M03.09a: Build the separate-process continuity and recovery journeys in section 12.
2. M03.09b: Run race, crash-boundary, registry-convergence, rollback, migration, permission, and privacy suites on real SQLite files.
3. M03.09c: Verify all M02 core tests and architecture negative controls remain green independently of adapters.
4. M03.09d: Run native Linux, macOS, and Windows CI plus pinned-compiler Linux and security checks.
5. M03.09e: Review field/constraint/requirement coverage and the M04-M07 handoff, with explicit evidence for every required scenario.
6. M03.09f: Update platform documentation, remove only verified M03 tasks, and append exact commands, candidate revision, and CI runs to `avances.md`.

M03 remains open if required native, permission, migration, or persistence evidence is missing. A successful in-memory test or cross-compilation is not a substitute for the corresponding real-file/native test.

## 12. Test design and gate journeys

### 12.1 Separate-process continuity

Use a test-only child harness compiled as an integration-test target, not an installed product executable or production CLI flag. Give it an explicitly isolated state root and synthetic project. Child stdout contains only allowlisted stage tokens, opaque IDs, and results; detailed test context stays in memory.

1. Process A resolves/initializes the workspace and explicitly imports two neutral definitions.
2. Register synthetic launch attempts and running observations through test-only orchestration of internal services.
3. Create a task, make it ready, claim it with the first instance, and reject the second claimant.
4. Append progress and prepare a handover; verify the task, closed claim, and three events commit together.
5. Close handles and end process A.
6. Process B opens the same canonical root and verifies the same workspace/definition/task IDs, accepted definition contents, immutable history, revision, and ordered events.
7. Register a new synthetic instance, claim the handed-over task, read predecessor context, and complete it.
8. Process C reopens and validates the final task, all claims, progress, handovers, and event watermarks.

This proves persisted coordination continuity, not survival of a real child process or a functioning daemon protocol.

### 12.2 Recovery and ambiguous completion

1. Process A claims a task and commits it.
2. Terminate A without running ordinary shutdown cleanup.
3. Process B first opens read-only/query state and confirms no automatic lifecycle mutation occurred.
4. Under test simulation of the future exclusive daemon owner, invoke internal lost-instance reconciliation.
5. Verify final instance metadata, closed claim, blocked task, and related events commit together.
6. Repeat reconciliation and confirm no duplicate events or revision increment.
7. Explicitly make the task ready and claim it from a different instance.
8. Separately kill a writer after commit but before notification; reopening must reveal exactly one committed operation without blindly replaying it.

### 12.3 Deterministic interleavings and failure points

Coordinate child processes/clients with barriers or pipes and bounded deadlines, never sleep-based assumptions. The parent controls which writer reaches a failpoint or commit barrier. Terminate only its own disposable test children.

| Failure/interleaving point | Evidence |
| --- | --- |
| Before registry reservation | No registered workspace or unexpected project files |
| After reservation, before database creation | Retry uses the same ID |
| During initial migration | No partially applied schema is accepted |
| After workspace commit, before registry ready | Retry does not emit another creation event |
| Two snapshots before either claim commits | One winner; loser conflicts after lock/revision check |
| Between task write and claim write | No partial task or claim after rollback/reopen |
| Between handover insertion and closure | No orphan handover or lost ownership |
| During event append/counter update | No confirmed partial stream or consumed sequence on rollback |
| During final-instance/task/claim bundle | Entire old or entire new state |
| Cancelled future with an acquired connection | No open transaction leaks into later pool users |
| Successful commit, failed notifier | Durable success and recoverable event stream |
| Snapshot read concurrent with writer | Rows and watermark describe one consistent commit |
| Equal timestamps for many entries | Stable sequence/append order and correct page boundaries |
| Database busy or long-lived reader | Bounded failure/checkpoint behavior without silent data loss |

### 12.4 Additional required tests

- Native directory aliases, case behavior, permission creation/inspection, restrictive ACL inheritance, missing roots, wrong owners, unexpected links/reparse points, and no default project writes.
- Unix non-UTF-8 and Windows native Unicode path codecs where representable; reject cross-OS decoding without corrupting paths.
- All schema fields round-trip through row codecs and domain reconstruction, including nullable actors, negative timestamps, zero/unknown exit codes, child-list ordering, and unsigned-counter exhaustion.
- Direct SQL negative cases for composite foreign keys, duplicate task/instance claims, mutable closed history, oversized byte fields, invalid booleans/statuses, and malformed event metadata.
- First import, explicit reload, absent reload, stable IDs, stale revisions, omitted definitions, batch failure, disabled definitions, and old launch snapshots.
- Query limits 0, 1, 50, 200, and 201; large histories read through indexed LIMIT queries; cursor expiry/future cursor; no full history materialization as a hidden first step.
- Migration from empty and populated initial schemas, checksum mismatch, newer versions, corruption, read-only mutation, unavailable storage, concurrent migrators, and original preservation.
- Synthetic markers in known and unknown TOML fields, commands/arguments, paths, row values, and parser/driver failures. Assert that runtime diagnostics and events omit those markers. Private database narratives may contain permitted synthetic content; do not confuse authorized private storage with public diagnostic output.

## 13. Verification commands and evidence

During implementation, use the narrowest applicable checks for the crate/task, including:

```text
cargo test -p relayterm-platform --locked
cargo test -p relayterm-config --locked
cargo test -p relayterm-persistence-sqlite --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
```

The first three commands become applicable only once those crates exist. Before closing M03, run:

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
git diff --check
```

Extend the existing CI matrix to run real-file and separate-process tests on native Linux, macOS, and Windows, including the Linux MSRV job. Record the SQLx version, bundled SQLite version, relevant features, compiler host, runner, candidate commit, and exact commands. Do not use a pre-existing M02 CI success as M03 evidence.

Re-run broader checks after meaningful code/lockfile changes or failures, not as a substitute for missing targeted evidence. Document limits of crash tests: terminating a process is not a physical power-loss or faulty-filesystem test.

## 14. Implementation order and session handoff

Recommended order:

1. M03.01 location/native-permission capability work and M03.03a dependency feasibility.
2. M03.02 configuration DTOs and pure application/domain extensions, checked against in-memory tests.
3. M03.03 codecs, migrations, constraints, connection handling, and validated reconstruction.
4. M03.04 registry initialization/reopen and durable import flows.
5. M03.05 task/claim transactions and real database races.
6. M03.06 append-only history and atomic handovers.
7. M03.07 sessions, durable events, snapshot watermarks, and query bounds.
8. M03.08 failure/recovery/migration/backup-mechanism tests, adding failpoints during earlier steps as needed.
9. M03.09 separate-process/native gate and documented handoff.

At each session start, read applicable instructions, check branch/working tree, consult `avances.md`, and choose the earliest unblocked child task. Work on a feature/fix branch; preserve unrelated local changes. Add any newly discovered child tasks to the queue before implementation.

Only remove a parent task after every required child outcome and relevant validation pass, appending its implementation and exact verification to `avances.md` in the same change. Keep partial tasks pending with the concrete next step. Do not turn this plan into a completed-checklist history inside `TODO.md`.

The handoff must give M04 bounded snapshot/event queries and explicit cursor/errors; M05 safe workspace discovery, storage assembly, and ownership-gated reconciliation; M06 reproducible durable continuity fixtures; M07 preserved launch configuration and honest session metadata; and M11 documented backup, storage, and resource-limit boundaries.

Publishing a branch, creating a PR, merging, or pushing requires the authorization defined by repository instructions. This planning request authorizes saving and verifying the document, not any of those external actions or M03 implementation.
