# M11 acceptance matrix

## Status contract

This matrix tracks the hardening candidate. `Passed` requires candidate-specific evidence. `Pending` means implementation or evidence remains. `Blocked` means an external requirement is unavailable. Earlier milestone results are regression inputs, not M11 completion.

Evidence records include candidate commit, command or procedure, native OS and architecture, tool versions, repetition, duration, and sanitized measurements. Cross-compilation never substitutes for native execution. Manual named-terminal and SSH observations are independent from automated PTY or ConPTY tests. The implementation owner is M11 unless a row explicitly assigns installation-only work to M12. Automated candidate evidence is tracked against `235196d`; a later evidence-only documentation commit is a descendant and does not change the tested behavior.

## Acceptance criteria

| Criterion | Owner | Automated evidence | Manual evidence or limitation | Candidate | Status |
| --- | --- | --- | --- | --- | --- |
| AC-1 | M11 | `cli::bootstrap_rejects_invalid_input_without_side_effects`, `durable_slice::durable_handover_survives_process_restart`, and `backup_restore::private_backup_is_exclusive_integrity_checked_and_reopenable` | Clean-machine installation belongs to M12 | `235196d` | Pending native reconciliation |
| AC-2 | M11 | Detached daemon, runtime ownership, IPC ownership and launch-console closure tests | Actual named-terminal closure is required | `235196d` | Pending manual matrix |
| AC-3 | M11 | Real PTY, TUI and integrated worktree tests each retain three running sessions | Observe the same behavior in every named terminal | `235196d` | Pending manual matrix |
| AC-4 | M11 | TUI workflow, terminal reconstruction and resource-runtime tests cover rendering, Unicode, input, resize, switching, exit and bounds | Local and SSH rendering observations are required | `235196d` | Pending manual matrix |
| AC-5 | M11 | Application and SQLite barrier races plus the integrated worktree journey prove one claim winner and no partial effects | None | `235196d` | Pending native reconciliation |
| AC-6 | M11 | TUI forms, protocol continuity and integrated worktree tests prove progress plus atomic handover | None | `235196d` | Pending native reconciliation |
| AC-7 | M11 | TUI, worktree and continuity gates prove attributed context transfer and completion by a second instance | None | `235196d` | Pending native reconciliation |
| AC-8 | M11 | Abrupt TUI detach, PTY disconnect and launch-console closure tests preserve daemon-owned children and reattach | Named-terminal and SSH disconnect observations are required | `235196d` | Pending manual matrix |
| AC-9 | M11 | Startup reconciliation, abrupt SQLite rollback and durable restart tests prove honest one-time loss handling | None | `235196d` | Pending native reconciliation |
| AC-10 | M11 | Worktree journeys, Git identity, cancellation and recovery tests prove isolation and conservative outcomes | None | `235196d` | Pending native reconciliation |
| AC-11 | M11 | Architecture compile tests and the core-only workspace command preserve client separation | None | `235196d` | Pending native reconciliation |
| AC-12 | M11 | Quality on Linux, macOS and Windows, pinned Rust 1.98.1 on Linux, plus Security | Clean-machine packaging belongs to M12 | `235196d` | Pending native reconciliation |
| AC-13 | M11 | Domain privacy, daemon diagnostics, TUI safe-text, sink and source/history scan tests | Sanitized display inspection is required | `235196d` | Pending manual matrix |
| AC-14 | M11 | Offline gate uses a positive network control and real Git, SQLite, IPC, PTY, backup and restore | Headless SSH observation is required | `235196d` | Pending SSH matrix |
| AC-15 | M11 | The agent-template gate configures and continues an unknown CLI through the real TUI | None | `235196d` | Pending native reconciliation |
| AC-16 | M11 | CLI contract tests, the single binary target and repository help/version checks | PATH and installer collision checks belong to M12 | `235196d` | Pending native reconciliation |

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

## Required native and manual matrix

Automated execution uses Ubuntu 24.04 stable, Ubuntu 24.04 with Rust 1.98.1, macOS 14 stable and Windows Server 2022 stable. The rows below are separate operator observations and cannot be inferred from CI.

| ID | Platform and connection | Shell | Candidate | Dated observer evidence | Status |
| --- | --- | --- | --- | --- | --- |
| MAN-LNX-1 | Identified local xterm-compatible terminal | Bash | Pending final candidate | Pending | Blocked on environment and observer |
| MAN-LNX-2 | Same local terminal | Configured generic shell | Pending final candidate | Pending | Blocked on environment and observer |
| MAN-LNX-3 | Fresh OpenSSH connection to Linux | Record server shell | Pending final candidate | Pending | Blocked on authorized SSH target and observer |
| MAN-MAC-1 | Terminal.app | Zsh | Pending final candidate | Pending | Blocked on observer |
| MAN-MAC-2 | Terminal.app | Bash | Pending final candidate | Pending | Blocked on observer |
| MAN-MAC-3 | Fresh OpenSSH connection to macOS | Record server shell and client terminal | Pending final candidate | Pending | Blocked on authorized SSH target and observer |
| MAN-WIN-1 | Windows Terminal with ConPTY | PowerShell | Pending final candidate | Pending | Blocked on environment and observer |
| MAN-WIN-2 | Windows Terminal with ConPTY | cmd.exe | Pending final candidate | Pending | Blocked on environment and observer |
| MAN-WIN-3 | Fresh Windows OpenSSH connection | Record server shell and client terminal | Pending final candidate | Pending | Blocked on authorized SSH target and observer |

Every row covers full-screen entry and exit, Unicode wide and combining characters, resize, minimum size, focus escape, keyboard help, color-independent state, form conflicts, detach and reattach, terminal restoration, actual window or connection closure, and survival of the same children while the daemon remains alive. Record terminal, shell, OS and `rt` versions, candidate SHA, UTC date, expected and actual results, and sanitized observer confirmation. Missing rows keep M11.07 and M11.10 open.
