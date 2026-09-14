# M12 usability verification

## Scope and source impact

M12 changes durable session presentation, schema migration, additive protocol operations, client session paging, every TUI form's rendering geometry, stale-form handling, and terminal input-acquisition feedback. The affected manual surface is therefore session naming and order, one representative task form cursor, two-client stale editing, two-client input exclusion, and the same interactions through one SSH terminal transport.

The M11 evidence for daemon detachment, child-process lifetime, PTY and ConPTY supervision, terminal parsing, bounded scrollback, task and claim rules, progress and handover atomicity, Git worktree effects, backup destination safety, offline runtime, resource ceilings, and terminal restoration remains applicable where M12 did not change those implementations. M12 automated tests repeat migration, backup readback, protocol uncertainty, real PTY input ownership, terminal restoration, and the integrated TUI journey at the changed boundaries. This assessment does not convert an affected row into inherited evidence.

## Automated gate

Run from the candidate source with the pinned lockfile:

```text
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo test -p relayterm-persistence-sqlite --locked
cargo test -p relayterm-daemon --test protocol_gate --locked -- --nocapture
cargo test -p relayterm-cli --test session_presentation --locked -- --nocapture
cargo test -p relayterm-cli --test tui_gate --locked -- --nocapture --test-threads=1
```

The tests use synthetic public-safe labels and drafts. Do not retain a real session name, child transcript, native private path, environment dump, database, socket, or terminal capture as a repository artifact.

## Consolidated native observation

Use one disposable non-production workspace and the exact candidate binary on each native target. Record the OS, architecture, terminal, shell, candidate commit, executable SHA-256, date, and result. PowerShell and cmd.exe may share one Windows row when the observed behavior is identical, but record both shells. Perform these steps:

1. Start `rt` in a native terminal, initialize the disposable workspace, and create three real sessions in order.
2. Confirm `Session 1`, `Session 2`, and `Session 3` appear oldest first with abbreviated identities. Open Details and compare the full session and instance IDs with administrative readback.
3. Rename two sessions to the same synthetic name. Confirm both remain distinct and in their original order. Clear one name with Ctrl-U and Ctrl-S, then confirm its default label returns without changing its IDs or process.
4. Open a task form. Type ASCII, `界`, and `e` followed by U+0301. Move with Left, Right, Home, and End, delete and insert text, switch fields, resize to 80 by 24 and back, and confirm the visible cursor follows the insertion point. Cancel and confirm the draft is discarded.
5. Open the same rename form in two clients. Commit a different synthetic value in client B, submit client A, and confirm the stale-revision guidance preserves A's draft. Press Ctrl-R once and inspect B's authoritative value without the form. Press Ctrl-R again and confirm A's unchanged draft returns with the reviewed revision. Discard it and verify B's value remains.
6. Attach both clients to one session. Let A acquire input and write a synthetic marker. Ask B to acquire input. Confirm B remains READ ONLY with the inline competing-owner guidance, A remains WRITER, and both continue receiving output. Release A, explicitly press `i` in B, and confirm B becomes WRITER. Detach both clients without terminating the child.
7. Quit and reopen `rt`. Confirm the three session IDs, creation order, names, and running children remain coherent while the daemon remains alive. Stop the daemon with the documented explicit cleanup command.

The candidate must pass once on native Linux, macOS, and Windows. A CI outer PTY or ConPTY run is automated native evidence and does not replace this observation.

## Focused SSH observation

Use an already authorized SSH service and a fresh SSH connection on any one supported native platform. Do not install, enable, reconfigure, or expose an SSH service solely for this check without operator authorization.

1. Start the same candidate through the SSH terminal and attach two clients to one existing synthetic session.
2. Repeat the task-form Unicode cursor check, one stale rename review, and one competing-input transfer.
3. Close one SSH connection while its TUI is attached, reconnect with a fresh SSH client, and verify the child and stable session identity remain available.
4. Confirm the terminal restores after normal exit and after closing the SSH client.

## Current evidence

Native macOS automated evidence on 2026-09-14 passed the complete TUI gate with six scenarios and one fixture helper ignored. The gate used the real `rt`, SQLite, Unix IPC, an outer PTY, three real child sessions, two TUI clients, duplicate and cleared names, a physical form cursor, stale rename review, exclusive input transfer, detach, reattach, termination, and bounded cleanup. The run reported navigation p95 26 ms, input-to-rendered-echo p95 149 ms, flood throughput 5,761,732 bytes/s, and startup p95 125 ms.

The consolidated human-observation rows for Linux, macOS, and Windows are pending against the final usability candidate. The focused SSH row is also pending. OpenSSH 10.2p1 is installed on the current macOS host, but no local TCP port 22 listener was present during the 2026-09-14 probe. No service was enabled or modified.
