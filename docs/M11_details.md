# M11: Reliability, privacy, and complete acceptance

## 1. Execution contract and scope

This is the execution contract for M11. It plans implementation and verification; its publication does not complete any M11 implementation task. Read [AGENTS.md](../AGENTS.md), [PROJECT_VISION.md](../PROJECT_VISION.md), [MVP_TECHNICAL_SPEC.md](../MVP_TECHNICAL_SPEC.md), [README.md](../README.md), [TODO.md](../TODO.md), recent [avances.md](../avances.md) entries, and the referenced ADRs before starting. Communicate in Spanish; write documentation and code comments in English.

M11 must demonstrate FR-1 through FR-9, NFR-1 through NFR-8, and the behavioral acceptance criteria using the integrated product. M12 owns distribution, clean installation, naming conflicts, release packaging, and final release sign-off. M11 is not permission to announce a completed MVP or publish a release.

Keep the daemon authoritative over durable coordination and real supervised sessions. Domain/application must remain independently testable. Clients use authenticated local IPC. Keep one canonical user executable, `rt`. Test helpers may be separate executables under test targets only. Do not introduce production fault flags, synthetic supervisors, provider accounts, transcript persistence, telemetry, TCP servers, hosted services, automatic exports, or destructive Git operations.

The implementation branch should be `feature/m11-hardening`, created from synchronized `main`, or an existing appropriate implementation branch continued without overwriting work. Before coding, copy all 50 atomic tasks in section 10 into TODO.md under their existing parents. Preserve existing user edits. Remove only verified tasks, appending exact evidence to avances.md in the same change. Never rewrite historical claims to conceal a newly found defect. Record corrective evidence instead.

Documentation deliverables during implementation:

- `docs/acceptance-matrix.md`: requirement-to-test mapping, workload thresholds, evidence status, and blockers.
- `docs/reliability-workflow.md`: reproducible integrated and fault scenarios and bounded recovery guidance.
- `docs/backup-restore.md`: exact supported command/procedure, safety boundaries, and verified restore journey.
- `docs/threat-model.md`: assets, trust boundaries, threats, controls, residual risks, review results.
- Updates to privacy, supported-platforms, TUI, worktree and daemon guides, relevant ADRs, and specification where a deliberate compatibility change is necessary.

Evidence records must identify the candidate SHA, test name/command, OS/build/architecture, Rust and Git versions, shell and terminal versions when applicable, repetition, result, duration, and sanitized measurements. Never commit raw terminal recordings, private absolute paths, database copies, environment dumps, user identities, or real credentials. Runtime artifacts live in disposable private locations outside the checkout.

## 2. Baseline and inherited risk investigation

### 2.1 Verified documentary baseline

The planning baseline is `c1db63bfee15a0d3f7485f292926d1d037ec6f35`. M10 implementation PR #19 merged as `c4af056898449f32a4e2347f771922a62847e031`; its delivery record PR #20 is also merged. The recorded native M10 gates are useful regression evidence, not proof that every earlier contractual edge case is implemented. Recheck the current source before editing.

Existing tests include CLI `durable_slice`, `tui_gate`, `agent_templates_gate`, and `worktree_gate`; daemon `pty_gate`; persistence, protocol, privacy, and architecture tests. Reuse their real composition. Do not copy entire harnesses or replace a UI interaction with direct database setup while claiming the interaction was tested.

### 2.2 Investigate before changing behavior

The following are source observations and test hypotheses, not claims of newly executed reproductions. For each, record the exact old behavior, a minimal failing test or bounded experiment, the desired contract, the fix if needed, and the passing regression. If not reproducible, document the tested preconditions and why the source concern is already protected elsewhere. A green historical workflow alone is insufficient.

| ID | Source starting point | Required investigation and acceptance |
| --- | --- | --- |
| R1 | `relayterm-git/src/lib.rs`, `run` | Git output is redirected to temporary regular files and checked after process exit. Demonstrate behavior while stdout/stderr exceed their limits and while a descendant retains handles. Enforce limits during production, including temporary disk consumption, readers, process teardown, and cancellation. Reading only a bounded prefix after unbounded disk writes does not meet the contract. |
| R2 | Git `command`, `reject_checkout_filters`, `add` | Environment inheritance and local-only filter inspection may permit injected configuration, include files, worktree configuration, attributes in the selected commit or nested directories, and external helpers. Use inert marker executables to test hooks, smudge/process filters, credential/remote helpers, pager and lazy fetch. None may run during Relayterm Git operations. |
| R3 | Git `add`; daemon `worktree_create` | Destination/branch checks precede the common-directory lock, and that lock ends before final registration. Race two daemon processes sharing one common Git directory, including different Relayterm homes. Revalidate under ownership, preserve durable intent identity, and prove that one operation cannot adopt another's effects. No automatic second add, deletion, or repair. |
| R4 | Git `verify_worktree` | It compares HEAD with the recorded initial commit. Test a user-created commit after successful creation. Launch should verify the same repository, branch and associated checkout, while allowing ordinary user development. Initial creation proof and later launch health are different checks. Branch replacement, detached HEAD and foreign checkout remain explicit failures. |
| R5 | Git `text`, discovery, porcelain parsing, path conversion | Exercise spaces, leading/trailing whitespace, Unicode, native Unix non-UTF-8 names, Windows disk/UNC/verbatim paths, case aliases, reparse points and malformed NUL records. Never silently trim or lossy-convert an identity-bearing path. Reject unsupported representations with a typed error before effects. Reconcile requirements with M10's native-path contract explicitly. |
| R6 | daemon worktree creation/reconciliation and Git admission | A 60-second Git create can outlast the client's 35-second request deadline. Prove accepted work survives client cancellation with a queryable stable receipt. Inventory all inspection/reconciliation/launch `spawn_blocking` paths, bound both active and waiting work, and retain conservative outcomes for any error after a possible Git effect, including I/O and malformed output. |
| R7 | daemon launch admission, task reconstruction, installed migrations | Exercise edit/deactivate/select/clear/cancel versus launch with deterministic barriers. The actual directory/argv and persisted task, definition, worktree, instance and session IDs must form one admitted snapshot. Open populated pre-worktree storage with an executable run away from the source tree. Never drop worktree association during reconstruction. |
| R8 | CLI `tests/support/m09_native.rs`; workflow install/version step | A bounded child poll followed by an unbounded reader join can still hang on inherited pipes. Test no newline, oversized line and orphaned writer; cleanup must be bounded. Every external command and each CI repetition must propagate its own failure, including PowerShell steps with multiple native commands. |
| R9 | SQLite `connection.rs`, `snapshot_to` | Existing `VACUUM INTO` checks destination existence and secures the generated file afterwards. Test exclusive destination ownership, symlink/reparse replacement, private permissions throughout creation, interruption, corrupt output and unsupported native path encodings. Backups must never overwrite or expose material during an intermediate phase. |
| R10 | TUI guide and platform manual matrix | `docs/tui-workflow.md` calls named-terminal checks optional, while M11.07 requires them. Correct the current guide during M11: automated console evidence remains valid but does not satisfy mandatory named-terminal and SSH observations. Missing required evidence blocks closure. |
| R11 | persistence snapshot/query paths, queues, logs | Audit materialization before pagination, accumulated closed sessions, worktree receipts, event/history caches, diagnostic retention, cancelled waiters and connection tasks. A bounded response does not prove bounded intermediate allocation. Preserve durable history while bounding reads and ephemeral state. |

Use [M10's contracts](M10_details.md), [the worktree ADR](decisions/0007-worktree-ownership.md), and [the environment ADR](decisions/0005-environment-privacy.md) to resolve these issues. Reproduction may use a controllable fake external program for fault injection, but successful acceptance still uses installed Git and native PTYs. Do not expose fake programs as a product mode.

For Git behavior, consult official [worktree documentation](https://git-scm.com/docs/git-worktree) and [Git environment/configuration controls](https://git-scm.com/docs/git). Select an explicit subprocess environment and tested checkout policy. Do not assume disabling one configuration scope prevents all external execution. Bound concurrent stdout/stderr draining and teardown on all OS targets; preserve uncertain durable receipts when termination cannot prove that no checkout was created.

## 3. Acceptance matrix and evidence rules

Create one row per criterion below in `docs/acceptance-matrix.md`, subdividing multi-part criteria so no single passing test hides an untested clause. Each row contains implementation owner, exact test functions, automated native results, required manual results, candidate SHA, limitations and status (`pending`, `passed`, or `blocked`). A link to an entire workflow without a test-to-assertion mapping is insufficient.

| Criterion | Required M11 evidence | Later boundary |
| --- | --- | --- |
| AC-1 | Real `rt` initializes private Git and non-Git workspaces; registry and schema reopen correctly; invalid roots leave no partial registration. | M12 clean installation and quick start. |
| AC-2 | Independent daemon, single owner, authenticated current-user IPC, contender rejection before recovery; terminal closure does not own daemon lifetime. | M12 installed command resolution. |
| AC-3 | At least three genuinely running native sessions, concurrent output/input, identity and lifecycle assertions. | None. |
| AC-4 | Authoritative full-screen reconstruction, Unicode, input, resize, switching, exit and bounded output; native automation and manual matrix. | None. |
| AC-5 | Competing clients/instances cannot obtain two open claims on one task or two claims for one instance; losers cause no history/event changes. | None. |
| AC-6 | User-attributed progress and complete structured handover, atomic claim closure and state transition. | None. |
| AC-7 | A different live instance obtains prior context and resumes; UI actions and durable ordered queries agree. | None. |
| AC-8 | TUI detach, abrupt client loss and actual terminal closure preserve daemon-owned children; independent reattach restores authoritative state. | None. |
| AC-9 | Graceful and abrupt daemon loss preserve committed state; unrecoverable sessions become lost once, active claims close and tasks block; no invented exit status. | No live PTY adoption or persisted screen after restart. |
| AC-10 | Two explicit task worktrees with independent files and correct immutable launch snapshots; partial Git outcomes and user commits handled honestly. | No automatic Git cleanup. |
| AC-11 | Core-only tests and architecture checks pass without concrete adapters or UI. | None. |
| AC-12 | Native Linux/macOS/Windows build/test/security results and pinned compiler checks. | M12 distributed artifacts and clean hosts. |
| AC-13 | Injected synthetic sensitive/ANSI markers absent from prohibited diagnostics/events/project artifacts; private authorized durable fields tested separately. | M12 scans actual release artifacts. |
| AC-14 | Real offline runtime, no provider credentials, graphical dependency or product TCP listener; headless workflow passes. | M12 clean install without hosted account. |
| AC-15 | Unknown neutral executable registered, edited and used through real TUI with ordered/empty/duplicate arguments; same domain/protocol semantics as templates. | No provider authentication required. |
| AC-16 | Preserve one `rt` entrypoint and side-effect-free help/version; no extra required user binary introduced. | M12 installation, PATH collisions, packaging and final sign-off. |

Record FR/NFR mapping alongside this table. FR-10 remains deferred; database backup is private disaster recovery, not a shareable project export. Required manual coverage may not be relabeled optional, postponed to M12, or accepted on the agent's own authority.

## 4. Integrated journey and fault contracts

### 4.1 Real integrated journey

Add `crates/relayterm-cli/tests/hardening_gate.rs` with shared bounded native support. Use a real `rt` binary, separate clients, real SQLite, installed Git, and Unix PTY or Windows ConPTY. Test fixture child programs use ordinary neutral definitions and production launch composition. No in-process replacement for the daemon in the successful journey.

1. Start in private disposable storage outside the repository. Record only synthetic path aliases. Initialize Git and non-Git projects. Keep a sentinel dirty source file and source HEAD for later comparison.
2. Through actual TUI controls, register a neutral executable with ordered, duplicate and empty arguments, edit it, check availability, and exercise deactivate/reactivate. Reopen a form after a concurrent revision change and prove conflict handling preserves the draft without automatic mutation retry.
3. Create two tasks and two explicit worktrees through the real TUI. Query durable receipts independently. Launch three sessions, including the platform default shell; two use their associated worktrees. Assert actual working directory and immutable snapshot IDs, not just visible labels.
4. Write distinct synthetic sentinels in the two task directories through test children. Verify source and other checkout unchanged. External test setup may create a user commit; Relayterm itself must not commit. Recheck launch after that commit for R4.
5. Race two legitimate claimants on one ready task with an explicit barrier. Exactly one claim succeeds. The loser receives a typed conflict/state result, and no phantom events or history appear.
6. Add progress and a structured handover through TUI forms. Verify required verification text, LocalUser attribution, open-claim count, and atomic `handover_ready` transition. A second instance claims and reads prior context from a new client.
7. Exercise explicit release to blocked, explicit reopening to ready, another claim and completion. Verify final tasks reject mutation while permitted historical user annotations remain append-only.
8. Under simultaneous full-screen output, switch sessions, send input using the exclusive lease, resize, browse retained history and return live. Reattach a second client, compare authoritative cells/modes/dimensions and sequence continuity. Never rebuild an incomplete parser from a cell snapshot plus raw bytes.
9. Detach and abruptly terminate clients independently. Close the originating native terminal/console owner in automation and repeat the manual named-terminal scenario. Reconnect and prove all three children retain the same live identities while the daemon is alive.
10. Gracefully stop and separately abruptly kill an owned test daemon while a task is active. Restart production `rt`, verify durable content and one-time loss reconciliation. Do not claim children can be adopted, terminal history survived, or unknown exits were observed.
11. Query ordered events and pages from independent clients after every boundary. Assert relational state, claim/history records, and event watermarks form consistent committed snapshots. Sequence ordering is per workspace; do not require gap-free global numbering or use timestamps as sequence numbers.
12. Perform explicit bounded teardown of processes the harness owns. Preserve material user data. Verify no runtime files, transcripts or environment values were generated inside source checkout content. The non-Git journey must still work when Git is unavailable.

### 4.2 Fault matrix

Fault tests must prove the harness reached the fault boundary before triggering it. Use barriers, acknowledgments, injected adapters or test-only executable composition; sleeps alone are not evidence. Test hooks are absent from ordinary `rt` CLI, protocol and release builds.

| Boundary | Injection | Required observable result |
| --- | --- | --- |
| IPC parser | Oversized length before allocation, partial prefix/body, invalid UTF-8/JSON, unknown version/field/operation, duplicate correlation ID | Bounded rejection/close, typed safe error when a response is possible, no mutation and no daemon-wide failure. |
| Authentication | Wrong peer where OS supports it, permissive ACL/mode, endpoint collision or replaced generation | Fail closed without adopting/removing a foreign endpoint; existing legitimate daemon remains usable. |
| Client request | Disconnect before admission, after admission, after commit but before response; cancellation during long Git operation | No commit before admission; committed operation remains queryable. Uncertain mutations are not automatically replayed. A receipt is evidence only for its exact original payload. |
| Subscription | Slow/nonreading subscriber, buffer overflow, stale/gapped cursor, generation change, repeated reconnect | Bounded resources; explicit resynchronization; no deadlock of unrelated reads/mutations/input; authoritative refresh without duplicate durable writes. |
| SQLite transaction | Lock timeout, injected write failure, kill before commit and after commit before notification, notifier failure | All-or-nothing entities/history/events; prior state remains readable; committed result is not turned into failure by notifier loss. No partial schema migration. |
| Spawn/persistence | Missing executable, permission denial, executable removed after resolution, post-spawn storage failure, child immediate nonzero exit | No fabricated running process; supervised orphan cleanup or explicit conservative state; unrelated sessions survive. Nonzero observed exit is not automatically supervisor failure. |
| Terminal | Output flood, malformed escape stream, partial UTF-8, resize/input race, revoked lease, child crash, descendant retains handle | Memory/frame limits hold; mode/state reconstruction is correct; stale writer rejected; teardown bounded; no spill to logs. |
| Daemon lifetime | Competing starts, ownership loss, graceful drain with pending transactions, abrupt kill during work | Only owner reconciles; transactions finish or roll back honestly; restart produces loss effects once; no PID reuse kill. |
| Git external effect | Fail before add, during add, after add before SQLite finalize, cancellation, two daemons sharing Git | Stable durable intent precedes effects; uncertain remains inspectable; explicit read-only reconciliation; no duplicate add, destructive compensation or foreign association. |
| Recovery input | Corrupt/newer/locked DB, missing worktree, changed branch/root, damaged backup | Preserve originals and return bounded actionable guidance. No silent recreation, force recovery, automatic Git repair or secret disclosure. |

For each injected failure, assert both the affected operation and an independent healthy operation. Bound total test time, child waits, pipe reads, thread joins and cleanup. A passing process exit code alone does not establish that all assertions ran.

## 5. Resource budgets and performance methodology

### 5.1 Inventory and exact-bound tests

Before optimization, inventory every production constant and owning layer in the acceptance matrix. Include bytes versus elements, per-session/per-connection/per-workspace/global scope, admission point, overflow result, cleanup trigger and regression test. Verify actual constants rather than copying obsolete numbers from a prior plan.

Retain existing limits unless a documented contract correction is justified. Current guides report eight live sessions, a 1 MiB raw continuation ring, 16,000 viewport cells, 64 UI input events, 64 invalidations, 256 client events or 1 MiB, 64 KiB pending session input, 1,000 diagnostics or 1 MiB, and 256 KiB total draft storage. Confirm every value in code and test exact limit and limit plus one. Include parsed history and alternate buffer allocations, not only the raw ring.

Cover protocol frame/message sizes; pending and executing requests; connection/subscriber counts; task/definition/worktree pages; history/event pages; all text/list UTF-8 bounds; retained launch receipts; closed-session caches; Git stdout 4 MiB/stderr 64 KiB and 4,096 inventory entries; worker and queued job counts; log retention; file descriptors/handles; and cancellation cleanup. Limits must be enforced before allocation or accumulation, not only before serialization.

Durable coordination history may grow over product lifetime. Do not delete history, silently truncate queries, or impose a new lifetime task limit to make a memory test pass. Bound SQL reads, transaction working sets, materialization, pages and caches; use explicit pagination and version-compatible cursors. If the current all-workspace snapshot contract requires unbounded reconstruction, redesign its read path with a deliberate compatibility decision and test concurrent pages for consistency.

### 5.2 Predeclared workloads and thresholds

Record these criteria before obtaining measurements. Do not choose a threshold after observing a failing candidate. Preserve inherited 100 ms navigation and 250 ms input reference targets and existing shared-runner distinction. Values below are M11 test budgets, not claims of prior measurements.

| Workload | Samples and environment | Pass rule |
| --- | --- | --- |
| Existing daemon startup | 100 tasks, 100 history entries, 3 live sessions, one warm-up and 20 launches per repetition | p95 from connection attempt to usable overview at most 2 seconds on each native target. Report cold bootstrap separately; do not fold compilation into startup. |
| Concurrent output and input | Three sessions; one emits at least 2 MiB/s for 120 seconds, another renders full-screen changes, third echoes 100 measured keys; measure 100 visible navigation changes | Reference native machine p95 navigation <=100 ms and echo <=250 ms. Shared CI p95 <=500 ms and <=1 second respectively; always report reference target misses. Cold/quiet measurements cannot substitute. |
| Maximum session retention | Eight real sessions, 120x40 viewport, each wraps all retention capacities at least three times | No configured allocation or queue bound exceeded; ninth live session rejected without spawn. Exited session resources released while durable metadata stays queryable. |
| Reconnection churn | 100 connect/attach/lease/revoke/disconnect cycles after warm-up, including 20 abrupt clients | Within 10 seconds after quiescence, active counters return to baseline; no accumulating connection tasks, waiters or child processes. OS handle/fd counts may differ by at most 16 from warmed baseline, with each retained category explained. |
| History scale | 10,000 tasks and 100,000 progress/event records seeded in private real SQLite using validated test composition | Interactive queries bounded by documented pages; no full-history UI materialization. Reference startup workload remains separate. Record query p95 and allocation maxima. |
| Memory plateau | Sample daemon and TUI separately every second during a 180-second run, first 60 seconds warm-up; use the same workload in each repetition | Each process <=512 MiB resident memory for the fixed workload, excluding child RSS but reporting it separately. Last 30-second median <= first post-warm-up 30-second median +32 MiB. Reconnect plateau must also pass. Deterministic allocation counters must pass independently of RSS. |

Record OS memory metric semantics (RSS/working set), debug/release profile, CPU/RAM and runner type without hostnames. Retain raw numerical samples only after privacy review. Compute nearest-rank p95 consistently, report median/p95/max and actual produced/consumed output rate. Do not report a configured fixture rate as measured throughput.

The reference performance run is required on an identified native developer-class environment; all three hosted OS targets must satisfy the CI budgets. An unavailable reference environment remains open. These thresholds must not be weakened simply because CI is slow. Investigate bottlenecks and harness overhead; any proposed policy change requires explicit rationale and review before new acceptance measurements. Correctness deadlines stay independent from performance thresholds.

Use a dedicated native resource test job if long workloads would exceed the existing 30-minute Quality job. Keep two independently failing M11 gate invocations per stable target. Size job timeouts from a bounded work inventory; do not drop inherited gates or hide their first failure.

## 6. Privacy, offline operation, and manual terminal acceptance

### 6.1 Threat model and data-flow review

Follow [ADR 0002](decisions/0002-local-ipc.md), [ADR 0003](decisions/0003-terminal-state.md), [ADR 0004](decisions/0004-sqlite-recovery.md), [ADR 0005](decisions/0005-environment-privacy.md), [ADR 0006](decisions/0006-agent-configuration.md), [ADR 0007](decisions/0007-worktree-ownership.md), and [ADR 0008](decisions/0008-rust-platforms.md). Domain policy remains defined by the specification.

Document assets and flows from CLI/TUI drafts through IPC, domain validation, SQLite, events, process argv/environment, terminal state and diagnostics. Distinguish trusted same-user processes, untrusted project/terminal content, other OS users, and administrators. The product is not a sandbox and does not defend against malicious code already running as the launching user.

Review Unix ownership/modes and symmetric peer checks, Windows protected DACLs through actual handles, runtime metadata replacement and cleanup, SQLite/WAL/SHM/backup permissions, symlink/reparse containment, explicit configuration imports, environment allowlists, executable resolution and argument-array invocation. Native ACL tests do not require exposing a real user's private data.

Inject synthetic markers into every applicable text, argv, path, configuration, environment and child-output boundary. Build a sink matrix: approved definition/task content may legitimately persist; environment values and terminal bytes must not enter prohibited durable fields; metadata events and ordinary diagnostics must omit narrative, argv and private paths. Do not assert that databases contain no paths at all, since authorized native paths are part of the durable model.

Test recognized credential patterns are rejected with field/category-only errors. Arbitrary markers which are not credential-shaped test sink privacy, not universal secret recognition. Test ANSI/OSC/control characters outside the terminal pane, panic/error formatting, Debug implementations, malformed payload errors and CI assertion output. Redact at construction, not only at final log printing.

If a diagnostic mode exists, audit its opt-in and retention; do not add a raw-dump mode for acceptance. No terminal recording or automatic artifact upload. Update stale bootstrap wording in SECURITY.md without inventing maintainer policies or sending a test vulnerability report.

### 6.2 Offline runtime proof

Prebuild locked binaries and test helpers. Then run the real integrated headless workflow under a clean private environment without provider credentials and without required network access. Use native OS-specific disposable isolation or process-scoped monitoring, not a change to the operator's global firewall or network settings.

For each OS, record the denial/monitoring mechanism and prove it is effective with a separate harmless synthetic probe. Verify no product TCP listener and no attempted telemetry, update, DNS or hosted request during startup, coordination, worktrees, reconnect, backup and restore. Inspect source/dependency call paths as supporting evidence; absence of network declarations alone is insufficient. Ensure dependencies are not downloaded during the measurement.

Native local IPC must remain functional. Child fixture commands are local and do not use network; normal user-configured children retain their own OS network permissions. SSH is a test transport in the manual matrix, not a Relayterm listener or hosted dependency. If native isolation/observation cannot be obtained on an OS, keep that evidence open and document precisely what weaker check ran.

### 6.3 Required manual matrix

Arrange access early while implementation and automated tests continue. Do not install or enable SSH servers, open firewall ports, use personal projects, or send third-party messages without authorization. Ask only for the missing environment or sanitized observations that cannot be obtained safely. Time elapsed does not count as approval or evidence.

| OS | Required local combinations | Required remote combination |
| --- | --- | --- |
| Linux | Bash and one configured generic shell in an identified xterm-compatible terminal | OpenSSH to Linux; record client terminal, server OS and shell. |
| macOS | Zsh and Bash in Terminal.app | OpenSSH to macOS with an authorized server and identified client. |
| Windows | PowerShell and cmd.exe in Windows Terminal using ConPTY | Supported Windows OpenSSH session; record server shell and terminal behavior. |

For each local combination, use synthetic data to verify: open/init, three live sessions, full-screen alternate-screen entry/exit, Unicode combining/wide characters, rapid resize and minimum size, focus escape, keyboard help, color-independent state with no-color mode, forms and conflicts, detach/reattach, and raw-mode/cursor/alternate-screen restoration after normal exit and recoverable failure. After actual terminal-window closure, reconnect from a new window and prove live child identity and context while the daemon remained alive.

For each SSH target, disconnect the actual connection and reattach through a fresh connection. Verify the same daemon and children, rendered state, resize and input lease behavior. Shell exit alone does not prove connection or terminal closure. Forced process kill cannot promise that a dead client executes restoration code; document OS/terminal recovery separately and ensure no false restoration claim.

Record dated sanitized observations with candidate SHA, versions, steps, expected/actual results and observer confirmation. Screenshots are optional and must contain only synthetic material; raw recordings are unnecessary. Missing Terminal.app, Windows Terminal, Linux local-terminal or SSH evidence leaves M11.07 and the final gate open. Do not convert this mandatory matrix into a list of future enhancements.

## 7. Backup and non-destructive restore

### 7.1 Selected scope and public interface

Use the existing SQLite `snapshot_to`/`VACUUM INTO` mechanism as the starting point, fixing its admission and privacy gaps. Do not implement a second persistence format or general export service. Provide user-operated administrative commands, proposed as `rt backup create --destination <new-private-directory>` and `rt backup restore --source <backup-directory> --destination <new-private-home>`, with workspace selection following existing global CLI conventions. Freeze final spelling in help and tests before publishing examples. No TUI backup screen is required.

The supported M11 backup unit is one selected workspace plus the minimal registry information required to restore it. Do not claim an atomic snapshot of every database in a multi-workspace home. Produce a versioned private manifest with workspace ID, schema/application compatibility, database member name, snapshot revision/event watermark, integrity hash and completion status. Avoid unnecessary original paths in the manifest and never emit it into ordinary logs. The database itself remains sensitive.

The public command must not require an external sqlite3 installation or source-tree migrations. Keep storage details in persistence and orchestration in application/daemon/CLI boundaries. If live backup is exposed through IPC, add a bounded authenticated capability/operation with explicit compatibility tests; never let a client supply arbitrary SQL or an unchecked filesystem target.

### 7.2 Backup algorithm

1. Validate a new absolute destination and an approved private parent; refuse any existing target, including empty directories/files, aliases, symlinks and Windows reparse targets. Secure a uniquely owned staging directory before creating files. Do not rely on existence-check followed by ordinary overwrite-capable creation.
2. Admit at most one backup per workspace with a bounded queue or immediate typed busy response. Use the actual SQLite snapshot facility, bound cancellation/deadline behavior, and keep unrelated session input responsive. Determine the manifest watermark from the resulting snapshot, not from a separate live query racing a writer.
3. Create the workspace snapshot and derive the required registry entry from validated identity. For a stopped-home procedure that snapshots the registry separately, validate membership explicitly and do not claim cross-database transactional consistency.
4. Verify schema/version, database integrity, foreign keys, workspace identity and state/event invariants before marking success. Ensure backup/WAL/SHM and staging permissions remain private throughout. Treat a partially written snapshot as incomplete, not as a usable backup.
5. Finalize the manifest and files with a tested no-overwrite publication operation. Verify durability behavior for the bundled SQLite version and selected synchronous setting, including file synchronization and directory publication where supported. Do not claim power-loss proof from a process-kill test.
6. Return a bounded result with identity/status; no raw database content or private path in normal diagnostics. A cancelled/failed operation must preserve source data and any material preexisting target; private owned incomplete staging may be quarantined with safe guidance. Never silently overwrite or auto-restore it.

SQLite documents `VACUUM INTO` as a consistent snapshot, but interruption can leave incomplete output. Its current documentation conditions completed-output synchronization on the source synchronous setting. Verify the bundled version and actual configuration rather than assuming a generic guarantee. See [VACUUM INTO](https://www.sqlite.org/lang_vacuum.html) and [SQLite backup mechanisms](https://www.sqlite.org/backup.html).

### 7.3 Restore algorithm and limits

1. Restore only while the source workspace daemon and destination runtime are stopped. Acquire the relevant ownership/exclusion controls; do not rely solely on a stale PID or a UI statement. Refuse an active original workspace because a cloned identity must not create two daemons operating on the same project/Git metadata.
2. Require a fresh private destination home. Refuse existing content and source/destination identity aliases. Read a bounded manifest, reject unknown versions, unexpected members, traversal, links/reparse targets and checksum mismatches. Do not extract arbitrary archives.
3. Copy only validated backup database material into owned staging, never move or mutate the original or the backup. Open and validate a copy with embedded migrations. Reject newer schemas and corrupt/inconsistent data with typed guidance. Test a populated older schema and transactional migration failure. Record any migration applied to the copy.
4. Build a minimal valid private registry for the restored workspace, preserving workspace/task/history IDs. Do not copy runtime sockets, locks, terminal buffers, process handles, cached leases, or live ownership. No automatic provider launch or worktree add.
5. Verify project/worktree references read-only. Missing or changed paths remain unavailable with explicit recovery guidance; never silently repoint roots or approve new locations. A relocated project is outside automatic restore and requires a separate explicit existing supported operation.
6. Publish the fresh home without replacing anything. Keep the original home stopped when activating the restored copy. On first production start, run ordinary loss reconciliation once and verify active claims close/tasks block consistently. A second start creates no duplicate loss effects.
7. State precisely what is absent: source files, Git object databases/checkouts, provider authentication, environment values, live PTYs and terminal history. Users back up source/Git independently. Database backup is neither encryption nor a portable multi-machine project migration guarantee.

Test successful live snapshot during concurrent task/history writes, read-only reopening, restore/restart, empty/existing targets, privacy, disk-full/injected write errors, lock timeout, interrupted snapshot, corrupt/truncated/checksum-modified data, newer schema, older migration, damaged registry membership, source identity collision and missing worktrees. Assert original bytes/logical state unchanged by restore; account for ordinary SQLite bookkeeping only when explicitly opening the source. Use original-file hashes before/after for corruption failure paths that must not write anything.

## 8. Verification commands and CI

Run narrow relevant regressions after each fix. During implementation, create `hardening_gate` as specified and a focused backup/restore integration target named `backup_restore`. Its exact crate owner may follow the chosen composition, but document the final command. Add offline/resource/manual evidence procedures with explicit completion criteria.

Mandatory existing checks:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo test -p relayterm-git --locked
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner --exit-code 1
git diff --check
```

Mandatory M11 integrated gate, run twice as separate invocations on each stable native OS:

```text
cargo test -p relayterm-cli --test hardening_gate --locked -- --nocapture --test-threads=1
```

Retain the existing detached daemon gate and both M06-M10 repetitions in Quality. Run the full workspace suite and core-only suite on Linux stable, Linux pinned 1.98.1, macOS stable and Windows stable. Keep native OS targets from the current workflow; a runner/toolchain change requires compatibility evidence, not a silent matrix reduction. Cross-compilation may supplement but never replace native runtime tests.

Each native command must fail its own CI step. Split multi-command installation/version steps or explicitly check each native exit status in PowerShell. Do not use `continue-on-error`, `|| true`, an ignored first repetition, hidden retry loops, or test annotations that skip required platforms. Catching a child failure only to print it is a failed harness design.

Run resource/offline tests natively on all three OS targets. Keep long resource jobs independently named and included in required acceptance checks. Security must retain pinned actions, restricted permissions, dependency/license/source scans, candidate-source and history secret scans, and negative controls. Any new dependency must satisfy the pinned compiler, license policy and lockfile.

Two consecutive successful gate executions are required for the final behavior candidate. A failed first pass stays visible and invalidates that pair; fix its cause and obtain a new complete pair. Repeatedly rerunning an unchanged flaky candidate until green is not acceptance. Record failure causes and fixes with synthetic evidence.

Manual observations are not inferred from CI and must remain separate rows. Reuse prior observations only if their exact covered behavior and candidate changes are reviewed, and rerun affected manual scenarios after fixes. Record a documentation-only descendant relationship explicitly if final evidence docs are committed after the tested code SHA.

## 9. Review, delivery, and blockers

Before finalization, reconcile every atomic task, AC clause, risk R1-R11, budget and manual cell against evidence. Review public descriptions against demonstrated behavior. Do not declare a defect fixed because a test was relaxed to fit it.

Critical/high security findings require a fix or an explicit maintainer-reviewed exception identifying impact, scope, rationale, mitigation, expiry/follow-up and reviewer. An agent cannot self-approve a risk exception. Acceptance-blocking behavior remains blocking regardless of severity; an exception cannot silently override the user/specification. Do not place sensitive vulnerability detail in public PRs.

Publish coherent commits and an implementation PR only when authorized. The handoff prompt may authorize push/PR/merge, but never force-push, branch deletion, release publication or destructive runtime operations. A draft/incomplete PR is allowed to expose progress, but is not completion.

Merge only after required reviews, Quality, Security, both native gate passes, resource/offline evidence, backup recovery and the complete manual matrix pass. Fetch before merging, resolve divergence without discarding work, synchronize local main to origin/main and monitor post-merge CI. Delivery may remain an open task until that evidence exists; record merge facts only after they exist, never fabricate a future log entry. If a follow-up documentation change is needed to record delivery, use a branch and the same required checks.

If external access, manual environments, required review or approval is unavailable, complete independent work and state the exact remaining blocker. Keep affected TODO tasks and final gate open; do not merge M11. Do not defer behavioral gaps to M12 or announce a completed MVP. Final report includes PR, merge SHA if any, actual commands, native/manual matrix, measurements, Git versions, CI and remaining limitations.

## 10. Atomic implementation tasks

The following 50 child tasks refine the ten existing M11 parents. Add them before coding. Each row includes its minimum verification; all applicable cross-cutting contracts above still apply. Finish a parent only when all its children are verified. Investigations that reproduce a defect include its fix and regression before closure.

| ID | Concrete task | Required verification |
| --- | --- | --- |
| M11.01a | Audit current baseline, prior PRs, instructions, contracts and R1-R11 coverage. | Source-to-test inventory distinguishes observations, reproductions and missing evidence. |
| M11.01b | Publish acceptance matrix for all AC/FR/NFR clauses with native/manual ownership. | No behavior clause lacks a test or explicit pending observation; M12 boundary exact. |
| M11.01c | Freeze workload, resource inventory and thresholds from section 5 before measuring. | Units/scopes/admission/overflow and native metric definitions reviewed; no results invented. |
| M11.01d | Prepare bounded shared harness and environment/evidence manifest; arrange manual access. | Harness negative controls propagate failure and bound orphan pipes/cleanup; unavailable environments stay pending. |
| M11.01e | Review compatibility and remediation ownership before consolidating APIs. | Every R1-R11 investigation has a child-task owner; protocol/schema changes preserve explicit compatibility and require no destructive migration. |
| M11.02a | Implement real TUI setup, neutral definitions, two task worktrees and three sessions. | Native product composition, actual cwd/argv/snapshot correspondence, source preservation and non-Git journey. |
| M11.02b | Integrate claim race, progress, atomic handover, resume, release and completion. | Independent clients verify state/history/events at each boundary and loser has no effects. |
| M11.02c | Integrate input/resize/history, detach, independent reattach and client closure. | Authoritative state equality, leases and same live children; no simulation or terminal transcript persistence. |
| M11.02d | Integrate graceful/abrupt daemon restart and durable recovery. | Commit/loss invariants, no duplicate reconciliation and no invented live adoption. |
| M11.02e | Reproduce/fix R3-R7 worktree identity, path, launch and partial-result risks. | User-commit launch, shared-Git two-daemon races, metadata identity, conservative receipts and installed migrations. |
| M11.03a | Add malformed/framing/authentication fault tests. | Preallocation bounds, safe rejection and unaffected healthy client. |
| M11.03b | Add admission/commit/response cancellation and unknown-result tests. | No automatic mutation replay; committed result recoverable by exact stable identity. |
| M11.03c | Add slow subscriber, cursor gap, reconnect and lease races. | Bounded queues and eventual authoritative refresh without duplicate durable writes. |
| M11.03d | Add storage kill/write/notifier and spawn/persistence failure tests. | Atomic state/history/events, cleanup, honest lifecycle, unrelated session survival. |
| M11.03e | Add Git phase failure barriers and shutdown while work is pending. | External uncertainty preserved; explicit reconciliation, no repeat add/destructive compensation. |
| M11.03f | Correct R8 harness and CI native-command failure propagation. | First command/repetition failure makes the job fail even if later commands succeed. |
| M11.04a | Reproduce/fix R1 Git streaming/disk/reader bounds. | Overflow during execution, both pipes, descendant handle retention and bounded teardown on all OSs. |
| M11.04b | Test terminal/input/frame/global session budgets and fairness. | Exact limit/plus one, flood and quiet control responsiveness; complete buffer accounting. |
| M11.04c | Correct R11 unbounded queries, caches and cancelled waiters. | Large durable history with bounded SQL/materialization/pages and no erased history. |
| M11.04d | Measure startup/navigation/input under declared sustained load. | Sample counts, actual rates and all native/reference thresholds recorded. |
| M11.04e | Measure memory plateau, handle/task cleanup and reconnect churn. | Numeric RSS/working-set and internal allocation results meet section 5; no hidden child resources. |
| M11.05a | Audit/fix native IPC, ownership and private filesystem boundaries. | Real Unix modes/peer checks and Windows DACL/reparse tests, including intermediate files. |
| M11.05b | Reproduce/fix R2 external Git hooks/filter/config/environment exposure. | Inert marker programs never execute across selected-commit, nested and included config cases. |
| M11.05c | Audit resolver/argv/env/import and launch snapshot data flow. | Empty/duplicate argv preserved, explicit environment only, definition/edit races and private diagnostics. |
| M11.05d | Add complete synthetic sensitive/ANSI source-to-sink tests. | Prohibited fields/logs/artifacts clean; authorized durable content distinguished; no universal detector claim. |
| M11.05e | Update privacy/ADR/spec contracts for verified hardening changes. | Documentation matches tests and version changes; raw data absent from evidence. |
| M11.06a | Build native disposable offline/monitoring procedures and negative controls. | Denial/monitoring mechanism independently detects a synthetic network probe on each OS. |
| M11.06b | Execute real offline workflow including Git, backup and restart without credentials. | All three native OS results, no product TCP/DNS/update/telemetry activity and local IPC works. |
| M11.06c | Review source/dependencies and publish bounded offline evidence. | Runtime/build/child/SSH distinctions explicit; no hosted prerequisite or unsupported claim. |
| M11.07a | Correct R10 optional/manual contradiction and publish observation script. | Required local/SSH cells and failure expectations match section 6.3 and TODO. |
| M11.07b | Execute Linux local-terminal and OpenSSH matrix. | Actual shell/terminal versions, candidate and sanitized observer results; real disconnect/closure. |
| M11.07c | Execute macOS Terminal.app and OpenSSH matrix. | Zsh/Bash results, restoration/full-screen/Unicode and live children after closure. |
| M11.07d | Execute Windows Terminal and Windows OpenSSH matrix. | PowerShell/cmd.exe, ConPTY resize/focus/restoration and real reconnect observations. |
| M11.07e | Resolve manual failures and reconcile complete manual acceptance. | Affected observations repeated after fixes; no missing OS/connection declared passed. |
| M11.08a | Freeze backup unit, manifest, CLI/admission and restore ownership contract. | ADR/spec/help consistency; single-workspace scope and no live-original clone collision. |
| M11.08b | Fix R9 and implement private no-overwrite consistent backup. | Concurrent writer snapshot watermark, bounds, integrity and intermediate permissions. |
| M11.08c | Implement staged non-destructive fresh-home restore with embedded migrations. | Identity/registry validation, originals preserved, missing roots honest and no automatic launch/Git effects. |
| M11.08d | Add native backup fault/compatibility/restart tests. | Corrupt/newer/locked/interrupted/existing/symlink/disk-full cases and one-time loss reconciliation. |
| M11.08e | Publish and execute exact user backup/restore procedure. | Real installed-layout binary away from source tree on all OSs; limitations and safe recovery tested. |
| M11.09a | Complete threat model and control/asset/residual-risk mapping. | Each trust boundary has concrete tested controls and honest limitations. |
| M11.09b | Audit dependencies, licenses, sources and repository/candidate privacy. | Mandatory scans and their negative controls pass for the actual candidate. |
| M11.09c | Fix acceptance/security findings or obtain explicit eligible reviewed exceptions. | No unreviewed critical/high finding; behavior blockers still block; affected regressions rerun. |
| M11.09d | Reconcile public documentation and security guidance. | No stale contradictory claims, private reporting checked without sending a report, no invented governance. |
| M11.10a | Integrate native M11 gates/resource jobs while retaining earlier gates. | Two separately failing passes per stable OS, pinned compiler checks and complete job propagation. |
| M11.10b | Execute all local mandatory controls and integrated/fault/backup regressions. | Exact successful commands recorded; unsupported local tests labeled, not claimed. |
| M11.10c | Obtain final native Quality/Security and resource/offline evidence. | Linux/macOS/Windows candidate-specific results, two gate passes and measured thresholds. |
| M11.10d | Reconcile all AC/manual/risk/budget rows and publish reliability guide. | No unresolved required behavior, all 50 tasks accounted for, M12-only boundaries explicit. |
| M11.10e | Create coherent commits and authorized implementation PR; resolve required review. | PR identifies problem/behavior/evidence; no skipped checks or self-approved exceptions. |
| M11.10f | Merge only verified M11 and synchronize local/remote main. | Actual merge SHA and equality verified, no force-push/deletion; delivery recorded only after fact. |
| M11.10g | Monitor post-merge CI and hand off M12 without implementing it. | Required workflows pass; report PR/merge/tests/native/manual/measurements/limitations, no MVP/release claim. |

Recommended order: M11.01, begin manual access preparation, inherited risk reproductions, correctness fixes and integrated journey, fault/resource/privacy/offline work, backup/restore, manual execution, security review, final native gates and delivery. Do not wait for manual access before making independent implementation progress. Do not merge while it remains absent.
