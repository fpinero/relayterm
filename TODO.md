# Pending tasks

## Roadmap contract

Deliver the single-user, local-first MVP in [MVP_TECHNICAL_SPEC.md](MVP_TECHNICAL_SPEC.md), guided by [PROJECT_VISION.md](PROJECT_VISION.md). The specification remains authoritative. References below use its functional requirements (FR), non-functional requirements (NFR), and numbered acceptance criteria (AC, section 13).

Keep the daemon responsible for durable state and supervised processes. Keep domain and application logic independent of terminal rendering, SQLite, PTY, Git, and provider implementations. Ship `rt` as the only required user-facing executable on Linux, macOS, and Windows.

### How to execute this queue

- Start with M05. Follow milestone dependencies and the task order within each milestone; consult `avances.md` for fulfilled dependencies. Complete M06 before production PTY or TUI implementation.
- Treat each task ID as one reviewable outcome, including its relevant tests and documentation. Split a task before coding if its implementation cannot be reviewed coherently in one session. Preserve its ID as a prefix for new child tasks.
- For a milestone ending in `[PLAN]`, complete its first planning task before implementation. Record the named decisions, contracts, failure cases, and test design, then refine the remaining tasks in this file. The marker identifies unresolved engineering details, not permission to expand MVP scope.
- At each session start, read repository instructions, check the branch and working tree, and consult `avances.md` for satisfied dependencies. Continue the earliest unblocked task. Work on a feature or fix branch, never directly on `main` or `master`.
- Every implementation task includes proportionate tests, public-safe fixtures, applicable documentation, and migration or compatibility notes. Run narrow checks while iterating and the broader Rust formatting, lint, and test checks before handoff. Do not claim platform coverage from cross-compilation alone.
- A milestone's exit gate is a pending verification task. Do not advance past it until the evidence passes. An unavailable platform or service remains a recorded limitation and an open gate, not a successful check.
- After a task passes verification, remove its entry and append its ID, result, and exact checks to `avances.md` in the same change. Remove an empty milestone after its gate passes. Keep completed work and test reports out of this queue.
- Before ending an incomplete session, refine the remaining task with the concrete next step and blocker if any. Do not mark unverified work complete. Keep acceptance coverage synchronized as tasks move to the logbook.

### Scope and sequencing decisions

Use the specification's recommended Rust stack and the pinned bootstrap toolchain. Verify support when adding dependencies; avoid unused dependencies and empty adapter crates until needed. Follow the eight [architecture decisions](docs/architecture/README.md), refining the risky ones at their implementation gates.

The remaining first-slice route is M05 through M06: daemon lifecycle, an administrative CLI workflow, and the durable vertical-slice gate. M07 adds real PTYs; M08 adds the TUI; M09 and M10 add templates and explicit worktree isolation; M11 and M12 validate and prepare the release candidate. After M08, M09 and M10 may proceed independently. Windows PTY behavior must be validated when that adapter is introduced.

Public export and disk-backed scrollback are optional and are deferred from this roadmap. Keep scrollback bounded in daemon memory. Do not add transcript ingestion, provider APIs, automatic task mutation from agent prose, autonomous orchestration, TCP listeners, hosted dependencies, telemetry, graphical clients, or automatic Git commits, merges, rebases, or deletion. Any later export proposal must first add exact-content preview, redaction, and destination confirmation as required by FR-10.

Keep task dependencies informational, as defined in the specification and [M02 storage contract](docs/M02_storage_contract.md); do not introduce a scheduler. Git worktree creation is included because AC-10 requires it, even though several related requirements use SHOULD.

## M04 follow-up: Complete protocol acceptance gaps

Depends on: merged M04 implementation. Specification phase: 1. Contract: [M04 detailed plan](docs/M04_details.md).

- M04.07c-M04.07g: Connect coalesced notifier wakeups to the durable event pump, implement a real bounded per-subscription queue with unsubscribe cleanup and slow-subscriber isolation, and add deterministic replay/race/resource tests.
- M04.08c: Add explicit local cancellation before and after writer ownership, preserve mutation uncertainty, close unusable correlations, and verify late responses cannot poison later calls.
- M04.09d: Verify malformed peers, connection/request limits, slow-client isolation, cursor error mapping, and service survival with unaffected clients.

## M05: Daemon lifecycle and administrative CLI

Depends on: M04. Specification phase: 1. Coverage: FR-1, FR-5, FR-6, FR-8, FR-9; AC-1, AC-2, AC-9, AC-16 foundations.

Outcome: `rt` can operate durable coordination state through an independent daemon.

- M05.01: Assemble the daemon from application services, SQLite, local transport, and injectable supervision. Implement endpoint discovery and singleton locking; test simultaneous starts and verify an existing live endpoint is never removed as stale.
- M05.02: Implement daemon start/status/stop behavior through `rt`, with OS-specific detachment from the launching client. Test closing the starter process and reconnecting from another client on each supported OS.
- M05.03: Implement orderly shutdown with transaction completion, IPC closure, and an explicit child-process policy ready for M07. Test interrupted requests and durable writes without depending on TUI cleanup.
- M05.04: Reconcile persisted starting/running sessions on daemon restart using the approved lost-session and claim policy. Commit related events atomically; test repeated restart is idempotent and historical exit details remain intact.
- M05.05: Add administrative commands for workspace init/open/status, agent configuration, tasks and claims, progress, handovers, history, and event inspection. Route all state through IPC; validate paths and arguments and provide script-usable output and failure exit codes.
- M05.06: Add structured severity levels, stable diagnostic event names, safe path aliases, and bounded log retention. Map missing executables, database failures, version mismatch, and IPC failures to actionable messages without logging payloads or sensitive arguments.
- M05.07: Document and verify the daemon/CLI workflow in disposable Git and non-Git projects, including directories with spaces and Unicode. Confirm state is stored outside project roots and two clients share one authoritative state.

## M06: First durable vertical slice

Depends on: M05. Specification phase: 1 exit gate. Coverage: FR-1, FR-5, FR-6, FR-9; AC-1, AC-5, AC-6, AC-7, AC-9, AC-11 foundations; section 18.

Outcome: prove continuity before adding real PTY and TUI complexity.

- M06.01: Build a process-level test harness with isolated data/runtime directories and synthetic instance supervision. Run the real daemon and administrative client against real SQLite and IPC; keep the fake supervisor confined to tests.
- M06.02: Automate workspace initialization, task creation, exclusive claim, rejected competing claim, progress, structured handover, release, and resume by another synthetic instance. Assert verification and next action remain readable without any private transcript.
- M06.03: Restart the daemon in that scenario and assert durable entities, claim history, honest lost-session reconciliation, and ordered events survive. Repeat state reads from two clients and check the source tree contains no runtime-private artifacts.
- M06.04: Publish the reproducible first-slice test instructions and verify the gate in Linux, macOS, and Windows CI. Run core/protocol checks without the TUI. Only proceed to production PTY/TUI work when this scenario passes; real-session acceptance remains pending in M07 and M11.

## M07: Real PTY supervision and reattachment [PLAN]

Depends on: M06. Specification phase: 2. Coverage: FR-2, FR-3, FR-4, FR-8, FR-9; NFR-2, NFR-3, NFR-8; AC-3, AC-4, AC-8 foundations.

Outcome: three independently supervised interactive sessions survive client disconnection with bounded resources.

- M07.01: Prototype `portable-pty` and provider-neutral terminal reconstruction on all three OS targets. Exercise alternate-screen output, cursor movement, split escape sequences, Unicode, resize, and reconnect after scrollback truncation. Refine the ADR with evidence, input ownership for multiple attached clients, resize authority, signals/termination semantics, and queue overflow policy before production implementation.
- M07.02: Add a synthetic interactive child fixture with deterministic modes for echo, size reporting, full-screen redraw, high-volume output, controlled exit, and crash. Verify it runs without network access or provider credentials on every OS.
- M07.03: Implement generic shell/custom-command launch configuration with executable discovery, argument arrays, validated working directories, and approved environment inheritance. Test missing/disabled commands, unsafe paths, spaces, Unicode, and secret-free error reporting.
- M07.04: Implement PTY allocation, child spawn, instance/session association, start failure, and exit observation. Persist lifecycle changes through application services and test a failed launch leaves no falsely running session.
- M07.05: Implement bounded binary output streaming and daemon-owned in-memory scrollback. Test non-UTF-8 bytes, split terminal sequences, retention boundaries, and high-volume output without persistent terminal captures.
- M07.06: Implement input routing, resize propagation, and explicit termination with the approved platform semantics. Test correct target selection and final status without affecting unrelated children.
- M07.07: Implement attach/detach and reconstruction from retained terminal state using the M07.01 design. Race output against attachment and verify ordering, truncation recovery, full-screen state, and multiple-client ownership behavior.
- M07.08: Implement session caps, per-client queue bounds, fairness, and backpressure. Test slow/disconnected consumers cannot exhaust memory or block control commands and other sessions.
- M07.09: Connect session IPC operations and CLI diagnostics to the real supervisor. Test launch/claim/release with real instances, child failure isolation, daemon shutdown policy, and restart reconciliation without attempting unsupported live-process recovery.
- M07.10: Verify the PTY gate on all three platforms: one shell and two synthetic interactive agents run concurrently; input, full-screen behavior, resize, exit, detach, and reattach pass. Close the client and SSH connection where available while keeping the host/daemon alive; record terminal-specific manual evidence when automation cannot establish behavior.

## M08: Complete TUI workflow [PLAN]

Depends on: M07. Specification phase: 3. Coverage: FR-1 through FR-6, FR-8; NFR-3, NFR-6, NFR-7.

Outcome: the user completes coordination and terminal workflows through `rt` without the administrative CLI.

- M08.01: Specify the minimum screen/navigation model, terminal focus escape binding, resize rules, forms, confirmation for session termination, reconnect states, and small-terminal behavior. Reuse M07 terminal semantics and define testable presentation state separate from transport effects.
- M08.02: Replace the default `rt` placeholder with workspace selection/open/init and daemon discovery/start followed by TUI connection. Test first launch, reconnect, unavailable workspace, and non-interactive/unsupported terminal errors.
- M08.03: Implement raw-mode and alternate-screen lifecycle with restoration on normal exit, errors, and panic. Verify the user's terminal remains usable after every tested exit path.
- M08.04: Implement workspace overview, task list/board, owner/status indicators, and agent/instance listing from client snapshots and events. Test empty/loading/error states and essential state without color.
- M08.05: Implement task create/edit/transition/claim/release forms and claim history. Test invalid transitions and competing claim errors preserve user context and reflect authoritative daemon state.
- M08.06: Implement task detail, progress append, structured handover creation/read, and resume by a different instance. Test required fields, length limits, validation errors, and visible verification/next action.
- M08.07: Implement shell/agent launch, attach/detach, terminal switching, input forwarding, resize, and exit reporting. Render provider-neutral terminal state correctly without opening SQLite or owning child processes.
- M08.08: Implement reconnect and fresh snapshot/event refresh with explicit disconnected state and safe handling of pending mutations. Test disconnect during editing and active terminal output without accidental duplicate writes.
- M08.09: Implement keyboard help, redacted event/diagnostic view, escaped untrusted text outside terminal panes, and minimum-size guidance. Test monochrome rendering, keyboard-only operation, Unicode, and small layouts.
- M08.10: Verify the TUI gate with the primary shell/agent/task/handover/resume scenario, client close/reopen, and session reattachment on each platform. Measure connection-to-interactive time against the two-second target and input responsiveness under output load; record environment and measurements.

## M09: Editable provider templates and generic integration

Depends on: M08. Specification phase: 4. Coverage: FR-2, FR-8; AC-14, AC-15.

Outcome: built-in providers and an unknown custom CLI use equivalent public configuration and lifecycle paths.

- M09.01: Add editable Claude Code, Codex, and OpenCode templates using the generic definition schema. Verify current executable names and supported arguments against official provider documentation during implementation; avoid automatic authentication, installation, or credential access.
- M09.02: Expose definition registration/editing/enabling through the TUI and existing neutral IPC/configuration contract. Test custom commands and built-in templates have the same editable fields and no privileged domain branches.
- M09.03: Add credential-free executable availability checks and actionable missing-command guidance. Test missing, disabled, and invalid commands without logging environment values or sensitive arguments.
- M09.04: Document how to add an arbitrary interactive CLI, configure capabilities and environment variable names, and authenticate within the provider's own tool. State that output does not automatically mutate tasks and no provider key is stored by Relayterm.
- M09.05: Verify an unfamiliar synthetic CLI can be registered, launched, assigned a task, detached, and replaced for handover without domain/TUI code changes. Run optional manual smoke checks for installed, already authenticated provider CLIs; unavailable providers must not block account-free automated acceptance.

## M10: Explicit Git worktree isolation [PLAN]

Depends on: M08. Specification phase: 4. Coverage: FR-7, FR-8; sections 6.7, 9.5; AC-10.

Outcome: two tasks can explicitly use separate working directories without destructive Git automation.

- M10.01: Refine the worktree ADR into naming/root validation, base-ref selection, task association schema, and a recoverable creation sequence across Git and SQLite. Specify duplicate requests, branch/path collisions, missing Git, and failure after Git succeeds but before the database commits. Recovery must preserve files and report partial outcomes.
- M10.02: Add repository detection and project-owned worktree listing through a narrow argument-array Git adapter. Test Git absence, non-Git roots, linked worktrees, unusual paths, and reliable machine-readable output parsing.
- M10.03: Implement branch/path validation and explicit worktree creation. Test traversal, option-like names, symlink escapes, preexisting branches/directories, concurrent collisions, and invalid base refs in disposable repositories.
- M10.04: Persist worktree records and task branch/path associations using versioned migration and events. Test restart, migration, partial creation reconciliation, and repeated requests without deleting or overwriting existing work.
- M10.05: Complete worktree create/list IPC operations and add the explicit TUI creation/selection flow. Launch task sessions in the validated associated worktree; test that changing one task's selection never changes another session's working directory.
- M10.06: Document prerequisite Git installation, conflict resolution, ownership, and manual cleanup guidance. Verify the gate with two tasks in separate worktrees, injected partial failure, and the complete non-Git workflow still functional on all three OS targets.

## M11: Reliability, privacy, and complete acceptance [PLAN]

Depends on: M09 and M10. Specification phase: 5. Coverage: FR-1 through FR-9; NFR-1 through NFR-8; all ACs, with distribution checks completed in M12.

Outcome: demonstrated MVP behavior and recovery under faults, with bounded resources and public-safe diagnostics.

- M11.01: Define a release acceptance matrix mapping each AC to automated tests, required manual observations, OS/shell/terminal combinations, and evidence locations. Define concrete workloads and pass thresholds for memory bounds, input latency, and startup timing before measurement. Treat unsupported required behavior as a blocker, not a documentation-only exemption.
- M11.02: Run the full daemon/client/real-PTY scenario: initialize, launch three sessions, reject competing claims, append progress, hand over, resume elsewhere, detach/reattach, restart, and use isolated worktrees. Assert durable state and events after each boundary on all three platforms.
- M11.03: Inject malformed/truncated IPC, request cancellation, client crashes, slow subscribers, child spawn failures/crashes, and daemon termination during writes. Verify transaction atomicity, useful errors, independent session survival, and recoverable reconnect behavior.
- M11.04: Test every configured bound: scrollback, terminal frames, pending requests/subscribers, event page/history delivery, session count, text lengths, and log retention. Measure memory and control/UI responsiveness during sustained output and verify bounded recovery after repeated reconnects.
- M11.05: Review current-user endpoint access, runtime/database permissions, path boundaries, configuration, environment handling, and sensitive text ingestion. Inject obvious fake secret/path markers and ANSI content; verify they do not leak into prohibited durable fields, diagnostics, or generated project artifacts. Document limitations of secret detection without promising universal detection.
- M11.06: Verify runtime workflows in an offline environment with synthetic agents and no provider credentials. Inspect endpoint/network behavior to establish that no TCP listener, telemetry, update request, or hosted service is required; distinguish dependency installation from product runtime.
- M11.07: Execute the manual terminal matrix for full-screen rendering, resize, Unicode, focus escape, no-color use, small windows, and SSH detach/reattach. Attach sanitized observations and retain open tasks for any missing OS evidence.
- M11.08: Add user-operated database backup and restore instructions/tooling appropriate to the SQLite ADR. Test a consistent backup, restore into an isolated location, version compatibility, locked/corrupt database guidance, and preserved originals without destructive overwrite.
- M11.09: Complete the threat model and privacy review, scan repository/fixtures/release candidates, and run dependency audit/license checks. Resolve critical/high findings or record explicit reviewed exceptions with rationale and follow-up; fix other acceptance-blocking findings and rerun affected checks.
- M11.10: Verify the hardening gate with passing cross-platform builds, formatting, linting, unit/integration/end-to-end tests, scans, and all behavior AC evidence. Carry only installation/release-specific checks into M12; do not declare the MVP complete yet.

## M12: Installable release candidate and final handoff

Depends on: M11. Specification phase: 5 final gate. Coverage: NFR-1, NFR-4; AC-12, AC-16 and final AC-1 through AC-16 sign-off.

Outcome: a reproducible candidate that a new user can install and use from the documented quick start.

- M12.01: Define and document supported release targets and reproducible release-build commands producing `rt` or `rt.exe`. Build candidate artifacts with license notices and checksums; verify fresh-machine execution without an undeclared runtime toolchain dependency.
- M12.02: Write installation/PATH guidance that checks for an existing `rt` command and explains resolution without overwriting an unrelated executable. Test clean installation and a synthetic naming conflict on Linux, macOS, and Windows.
- M12.03: Write the quick start covering workspace initialization, three sessions, claim conflict, progress/handover/resume, detach/reattach, restart limits, and optional explicit worktrees. Execute the published instructions from a clean environment using synthetic agents and no hosted account.
- M12.04: Document upgrade and compatibility behavior, backup/restore, shutdown, missing executables, unsupported terminals, IPC/database errors, and recovery of lost sessions. Verify command examples and link each recovery procedure to tested behavior.
- M12.05: Reconcile README, vision, specification, and architecture documentation with the implemented candidate, retaining MVP exclusions and recording deliberate specification changes. Obtain maintainer choices for governance, code of conduct, and release policy before a public release; confirm the private reporting route still works.
- M12.06: Verify the final gate: all 16 ACs have passing evidence for applicable automated and manual checks on Linux, macOS, and Windows, all phase exit criteria are met, and clean-machine installation/quick start passes. Record candidate revision, commands, matrix results, and remaining non-blocking limitations in the release handoff and `avances.md`.
- M12.07: Prepare local release notes and the reviewed artifact inventory. Hand off the candidate for an explicit maintainer publishing decision. Do not push, open a PR, merge, tag, or publish a release as an implied step of this roadmap.

## Acceptance coverage for remaining work

This index identifies the planned proof for each specification criterion. As milestones are completed, move their evidence to `avances.md` and retain only outstanding coverage here.

| Criterion | Planned implementation | Required proof |
| --- | --- | --- |
| AC-1: Private workspace initialization | M05 | M06.03; M11.02; M12.03 |
| AC-2: Independent daemon and restricted IPC through `rt` | M05, M08 | M05.02; M08.10; M11.05 |
| AC-3: Three real concurrent sessions | M07 | M07.10; M11.02 |
| AC-4: Full-screen input, resize, switching, exit, bounds | M07, M08 | M07.10; M08.10; M11.04; M11.07 |
| AC-5: Exclusive instance claim | M05 | M06.02; M11.02 |
| AC-6: Progress and structured handover | M08 | M06.02; M08.06; M11.02 |
| AC-7: Another instance resumes shared work | M05, M08 | M06.02; M08.10; M11.02 |
| AC-8: TUI detach and reattach preserves children | M05, M07, M08 | M07.10; M08.10; M11.07 |
| AC-9: Durable restart and honest session loss | M05, M07 | M06.03; M07.09; M11.03 |
| AC-10: Task session in an explicit worktree | M10 | M10.06; M11.02 |
| AC-11: Core and protocol independent from TUI | M06, M11 | M06.04; M11.10 |
| AC-12: Cross-platform CI and quality checks | All remaining implementation milestones | M11.10; M12.06 |
| AC-13: No injected secrets, personal paths, or transcripts in logs/artifacts | M05, M07, M08 | M11.05; M11.09 |
| AC-14: No hosted service, API key, or graphical requirement | M05, M07, M09 | M09.05; M11.06; M12.03 |
| AC-15: Unknown CLI requires no domain/TUI changes | M07, M09 | M09.05 |
| AC-16: Single `rt` executable and naming conflict guidance | M05, M12 | M12.01; M12.02 |

FR-10 remains conditional: export is deferred, and no automatic project export may be introduced. Remaining NFR coverage is carried by M12 (platforms), M03-M07/M11 (reliability), M07-M08/M11 (performance), M03-M04/M12 (compatibility), M05/M11 (observability), M08/M11 (accessibility), M06 (maintainability), and M03-M04/M07/M11 (resource limits).
