# M11 acceptance matrix

## Status contract

This matrix tracks the hardening candidate. `Passed` requires candidate-specific evidence. `Pending` means implementation or evidence remains. `Blocked` means an external requirement is unavailable. Earlier milestone results are regression inputs, not M11 completion.

Evidence records include candidate commit, command or procedure, native OS and architecture, tool versions, repetition, duration, and sanitized measurements. Cross-compilation never substitutes for native execution. Manual named-terminal and SSH observations are independent from automated PTY or ConPTY tests.

## Acceptance criteria

| Criterion | Automated evidence | Manual evidence | Current M11 status |
| --- | --- | --- | --- |
| AC-1 | `cli::bootstrap_rejects_invalid_input_without_side_effects`, `durable_slice::durable_handover_survives_process_restart`, and `backup_restore::private_backup_is_exclusive_integrity_checked_and_reopenable` | M12 clean installation only | Automated locally; pending final native candidate |
| AC-2 | `cli::detached_daemon_survives_starters_and_reopens_durable_state`, `runtime_ownership_precedes_recovery_and_rejects_a_contender`, IPC ownership tests, and `durable_slice::closing_launch_console_keeps_daemon_reachable` | Actual terminal closure in the manual matrix | Automated locally; manual matrix pending |
| AC-3 | `pty_gate::three_real_ptys_survive_client_disconnect_and_reconstruct`, `tui_gate::tui_initializes_launches_detaches_and_reopens_without_stopping_children`, and `worktree_gate::real_tui_creates_and_launches_two_task_worktrees` | Observe three sessions in each named terminal | Automated locally; manual matrix pending |
| AC-4 | The TUI workflow gate, terminal reconstruction unit tests, and resource-runtime gate cover full-screen state, Unicode, input, resize, switching, exit and bounds | Local and SSH rendering matrix | Automated locally; manual matrix pending |
| AC-5 | `application::two_transactions_race_without_sleeps_and_only_one_claim_commits`, `persistence::independent_clients_race_one_claim_without_partial_events`, and the integrated worktree journey | None | Automated locally; pending final native candidate |
| AC-6 | `tui_gate::tui_initializes_launches_detaches_and_reopens_without_stopping_children`, `protocol_gate::two_clients_complete_a_durable_handover_journey`, and the integrated worktree journey | None | Automated locally; pending final native candidate |
| AC-7 | The TUI and worktree journeys plus `continuity_gate_preserves_context_attribution_and_event_order` assert second-instance context and completion | None | Automated locally; pending final native candidate |
| AC-8 | TUI abrupt detach, PTY client disconnect and launch-console closure tests preserve daemon-owned children and allow independent reattach | Named terminal and SSH disconnect observations | Automated locally; manual matrix pending |
| AC-9 | `startup_reconciles_active_claim_once_before_readiness`, `sqlite_kill_gate::killing_the_server_before_sqlite_commit_rolls_back_the_whole_mutation`, and durable restart journeys | None | Automated locally; pending final native candidate |
| AC-10 | Both worktree-gate journeys, native Git identity tests, `worktree_cancellation_gate`, and `worktree_recovery` cover isolation, immutable launch identity and partial outcomes | None | Automated locally; pending final native candidate |
| AC-11 | `architecture` compile tests and the core-only workspace command | None | Automated locally; pending final native candidate |
| AC-12 | Quality on Linux, macOS and Windows, pinned Rust 1.98.1 on Linux, plus Security | None | Pending final candidate checks |
| AC-13 | Domain privacy tests, daemon diagnostic tests, TUI safe-text tests, durable-slice sink assertions and candidate/history scans | Sanitized display inspection | Automated locally; manual matrix pending |
| AC-14 | `offline_gate::daemon_uses_local_ipc_without_network_endpoints` with a positive loopback control, real Git, SQLite, IPC, PTY, backup and restore | Headless SSH workflow | Automated locally; SSH matrix pending |
| AC-15 | `agent_templates_gate::unknown_cli_is_configured_and_continued_through_the_real_tui` | None | Automated locally; pending final native candidate |
| AC-16 | `cli::command_contracts_have_no_runtime_side_effects`, one binary target, and help/version repository checks | Installation conflict belongs to M12 | Automated locally; pending final native candidate |

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
