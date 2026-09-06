# M02 storage and application contract

## Implementation boundary

M02 implements deterministic domain coordination and generic asynchronous application services. The in-memory adapter, clock, ID generator, notifier, failure injection, and synthetic instance helpers live exclusively in `crates/relayterm-application/tests/support`. No production command fabricates a running process. The administrative CLI, daemon runtime, SQLite, IPC transport, PTY, Git, and TUI remain assigned to later milestones.

The [detailed plan](M02_details.md) records the behavioral decisions and limits. This inventory connects those decisions to the types that M03 must persist. Entity `record()` methods expose immutable views; `restore()` validates boundary records, and `WorkspaceState::restore` additionally validates membership, references, ownership, and history. Do not deserialize directly into entities or write an adapter that skips aggregate validation.

## Record inventory

All IDs are distinct UUID wrappers retaining the M01 workspace/task API. `Timestamp` is normalized to UTC through a checked conversion, with a checked Unix nanosecond representation for events. `time` is locked to 0.3.55 without local-time-zone features. No service reads wall time or generates random IDs directly.

| Record | Persisted fields |
| --- | --- |
| `WorkspaceRecord` | `id`, `display_name`, `project_root`, `created_at`, `updated_at`, `schema_version` |
| `AgentDefinitionRecord` | `id`, `workspace_id`, `display_name`, `command`, `arguments`, `environment_allowlist`, `capabilities`, `enabled` |
| `TaskRecord` | `id`, `workspace_id`, `content`, `status`, `claimed_by_instance_id`, `worktree_id`, `created_at`, `updated_at` |
| `TaskContent` | `title`, `description`, `priority`, `scope_paths`, `acceptance_notes`, `dependency_ids` |
| `AgentInstanceRecord` | `id`, `session_id`, `workspace_id`, `agent_definition_id`, `task_id`, `launch_definition`, `working_directory`, `status`, `started_at`, `last_observed_at`, `ended_at`, `exit_code`, `terminal_size` |
| `LaunchDefinitionSnapshot` | Stable definition ID, display name, command, arguments, environment names, capabilities, and enabled state accepted for one configured launch |
| `TerminalSize` | `rows`, `columns` through validated construction |
| `ClaimRecord` | `id`, `workspace_id`, `task_id`, `instance_id`, `requested_by`, `opened_at`, `closed_at`, `close_reason`, `closed_by` |
| `ProgressEntryRecord` | `id`, `workspace_id`, `task_id`, `agent_instance_id`, `summary`, `verification`, `created_at` |
| `HandoverRecord` | `id`, `workspace_id`, `task_id`, `from_agent_instance_id`, `content`, `created_at` |
| `HandoverContent` | `summary`, `decisions`, `changed_paths`, `verification_performed`, `open_questions`, `recommended_next_action` |
| `EventRecord` | `sequence`, `event_id`, `workspace_id`, `event_type`, `entity_id`, `timestamp`, `actor`, `payload_version`, `payload` |
| Workspace commit metadata | Per-workspace `revision` and append order for historical rows |

`TaskContent` and `HandoverContent` group fields in Rust; this does not require JSON blobs in SQLite. Preserve relational references and constraints when selecting storage columns in M03. `last_observed_at` preserves lifecycle time ordering after startup, including exit observations that would otherwise precede the observed transition to running. It is additional supervision metadata, not another lifecycle state machine.

## Reference and history constraints

- Every row belongs to the transaction's workspace. Task dependencies, definition references, launch task context, claim references, and annotation references resolve within that workspace.
- Definition IDs and entity IDs are unique within their entity tables. Instance IDs and session IDs have a one-to-one relationship. M03 must enforce database key uniqueness and reference integrity independently of service validation.
- Active tasks have exactly one open claim, and every other task has none. An instance has at most one open claim and must be running to receive it. Closed claim intervals for the same task or instance cannot overlap.
- Retain closed claims and append-only progress/handovers. Closing a claim creates its one closure record; further closure attempts fail. `closed_by` records the intervention actor independently of `requested_by`.
- A completed task requires completion history. A blocked task requires explicit-release, blocking, or instance-end history. A task ready for handover requires handover closure history; each handover closure has a matching attributed handover record.
- `worktree_id` is always absent in M02. Do not create placeholder worktree associations.
- Disabled definitions cannot receive new launch attempts. Existing instance metadata remains historical evidence.
- Preserve append order when loading claims, progress, and handovers. Timestamps are validated but do not replace event sequences or distinguish operations sharing a timestamp.

## Validation inventory

| Input | Bound or rule |
| --- | --- |
| Name, title, capability name, environment name | 256 UTF-8 bytes; required names are nonempty |
| Command and individual argument | 4 KiB; command nonempty; arguments preserved exactly, including repeated arguments |
| Argument list | 128 entries and 32 KiB total |
| Environment and capability sets | 128 entries; no duplicates; environment names use ASCII letters, digits after the first character, and underscores; Windows duplicates are compared case-insensitively |
| Description and acceptance notes | 16 KiB each |
| Progress summary and verification | 8 KiB each; summary required |
| Handover summary, verification, next action | 8 KiB each, required; explicit `Not run: ...` verification is valid |
| Decisions and open questions | 16 KiB each |
| Complete handover text and paths | 64 KiB total |
| Scope and changed paths | 128 entries; 4 KiB per path; no duplicates, absolute paths, drive-qualified paths, or parent escape components |
| Dependency IDs | 128 distinct existing references; no self-reference; cycles are informational and allowed |
| Native root and working-directory paths | Nonempty; lexical control/escape and 4 KiB display-byte checks, without filesystem canonicalization |
| Terminal dimensions | 1 through 1,000 rows and columns |
| History page | Default 50, maximum 200; reject zero or larger sizes |
| Schema and event payload versions | Version 1; unknown versions rejected |

Narrative fields allow tabs and line feeds. Single-line fields reject both; all reject NUL and other unsupported controls. No command, argument, or path is silently normalized. Recognized credential-pattern checks follow [ADR 0005](decisions/0005-environment-privacy.md), but cannot identify every arbitrary secret. Entities and boundary records with sensitive fields have no `Debug` or automatic entity deserializer.

M03/M07 must perform actual path canonicalization, filesystem identity checks, and allowed-root validation. Lexically valid native paths are not claimed to be canonical or safe execution roots.

## Transaction protocol

`Store::begin` returns a transaction containing a validated snapshot and its revision. Revision zero means no workspace exists. All application ports use generic traits with `Send` futures, without Tokio, SQLite, SQLx, or `async-trait` in the core.

`Service` constructs an opaque `WriteBatch` from a validated pure `Changes` value or workspace creation. The batch couples the new state with the exact pending event payloads. The initial implementation uses whole-workspace snapshots to make invariants explicit. M03 can load and write relational rows, but must preserve this contract; snapshot copying is not a scalability claim.

An adapter commit must:

1. Compare the captured revision with the current revision inside the storage transaction.
2. Call `WriteBatch::validate` against the current snapshot and enforce storage key, reference, and claim-exclusivity constraints.
3. Prepare all row changes and event records without partial writes.
4. Reject duplicate event IDs, including collisions with already committed events in the same workspace. Separate workspace databases do not claim a cross-file atomic event-ID reservation.
5. Assign strictly increasing per-workspace event sequences, starting at 1, and increment the workspace revision for a material commit. Detect integer exhaustion before writes.
6. Atomically persist all rows, histories, revision, and confirmed events.
7. Return a `Committed` value only after successful commit.

An exactly repeated final observation or a reconciliation with no remaining live instances produces no events, revision change, or notification. A stale transaction still conflicts. Errors before commit preserve the current snapshot and event stream. The service does not retry failed or ambiguous operations automatically.

After commit, the service calls `EventNotifier`. Failure is reported as `notification_delivered: false` alongside the successful `Committed` value, not as a failed mutation. M03/M04 must retain events so consumers can recover through durable queries.

## Events, queries, and actor routing

Payload variants use stable `snake_case` names for workspace, definition, and task creation, definition updates, task edits and transitions, instance registration and observations, claim opening and closure, progress, and handover preparation. Definition updates list changed field names without their values. Payloads contain only typed identities, states, closure reasons, and changed-field names. They never copy narrative text, arguments, environment entries, private paths, or terminal bytes. `event_type` and `entity_id` are checked against the typed payload; payload version remains independent of IPC version 1.

Deserialize confirmed events through `WorkspaceEvent` so unknown variants, fields, invalid IDs, and invalid versions produce safe diagnostics without echoing input. Boundary `EventRecord` is an explicit adapter DTO, not a substitute for the validated event type.

`Service::snapshot` queries workspace, definitions, tasks, current claims, and instances through immutable views. M03 adds a durable read port that returns snapshot revision, last event sequence, and retention floor from one SQLite read transaction, plus bounded event pages. `Service::history` retains offset compatibility for pure tests. Durable clients use database indexes and sequence cursors. M04 owns subscription after a watermark and resnapshot behavior.

Client requests use `Service::execute`; it rejects `System`. `register_instance` and `observe` are internal supervisor entry points, and future IPC routing must not expose unrestricted synthetic observations. User interventions retain user attribution, with absent optional instance attribution in user progress and handovers.

## Verification boundary

The tests cover all 49 task pairs and 36 instance pairs, continuity and loss recovery, actor guards, exclusive claims, dependency cycles and foreign references, reconstruction, serialization privacy, exact size limits, pagination, clock reversal, deterministic concurrent commits, injected storage failures, and notifier failure after commit.

M02 passed the repository's locked workspace/core checks on native Linux, macOS, and Windows, plus the security workflow, on 2026-09-06. See [platform evidence](supported-platforms.md) and [the logbook](../avances.md) for the verified candidate and CI runs. M02 is closed; subsequent milestones remain in [the queue](../TODO.md).
