# Pending tasks

## Roadmap contract

Deliver the single-user, local-first MVP in [MVP_TECHNICAL_SPEC.md](MVP_TECHNICAL_SPEC.md), guided by [PROJECT_VISION.md](PROJECT_VISION.md). The specification remains authoritative. References below use its functional requirements (FR), non-functional requirements (NFR), and numbered acceptance criteria (AC, section 13).

Keep the daemon responsible for durable state and supervised processes. Keep domain and application logic independent of terminal rendering, SQLite, PTY, Git, and provider implementations. Ship `rt` as the only required user-facing executable on Linux, macOS, and Windows.

### How to execute this queue

- Start with M12. Follow milestone dependencies and the task order within each milestone; consult `avances.md` for fulfilled dependencies.
- Treat each task ID as one reviewable outcome, including its relevant tests and documentation. Split a task before coding if its implementation cannot be reviewed coherently in one session. Preserve its ID as a prefix for new child tasks.
- For a milestone ending in `[PLAN]`, complete its first planning task before implementation. Record the named decisions, contracts, failure cases, and test design, then refine the remaining tasks in this file. The marker identifies unresolved engineering details, not permission to expand MVP scope.
- At each session start, read repository instructions, check the branch and working tree, and consult `avances.md` for satisfied dependencies. Continue the earliest unblocked task. Work on a feature or fix branch, never directly on `main` or `master`.
- Every implementation task includes proportionate tests, public-safe fixtures, applicable documentation, and migration or compatibility notes. Run narrow checks while iterating and the broader Rust formatting, lint, and test checks before handoff. Do not claim platform coverage from cross-compilation alone.
- A milestone's exit gate is a pending verification task. Do not advance past it until the evidence passes. An unavailable platform or service remains a recorded limitation and an open gate, not a successful check.
- After a task passes verification, remove its entry and append its ID, result, and exact checks to `avances.md` in the same change. Remove an empty milestone after its gate passes. Keep completed work and test reports out of this queue.
- Before ending an incomplete session, refine the remaining task with the concrete next step and blocker if any. Do not mark unverified work complete. Keep acceptance coverage synchronized as tasks move to the logbook.

### Scope and sequencing decisions

Use the specification's recommended Rust stack and the pinned bootstrap toolchain. Verify support when adding dependencies; avoid unused dependencies and empty adapter crates until needed. Follow the eight [architecture decisions](docs/architecture/README.md), refining the risky ones at their implementation gates.

The durable first-slice, real PTY, TUI, editable agent-template, and explicit worktree-isolation gates are complete. M12 prepares the installable release candidate using the completed M11 evidence.

Public export and disk-backed scrollback are optional and are deferred from this roadmap. Keep scrollback bounded in daemon memory. Do not add transcript ingestion, provider APIs, automatic task mutation from agent prose, autonomous orchestration, TCP listeners, hosted dependencies, telemetry, graphical clients, or automatic Git commits, merges, rebases, or deletion. Any later export proposal must first add exact-content preview, redaction, and destination confirmation as required by FR-10.

Keep task dependencies informational, as defined in the specification and [M02 storage contract](docs/M02_storage_contract.md); do not introduce a scheduler. Git worktree creation is included because AC-10 requires it, even though several related requirements use SHOULD.

## M12: Usability, installable release candidate, and final handoff

Depends on: M11. Specification phase: 5 final gate. Coverage: NFR-1, NFR-4; AC-12, AC-16 and final AC-1 through AC-16 sign-off.

Outcome: a reproducible candidate with usable session identification and editing feedback that a new user can install and use from the documented quick start.

Execution contract: [M12 detailed plan](docs/M12_details.md). Continue at M12.00h. The five usability proposals are scheduled here and do not reopen M11 acceptance.

- M12.LINUX-RETEST: Physically retest the correction branch on Linux and the affected paths on macOS/Windows: disconnected indicators, release chord navigation, bounded diagnostics, and rename context after review. Reconcile acceptance of the demonstrated SSH transport-loss recovery and fixed-grid history limits recorded in docs/m12-linux-final-report.md; preserve the original failed observations.

- M12.NATIVE: Complete Linux correction-candidate acceptance while retaining the 9cf91f7 baseline, affected macOS header/writer-size retests, and the focused SSH observation through an authorized connection. Reconcile clean-machine and global acceptance evidence. Preserve the Windows source mapping in docs/m12-windows-manual-observations.md; use the existing Linux task prompt in docs/m12-native-continuation.md.


- M12.00: Complete UX-SESSION-NAMES, UX-SESSION-ORDER, UX-FORM-CURSOR, UX-CONFLICT-MESSAGE, and UX-INPUT-ACQUIRE-MESSAGE before freezing release artifacts. Preserve durable identity, bounded state, typed error semantics, and read-only input ownership.
  - M12.00h: Verify consolidated usability changes and update help/privacy/compatibility. Completion proof: Targeted native/manual matrix and affected SSH coverage recorded; unaffected M11 evidence mapped.

- M12.02: Write installation/PATH guidance that checks for an existing `rt` command and explains resolution without overwriting an unrelated executable. Test clean installation and a synthetic naming conflict on Linux, macOS, and Windows.
  - M12.02a: Write explicit user-local installation and discovery procedures. Completion proof: Commands validated per shell and actual installed resolution identified.
  - M12.02c: Verify clean installations on three targets. Completion proof: Runtime inventory, checksum, target, help/version and TUI startup recorded.
  - M12.02d: Write and test removal and platform-warning guidance. Completion proof: Only owned install files removed; state and unrelated PATH entries preserved; signing claims accurate.
- M12.03: Write the quick start covering workspace initialization, three sessions, claim conflict, progress/handover/resume, detach/reattach, restart limits, and optional explicit worktrees. Execute the published instructions from a clean environment using synthetic agents and no hosted account.
  - M12.03b: Execute installed initialization and three-session UI journey. Completion proof: All targets; new names/order/cursor and stable session identity observed.
- M12.05: Reconcile README, vision, specification, and architecture documentation with the implemented candidate, retaining MVP exclusions and recording deliberate specification changes. Obtain maintainer choices for governance, code of conduct, and release policy before a public release; confirm the private reporting route still works.
  - M12.05c: Obtain only missing publication policy choices. Completion proof: Existing sole-maintainer decision respected; optional preferences distinguished from technical gates.
- M12.06: Verify the final gate: all 16 ACs have passing evidence for applicable automated and manual checks on Linux, macOS, and Windows, all phase exit criteria are met, and clean-machine installation/quick start passes. Record candidate revision, commands, matrix results, and remaining non-blocking limitations in the release handoff and `avances.md`.
  - M12.06c: Complete final installation/usability evidence matrix. Completion proof: Candidate/hash mapping, affected manual observations and clean-runtime results complete.
  - M12.06d: Reconcile AC-1 through AC-16, phases, budgets and limitations. Completion proof: No unresolved acceptance blocker or unreviewed critical/high security issue.
  - M12.06e: Audit final artifact inventory against the tested candidate. Completion proof: Hashes/version/features/notices/signing state match; no untested rebuild substituted.
- M12.07: Prepare local release notes and the reviewed artifact inventory. Hand off the candidate for an explicit maintainer publishing decision. Delivery authorization does not imply a tag, package upload, or public release.
  - M12.07c: Present candidate and stop for publication decision. Completion proof: Owner receives concrete candidate; no unauthorized external publication or release claim.

## Acceptance coverage for remaining work

This index identifies the planned proof for each specification criterion. As milestones are completed, move their evidence to `avances.md` and retain only outstanding coverage here.

| Criterion | Planned implementation | Required proof |
| --- | --- | --- |
| AC-1 through AC-16 | M12.06 | Reconcile existing M11 evidence with the release candidate and repeat affected behavior only. |
| AC-1, AC-14 | M12.03 | Execute the published quick start from a clean installation without hosted accounts. |
| AC-12, AC-16 | M12.02 | Verify clean installation, PATH, and an existing unrelated command against the already verified release artifacts. |

FR-10 export remains deferred. M12 retains installation, compatibility, documentation, and release acceptance work; completed behavior evidence is in the acceptance matrix and delivery log.
