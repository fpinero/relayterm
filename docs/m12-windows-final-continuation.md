# M12 Windows continuation after macOS validation

The focused Windows physical round is complete. See the consolidated
[Windows final report](m12-windows-final-report.md) and the
[Linux delta continuation](m12-linux-final-delta-continuation.md) for current
results and the bounded next task. The preparation procedure below is retained
as the original execution contract, not a request for another visual round.

## Purpose and authority

Complete only the missing Windows correction observations and final-candidate
preparation. Do not repeat the complete M11 or original M12 manual journey.
This handoff follows the Linux delivery and the completed focused macOS round.
Global M12 acceptance is still open; M13 does not start until its exit gate passes.

Repository: https://github.com/fpinero/relayterm
Delivery branch: `fix/m12-macos-final-candidate`.
Production source: `691a8fbb45980658b98d647a85ea8305b2325938`.
Completed macOS evidence commit: `0010305e52c11dcf8a4e0b5337f2aa6292841afc`.
Later delivery documentation commits contain no product code changes.
Fetch and verify these objects and ancestry rather than trusting an old checkout.
Record the actual fetched branch head in the Windows evidence.

This document and [the cross-platform continuation](m12-cross-platform-continuation.md)
supersede older instructions identifying 9cf91f7 as the latest candidate or
requiring the now-completed macOS minimum-layout/writer retests.

## Required reading

Read AGENTS.md, PROJECT_VISION.md, MVP_TECHNICAL_SPEC.md, README.md, TODO.md,
and the relevant latest avances.md entries, then:

- [M12 detailed contract](M12_details.md), especially sections 5 through 10.
- [Cross-platform continuation](m12-cross-platform-continuation.md).
- [Current candidate report](m12-macos-correction-candidate.md).
- [Completed macOS observations](m12-macos-manual-observations.md).
- [Prior Windows observations](m12-windows-manual-observations.md).
- [Linux final report](m12-linux-final-report.md).
- [Candidate handoff](m12-candidate-handoff.md).
- [Release builds](release-builds.md), [installation](install.md),
  [quick start](quick-start.md), [compatibility](supported-platforms.md),
  [acceptance matrix](acceptance-matrix.md), and the checked-in Quality/Security workflows.

Communicate in Spanish. Write documentation and code comments in English.
Preserve private runtime state and screenshots outside Git. Use synthetic names.

## Source and evidence mapping

| Source | Meaning and reuse boundary |
| --- | --- |
| d760554 | Corrected handover projection; earlier Windows installed task/session/recovery journey. Retain unchanged evidence. |
| 0bc4b63 | Minimum-width header correction; physical Windows header/footer passed. |
| 9cf91f7 | Resize immediately on input acquisition; Windows first acquisition, both transfers, read-only resize isolation and exact-installed gates passed. |
| 15794ad | Linux fixes: modified release keys do not select Events, disconnected writer feedback is truthful, repeated transport diagnostics are bounded, reviewed rename context refreshes, detach guidance matches behavior. Linux targeted physical checks passed. |
| 7f18686 | Consolidated Linux delivery, documentation only after the corrected code. |
| 691a8fb | Latest product change: typed form feedback distinguishes Info from Error. Refresh/review/adoption are informational; validation, stale rejection and uncertain outcomes remain errors. |
| 0010305 | Completed focused macOS physical evidence for 691a8fb. Later documentation does not replace the tested executable identity. |

The 691a8fb change does not alter the daemon protocol, schema, dependency graph,
lease rules, session resizing, conflict guarantees or explicit submission.
The draft and cursor survive adoption; only Ctrl-S submits. No mutation is
replayed automatically. Do not weaken these guarantees to simplify a test.

## Completed macOS evidence, no Windows substitute

Mac used native Terminal.app on macOS 26.5.2 arm64, Rust/Cargo 1.98.1,
Apple Clang 21.0.0 and SDK 26.5, version 0.1.0, empty production features, unsigned.
Two clean offline builds and normalized archives were byte-identical.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| macOS executable | 11,050,320 | 4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060 |
| macOS archive | 4,386,181 | 4d8c1dda4d7fa9ea412928365e8dbc9b550f02d0f3766b3462c1cd7832955007 |

Local full workspace tests reported 198 passing executions, zero failures and
16 ignored helpers/opt-in tests. Formatting, all-target check/Clippy, core-only
checks, build, dependency/license audit, source/history privacy scans and negative
controls passed. Release tooling ran 22 tests with two Windows-only skips.
The exact installed binary passed eleven presentation/TUI/worktree/backup tests,
with one helper ignored. Exact commands and runtime measurements are in the report.

All focused physical Mac checks passed: six screens at 80x24, 79x24 warning and
recovery, writer acquisition and both transfers with independent live size reads,
observer resize isolation, connected/disconnected release navigation, truthful
non-writer feedback, stable diagnostics, restored shell input on exit, stale
rename, two-step review, refreshed label, retained draft/cursor, Info guidance,
actual validation Error and safe discard. No additional Mac visual round is
needed for this documentation delivery. This is not Windows runtime evidence.

## Prepare Windows autonomously

1. Inspect the branch, status and local changes before modifying files. Fetch
   origin and verify the delivered branch contains the source and evidence above.
   Work on a suitable existing work branch or a new fix branch from the fetched
   delivery. Never edit main, discard/stash unrelated work or reset over changes.
2. Use two clean detached clones at exact production source 691a8fb, outside the
   evidence working tree, with separate fresh build outputs. Record the evidence
   working-tree commit separately. Do not rebuild just to embed report hashes.
3. Inventory native Windows version/architecture, terminal, shell, Rust/Cargo,
   MSVC compiler/linker and SDK. Inspect actual tools and script help. Preserve
   version 0.1.0, empty production features and signing status. Fetch the locked
   public dependencies before offline builds when needed.
4. Execute build_release.py build twice with target x86_64-pc-windows-msvc and
   --offline; compare build records. Package twice and compare manifests. Verify
   archive and external manifest hashes before extraction, and inspect the exact
   nine-file inventory. Inspect PE architecture/imports and the Visual C++ x64
   runtime requirement. Retain both packages and records outside Git.
5. Run smoke_release.py on the ZIP. Install its extracted executable with the
   packaged PowerShell helper into a new dedicated directory. Preserve earlier
   binaries, workspaces and backups. Verify hash, help/version, absolute execution
   and actual discovery from PowerShell and cmd.exe. Test collision preservation.
   Do not silently install runtimes or change shell profiles/security settings.
6. Set RELAYTERM_TEST_RT to that exact installed rt.exe. Run session_presentation,
   tui_gate, worktree_gate and backup_restore with --locked, --nocapture and
   --test-threads=1. Use the checked-in command syntax. Check LASTEXITCODE after
   every native PowerShell command and propagate any failure immediately.
7. Run relevant local verification from M12_details section 10. Inspect hosted
   runs before repeating already completed native CI. Ordinary cargo test does
   not execute all feature-gated or opt-in workloads. Preserve the complete
   Quality matrix, separate repetitions, pinned Linux job and Security controls.

Do not run an older installed rt.exe accidentally. A version string alone does
not identify the candidate. Do not substitute an untested executable after the
physical checks. A genuine product correction requires a new source identity,
impact analysis, rebuild and affected retests only.

## Focused Windows physical procedure

Prepare two clients on the same disposable workspace and one synthetic shell.
Guide the operator in small steps, waiting for the relevant observation before
changing the next state. Read back durable values and live dimensions yourself
when a safe CLI operation can establish them. Do not ask for screenshots of
assertions already established administratively.

### Connected release and repeated shortcut

Acquire input, verify INPUT/WRITER, release using the actual native Ctrl-] chord,
and repeat the chord while already read-only. Sessions must remain selected.
Record the physical key combination actually used. Do not assume the Spanish
Mac or Linux AltGr mapping applies to Windows. A rejected competing client must
remain read-only and must not revoke the writer.

### Stale rename and informational feedback

1. A opens the session rename form, replaces the field with Draft A and leaves
   it unsaved. B saves Saved B for the same session identity.
2. A submits its old draft. Observe the workspace-changed Error, retained Draft A
   and cursor. Read back Saved B and its revision independently.
3. First Ctrl-R exposes Saved B in the authoritative review view. Stop at that
   checkpoint before the next keypress.
4. Second Ctrl-R restores the form with Current label: Saved B, Draft A intact,
   visible cursor and Info-prefixed adoption guidance. The message says Ctrl-S
   performs a new explicit submission. Observe wrapping and confirm no Error
   prefix on the informational message.
5. Independently compare the saved name/revision before and after adoption; no
   automatic write must occur. Esc and the discard confirmation must retain the
   winner. Read back the same winner/revision again.
6. In a sufficiently large second window, submit an empty task form. Observe
   Error: Title is required., the open form and visible cursor. Verify no task
   was created, then close the form. This contrasts a real error with Info.

### Disconnect, diagnostics and native exit

1. Establish physical INPUT/WRITER on an attached client before stopping the
   disposable daemon. A screenshot of a session list is not writer evidence.
   Only use lifecycle permissions actually authorized for this Windows task.
2. Stop only the identified disposable daemon with explicit session termination.
   Observe DISCONNECTED, NAVIGATION and READ ONLY/reconciliation guidance,
   without confirmed WRITER or misleading Attaching. Retained output or cached
   running metadata is not evidence that a stopped child is alive.
3. Repeat the release chord more than twice while disconnected. It must stay
   on Sessions. Open Events deliberately and observe at least 20 seconds across
   polling cycles. There must be no continuous transport-diagnostic flood.
4. Distinct disconnect episodes can produce distinct entries separated by events;
   do not misclassify an older retained notice as continuous flooding. Preserve
   both the actual count and temporal observation.
5. Quit the clients with q, closing any open form first. Confirm alternate-screen
   restoration, normal prompt/cursor and successful synthetic echo execution in
   each native shell. No stopped test daemon should be restarted just to query
   cleanup status. Retain artifacts and state unless cleanup is explicitly needed.

## What not to repeat

- No full M11 or original M12 task/handover/backup manual journey solely for this fix.
- No manual Windows 80x24/header round or writer-width transfer round already
  accepted at 0bc4b63/9cf91f7, unless a later change affects those paths.
- No duplicate manual rounds merely because CI has two independent repetitions.
- No claims that macOS evidence, WSL, cross-compilation or an old package proves
  native Windows behavior for the new candidate.

The exact installed automated gates still run on the final Windows artifact.
This is package identity verification, not a request to repeat all manual history.

## Remaining global requirements

| Item | Required disposition |
| --- | --- |
| Linux | Rebuild/package/test final source natively; physically observe only changed Info/validation/cursor presentation. Preserve completed 15794ad correction evidence. |
| SSH | One authorized connection must cover changed review/disconnected feedback, normal exit and reconnect identity. Reuse unchanged size/continuity evidence. Do not change SSH services without authority. |
| Quality/Security | Read actual workflow runs and source SHAs. Final jobs must pass; do not reuse 491d5f1 runs as proof of 691a8fb. Branch pushes alone do not trigger these workflows, which require PR, main push or workflow_dispatch. |
| Clean runtime | Record an identified environment without reliance on build checkout/toolchain and with audited prerequisites; final-artifact installation/quick start on all targets remains distinct from developer-host PATH masking. |
| AC-1 through AC-16 | Map inherited evidence and affected results using the current matrix. Reconcile all phase exits, fixed resource budgets, installed artifacts/notices and compatibility. |
| Known limits | Preserve failed interrupted-SSH automatic restoration, reproduced without Relayterm, and fixed-grid historical clipping. Explicitly reconcile acceptance; do not silently turn either into a passing observation. |
| Publication | Merge, tags, releases, signatures/notarization and package publication are separate actions. M12 is not globally complete while mandatory proof is missing. |

## Evidence and delivery discipline

Append Windows results to its existing observation report, with source SHA,
artifact hashes, OS/toolchain, exact executed commands, actual results and limits.
Update TODO with pending work only and append verified results to avances.
Update the candidate, continuation and acceptance matrices without rewriting old
failure records. Distinguish operator actions from assistant readback. Preserve
old binaries, backups and original runtime state. No raw captures, credentials,
private paths, environment dumps or terminal transcripts belong in Git.

The Mac maintainer authorized pushing this delivery branch and starting its
Quality/Security workflows. That authorization is not an automatic instruction
for the Windows agent to push further changes, merge, tag or publish. Complete
and verify local Windows work, then provide a precise handoff and seek external
delivery authorization if it has not been granted in that destination task.

## Published delivery and hosted verification

The delivery branch was pushed on 2026-09-21. Hosted workflows were explicitly
dispatched at `167563e7452d4bc188b0c0ac0d6434327b292b85`:

- [Quality run 35629938330](https://github.com/fpinero/relayterm/actions/runs/35629938330).
- [Security run 35629941870](https://github.com/fpinero/relayterm/actions/runs/35629941870).

Both were queued when this delivery record was written, not verified passing.
Inspect their live conclusions before using them as acceptance evidence. This
record and its logbook update are documentation-only descendants of that SHA.
The tested product source remains 691a8fb. No PR, merge, tag or release was made.

## Windows continuation result

The 2026-09-21 Windows continuation completed reproducible native preparation,
dedicated installation and the focused physical correction round on the retained
691a8fb executable. See [the Windows evidence](m12-windows-manual-observations.md)
for hashes, exact-installed automated gates, operator observations and independent
readbacks. Both native clients exited successfully and restored usable shells;
the candidate daemon remains stopped and private artifacts are retained.

Local commit e23be95 corrects Windows test-harness footer indexing and native exit
assertions without changing product code or replacing the tested binary. Security
35629941870 passed; Quality 35629938330 failed both Windows jobs on the original
harness. Hosted rechecks require authorized remote delivery. The local debug
input-latency misses remain distinct from the passing installed release gate.
Linux presentation, affected SSH, independent clean environments and final
acceptance remain open. M12 is not globally complete; M13 has not started.
