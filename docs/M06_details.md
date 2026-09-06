# M06: First durable vertical slice

## 1. Purpose and execution contract

M06 proves the Phase 1 exit gate in [specification section 18](../MVP_TECHNICAL_SPEC.md): a workspace retains authoritative coordination context across separate clients and daemon restarts. The test must use actual SQLite files, authenticated local IPC, the administrative `rt` executable, and daemon processes. Synthetic instance lifecycle observations are permitted only inside test executables. A passing in-memory test or an in-process server alone cannot close this milestone.

This document is an implementation contract, not evidence that M06 is implemented. Its documentary delivery closes only M06-DOC. Keep M06.01 through M06.04 pending until their respective gates pass. Read [AGENTS.md](../AGENTS.md), [project vision](../PROJECT_VISION.md), [specification](../MVP_TECHNICAL_SPEC.md), [README](../README.md), [TODO](../TODO.md), and recent [logbook entries](../avances.md) before execution.

Implement on `feature/m06-durable-slice` from synchronized `main`, or continue a suitable existing implementation branch without overwriting work. Add the child tasks in section 10 to TODO before coding. Remove each child only after its stated checks pass and append its evidence to avances in the same change. Preserve previous logbook bytes. A session handoff must identify remaining IDs, commands, failing assertions, candidate revision, and outstanding native evidence.

### 1.1 Scope and acceptance coverage

| Requirement | M06 evidence | Explicit limit |
| --- | --- | --- |
| FR-1, AC-1 | Initialize and reopen an existing directory through real `rt` | No TUI onboarding |
| FR-5, AC-5 and AC-6 | Exclusive claims, lifecycle, durable progress and structured handover | No autonomous task interpretation |
| FR-6, AC-7 foundation | A different synthetic instance resumes from persisted context | No claim of real agent execution |
| FR-9, AC-9 foundation | Daemon restart preserves records and honestly reconciles live metadata | No recovery of live OS processes after host restart |
| AC-11 foundation | Independent clients read authoritative state and event order | No graphical client or interactive performance claim |
| Section 18 | One reproducible process-level continuity scenario | Production PTY and TUI gates remain M07/M08/M11 |

Do not implement provider templates, PTY processes, terminal reconstruction, TUI screens, worktrees, scheduling, transcript ingestion, public export, TCP, telemetry, hosted services, or production simulation switches. Keep IPC version 1 and current storage schema unless a reproduced prerequisite defect makes a deliberate, reviewed contract change necessary. M06 should primarily add tests and documentation, with narrow production fixes only for reproduced continuity defects.

## 2. Baseline and implementation map

The planning baseline is merged M05 commit `c8d4076598f8229717f9e7b3d3e64a9d5fa59c92`, PR #9. Reinspect the actual implementation at execution time. Prior green CI establishes the tests that ran, not every guarantee mentioned by earlier plans.

| Existing location | Responsibility and reuse |
| --- | --- |
| `crates/relayterm-domain/src` | Task/claim/instance invariants and typed events |
| `crates/relayterm-application/src/lib.rs` | Service, registration, internal observations, revision-aware transactions |
| `crates/relayterm-persistence-sqlite/tests/continuity.rs` | Durable and subprocess test patterns; not a substitute for CLI coverage |
| `crates/relayterm-daemon/src/runtime.rs` | Bootstrap, discovery, composition, recovery, readiness and detached entry point |
| `crates/relayterm-daemon/src/lib.rs` | WorkspaceServer, dispatch, admission/drain and event delivery |
| `crates/relayterm-daemon/tests/protocol_gate.rs` | Separate helper processes, two-client journey and uncertainty regressions |
| `crates/relayterm-daemon/tests/runtime_gate.rs` | Recovery and admitted-write shutdown tests |
| `crates/relayterm-cli/src/main.rs` | Public administrative commands and bootstrap client |
| `crates/relayterm-cli/tests/cli.rs` | Native shell starter, isolated roots and production lifecycle gate |
| `crates/relayterm-client/src` | Snapshot staging, revision/watermark handling and reconnect |
| `.github/workflows/ci.yml` | Native quality matrix and dedicated process gate |

Proposed new locations are `crates/relayterm-cli/tests/durable_slice.rs`, `crates/relayterm-cli/tests/support/`, and `docs/first-durable-slice.md`. The integration target belongs beside `rt` so `CARGO_BIN_EXE_rt` identifies the actual candidate executable without PATH ambiguity or nested Cargo builds. Add only needed dev-dependencies for fixture composition. Reuse daemon tests for internal barriers that cannot be reached by public CLI. File names may be refined before implementation, but retain one discoverable command running all required M06 parent scenarios.

### 2.1 Prerequisite investigations

1. In `serve_workspace`, the baseline calls `Observation::ReconcileLost` before `WorkspaceServer::bind` acquires exclusive endpoint ownership. Build a deterministic contender regression with a live owner and an active synthetic claim. A rejected second runtime must leave instances, claims, tasks, revision and events unchanged. If reproduced, acquire authoritative workspace runtime ownership before any recovery mutation, retain it through shutdown, and preserve Unix stale-endpoint and Windows first-pipe protections. A bootstrap-only lock is insufficient when the direct internal runtime entry point can bypass it. Avoid taking the same lock twice or replacing an endpoint owned by another generation.
2. `finish_line_after_exit` bounds child exit but then calls synchronous `read_until` without a byte/deadline bound. A child can block on full stdout before exit, omit a newline, or leave a pipe inherited by a descendant. Test all three. Replace affected harness reads with bounded concurrent draining and explicit cancellation/cleanup. Inspect the production bootstrap reader and subsequent child wait for the same whole-operation deadline issue; fix only reproduced defects, preserving mutation uncertainty.
3. Existing process tests and an in-process recovery test do not prove the complete CLI journey survives a production restart. M06 must bridge that gap rather than relabel those tests.
4. [M05 section 8.5](M05_details.md) requires terminal-close evidence, whereas [platform evidence](supported-platforms.md) records shell-process exit and explicitly excludes terminal-window closure. Record this as inherited missing evidence, not a proven implementation failure. Locate any valid candidate-specific record first. If absent, keep a separate pending prerequisite verification task, obtain a native console/session harness or sanitized manual evidence, and reconcile documentation by appending a correction. Do not silently move an existing requirement to M07 or claim synthetic sessions prove it. Missing required evidence blocks final milestone closure/merge; unrelated M06 work can continue.
5. Inspect shutdown, recovery transaction scope, event payload cardinality and CLI page envelopes before encoding exact expectations. Do not invent signal support or a one-event-per-command rule. Graceful shutdown below uses the implemented generation-targeted command; forced termination is a separate crash scenario.

For each investigation, record reproduction, result, minimal correction if needed, and test name. A suspicious code sequence alone is not proof of an observed failure.

## 3. Harness architecture and isolation

### 3.1 Process boundaries

The parent test orchestrates, owns child handles and checks results. Public mutations and ordinary state reads run in separate actual `rt` subprocesses against a real daemon endpoint. At least two client processes must be independently started; alternating calls on two in-process client objects is supplementary coverage only.

There are two daemon modes of testing, never two product modes:

- The unmodified production `rt` start/stop/bootstrap path proves initialization, detached lifetime, production recovery and reopen behavior.
- A test executable hosts the same production server, persistence, dispatch and lifecycle composition, with a test-only synthetic supervisor. It permits lifecycle registration after startup recovery, then receives ordinary CLI traffic. It must share the production recovery/readiness/ownership code, not maintain a copied simplified runtime.

Use an integration-test executable subprocess helper, following the existing ignored-helper pattern. The parent invokes the exact helper test with an explicit role. An ignored helper is acceptable only when non-ignored parent tests demonstrably run it; the gate itself cannot be ignored. Integration tests do not cause dependency libraries to compile with `cfg(test)`. Do not rely on that misconception to hide a synthetic production API.

If production composition needs a seam, expose a narrow reusable internal lifecycle/composition abstraction with an empty production implementation. Keep synthetic implementations and their control decoder exclusively in test source/dev-dependencies. Do not add production fake commands, feature flags, environment switches, hidden RPCs, provider processes or sample sessions. Document the seam and check the normal release dependency graph. A test host that skips recovery or uses a memory store fails this contract.

### 3.2 Synthetic lifecycle authority

Only the test host may call validated application registration and observation services. The fixture may register a configured definition or generic shell, observe running/final status, acknowledge durable application completion, and synchronize a bounded barrier. It must not directly edit SQL rows or bypass task operations. All task, claim, progress and handover actions in the main journey go through `rt` and public IPC.

Use a parent-owned inherited control channel such as the helper's stdin/stdout, with bounded messages and IDs. It is not the public IPC endpoint and is never available in `rt`. Restrict it to the child helper process, validate message shape, and reject unknown controls. A readiness acknowledgement must follow production ownership, database open and recovery. A registration acknowledgement follows commit and includes the resulting instance/session IDs. No credentials or real commands are executed.

Registration must occur after recovery: pre-seeding a running instance and starting production correctly marks it lost and therefore cannot support a new claim. Never disable recovery to make that fixture work. After a restart, use new synthetic instance IDs through a newly started test host if more work needs a running claimant. Preserve the old instances as lost history.

Public commands always use `LocalUser`. A user claiming on behalf of instance A remains user-attributed; CLI progress has nullable instance attribution and CLI handover has nullable source-instance attribution. The claim identifies the actual owner. Do not assert CLI requests impersonate an instance. Existing direct-service ownership tests remain responsible for instance-actor authorization; add a focused regression only if needed.

### 3.3 Roots, configuration and identities

Each parent scenario owns a unique short native temporary directory with separate project and private-home children. Both must be outside the checkout; private-home must be outside the disposable project. Pass explicit native `--workspace` and `--home` arguments to every `rt` process. Do not repurpose HOME or rely on a developer registry/configuration. Do not mutate process-global environment in parallel Rust tests; set child-specific environment only and remove conflicting Relayterm overrides from child commands.

Run the main journey for a disposable non-Git project and a disposable Git project, both with spaces and Unicode in their project names. Keep Unix runtime socket paths short independently of project names. Use argument arrays, never interpolate paths into shell source. Git initialization is fixture setup only; no user repository commits or worktrees. Do not require any provider executable, credentials or network for the scenario.

Use neutral synthetic definitions such as `Fixture alpha` and `Fixture beta`, nonexistent command names and environment names without values. Use known synthetic summaries, decisions, relative changed paths, verification and next action. Keep `worktree_id` empty. Compare generated IDs by relations and uniqueness, not hardcoded UUIDs. Use monotonic deadlines for waits; compare domain time invariants rather than assuming wall-clock timestamp uniqueness.

### 3.4 Bounds and cleanup

Initial harness limits: readiness and finite-command deadline 30 seconds each, orderly stop deadline 30 seconds, forced-child reap deadline 10 seconds, and parent scenario deadline 180 seconds. Implement shared constants. Any increase needs measured native evidence and a recorded reason, not an unbounded retry. These are test bounds, not changes to product protocol limits.

Drain stdout and stderr concurrently from spawn. For finite JSON commands, enforce a 1 MiB cap per stream and exactly one response envelope. Helper control messages are at most 64 KiB and one acknowledgement per request. For streaming reads, bound individual records to the existing protocol maximum and retain a bounded rolling collection sufficient for the scenario. Never buffer an entire unbounded watch. Check cap plus one byte to detect oversize output.

Cover the entire operation: child execution, reads, acknowledgement, exit and cleanup. A timeout future around a blocking read does not cancel the underlying reader. Prefer cancellable async pipes and explicit child ownership; if threads are used, prove they can terminate and be joined. Missing newline, inherited open pipe, oversized output and blocked stderr must all terminate under the bound. Report only scenario stage, status and safe error code, not raw stdout/stderr or request bodies.

Use an explicit cleanup path that requests generation-targeted shutdown, then kills and reaps only test-owned child handles if needed. Never kill an arbitrary PID discovered from the filesystem. Check endpoint release after exit, close readers and subscriptions, and only then remove the parent-owned temporary directory. Cleanup on assertion failure must preserve the original failure and report a separate cleanup failure safely. Test helper children remain joinable; production detached children use generation-aware stop. If such a child cannot be stopped safely, report that instead of deleting its live database.

Fixed sleeps are not synchronization. Use bounded readiness probes and acknowledgements; brief backoff inside a monotonic-deadline loop is allowed. Use barriers for exact races and crash windows. Never depend on scheduler timing to prove ordering.

## 4. Main continuity journey

Implement `durable_handover_survives_process_restart` as a non-ignored parent, parameterized over Git and non-Git roots. Prefer separate assertions and named stages over one opaque script.

1. Snapshot the disposable source tree. Run actual `rt workspace init` through a disposable native shell, parse the JSON envelope, retain workspace ID and generation, and observe the starter exit. Verify a fresh `rt workspace status` process connects to that daemon. Repeat init/open as supported and assert one workspace registration, no duplicate workspace event and stable identity.
2. Stop the exact production generation and wait for it to release ownership. Start the test-host daemon through the shared composition and require readiness. Verify actual `rt` connects to its normal endpoint. Keep the production-start evidence distinct from the test-host evidence.
3. Register two enabled neutral definitions through public CLI. Through the test-only supervisor register A and B against those definitions, then observe both running. Assert distinct instance/session IDs, workspace membership, immutable launch-definition snapshots and nullable launch task context. Never execute either definition.
4. Create task T with explicit acceptance notes, a relative scope path and normal priority. Read it back: backlog, no owner, no open claim. Transition backlog to ready with a current expected revision.
5. Claim T for A using actual CLI. Assert active, owner A, one open claim, correct requester attribution, and durable claim creation. Capture the claim ID.
6. Attempt a claim for B and then a repeated claim for A. Both must be authoritative rejections and leave revision, watermark, entity state and claim history unchanged. Use current state so the second-claim test is not merely a stale revision test.
7. Append progress through CLI with a nonempty summary and explicit verification. Fetch history through a fresh CLI process and compare exact safe fixture content and user attribution.
8. Attempt a handover with empty required verification or missing next action, according to the current validators. Assert rejection before any partial handover, claim close, task state or event. If next action is currently optional, use required verification for rejection and record the existing field rule rather than silently changing M02.
9. Create a valid structured handover H: summary, decisions, changed relative paths, verification performed, open questions and recommended next action. Query after acknowledgement: exactly H exists, T is handover_ready, claim A is closed for handover, owner is absent, and no claim is open. These changes must belong to the same durable transaction/revision.
10. Explicitly attempt release now and assert no-claim rejection with no effects. Handover already released ownership; do not model it as handover followed by a required successful release.
11. Terminate client A. A fresh client B retrieves H and all prior progress using only task identity and public reads. Assert exact verification and recommended-next-action strings. Do not pass prior narrative content from the parent as a substitute for reads. Expected fixture values are used only by assertions.
12. Claim T for B. Assert a new open claim with a different ID; claim A remains immutable. Append a second progress entry and complete T through the allowed active-to-done transition. Assert both claims closed, final reason completion for B, no owner, unchanged H and preserved progress order. Verify final tasks cannot reopen.
13. Capture a coherent state baseline and the full event prefix. Stop and restart the test host with the same private home, then read using new `rt` processes. T remains done, H and progress remain exact, all closed claims survive, and previously nonfinal synthetic instances are honestly lost. The daemon restart may append legitimate recovery events; compare the entire old prefix and the explicitly expected suffix, not an unchanged total event count.
14. Stop the host and start the real production daemon through `rt daemon start`. Assert a fresh generation, same workspace ID and the same durable business state. No synthetic control exists in this process. Reopen once more and verify recovery has no duplicate effect. Stop the exact generation and check source-tree cleanliness.

A generation change is not an entity identity change. A daemon exit must never mark T done on its own. Completion in step 12 is the only completion command. Keep the main task journey small enough to diagnose; use additional tasks for the variants below.

### 4.1 Explicit release and negative ownership

On a separate task R, claim for a running free instance, release explicitly, and assert blocked, no owner and a closed immutable claim with the explicit-release reason. A second release fails. Claiming blocked fails. Explicitly transition blocked to ready and claim again, producing a new claim ID. Test that an instance already owning T cannot claim another ready task, with no changes to either task. Preserve existing one-task-per-instance and one-claim-per-task constraints.

Use a dependency referencing an unfinished same-workspace task in at least one successful claim to establish informational behavior. Add one cross-workspace reference rejection using a separate isolated workspace and verify neither workspace changes. Do not build a scheduler or repeat the entire M02 state matrix inside this process gate.

## 5. Restart and recovery scenarios

Implement `active_claim_is_recovered_after_daemon_restart` with orderly and abrupt-exit variants. Both use the same SQLite files and new client processes after restart.

1. Start a test host, register running A and idle running B, and a starting C. Also register an already-final instance through valid observations. Create and claim R for A; append verified progress and retain a previously completed task and handover as unaffected controls.
2. Capture workspace revision, event watermark and exact immutable records through coherent public reads. Ensure acknowledged writes have completed before the ordinary crash boundary.
3. Orderly variant: send generation-targeted stop. Crash variant: force termination of the owned test-host child handle and reap it without sending shutdown. Do not delete database, WAL, SHM or endpoint files to make restart pass.
4. Start the actual production daemon. Readiness must follow successful recovery under exclusive ownership. A, B and C become lost with ended_at and unknown exit code. R becomes blocked and its claim closes for instance end. B/C without claims cause no task changes. The already-final instance and all completed work retain their original records.
5. Assert state/history/event atomicity and strictly ordered new events using the current application batch contract. No ready response may expose active R with no running owner, or lost A with an open claim. Test observers cannot see a partial recovery transaction.
6. Stop/start production again and compare revision and event watermark: if no newly nonfinal instances exist, reconciliation is a no-op. Runtime generations may change without creating durable domain events.
7. To continue work, stop production, start the test host with normal recovery, and only then register new running D. Claiming still-blocked R fails. Explicitly make R ready and claim for D. D reads the old progress/handover via actual CLI, appends new progress and completes. A must remain lost and its closed claim unchanged.
8. Restart production once more and verify final state and all history. This distinguishes resuming work from reviving an old synthetic process.

### 5.1 Transaction and connection failure boundaries

Retain the M04/M05 deterministic internal regressions and add process-level evidence where it establishes a missing boundary:

| Boundary | Required result |
| --- | --- |
| Competing claims from separate clients released by a parent barrier | Exactly one success, one authoritative conflict; one open claim; no loser events |
| Disconnect after commit before response | Client reports uncertainty; reconnect/read identifies exactly one effect; no automatic mutation replay |
| Stop after mutation admission before commit | Stop drains the accepted operation; acknowledged result or unknown result is resolved from durable state |
| Kill after an acknowledged commit | Restart preserves the entire committed operation |
| Controlled kill before a commit is permitted | No partial entity/history/event update; recovered pre-operation state |
| Second runtime while first owns live work | Rejected contender cannot reconcile or mutate the owner's state |

Use a test-only transaction/response barrier and acknowledgements to locate the window. If the architecture cannot expose a deterministic process barrier without a broad change, retain the exact in-process transaction fault test and pair it with a process crash after acknowledgement, clearly identifying which level proves each assertion. Do not pretend a random sleep then kill proves a specific precommit window. Never introduce fault-injection controls in production `rt` or public IPC.

For concurrent claims, avoid serializing the requests in the parent. Open both clients first, release both, wait for both results, and then read authoritative state. Do not assume which claimant wins. Match current conflict semantics; a stale expected revision and ownership conflict are distinct negative tests.

## 6. Read consistency, history and events

Public JSON output uses the existing schema-versioned envelope. Revisions and sequences are decimal strings; parse to checked integer types, never floating-point numbers. Validate command name, success flag, exit code and result/error shape. Reject extra finite-command output instead of scraping human text.

The daemon is the source of truth. The parent must not read SQLite directly while serving as its CLI oracle. An additional offline storage inspection after all processes stop is allowed to check integrity and explain a failed assertion, using validated store readers, and is not the primary acceptance path.

For each coherent baseline, collect all snapshot pages at one revision and watermark, restart the entire staged snapshot if the revision changes, and publish only a complete result. Do not compare unrelated first pages collected during mutations. Preserve the existing page limits (default 50, maximum 200) and the existing cursor contract. Force multiple pages with a deliberately small accepted limit such as 2 and enough progress/events to cross it; assert no duplicates, omissions or unstable ordering. Reuse current cursor-expiry tests rather than generating thousands of records unnecessarily.

Map successful commands to actual typed events and transaction revisions before writing assertions. One command can emit multiple events. Failed commands, repeated final observations and a no-op recovery must not append events or advance the durable revision. Verify unique event IDs, workspace ownership, supported payload version, strict sequence order, unchanged pre-restart prefix and correct expected suffix. Do not order by timestamps or assume sequence equals revision.

Start an event observer from a captured watermark before a mutation, then compare delivered events with durable event-list results. Terminate/reconnect a client, resume from the last contiguous cursor and assert no lost application effects. Respect caught-up and resnapshot-required controls. Existing uncertainty tests remain required; the harness never retries a mutation simply because an invocation timed out.

Read the same task, claims, progress and handover from two fresh CLI processes after writes quiesce, compare normalized typed results, and assert the shared snapshot revision/watermark where the API provides them. IDs/timestamps are persisted values and must survive restart exactly unless the record is one legitimately changed by recovery.

## 7. Privacy and source-tree cleanliness

Capture the initial disposable project file set and file contents before initialization. After the journey and all shutdown/restart stages, compare both. For Git fixtures also compare `git status --porcelain` and inspect ignored/untracked files, since Git status alone misses ignored databases and logs. No registry, SQLite database/WAL/SHM, socket, lock, diagnostic file, transcript, generated configuration or hidden Relayterm state may appear in either project. Permit only fixture files deliberately created before the baseline.

Inspect private diagnostics and serialized event payloads against safe synthetic canaries placed in allowed narrative fields and private fixture paths. Authorized task/history reads must return narrative fields; event payloads and diagnostic records must not copy them. Test errors separately for absence of submitted values. Use clearly synthetic noncredential canaries rather than real-looking keys that unnecessarily trip repository scanners. Preserve existing credential-pattern rejection tests.

Never upload private temporary homes, SQLite files, terminal transcripts, full CLI output, environment dumps or raw logs as CI artifacts. A report may contain scenario/test names, platform, toolchain, commit, safe IDs if needed, pass/fail stage and durations. Failure diagnostics must be actionable without exposing data. Validate record/rotation bounds through the existing diagnostic suite and add only missing regressions needed by this harness.

Prove no production fake capability by inspecting normal command help, reserved-operation behavior, source placement and the release dependency graph. A test-only process host is not evidence of actual agent supervision. State this limitation in the published guide and native evidence.

## 8. Native verification and inherited evidence

Required automated execution: Linux `ubuntu-24.04` stable and pinned 1.98.1, macOS `macos-14` stable, Windows `windows-2022` stable. Preserve existing checks and native IPC security tests. Cross-compilation is supplementary only. Do not skip Windows or mark required tests ignored to obtain green CI.

Add a dedicated, visibly named durable-slice workflow step before the full suite, with a finite timeout sized for both project variants and restart tests. Suggested initial job-step bound is 10 minutes, with per-scenario bounds from section 3.4. Confirm the existing whole-suite five-minute budget can accommodate the new tests; adjust only from measurements. The existing 20-minute job budget must still cover all checks or be deliberately revised with evidence. Do not multiply redundant Cargo invocations just to increase apparent evidence.

Run the gate twice on each native target during acceptance to expose cleanup/collision defects, with isolated roots each time. At least one normal full workspace run must retain ordinary test parallelism. Record test counts so zero matching tests cannot pass unnoticed. A narrowly targeted repeat after fixing a failure is useful; also rerun the full affected native matrix for the final candidate.

For the inherited M05 terminal-close gap, first search for existing sanitized evidence. If unavailable, use native console/session lifetime automation where feasible without introducing production PTY support. Otherwise request a precise manual run with OS, shell/terminal, candidate revision, launch, actual terminal closure, independent reconnect and cleanup observations. Keep the missing record pending until supplied. Shell exit alone is reported as shell exit. Do not claim interactive full-screen, SSH reattachment or real session survival from this M06 gate.

## 9. Verification commands and deliverables

During iteration:

```text
cargo test -p relayterm-cli --test durable_slice --locked -- --test-threads=1
cargo test -p relayterm-daemon --test runtime_gate --locked
cargo test -p relayterm-daemon --test protocol_gate --locked
cargo test -p relayterm-cli --test cli --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
```

Before implementation closure:

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

Use `python` on runners where that is the configured interpreter. Preserve the existing security workflow and negative controls. New dev-dependencies must respect MSRV, license policy, lockfile and architecture checks. Domain/application/protocol must still test independently of SQLite, IPC, Git, PTY and TUI adapters.

Publish `docs/first-durable-slice.md` with prerequisites, exact commands, fixture boundaries, expected assertions, clean shutdown, safe failure interpretation and limitations. Commands invoking synthetic sessions must be explicitly test commands, not advertised as available `rt` launch operations. Update README current status and supported-platforms with factual evidence only after verification. Add a concise regression/contract note if a prerequisite fix changes lifecycle ownership. Do not rewrite historical avances claims; append corrections with evidence.

Record candidate SHA, workflow/run URLs, native OS/toolchain and test names/results. After all required evidence exists, publish coherent commits, create a PR to main and watch Quality and Security. Correct in-scope failures and rerun affected checks. Merge only after the required checks/reviews pass and all M06 gates plus the inherited required evidence are satisfied. Do not bypass branch protections. Synchronize local main with origin/main and verify equal commits, then monitor postmerge CI. Report concrete blockers and keep affected TODO tasks open if credentials, reviews or native evidence are unavailable.

## 10. Atomic implementation tasks

Add these IDs under the existing M06 parents before implementation. Child verification can reuse one shared gate when it actually asserts the relevant behavior; cite the exact test and candidate.

### M06.01: Process harness and prerequisite audit

- M06.01a: Reinspect the baseline and reproduce the ownership-before-recovery and whole-operation I/O deadline risks. Add narrow regression tests and fix reproduced continuity defects. Verify the losing runtime has no durable effects and all malformed helper-output cases terminate.
- M06.01b: Create the isolated native fixture layout and child owner/cleanup abstraction. Verify Git/non-Git roots, spaces/Unicode, short endpoints, bounded concurrent output, timeout cleanup and no global environment mutation.
- M06.01c: Connect a test-executable host to shared production composition with lifecycle-only synthetic controls. Verify post-recovery registration acknowledgements, real SQLite/IPC and ordinary `rt` connectivity. Demonstrate that normal production builds expose no synthetic capability.
- M06.01d: Add deterministic readiness, concurrency and fault barriers. Verify the parent tests actually invoke helpers, no gate is ignored, and independent parent runs do not share state.
- M06.01e: Reconcile the inherited M05 terminal-close evidence discrepancy. Obtain missing native evidence or retain this task as explicitly blocked. Append factual correction/evidence without rewriting previous logbook entries.

Gate: a bounded native process harness runs the real administrative client and shared daemon runtime, cleans up safely, and has no production simulation path. Missing M06.01e evidence does not prevent independent coding, but prevents final closure.

### M06.02: Durable coordination journey

- M06.02a: Implement main-journey steps 1-7 for both project kinds. Assert identities, revisions, LocalUser attribution, exclusive claims and exact persisted progress content.
- M06.02b: Implement steps 8-12, including failed handover rollback, atomic handover release, rejected redundant release, fresh-client context reads, successor claim and explicit completion.
- M06.02c: Implement explicit release/reopen/reclaim, one-task-per-instance rejection, informational dependency claim and cross-workspace rejection. Verify every negative operation preserves state/history/revision/events.
- M06.02d: Add the separate-client competing-claim barrier scenario and verify exactly one winner without assuming which client wins.

Gate: all coordination actions use public CLI/IPC, one coherent handover travels between separate clients and distinct synthetic instances, and history remains append-only.

### M06.03: Restart, consistency and privacy

- M06.03a: Implement main-journey steps 13-14 and the orderly/abrupt active-claim recovery variants. Verify actual production startup, lost reconciliation before ready, immutable historical records and idempotent second recovery.
- M06.03b: Implement explicit continuation by a newly registered instance after recovery. Verify old instances/claims never revive and recovered tasks require blocked-to-ready before a new claim.
- M06.03c: Verify transaction/connection boundaries in section 5.1, documenting process versus internal fault-test evidence accurately. Preserve no-replay uncertainty and drain guarantees.
- M06.03d: Implement complete bounded pagination, coherent two-client reads, event-prefix/suffix assertions and reconnect observation. Verify exact content continuity without private transcript input.
- M06.03e: Implement source-tree comparisons and safe canary checks for events/diagnostics/errors; verify private artifacts stay outside both project variants and CI evidence excludes raw state.

Gate: restart preserves acknowledged work and consistent history, active claims reconcile honestly, clients agree, and no private artifact enters the project.

### M06.04: Reproducibility and native exit gate

- M06.04a: Publish the executable test guide and update truthful architecture/status documentation. Verify every documented command, path and scope statement.
- M06.04b: Add the named native CI gate with bounded execution and preserve all existing checks. Obtain two gate runs on each native OS and ordinary-parallel workspace coverage at the final candidate.
- M06.04c: Run all section 9 controls and record exact candidate/platform evidence, including resolution of M06.01e. Correct related failures; unavailable required evidence remains open.
- M06.04d: Close only verified queue entries, append evidence, deliver the implementation PR, merge after checks/reviews, synchronize main and verify postmerge CI. Record PR, merge commit and remaining limitations.

Gate: all M06 behaviors and required native evidence pass. Documentation or a PR alone does not satisfy this gate.

## 11. Final acceptance checklist and downstream handoff

The implementation reviewer must be able to locate evidence for each item:

- Real `rt` initialization and production detached startup, with a separate reconnecting client.
- Synthetic lifecycle controls restricted to test executables and shared production composition.
- Exclusive task and instance claims, rejected repeats and competing processes with no partial effects.
- Append-only progress and a structured handover with readable verification and next action.
- Atomic handover release and a distinct successful explicit-release/reopen/reclaim scenario.
- A successor instance reading prior context through public queries and explicitly completing work.
- Graceful and forced daemon restarts using the same real private SQLite state.
- Honest lost-session reconciliation, blocked active work, preserved final records and no duplicate recovery.
- Production restart after the fixture host, without any fake supervisor in `rt`.
- Coherent bounded reads from two clients, ordered durable events and explicit uncertain-write recovery.
- Bounded waits, output and cleanup, including pipe failure regressions and runtime-contender exclusion.
- Clean disposable source trees and public-safe diagnostics/evidence.
- Passing native Linux, macOS and Windows matrix, core-only checks and required inherited terminal-lifetime evidence.
- Updated test guide, append-only logbook, accurate pending queue and reviewed green delivery.

M07 inherits validated lifecycle observations and durable recovery semantics, but must implement and empirically verify actual OS child handles, PTYs, termination, terminal reconstruction and reattachment. M08 inherits public CLI/protocol coordination behavior as the TUI's reference. M11 still owns the full real-session acceptance matrix. M06 must never describe synthetic continuity as real terminal or agent survival.
