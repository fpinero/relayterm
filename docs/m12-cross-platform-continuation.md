# M12 validation status and cross-platform continuation

## Current source and scope

Current Linux execution entry point:
[Linux final delta continuation](m12-linux-final-delta-continuation.md).
Current consolidated Windows result:
[Windows final report](m12-windows-final-report.md).
The Linux entry point narrows the generic checklist below and prevents repeating
the already completed native journey or correction round.

Updated 2026-09-21. Latest physically verified Linux correction:
`15794ad6e7f9459c89c512e679f5648bf9a3d231`, on `fix/m12-linux-findings`.
Commit `0cf6bea` records its completed targeted Linux physical retests and
changes documentation only. Later documentation-only descendants may reuse
that behavior evidence. No push is implied; a destination checkout must actually
contain the source and evidence before starting. Do not assume remote availability.

This document updates the source selection and pending scope in
[the original native contract](m12-native-continuation.md), whose preparation,
privacy, operator-observation, backup, and publication rules still apply.
The [Linux final report](m12-linux-final-report.md) and
[chronological observations](m12-linux-manual-observations.md) retain failures,
limits, assistant verification, and physical observations separately.

The macOS continuation corrects informational form feedback at
`691a8fbb45980658b98d647a85ea8305b2325938`, based on the complete Linux handoff
`7f1868620ef537f27927fe22224e60d3a3babb8c`. This is the new candidate source
for native preparation, not a globally accepted artifact freeze. See the
[macOS correction candidate](m12-macos-correction-candidate.md) for current
verification and exact remaining actions. Linux's 15794ad physical evidence
remains valid for unchanged behavior; only the changed informational form
presentation needs an additional Linux physical check. Any further code change
needs impact analysis, a new source identity, and affected retests. Do not
substitute a later executable under an older artifact hash.

For the next Windows task, use the [detailed Windows final continuation](m12-windows-final-continuation.md), which contains artifact preparation, focused physical checkpoints, evidence boundaries and remaining global gates.

Windows preparation on 2026-09-21 now has two identical native 691a8fb builds
and packages, verified dedicated installation, and twelve passing exact-installed
checks using a local test-only correction. See the appended
[Windows evidence](m12-windows-manual-observations.md). The focused physical
correction round passed, including restoration of both native shells. Local
debug latency and clean-runtime acceptance remain open.
Security 35629941870 succeeded; Quality 35629938330
failed only its two Windows jobs on footer/exit assertions in the original TUI
harness. Other native and release jobs succeeded. Preserve those failures in
the original record. The authorized recheck has now passed: Quality 35637866692
completed all seven jobs and Security 35637869615 passed on e4431f7. Both Windows
jobs and all independent repetitions passed. Reuse these results for later
documentation-only descendants; see the Windows final report for exact mapping.

## Linux results completed

| Area | Verified result | Candidate boundary |
| --- | --- | --- |
| Native preparation and installation | Ubuntu 24.04.5 x86_64; toolchain/runtime inventory; two identical builds and packages; nine-entry inventory, checksums, ELF dependencies, extracted smoke, dedicated installation, collision refusal, scoped removal | 9cf91f7; packages do not certify the later code |
| Session presentation | Neutral shell definitions, distinct stable IDs, creation order, duplicate and cleared names, details, detach/reattach, client quit/reopen | 9cf91f7; extra and terminated sessions preserved in evidence |
| Forms and minimum layout | ASCII/wide/combining editing, cursor movement, insert/delete, Ctrl-U, field switching, multiline, validation, cancellation, six tabs at measured minimum, subminimum notice and recovery | 9cf91f7; code-point and full-Help limitations retained |
| Concurrent editing | Stale submission rejected; draft retained; authoritative review and explicit revision adoption; discard preserves winner | 9cf91f7 and focused 15794ad retest |
| Input and resize | Competing input rejected, explicit transfers both directions, actual snapshot sizes, read-only resize isolation, fresh long commands wrap at writer width | Native and loopback SSH on 9cf91f7 |
| Task coordination | Neutral claim, competing rejection, progress, immediate CURRENT/handover_ready/unowned state, retained history, different successor, completion and reopen | 9cf91f7 with independent readback |
| Backup and recovery | Live backup, authorized stop, fresh-home restore, stable durable identities and records, honest lost sessions, physical history navigation | 9cf91f7; original backup/home retained; empty worktree fixture |
| SSH continuity | Unicode editing, stale review, input transfer, interrupted client, surviving child/identity, reconnection, normal exit | 9cf91f7; interrupted visual recovery failed, not relabelled |
| Corrected UI | Release and repeated chord, no Events jump while disconnected, truthful non-writer indicators, one disconnect diagnostic over time, clean exit, refreshed rename label with draft intact | Physical Linux 15794ad, followed by independent discard readback |
| Automated correction verification | 198 reported release-mode workspace test executions passed, zero failed, 16 ignored helpers/opt-in tests; formatting and all-target Clippy passed | 15794ad; not the complete final CI/security gate |

The correction executable hash is
`d580683ad921e304d4204e522de805226df8c3734ba667849f9726d187e2f926`.
It is a separately copied tested executable, not a newly double-built release
archive. Final correction-source packaging remains required.
The operator also confirmed q closed the final correction client cleanly.
No additional physical Linux action is needed for the completed retest round.

## Required visual checks by platform

| Target | Required next physical observations | Evidence that need not be repeated wholesale |
| --- | --- | --- |
| macOS, native Terminal.app | Focused native round complete on 691a8fb; no remaining native visual correction check | Six-screen minimum/recovery, both writer-size transfers, observer isolation, corrected connected/disconnected feedback, diagnostic stability, clean exit and reviewed rename/Info/error/discard passed; see the macOS report. Earlier unchanged journey evidence retains its source mapping. |
| Windows, native Windows Terminal/ConPTY | Focused native correction round complete on installed 691a8fb; installed discovery and hosted rechecks passed. Local debug latency and clean-runtime acceptance remain separate gates. | Connected/repeated release, stale rename review, draft/cursor/context, Info/Error, no automatic write, discard, disconnected writer, stable diagnostics and both shell exits passed. Header and writer-size observations retain their mapped 0bc4b63/9cf91f7 sources. |
| Linux, GNOME Terminal | Informational form feedback and validation-error/cursor contrast on 691a8fb only | Completed 15794ad correction retests and unchanged 9cf91f7 journey |
| One authorized SSH transport | Repeat changed conflict/review and disconnected navigation/feedback on the final candidate; verify normal exit and reconnection identities | Unchanged writer-size and continuity evidence remains mapped to 9cf91f7; do not repeat solely because the client OS changes |

Apply only the rows still outstanding for the destination, using one corrected
installed executable and synthetic private state. For local Linux, only item 5
remains; the conflict setup needed to reach Info is not a reopened acceptance
test. Items 1 through 4 already passed locally on 15794ad. The separate SSH row
retains its affected scope. The following is a reference checklist, not an
instruction to repeat every item on every platform:

1. Acquire input, release with the terminal's actual Ctrl-] chord, and repeat it
   in READ ONLY. Sessions must remain selected. Record actual physical keys;
   Linux Ctrl+AltGr+] is not a universal keyboard prescription.
2. With explicit destination authorization to stop the disposable daemon while
   writing, verify DISCONNECTED, navigation mode, no confirmed WRITER, and no
   misleading Attaching state. Retained output is not evidence of a live child.
3. Repeat the release chord while disconnected. It must not open Events. Open
   Events deliberately and wait across several polling cycles; the transport
   diagnostic must not flood the view. Quit and verify clean local shell input.
4. Reopen an isolated workspace, keep a rename draft in one client, and rename
   the same identity through a second client. Submit the stale draft, observe
   rejection, press Ctrl-R to review, then Ctrl-R to adopt. The current label
   must match the winner while the original draft remains unchanged. Discard
   and independently read back the winner and revision.
5. On 691a8fb, observe the informational success guidance
   and one actual validation error, including the physical cursor and wrapping.

Do not require two complete manual repetitions merely because CI has two native
repetitions. Do not claim macOS evidence from Linux or Windows evidence from WSL.

## Remaining milestone gates

1. Verify candidate 691a8fb on the remaining targets and freeze its mapped artifacts after the affected usability checks pass.
2. Run the exact required local and native CI commands/features from
   M12_details.md section 10 and the checked-in Quality/Security workflows.
   Hosted e4431f7 coverage is complete in the runs above; reuse it for unchanged
   product code and documentation-only descendants instead of dispatching again.
   Ordinary cargo test leaves opt-in gates ignored. Retain both independent
   inherited native repetitions, pinned-compiler coverage, full dependency audit,
   privacy/history scans, and negative controls. Older CI links are historical.
3. Build and package twice per final native target, inspect native dependencies,
   retain artifacts, and test the exact extracted executables with
   RELAYTERM_TEST_RT. Include presentation, TUI, worktree and backup/restore
   gates. Repeat compatibility/upgrade coverage required by the final matrix.
4. Complete final-artifact clean-runtime installation evidence on all targets.
   Masking PATH on a developer host alone is not independent clean-runtime proof.
   Identify prerequisites, warnings, resolution, and smoke/quick-start results.
5. Complete affected visual rows above and reconcile limitations explicitly.
   A minimal SSH-only control experiment reproduced loss of remote screen-cleanup
   bytes when the local SSH client was killed. Local reset recovery is documented;
   it is not automatic restoration. Fixed-grid historical clipping is also
   documented and tested separately from new command wrapping. Decide their
   acceptance against the stated contract; do not silently waive failed rows.
6. Reconcile AC-1 through AC-16, phase exit criteria, unchanged M11 impact mapping,
   fixed performance limits, final artifact hashes/notices and source identities.
   The installed worktree gate passed; the optional manual worktree journey was
   not performed in Linux and its backup fixture had no populated worktree rows.
7. Update pending-only TODO, append-only avances, platform reports and candidate
   handoff. A prepared local candidate, accepted M12, and a published release are
   distinct outcomes. Publication preferences are separate from technical proof.

## Continuation prompt for the destination agent

Continue Relayterm M12 on this native host. Read AGENTS.md, the project vision,
technical specification, README, TODO, docs/M12_details.md,
docs/m12-cross-platform-continuation.md, docs/m12-native-continuation.md, and
this platform's observation report. Communicate in Spanish and write sanitized
English evidence. Confirm the fetched checkout contains production correction
15794ad6e7f9459c89c512e679f5648bf9a3d231 and Linux evidence 0cf6bea or their
reviewed descendants. If later code exists, inspect the diff and identify the
final tested source; do not silently rebuild 9cf91f7 as the latest candidate.
Preserve existing work and use an appropriate work branch.

Prepare native final-artifact builds and installed gates autonomously, preserving
exact source/toolchain/hash identities. Follow the platform-specific affected
visual checklist in the continuation document, with one operator checkpoint at
a time and independent state readback. Keep previous observations mapped to their
actual source and repeat affected behavior only. Obtain explicit local permission
before stopping a daemon with live sessions or changing an SSH service; do not
assume this destination inherits machine-specific operations from another host.
Record passes, failures, unavailable evidence and limitations separately. Keep
screenshots, runtime data and credentials outside Git. Finish with tested evidence,
remaining atomic M12 gates and a focused local commit. Do not push, merge, tag,
or publish without an explicit destination request.
