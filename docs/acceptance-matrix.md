# M11 acceptance matrix

## Status contract

This matrix tracks the hardening candidate. `Passed` requires candidate-specific evidence. `Pending` means implementation or evidence remains. `Blocked` means an external requirement is unavailable. Earlier milestone results are regression inputs, not M11 completion.

Evidence records include candidate commit, command or procedure, native OS and architecture, tool versions, repetition, duration, and sanitized measurements. Cross-compilation never substitutes for native execution. Manual named-terminal and SSH observations are independent from automated PTY or ConPTY tests.

## Acceptance criteria

| Criterion | Automated evidence | Manual evidence | Current M11 status |
| --- | --- | --- | --- |
| AC-1 | `durable_slice`, `hardening_gate`, backup and restore tests | M12 clean installation only | Pending final candidate |
| AC-2 | Daemon lifecycle, IPC permission and ownership gates | Actual terminal closure in the manual matrix | Pending final candidate |
| AC-3 | Three native real sessions in retained PTY and TUI gates | Observe three sessions in each named terminal | Pending integrated M11 gate and manual matrix |
| AC-4 | Full-screen, Unicode, input, resize, switch, exit and bound assertions | Local and SSH rendering matrix | Pending manual matrix |
| AC-5 | Deterministic competing-claim tests | None | Pending integrated M11 journey |
| AC-6 | Progress and atomic handover tests through retained TUI and IPC gates | None | Pending integrated M11 journey |
| AC-7 | Second-instance continuation and durable context assertions in retained gates | None | Pending integrated M11 journey |
| AC-8 | Abrupt client and console-owner loss with independent reattach | Named terminal and SSH disconnect observations | Pending manual matrix |
| AC-9 | Graceful and abrupt restart, one-time loss reconciliation | None | Pending final candidate |
| AC-10 | Two real Git worktrees, immutable launch snapshots and partial-result recovery | None | Pending final candidate |
| AC-11 | Architecture gate and core-only test command | None | Pending final candidate |
| AC-12 | Quality and Security on Linux, macOS and Windows, plus pinned Rust on Linux | None | Pending final candidate |
| AC-13 | Synthetic marker sink tests and repository/candidate scans | Sanitized display inspection | Pending final candidate |
| AC-14 | Native monitored offline product workflow | Headless SSH workflow | Pending native evidence |
| AC-15 | Unknown executable configured and launched through the real TUI | None | Pending final candidate |
| AC-16 | One `rt` entrypoint and side-effect-free help/version | Installation conflict belongs to M12 | Pending regression |

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

| Platform | Automated | Required manual local | Required SSH |
| --- | --- | --- | --- |
| Linux | Ubuntu 24.04 stable and Rust 1.98.1 | Bash and configured generic shell in an identified xterm-compatible terminal | OpenSSH to Linux |
| macOS | macOS 14 stable | Zsh and Bash in Terminal.app | OpenSSH to macOS |
| Windows | Windows Server 2022 stable | PowerShell and cmd.exe in Windows Terminal | Supported Windows OpenSSH session |

Every manual cell covers full-screen entry/exit, Unicode wide and combining characters, resize, minimum size, focus escape, keyboard help, color-independent state, form conflicts, detach/reattach, terminal restoration, actual window or connection closure, and survival of the same children while the daemon remains alive. Missing cells keep M11.07 and M11.10 open.
