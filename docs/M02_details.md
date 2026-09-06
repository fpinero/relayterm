# M02: Pure domain and application contracts

## Delivery and scope

This document records the M02 implementation plan. Saving it completes only the separate M02-DOC documentation task. Implementation status and remaining gate evidence are tracked in the [work queue](../TODO.md) and [logbook](../avances.md); the plan itself is not evidence of implemented behavior. See the [storage and application contract](M02_storage_contract.md) for the implemented record inventory and M03 adapter obligations.

Read the [project vision](../PROJECT_VISION.md), [MVP technical specification](../MVP_TECHNICAL_SPEC.md), [README](../README.md), and [architecture decisions](architecture/README.md) before implementation. The specification remains authoritative. The deliberate additions below must be incorporated into it in M02.01 before implementing the affected behavior.

M01 supplies `WorkspaceId`, `TaskId`, protocol version validation, and empty application boundaries. M02 extends those foundations into a library that coordinates tasks, ownership, progress, handovers, and instance states through deterministic services and in-memory test adapters. It does not implement SQLite, IPC transport, real processes, Git, or the TUI.

The documentation delivery workflow is:

1. Check Git status and create `feature/m02-details` from the appropriate branch before editing.
2. Preserve the existing local edit on the first line of `TODO.md`.
3. Register document preparation as the independent M02-DOC task.
4. Create this document and add a localized link from M02.
5. Verify references, M02.01-M02.09 coverage, English language, and formatting.
6. Remove only M02-DOC and append its exact verification to `avances.md`.

## Behavioral decisions

### Actors and ownership

| Actor | Responsibility |
| --- | --- |
| `LocalUser` | Explicit workspace, task, definition, and recovery management |
| `Instance(AgentInstanceId)` | Work and annotations attributed to an instance |
| `System` | Internal lifecycle observations and reconciliation |

`LocalUser` stores no username, email, or personal identity. `System` is an internal actor, not an identity that a future client may freely select. Instance actions on active work require ownership. The user may intervene explicitly under user attribution, without pretending the operation originated from an agent.

The confirmed decisions are:

- When an instance ends or becomes lost while owning an active task, close its claim and move the task to `blocked`.
- Preparing a handover creates the record, changes the task to `handover_ready`, and closes its claim in one transaction.
- Dependencies are informational references. They neither prevent activation nor invoke scheduling.
- The user may cancel any non-final task. This deliberately extends specification section 5.4 and must be documented there.

### Task state machine

| Source | Allowed destinations |
| --- | --- |
| `backlog` | `ready`, `cancelled` |
| `ready` | `active`, `cancelled` |
| `active` | `blocked`, `handover_ready`, `done`, `cancelled` |
| `blocked` | `ready`, `cancelled` |
| `handover_ready` | `active`, `cancelled` |
| `done` | None |
| `cancelled` | None |

Creating a task sets `backlog` with no owner. Entering `active` requires acquiring a claim; a generic status update cannot bypass this operation. Entering `handover_ready` requires a valid handover. Leaving `active` closes its claim within the same operation.

An active task has exactly one open claim; every other state has none. Same-state transitions are rejected and do not provide implicit idempotency. `done` and `cancelled` are final, with no reopening in M02. Editing title or description does not change status or ownership. Final tasks reject mutations except the user's permitted new historical annotations.

There is no direct `active → ready` transition. Voluntary release blocks an active task; making it available again requires an explicit `blocked → ready` transition.

| Operation | Actor and guard |
| --- | --- |
| Administrative task transitions | `LocalUser`, subject to the state table and dedicated claim/handover operations |
| Acquire a claim | The receiving instance itself, or `LocalUser` on its behalf; instance must be `running` |
| Block, complete, release, or prepare handover on active work | Owning instance or explicitly attributed `LocalUser` |
| Cancel a task | `LocalUser` may cancel any non-final task; an instance acting on active work must own it |
| Cancel an unowned task | `LocalUser` only |
| Append progress | Owning instance on active work or explicitly attributed `LocalUser` |
| Append a historical correction after closure | `LocalUser`, as a new entry without reopening |
| Observe lifecycle or reconcile loss | Internal `System` service |

### Claims

- At most one open claim exists per task and per instance.
- Only a `running` instance may receive a claim.
- The user may request a claim for a specific instance; history records the user as requester.
- Claims may be acquired only from `ready` or `handover_ready`, and acquisition moves the task to `active`.
- Reject a second claim even if requested by the same instance.
- Reject release of a nonexistent claim without changes.
- Never delete or reuse closed claims.
- Use enumerated closure reasons: explicit release, blocking, handover, completion, cancellation, and instance end.
- A handover does not transfer ownership automatically. The next instance must claim the task.

### Instances and terminal sessions

Represent a supervised instance and its session as one domain record with distinct `AgentInstanceId` and `TerminalSessionId`, related one to one. Do not duplicate lifecycle state machines for the same process.

| Source | Allowed destinations |
| --- | --- |
| `starting` | `running`, `failed`, `terminated`, `lost` |
| `running` | `exited`, `failed`, `terminated`, `lost` |
| `exited`, `failed`, `terminated`, `lost` | None |

`starting` represents a launch attempt. A startup failure is `failed`, without an invented exit code. `exited` means an observed process exit; a nonzero code does not necessarily indicate a supervision failure. `terminated` requires confirmation of termination, not merely a request. `lost` means the outcome can no longer be reconstructed. Unknown exit codes are `None`.

Final states require `ended_at`. An exactly repeated final observation is a no-op; an incompatible observation is rejected. The optional `task_id` is launch context and does not replace claim history. Instance exit never automatically completes a task.

Updating final instance state, closing its claim, and blocking its active task are atomic. An instance without a claim causes no task changes. M06 uses test-only simulation; no production command or flag may fabricate running processes. M07 connects real supervision.

### Informational dependencies

Add `dependency_ids` to `Task` and specification section 5.4.

- All references must exist in the same workspace.
- Reject self-references, duplicates, and references to another workspace.
- Limit the list to 128 entries.
- Allow cycles between distinct tasks because this relationship is not a scheduling graph.
- Open, cancelled, or blocked dependencies do not prevent claims.
- Do not automatically resolve, complete, or propagate state to dependent tasks.
- Do not add a scheduler or topological ordering.
- Future interfaces must describe dependencies as informational.

## Models and technical contracts

### Types and invariants

Preserve M01's valid identifier API. Add distinct identifiers for definitions, instances, sessions, claims, progress, handovers, events, and worktrees.

Mutable fields are private. Domain operations validate invariants before producing changes. Reconstruction from persistence also validates records; no public constructor may bypass validation.

| Model | Fields and decisions |
| --- | --- |
| `Workspace` | All specification section 5.1 fields, private root, stable identity, explicit schema version |
| `AgentDefinition` | All section 5.2 fields, separate command and arguments, environment variable names only, neutral capabilities |
| `AgentInstance` | All section 5.3 fields plus `session_id` and workspace membership |
| `Task` | All section 5.4 fields plus `dependency_ids` and workspace membership |
| `Claim` | Claim, task, and instance IDs; requesting actor; opening and closure timestamps; closure reason |
| `ProgressEntry` | All section 5.5 fields, immutable after creation |
| `Handover` | All section 5.6 fields, immutable after creation |
| `WorkspaceEvent` | All section 5.7 fields, typed and versioned payload |

`worktree_id` remains empty throughout M02. Define its reference type without an operation that associates nonexistent worktrees. Priority is a neutral enumeration: `low`, `normal`, `high`, `urgent`, defaulting to `normal`. It does not drive automatic scheduling.

### Deterministic time and identifiers

Introduce application ports `Clock` and `IdGenerator`. Services do not call the system clock or generate random UUIDs directly. Use a UTC-normalized `Timestamp` over `time::OffsetDateTime`; representation and conversion do not replace the injected clock.

Reject invalid temporal sequences, including closure before opening. If the clock moves backward and would produce an invalid record, return a typed error without persisting changes. Event order depends on `sequence`, not timestamp comparison. During implementation, select and lock a compatible `time` version without enabling local-time-zone features.

### Initial validation limits

All sizes are UTF-8 bytes, not visual character counts.

| Field | Limit |
| --- | --- |
| Names and task title | 256 bytes, nonempty |
| Description and acceptance notes | 16 KiB per field |
| Progress or handover summary | 8 KiB, nonempty |
| Verification and next action | 8 KiB per field |
| Decisions and open questions | 16 KiB per field |
| Complete handover | 64 KiB |
| Individual argument | 4 KiB |
| Argument list | 128 entries and 32 KiB total |
| Environment allowlist and capabilities | 128 entries per list |
| Scope paths and changed paths | 128 entries, 4 KiB per path |
| Dependency IDs | 128 entries |
| Terminal dimensions | 1 through 1,000 rows and columns |

Narrative fields allow newlines and tabs, but reject NUL and unsupported control characters. Single-line fields reject line breaks. Handover verification is explicit and nonempty; `Not run: ...` is allowed when no tests ran. Required next action must also be validated.

Do not silently normalize commands, arguments, or paths. Lists representing sets reject duplicates.

### Paths and secrets

Workspace roots and working directories use native path types. M02 performs lexical validation without filesystem access. Actual canonicalization and allowed-root checks belong to M03/M07. Do not call a path `CanonicalPath` when M02 cannot demonstrate canonicalization.

Scope and changed paths are workspace-relative; reject absolute paths and escape components. The domain neither invokes a shell nor interprets arguments as compound commands. Environment entries are names, without values or assignments.

Entities containing sensitive text, arguments, or paths must not expose their contents through `Debug`. Apply pure recognized-credential-pattern validation following [ADR 0005](decisions/0005-environment-privacy.md). Errors identify the field without echoing its value. Pattern detectors cannot recognize every arbitrary secret, and documentation must state this limitation.

### Events and serialization

Keep two representations:

1. `PendingEvent`: produced by the use case, without a confirmed sequence.
2. `WorkspaceEvent`: persisted event whose sequence is assigned by the transaction.

The initial payload version is 1, with explicit variants for entity creation or changes, states, claims, progress, and handovers.

- Use stable `snake_case` names.
- Include IDs, states, enumerated reasons, and names of changed fields.
- Do not copy descriptions, arguments, environments, private paths, or progress/handover contents into events.
- Future clients query authorized content instead of treating events as complete snapshots.
- Failed mutations produce no confirmed events.
- Sequences are strictly increasing per workspace.
- Explicitly reject unknown payload variants and unsupported versions.
- Keep payload version independent from IPC protocol version.

Add Serde for explicit serializable representations. Do not indiscriminately derive entity deserialization that bypasses validated constructors.

### Persistence and transactions

Application services are generic over asynchronous ports compatible with a future SQLx adapter, without adding Tokio or SQLite to the core.

| Port | Contract |
| --- | --- |
| `Clock` | Supply deterministic current time |
| `IdGenerator` | Supply typed identifiers through an injected dependency |
| `Store` | Open a transaction |
| `Transaction` | Read records and atomically commit a typed change set |
| `EventNotifier` | Notify after commit that new information is available |

Use generic traits with `Send` futures, without a mandatory `async-trait` dependency. Tests may run futures with an in-memory test executor.

Each transaction:

1. Reads necessary entities and the workspace revision.
2. Validates references, actor, state, limits, and ownership.
3. Builds changes without modifying committed objects.
4. Submits changes and pending events to commit.
5. Checks the revision and exclusivity invariants.
6. Persists everything or nothing.
7. Assigns sequences and returns confirmed events.
8. Notifies after commit.

The in-memory store uses a revision per workspace to detect concurrent transactions opened against the same state. The loser returns a conflict without partial writes. Notifier failure after commit does not turn a committed operation into a failed operation or cause its repetition. Durable events enable later synchronization recovery.

Do not introduce Git, command-launch, or PTY ports in M02: no production use case needs them. Lifecycle observations enter internal services; M07 supplies the actual supervisor.

## Atomic implementation tasks

### M02.01: Formalize the rules

1. Incorporate task, session, and permission tables into the contractual documentation.
2. Update specification section 5.4 with `dependency_ids` and cancellation from every non-final state.
3. Clarify additional transitions from `starting` and exit-code semantics in section 5.3.
4. Document unique open claims per task and instance.
5. Document release/blocking on loss and atomic handover.
6. Define `LocalUser`, instance, and system attribution.
7. Define test-only simulation for M06, without a production mode that fabricates active processes.

Verification: cross-review tables and examples so this plan and the updated specification contain no contradictory rules. This documentary task requires no real processes. Saving this plan alone does not complete it.

### M02.02: Identifiers and base entities

1. Organize domain modules without changing M01's valid ID API.
2. Add new IDs and tests proving type distinction.
3. Implement `Timestamp`, text validators, and list validators.
4. Implement workspace and agent definition with all required fields.
5. Define neutral capabilities as validated names, without provider types.
6. Add validated creation and reconstruction constructors.
7. Add errors that do not disclose sensitive values.

Verification: exact boundaries, multibyte UTF-8, empty data, empty command, NUL, duplicates, and invalid reconstruction.

### M02.03: Tasks and transitions

1. Implement `Task`, priority, and initial state.
2. Implement editable fields without permitting changes to identity, history, or ownership.
3. Implement the table's pure transitions.
4. Require dedicated commands for activation and handover preparation.
5. Implement informational dependencies and membership validation.
6. Reject final-task mutations except new historical annotations allowed to the user.

Verification: all 49 state pairs, each with otherwise valid preconditions; every rejection preserves the original state.

### M02.04: Claims and history

1. Implement claim opening and immutable closure history.
2. Implement acquisition from `ready` and `handover_ready`.
3. Implement release with transition to `blocked`.
4. Enforce exclusivity per task and instance.
5. Validate owners and actors.
6. Implement explicit user intervention without falsifying attribution.
7. Expose current-claim and history queries.

Verification: two claimants, two tasks for one instance, repeated request by the same owner, invalid release, and release/reopen/reclaim sequence.

### M02.05: Instance lifecycle

1. Implement the instance/session record and complete metadata.
2. Implement the state matrix.
3. Validate timestamps, dimensions, and exit codes.
4. Implement startup, exit, and loss observations.
5. Combine final instance state, claim closure, and task blocking.
6. Treat identical final observations as no-ops.
7. Create helpers in `tests/support`, without production CLI commands or flags for fictional processes.

Verification: all 36 state pairs, failure before startup, nonzero exit, loss without exit code, repeated notifications, and no effects on unrelated tasks. Distinguish repeated observations from transitions to final states.

### M02.06: Progress and handovers

1. Implement append-only records.
2. Permit owning-instance progress and explicitly attributed user annotations.
3. Permit new user historical corrections after closure without reopening the task.
4. Create handovers only for active tasks with valid claims.
5. Atomically create the handover, change task state, and close the claim.
6. Represent user attribution as `None` in optional instance fields.
7. Implement ordered, paginated progress and handover reads.

Verification: required fields, preserved history, rejection of a different instance, and complete continuity between two instances.

### M02.07: Events and errors

1. Implement pending and confirmed events.
2. Define payloads and stable names.
3. Implement serialization and versioned event reading.
4. Introduce typed validation, state, actor-authorization, reference, conflict, time, and storage errors.
5. Ensure errors and diagnostics contain no supplied values.
6. Exhaustively test payloads for content leaks.

Verification: round trips, unknown variants and versions, absence of sensitive contents, and correspondence between mutations and events.

### M02.08: Services and in-memory doubles

Implement services to create and query workspaces and definitions; create, edit, query, and transition tasks; claim and release; append progress and prepare handovers; query history; record internal instance observations; and reconcile lost instances.

Then:

1. Implement a transactional in-memory store with revisions.
2. Implement deterministic clocks and IDs.
3. Implement recording and failing notifiers.
4. Add failure injection before commit and during commit validation.
5. Race two transactions opened against the same revision without sleeps.
6. Check atomicity using results and confirmed events.
7. Bound pagination to default 50 and maximum 200; reject zero and excessive sizes.

Verification: entity changes, history, and events remain consistent; reject cross-workspace references. Use only the ports required in this document, even though the roadmap lists possible later adapter ports.

### M02.09: Domain gate

Build a complete continuity test:

1. Create a workspace and neutral definitions.
2. Introduce two instances through synthetic test observations.
3. Create a task and move it to `ready`.
4. Claim it with the first instance.
5. Reject a claim from the second instance.
6. Append progress with verification.
7. Prepare a handover and verify atomic release.
8. Claim the task with the second instance.
9. Read the prior context.
10. Complete the task.
11. Check history and ordered events.

Add a recovery journey:

1. Claim a task.
2. Mark its instance lost.
3. Verify the task is blocked and the claim closed.
4. Repeat the observation without duplicate effects.
5. Explicitly reopen the task as available.
6. Claim it from another instance.

This gate proves deterministic domain coordination. It does not claim real persistence or restart survival, which belong to M03-M06.

## Verification and compatibility

### Required evidence

| Area | Minimum evidence |
| --- | --- |
| Tasks | 49 state pairs and actor/claim guards |
| Instances | 36 pairs and consistent terminal metadata |
| Claims | Exclusivity per task and instance; intact history |
| Handover | Indivisible creation, transition, and claim closure |
| Loss | Atomic final instance state, closed claim, and blocked task |
| Dependencies | Informational behavior without blocking; valid references |
| Transactions | Race, rollback, and conflict without partial effects |
| Events | Per-workspace order, versioning, and content-safe payloads |
| Notification | Failure after commit without repeating the operation |
| Validation | Size boundaries, Unicode, controls, lists, and paths |
| Privacy | Errors and serialization exclude injected sensitive values |
| Architecture | Domain/application without concrete adapters |

Run narrow checks during implementation:

```text
cargo test -p relayterm-domain --locked
cargo test -p relayterm-application --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
```

Before closing implementation:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
git diff --check
```

These are required future implementation checks, not claims that they ran when this document was saved. Verify this documentation delivery with repository contracts, a focused link/task/table/limit check, and whitespace validation.

M01's architecture test must continue passing. Allow the `application → domain` dependency without exceptions for SQLite, Tokio, Git, PTY, or TUI. New serialization, time, and test-execution dependencies must respect the MSRV, license policy, and lockfile.

### Compatibility and handoff to M03

- Keep IPC version 1: M02 adds no transport and changes no published IPC contract.
- Version event payloads from the start.
- Create no SQLite migrations in M02.
- Deliver an explicit inventory of fields, references, limits, and constraints to M03, covering every model in specification section 5 and the additions in this document.
- Record informational dependencies and expanded cancellation as deliberate specification additions in M02.01.
- Preserve M01's active-configuration/import policy.
- Preserve local-first operation and neutral, replaceable daemon/client boundaries on Linux, macOS, and Windows.

### Execution order and session continuity

1. Contracts and specification updates.
2. Base types, validation, and entities.
3. Task state machine and claims.
4. Instances and recovery.
5. Progress, handovers, and events.
6. Transactional services and doubles.
7. Races, failures, and continuity tests.
8. Full validation and cross-platform evidence.

Remove each implementation task from `TODO.md` only after its verification succeeds, and append its result and exact checks to `avances.md` in the same change. Keep incomplete or unverified tasks pending. M02 remains open while required evidence is missing. Its completion must not advance the persistence, IPC, or real-supervision guarantees assigned to later milestones.
