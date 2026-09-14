# Pending tasks

## Roadmap contract

Deliver the single-user, local-first MVP in [MVP_TECHNICAL_SPEC.md](MVP_TECHNICAL_SPEC.md), guided by [PROJECT_VISION.md](PROJECT_VISION.md). The specification remains authoritative. References below use its functional requirements (FR), non-functional requirements (NFR), and numbered acceptance criteria (AC, section 13).

Keep the daemon responsible for durable state and supervised processes. Keep domain and application logic independent of terminal rendering, SQLite, PTY, Git, and provider implementations. Ship `rt` as the only required user-facing executable on Linux, macOS, and Windows.

### How to execute this queue

- Start with M11. Follow milestone dependencies and the task order within each milestone; consult `avances.md` for fulfilled dependencies.
- Treat each task ID as one reviewable outcome, including its relevant tests and documentation. Split a task before coding if its implementation cannot be reviewed coherently in one session. Preserve its ID as a prefix for new child tasks.
- For a milestone ending in `[PLAN]`, complete its first planning task before implementation. Record the named decisions, contracts, failure cases, and test design, then refine the remaining tasks in this file. The marker identifies unresolved engineering details, not permission to expand MVP scope.
- At each session start, read repository instructions, check the branch and working tree, and consult `avances.md` for satisfied dependencies. Continue the earliest unblocked task. Work on a feature or fix branch, never directly on `main` or `master`.
- Every implementation task includes proportionate tests, public-safe fixtures, applicable documentation, and migration or compatibility notes. Run narrow checks while iterating and the broader Rust formatting, lint, and test checks before handoff. Do not claim platform coverage from cross-compilation alone.
- A milestone's exit gate is a pending verification task. Do not advance past it until the evidence passes. An unavailable platform or service remains a recorded limitation and an open gate, not a successful check.
- After a task passes verification, remove its entry and append its ID, result, and exact checks to `avances.md` in the same change. Remove an empty milestone after its gate passes. Keep completed work and test reports out of this queue.
- Before ending an incomplete session, refine the remaining task with the concrete next step and blocker if any. Do not mark unverified work complete. Keep acceptance coverage synchronized as tasks move to the logbook.

### Scope and sequencing decisions

Use the specification's recommended Rust stack and the pinned bootstrap toolchain. Verify support when adding dependencies; avoid unused dependencies and empty adapter crates until needed. Follow the eight [architecture decisions](docs/architecture/README.md), refining the risky ones at their implementation gates.

The durable first-slice, real PTY, TUI, editable agent-template, and explicit worktree-isolation gates are complete. M11 and M12 validate and prepare the release candidate.

Public export and disk-backed scrollback are optional and are deferred from this roadmap. Keep scrollback bounded in daemon memory. Do not add transcript ingestion, provider APIs, automatic task mutation from agent prose, autonomous orchestration, TCP listeners, hosted dependencies, telemetry, graphical clients, or automatic Git commits, merges, rebases, or deletion. Any later export proposal must first add exact-content preview, redaction, and destination confirmation as required by FR-10.

Keep task dependencies informational, as defined in the specification and [M02 storage contract](docs/M02_storage_contract.md); do not introduce a scheduler. Git worktree creation is included because AC-10 requires it, even though several related requirements use SHOULD.

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
| AC-1 through AC-16 | M12.06 | Reconcile existing M11 evidence with the release candidate and repeat affected behavior only. |
| AC-1, AC-14 | M12.03 | Execute the published quick start from a clean installation without hosted accounts. |
| AC-12, AC-16 | M12.01, M12.02 | Verify release artifacts, clean installation, PATH, and an existing unrelated command. |

FR-10 export remains deferred. M12 retains installation, compatibility, documentation, and release acceptance work; completed behavior evidence is in the acceptance matrix and delivery log.

## Usability follow-up proposals

- UX-SESSION-NAMES: Plan user-editable session names so users can identify and select sessions without comparing full UUIDs. Preserve stable internal session and instance IDs and keep them available in details. Define persistence, rename behavior, and concise list labels before implementation. Requested during M11 manual observation; scheduling remains pending and this proposal does not add an M11 acceptance gate.

- UX-SESSION-ORDER: Review predictable session ordering alongside editable names; consider creation order with new sessions appended at the end, preserving selection by identity. Scheduling remains pending.

- UX-FORM-CURSOR: Investigate the reported invisible insertion cursor in task edit forms and provide a visible editing position. Keep Ctrl-U documented as clearing the active field, not enabling editing. Scheduling remains pending.

- UX-CONFLICT-MESSAGE: Replace the generic rejected-request message for a stale form submission with safe guidance that identifies a concurrent edit, states that the local draft was retained, and explains explicit discard or reconciliation. Scheduling remains pending and this proposal does not add an M11 acceptance gate.

- UX-INPUT-ACQUIRE-MESSAGE: Surface a safe inline explanation when a competing input acquisition is rejected, while retaining the redacted Events diagnostic and read-only state. Scheduling remains pending and this proposal does not add an M11 acceptance gate.
