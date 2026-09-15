# M11 acceptance matrix

## Status contract

This matrix tracks the hardening candidate. `Passed` requires candidate-specific evidence. `Pending` means implementation or evidence remains. `Blocked` means an external requirement is unavailable. Earlier milestone results are regression inputs, not M11 completion.

Evidence records include candidate commit, command or procedure, native OS and architecture, tool versions, repetition, duration, and sanitized measurements. Cross-compilation never substitutes for native execution. Manual named-terminal and SSH observations are independent from automated PTY or ConPTY tests. The implementation owner is M11 unless a row explicitly assigns installation-only work to M12. Automated candidate evidence is tracked against `d6efd7f`; a later evidence-only documentation commit is a descendant and does not change the tested behavior.

## Acceptance criteria

| Criterion | Owner | Automated evidence | Manual evidence or limitation | Candidate | Status |
| --- | --- | --- | --- | --- | --- |
| AC-1 | M11 | `cli::bootstrap_rejects_invalid_input_without_side_effects`, `durable_slice::durable_handover_survives_process_restart`, and `backup_restore::private_backup_is_exclusive_integrity_checked_and_reopenable` | Clean-machine installation belongs to M12 | `d6efd7f` | Passed for M11 |
| AC-2 | M11 | Detached daemon, runtime ownership, IPC ownership and launch-console closure tests | Actual named-terminal closure passed on [Linux](m11-linux-manual-observations.md), [macOS](m11-macos-manual-observations.md), and [Windows](m11-windows-manual-observations.md) | `d6efd7f` plus manual candidates | Passed |
| AC-3 | M11 | Real PTY, TUI and integrated worktree tests each retain three running sessions | Three-session retention passed in every required named terminal and SSH row | `d6efd7f` plus manual candidates | Passed |
| AC-4 | M11 | TUI workflow, terminal reconstruction and resource-runtime tests cover rendering, Unicode, input, resize, switching, exit and bounds | Local and SSH rendering passed across all nine required manual rows | `d6efd7f` plus manual candidates | Passed |
| AC-5 | M11 | Application and SQLite barrier races plus the integrated worktree journey prove one claim winner and no partial effects | None | `d6efd7f` | Passed |
| AC-6 | M11 | TUI forms, protocol continuity and integrated worktree tests prove progress plus atomic handover | None | `d6efd7f` | Passed |
| AC-7 | M11 | TUI, worktree and continuity gates prove attributed context transfer and completion by a second instance | None | `d6efd7f` | Passed |
| AC-8 | M11 | Abrupt TUI detach, PTY disconnect and launch-console closure tests preserve daemon-owned children and reattach | Actual local-window and SSH-connection closure preserved daemon-owned children across the manual matrix | `d6efd7f` plus manual candidates | Passed |
| AC-9 | M11 | Startup reconciliation, abrupt SQLite rollback and durable restart tests prove honest one-time loss handling | None | `d6efd7f` | Passed |
| AC-10 | M11 | Worktree journeys, Git identity, cancellation and recovery tests prove isolation and conservative outcomes | None | `d6efd7f` | Passed |
| AC-11 | M11 | Architecture compile tests and the core-only workspace command preserve client separation | None | `d6efd7f` | Passed |
| AC-12 | M11 | Quality on Linux, macOS and Windows, pinned Rust 1.98.1 on Linux, plus Security | Clean-machine packaging belongs to M12 | `d6efd7f` | Passed for M11 |
| AC-13 | M11 | Domain privacy, daemon diagnostics, TUI safe-text, sink and source/history scan tests | Sanitized display inspection passed; personal prompts and screenshot paths were excluded from public records | `d6efd7f` plus manual candidates | Passed |
| AC-14 | M11 | Offline gate uses a positive network control and real Git, SQLite, IPC, PTY, backup and restore | Fresh operator-controlled OpenSSH observations passed on Linux, macOS, and Windows | `d6efd7f` plus manual candidates | Passed |
| AC-15 | M11 | The agent-template gate configures and continues an unknown CLI through the real TUI | None | `d6efd7f` | Passed |
| AC-16 | M11 | CLI contract tests, the single binary target and repository help/version checks | PATH and installer collision checks belong to M12 | `d6efd7f` | Passed for M11 |

FR-1 through FR-9 map to AC-1 through AC-10 and AC-15. FR-10 remains deferred because M11 private database recovery is not a project export. NFR-1 maps to the native matrix, NFR-2 to fault isolation and atomicity, NFR-3 to the fixed performance workloads, NFR-4 to protocol/database compatibility, NFR-5 to diagnostics, NFR-6 to TUI and manual observations, NFR-7 to architecture tests, and NFR-8 to the complete resource inventory.

## Fixed performance workloads

These criteria were declared in `docs/M11_details.md` before M11 measurements and must not be adjusted after observing a candidate.

| Workload | Samples | Pass condition |
| --- | ---: | --- |
| Existing daemon startup with 100 tasks, 100 history entries and 3 sessions | 20 after one warm-up | Native p95 at most 2 seconds |
| Navigation while another session emits at least 2 MiB/s | 100 | Reference p95 at most 100 ms; shared CI p95 at most 500 ms |
| Input to parsed visible echo under the same load | 100 | Reference p95 at most 250 ms; shared CI p95 at most 1 second |
| Eight-session retention at 120 by 40, wrapping each buffer three times | One complete journey per repetition | Ninth session rejected before spawn; no bound exceeded |
| Reconnect churn | 100 cycles, 20 abrupt | Counters return to baseline within 10 seconds; handle difference at most 16 |
| Memory plateau | 180 seconds, 60-second warm-up | Daemon and TUI each at most 512 MiB; last median at most first median plus 32 MiB |

Report median, nearest-rank p95, maximum, measured output rate, RSS or working-set semantics, build profile and child memory separately. Compilation time is not runtime startup. A reference-target miss remains visible even if the shared-runner guardrail passes.

## Resource inventory

The implementation gate must verify each constant against current source. The initial inventory is:

| Resource | Declared bound | Scope and required overflow behavior |
| --- | ---: | --- |
| Live sessions | 8 | Workspace; ninth request rejected before spawn |
| Raw terminal continuation | 1 MiB | Session; oldest raw continuation discarded with explicit resnapshot semantics |
| Visible viewport | 16,000 cells | Attachment/session view; invalid dimensions rejected |
| Pending session input | 64 KiB | Session; overflow rejected without partial input |
| UI input and invalidations | 64 each | Client; bounded coalescing without mutation retry |
| Client event retention | 256 events or 1 MiB | Connection; gap requires authoritative refresh |
| Diagnostics | 1,000 entries or 1 MiB | Client; sensitive text never retained |
| Complete form drafts | 256 KiB | Client; explicit discard required |
| JSON frame | 8 MiB | Connection; reject before oversized allocation |
| Terminal data frame | 16 KiB | Frame; split output without loss of parser continuity |
| Git stdout/stderr | 4 MiB / 64 KiB | Process; stop reading and terminate boundedly on overflow |
| Git worktree inventory | 4,096 entries | Operation; reject excess explicitly |
| Collection page | 200 items, 6 MiB | Query/response; no full-history materialization |

Durable history is append-only. Resource hardening must bound SQL reads, pages, caches, queues, tasks, handles and temporary storage without silently deleting durable records.

## Automated native candidate evidence

Candidate `d6efd7f1370c6e98aa558b1014494aa1ed61c6d3` passed [Quality run 34825227119](https://github.com/fpinero/relayterm/actions/runs/34825227119) and [Security run 34825227292](https://github.com/fpinero/relayterm/actions/runs/34825227292) on 2026-09-14. Quality used Rust 1.98.1 on every native stable runner and on the pinned Linux job. Every stable job ran two independent repetitions of the M11 journey, protocol faults, abrupt SQLite rollback, Git cancellation, Git descendant containment, durable scale, offline runtime and sustained resource gates. Backup and restore, the complete workspace suite, core-only suite, build and repository controls also passed.

| Native runner | Git | Integrated journey, ms | History p95, ms | Daemon maximum, bytes | TUI maximum, bytes | Child maximum, bytes | Handle baseline to final | Sustained output, bytes/s |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: |
| Ubuntu 24.04 x86_64 | 2.55.0 | 2,304 / 2,363 | 4 / 3 | 348,135,424 / 347,222,016 | 21,798,912 / 21,794,816 | 32,624,640 / 32,636,928 | 45 to 46 / 45 to 45 | 4,253,264 / 4,235,753 |
| macOS 14 arm64 | 2.55.0 | 9,380 / 7,975 | 2 / 4 | 436,649,984 / 441,565,184 | 18,006,016 / 17,498,112 | 20,922,368 / 21,102,592 | 51 to 51 / 51 to 53 | 4,827,924 / 4,865,431 |
| Windows Server 2022 x86_64 | 2.55.0.windows.5 | 7,229 / 7,153 | 2 / 2 | 376,545,280 / 372,883,456 | 22,102,016 / 22,265,856 | 39,583,744 / 39,575,552 | 245 to 250 / 245 to 245 | 3,434,472 / 3,457,845 |

All resource repetitions stayed below 512 MiB, their final medians stayed within 32 MiB of their first medians, handle differences stayed within 16, and output remained above 2 MiB/s. The durable scale repetitions each retained 10,000 tasks, 100,001 progress records after mutation and ordered durable events while returning bounded 200-item pages.

| Native runner | Startup p95, ms | Navigation p95, ms | Input p95, ms | Flood output, bytes/s |
| --- | ---: | ---: | ---: | ---: |
| Ubuntu 24.04 | 142 / 162 | 21 / 21 | 164 / 164 | 4,064,527 / 4,017,359 |
| macOS 14 | 359 / 380 | 167 / 168 | 308 / 273 | 3,948,180 / 3,838,062 |
| Windows Server 2022 | 127 / 127 | 21 / 21 | 185 / 186 | 3,268,530 / 3,299,099 |

Linux and Windows met the reference navigation and input targets. The shared macOS runner missed those reference targets but passed the predeclared 500 ms navigation and one-second input guardrails. Native local macOS reference runs on an Apple M1 Pro with 16 GiB, macOS 26.5.2, Rust 1.98.1 and Git 2.53.0 reported 126/113 ms startup p95, 34/36 ms navigation p95, 148/149 ms input p95 and 5,599,759/5,708,686 bytes/s. These values are observations for the declared workloads, not universal performance guarantees.

## Required native and manual matrix

Automated execution uses Ubuntu 24.04 stable, Ubuntu 24.04 with Rust 1.98.1, macOS 14 stable and Windows Server 2022 stable. The rows below are separate operator observations and cannot be inferred from CI.

| ID | Platform and connection | Shell | Candidate | Dated observer evidence | Status |
| --- | --- | --- | --- | --- | --- |
| MAN-LNX-1 | GNOME Terminal 3.52.0 with VTE 0.76.0 | Bash 5.2.21 | `05af627`, SHA-256 `8417a0998bef` | [2026-09-13 observations](m11-linux-manual-observations.md) | Passed |
| MAN-LNX-2 | GNOME Terminal 3.52.0 with VTE 0.76.0 | Dash 0.5.12 | `05af627`, SHA-256 `8417a0998bef` | [2026-09-13 observations](m11-linux-manual-observations.md) | Passed |
| MAN-LNX-3 | Fresh loopback OpenSSH connection to Linux from GNOME Terminal | Bash 5.2.21 | `05af627`, SHA-256 `8417a0998bef` | [2026-09-13 observations](m11-linux-manual-observations.md) | Passed |
| MAN-MAC-1 | Terminal.app | Zsh | `69ff8a8`, matching observed v4 SHA-256 `ebf27bbd945a` | [2026-09-10 to 2026-09-11 composite observations](m11-macos-manual-observations.md) | Passed; repeat affected behavior if source changes |
| MAN-MAC-2 | Terminal.app | Bash | `69ff8a8`, matching observed v4 SHA-256 `ebf27bbd945a` | [2026-09-10 to 2026-09-11 composite observations](m11-macos-manual-observations.md) | Passed; repeat affected behavior if source changes |
| MAN-MAC-3 | Fresh loopback OpenSSH connection to macOS from Terminal.app | Bash 3.2.57 | `69ff8a8`, matching observed v4 SHA-256 `ebf27bbd945a` | [2026-09-10 to 2026-09-11 composite observations](m11-macos-manual-observations.md) | Passed; repeat affected behavior if source changes |
| MAN-WIN-1 | Windows Terminal 1.24.11911.0 with ConPTY | PowerShell 7.6.5 | `e031af8`, SHA-256 `94e37a31fad1`; later corrections did not affect this row | [2026-09-12 to 2026-09-13 composite observations](m11-windows-manual-observations.md) | Passed; repeat affected behavior if source changes |
| MAN-WIN-2 | Windows Terminal 1.24.11911.0 with ConPTY | cmd.exe 10.0.19045.7663 | `f3d0ca6`, SHA-256 `01effc97672f`; initial cwd failure superseded | [2026-09-12 to 2026-09-13 composite observations](m11-windows-manual-observations.md) | Passed; repeat affected behavior if source changes |
| MAN-WIN-3 | Fresh loopback Windows OpenSSH connection from Windows Terminal | cmd.exe 10.0.19045.7663 | `136d46c`, SHA-256 `41c49871965d`; pre-fix daemon-loss evidence superseded | [2026-09-12 to 2026-09-13 composite observations](m11-windows-manual-observations.md) | Passed; repeat affected behavior if source changes |

Every row covers full-screen entry and exit, Unicode wide and combining characters, resize, minimum size, focus escape, keyboard help, color-independent state, form conflicts, detach and reattach, terminal restoration, actual window or connection closure, and survival of the same children while the daemon remains alive. The reports record terminal, shell, OS and `rt` versions, candidate SHA, UTC date, expected and actual results, and sanitized observer confirmation. All required manual rows are complete. The final Git-only source correction does not alter the covered behavior, so no manual row requires repetition.

The complete AC, R1-R11, budget, manual, security, and M12-boundary disposition is recorded in the [M11 final technical review](m11-final-review.md). The supplemental local Windows resource miss remains visible there and in the Windows observation report. It did not change the predeclared threshold and is not relabeled as a pass.

## M12 candidate evidence

Source `491d5f1037450c362525b289fa6361d834fc0b6f` passed [Quality run 34947043434](https://github.com/fpinero/relayterm/actions/runs/34947043434) and [Security run 34947043380](https://github.com/fpinero/relayterm/actions/runs/34947043380). The native release jobs built and packaged the source twice on all three targets, compared byte-identical copies, inspected the exact nine-entry archive and native dependencies, and ran the extracted smoke, complete TUI, session presentation, worktree, and backup/restore gates. Stable Linux, macOS, and Windows jobs each retained two separately reported inherited gate repetitions. The [M12 candidate handoff](m12-candidate-handoff.md) records exact artifact hashes and job links.

This automated evidence covers the implementation and release mechanics affected by M12. It does not replace the operator-controlled observations required for editable names, stable ordering, the physical form cursor, stale-edit guidance, competing-input guidance, clean installation, the complete interactive quick start, upgrade, removal, and one focused SSH connection. Those rows remain pending under M12.00h, M12.02a, M12.02c, M12.02d, M12.03b, and M12.06c. AC-1, AC-12, AC-14, and AC-16 therefore retain their passed M11 behavior evidence but cannot receive final installable-candidate sign-off until the M12 manual matrix is complete.
