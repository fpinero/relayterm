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

Execution contract: [M12 detailed plan](docs/M12_details.md). Start at M12.00a. The five usability proposals are scheduled here and do not reopen M11 acceptance.

- M12.00: Complete UX-SESSION-NAMES, UX-SESSION-ORDER, UX-FORM-CURSOR, UX-CONFLICT-MESSAGE, and UX-INPUT-ACQUIRE-MESSAGE before freezing release artifacts. Preserve durable identity, bounded state, typed error semantics, and read-only input ownership.
  - M12.00b: Implement private metadata, creation ordinal, migration and atomic rename through domain/application/persistence. Completion proof: Validation, competing revision, rollback, old-schema backfill, restart and backup round-trip pass.
  - M12.00c: Expose typed rename and bounded ordered reads through daemon/protocol/client/CLI. Completion proof: Capability fallback, revision/cursor contract, more than 200 rows and uncertainty tests pass without full-history loads.
  - M12.00d: Add session labels, rename form, details and stable list/page selection. Completion proof: Duplicate/clear names, background creation, rename and page transitions preserve identity and lease.
  - M12.00e: Implement visible cursor and active-field viewport for all forms. Completion proof: UTF-8/cell-width/resize/clipping tests and actual native visibility pass.
  - M12.00f: Add precise stale-form feedback and explicit reconciliation. Completion proof: Two-client conflict preserves winner/draft; unknown outcome remains distinct and never replays.
  - M12.00g: Add inline competing-input feedback with safe fallback. Completion proof: Owner retains lease, rejected reader stays read-only, subsequent explicit acquisition works.
  - M12.00h: Verify consolidated usability changes and update help/privacy/compatibility. Completion proof: Targeted native/manual matrix and affected SSH coverage recorded; unaffected M11 evidence mapped.

- M12.01: Define and document supported release targets and reproducible release-build commands producing `rt` or `rt.exe`. Build candidate artifacts with license notices and checksums; verify fresh-machine execution without an undeclared runtime toolchain dependency.
  - M12.01a: Freeze release targets, runtime baselines, version and production feature set. Completion proof: Evidence-backed target table distinguishes tested and minimum OS; no unsupported architecture claims.
  - M12.01b: Implement repeatable native release build recipe. Completion proof: Clean locked builds twice per target; hashes compared and differences explained.
  - M12.01c: Assemble bounded portable archives, manifests and notices. Completion proof: Exact safe inventory, executable bits, hashes and transitive/native license obligations verified.
  - M12.01d: Inspect runtime dependencies and execute extracted smoke tests. Completion proof: No hidden build-tree/toolchain dependency; native daemon/PTY/shutdown proof per target.
  - M12.01e: Integrate release verification without publication privileges. Completion proof: Native jobs and required checks propagate failures; no test-hooks or test executables shipped.
- M12.02: Write installation/PATH guidance that checks for an existing `rt` command and explains resolution without overwriting an unrelated executable. Test clean installation and a synthetic naming conflict on Linux, macOS, and Windows.
  - M12.02a: Write explicit user-local installation and discovery procedures. Completion proof: Commands validated per shell and actual installed resolution identified.
  - M12.02b: Implement/verify collision handling and safe extraction behavior. Completion proof: Existing files/aliases/functions and invalid artifacts never overwritten or reported as successful install.
  - M12.02c: Verify clean installations on three targets. Completion proof: Runtime inventory, checksum, target, help/version and TUI startup recorded.
  - M12.02d: Write and test removal and platform-warning guidance. Completion proof: Only owned install files removed; state and unrelated PATH entries preserved; signing claims accurate.
- M12.03: Write the quick start covering workspace initialization, three sessions, claim conflict, progress/handover/resume, detach/reattach, restart limits, and optional explicit worktrees. Execute the published instructions from a clean environment using synthetic agents and no hosted account.
  - M12.03a: Write synthetic quick start from implemented commands. Completion proof: Steps cover section 7 with shell-correct syntax and no provider prerequisite.
  - M12.03b: Execute installed initialization and three-session UI journey. Completion proof: All targets; new names/order/cursor and stable session identity observed.
  - M12.03c: Execute claim/progress/handover/resume and explicit worktree journey. Completion proof: Durable readback and isolated cwd prove outcomes without production projects.
  - M12.03d: Execute closure/restart/backup recovery portions and reconcile guide. Completion proof: Same children while daemon lives, honest loss after restart, validated fresh-home restore.
- M12.04: Document upgrade and compatibility behavior, backup/restore, shutdown, missing executables, unsupported terminals, IPC/database errors, and recovery of lost sessions. Verify command examples and link each recovery procedure to tested behavior.
  - M12.04a: Define binary/protocol/schema compatibility and upgrade sequence. Completion proof: Old/new pair behavior and backup requirement documented from tests.
  - M12.04b: Test populated pre-M12 upgrade and metadata preservation. Completion proof: Original/backup retained, migration atomic, no duplicate ordinals, names preserved.
  - M12.04c: Verify rollback and failed-upgrade preservation. Completion proof: Older binary cannot damage newer state; fresh-home rollback uses compatible backup.
  - M12.04d: Publish and test troubleshooting actions. Completion proof: Every listed failure maps to truthful action with no destructive default.
- M12.05: Reconcile README, vision, specification, and architecture documentation with the implemented candidate, retaining MVP exclusions and recording deliberate specification changes. Obtain maintainer choices for governance, code of conduct, and release policy before a public release; confirm the private reporting route still works.
  - M12.05a: Reconcile README/specification/ADRs/platform/help documentation. Completion proof: Current behavior, privacy and exclusions consistent; no premature release claim.
  - M12.05b: Reconcile license and security reporting material. Completion proof: Archive obligations accounted for and reporting route inspected without unsolicited messages.
  - M12.05c: Obtain only missing publication policy choices. Completion proof: Existing sole-maintainer decision respected; optional preferences distinguished from technical gates.
  - M12.05d: Review public artifact and evidence privacy. Completion proof: Source/history scans and synthetic negative controls pass; names/drafts/transcripts absent from logs.
- M12.06: Verify the final gate: all 16 ACs have passing evidence for applicable automated and manual checks on Linux, macOS, and Windows, all phase exit criteria are met, and clean-machine installation/quick start passes. Record candidate revision, commands, matrix results, and remaining non-blocking limitations in the release handoff and `avances.md`.
  - M12.06a: Freeze final source and run local required verification. Completion proof: Formatting, checks, lint, full/core tests, scans and focused M12 tests pass.
  - M12.06b: Obtain final native CI and release-build results. Completion proof: Every required target and independent inherited repetition passes without hidden retries.
  - M12.06c: Complete final installation/usability evidence matrix. Completion proof: Candidate/hash mapping, affected manual observations and clean-runtime results complete.
  - M12.06d: Reconcile AC-1 through AC-16, phases, budgets and limitations. Completion proof: No unresolved acceptance blocker or unreviewed critical/high security issue.
  - M12.06e: Audit final artifact inventory against the tested candidate. Completion proof: Hashes/version/features/notices/signing state match; no untested rebuild substituted.
- M12.07: Prepare local release notes and the reviewed artifact inventory. Hand off the candidate for an explicit maintainer publishing decision. Do not push, open a PR, merge, tag, or publish a release as an implied step of this roadmap.
  - M12.07a: Prepare local release notes and handoff. Completion proof: Complete evidence links, inventory, upgrade instructions and known limitations.
  - M12.07b: Reconcile pending queue and append verified milestone results. Completion proof: Only actual completed tasks removed; blocked tasks retain precise next step.
  - M12.07c: Present candidate and stop for publication decision. Completion proof: Owner receives concrete candidate; no unauthorized external publication or release claim.

## Acceptance coverage for remaining work

This index identifies the planned proof for each specification criterion. As milestones are completed, move their evidence to `avances.md` and retain only outstanding coverage here.

| Criterion | Planned implementation | Required proof |
| --- | --- | --- |
| AC-1 through AC-16 | M12.06 | Reconcile existing M11 evidence with the release candidate and repeat affected behavior only. |
| AC-1, AC-14 | M12.03 | Execute the published quick start from a clean installation without hosted accounts. |
| AC-12, AC-16 | M12.01, M12.02 | Verify release artifacts, clean installation, PATH, and an existing unrelated command. |

FR-10 export remains deferred. M12 retains installation, compatibility, documentation, and release acceptance work; completed behavior evidence is in the acceptance matrix and delivery log.
