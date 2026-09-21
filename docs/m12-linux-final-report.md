# M12 Linux final validation report

## Decision

The required Linux observation attempts and authorized focused SSH journey are
complete for this validation round. Acceptance is not granted: interrupted SSH
terminal restoration failed, the documented detach shortcut failed, and other
UI findings require diagnosis. No production correction, global M12 sign-off,
release freeze, push, or cross-platform acceptance is implied.

This report summarizes the [detailed Linux observations](m12-linux-manual-observations.md)
against the [original continuation contract](m12-native-continuation.md),
[usability procedure](m12-usability-verification.md), and
[installed quick start](quick-start.md). Earlier pending statements in the
chronological record are superseded only by their later recorded observations.

## Candidate and environment

Validated on 2026-09-21, Ubuntu 24.04.5 LTS x86_64, GNOME Terminal 3.52.0,
VTE 0.76.0, Spanish keyboard, Bash 5.2.21 and Dash 0.5.12. Native target:
`x86_64-unknown-linux-gnu`. Source:
`9cf91f7164235d35beda81d9c8c2d4200b703aad`. Rust/Cargo 1.98.1,
LLVM 22.1.8, GCC 13.3.0, GNU ld 2.42, glibc 2.39.
Version 0.1.0, empty default production features, unsigned.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Installed executable | 13,030,408 | `b2bbc872bf4b7c8175f07dfd66b64d2cc8a02bf66ae6c409ee33542c0a9fcfd4` |
| Release archive | 4,826,579 | `0f7add246e4406eb7a30a955844a9a04d51026014ca69301232d3d536cd3450b` |

Two isolated clean builds and packages were byte-identical. Archive inventory,
checksums, native ELF dependencies, extracted smoke, and dedicated installation
passed. All observed shared libraries resolve; maximum symbol requirement is
GLIBC_2.39. This is not evidence for older distributions. Artifacts and runtime
evidence remain outside Git.

## Results

| Check | Result and evidence boundary |
| --- | --- |
| Installed executable gates | Passed with the corrected test-only resize synchronization harness: ten tests, one helper ignored. Original synchronization and loaded/debug echo-budget failures remain recorded. |
| Source and tooling checks | Serial workspace tests, formatting, Clippy, repository checks, release tooling, secret scan, and audit negative controls passed as recorded. No new full advisory audit is claimed. |
| Installation safety | Collision refusal preserved a synthetic unrelated command. Scoped removal passed for a separate owned install with no daemon running from it. |
| Session presentation | Operator observed neutral definitions, ordered distinct identities, duplicate names, cleared name, and persistence across client reopen. Extra and terminated sessions remain recorded. |
| Form editing | Operator confirmed ASCII/wide/combining sample editing, movement, insertion, Backspace, Delete, field navigation, multiline text, Ctrl-U, resize preservation, validation rejection, and cancellation. Screenshots do not independently establish Unicode code points. |
| Minimum layout | All six screens visited at the measured minimum; complete header and Tasks/Sessions footer observed. Below-minimum notice and enlargement recovery passed. Full Help content accessibility was not established. |
| Stale rename | Local and SSH rejection, retained draft, two-step review, and discard passed. Stale context label remains a finding. |
| Input exclusion and size | Local and SSH first acquisition, rejected competing acquisition, read-only resize isolation, both transfer directions, and fresh long-line wrapping passed. Independent live snapshots confirmed writer dimensions. |
| Detach and reopen | Implemented Ctrl-] release followed by Esc detach passed; new Session 6 remained running. Ctrl+AltGr+] was confirmed on this operator's keyboard. Documented Ctrl-Space failed. |
| Task handover | Neutral-session claim, competing rejection, progress, immediate CURRENT/handover_ready/unowned state, saved history, distinct successor, completion, and quit/reopen passed with independent readback. |
| Live backup and restore | Authorized original daemon stop and fresh-home restore passed. Stable identities and durable records retained; prior running sessions became lost. Original home and backup preserved. Physical history paging passed. |
| SSH loss and reconnection | Scoped SSH client SIGTERM, child survival, lease reacquisition, fresh connection, and stable identities passed. This was a client interruption, not a network-blackhole test. |
| Terminal recovery | Normal local and SSH exit and disconnected local exit passed. Interrupted SSH left stale TUI content over the outer shell; manual reset was needed. Automatic visual recovery failed. |

## Findings requiring follow-up

1. Interrupted SSH visual recovery failed. Determine responsibility across the
   SSH client, terminal, and Relayterm before choosing a correction; preserve
   the original failed result and retest the same interruption scenario.
2. Quick start prescribes Ctrl-Space detach, but the candidate handler implements
   Ctrl-] release and Esc detach. Align the intended behavior, documentation,
   help, and regression coverage, then verify physically.
3. After daemon stop, DISCONNECTED coexists with stale INPUT/WRITER and
   Attaching indicators. Repeated transport-loss diagnostics fill Events.
4. The release chord switched a disconnected client to Events without an
   operator 5 keypress. Capture native key events and reproduce before assigning
   a cause. Successful release while connected does not close this finding.
5. Stale rename reconciliation retains an old context label despite authoritative
   value review. Investigate the context refresh separately from data integrity,
   which passed.
6. Older content appeared clipped after wide-to-narrow transfer. Fresh command
   wrapping and live dimensions passed; historical display behavior still needs
   comparison with the intended terminal model.

## Limitations and retained state

The optional manual Git worktree journey was not performed; the installed
worktree gate passed. The restored fixture contained no worktrees, so populated
worktree metadata restoration was not physically established. No raw screenshots,
transcripts, private identities, runtime databases, or credentials are committed.

The original daemon was stopped with explicit authorization. The restored
workspace and its new running Session 6 are retained at the final operator
checkpoint. The temporary authorized loopback SSH service and private fixtures
are not claimed removed by this report. Cleanup must target only these owned
resources and preserve the backup and candidate evidence.

Windows and macOS results retain their original source mappings. No unavailable
platform check, global acceptance, or failed recovery check is marked passing.

## Proposed next work

Review and prioritize the findings above, then implement focused corrections on
a work branch. Retest affected paths on a newly identified candidate, preserving
this baseline report and its failures. Reconcile the remaining macOS and global
matrix before freezing M12. Publishing evidence or creating a pull request
requires a separate operator request.
