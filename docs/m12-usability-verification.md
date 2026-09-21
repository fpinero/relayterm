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

Source `491d5f1037450c362525b289fa6361d834fc0b6f` passed [Quality run 34947043434](https://github.com/fpinero/relayterm/actions/runs/34947043434) and [Security run 34947043380](https://github.com/fpinero/relayterm/actions/runs/34947043380) on 2026-09-15. The native stable jobs ran two independently reported complete TUI repetitions on Linux, macOS, and Windows. The three release jobs additionally ran one complete TUI gate, session presentation, worktree, and backup/restore journey through the exact extracted release executable.

The gates used real `rt` or `rt.exe`, SQLite, native IPC, outer PTY or ConPTY, three real child sessions, two TUI clients, duplicate and cleared names, a physical form cursor, stale rename review, exclusive input transfer, detach, reattach, termination, and bounded cleanup. The release-specific Windows session presentation gate completed in 0.69 seconds after the test harness stopped waiting for EOF on a pipe handle inherited by the detached daemon.

One installed macOS arm64 artifact built from the same source passed the session presentation, complete TUI, two real worktree, and private backup/restore gates. It reported startup p95 77 ms, navigation p95 30 ms, input-to-rendered-echo p95 122 ms, and flood throughput 64,108,771 bytes/s. This is automated native evidence through an installed executable, not the physical Terminal.app observation required below.

The Linux human-observation row remains pending. Windows installation, session
identity, Unicode editing, stale review, input exclusion and transfer, client
reopen, handover, successor claim, completion, and fresh-home recovery were
observed on d760554. [The Windows observations](m12-windows-manual-observations.md)
separate operator observations from assistant checks and record the exposed
header and terminal-width defects. The macOS observation and corrected-candidate impact
mapping are recorded below. The focused SSH row also remains pending. Computer
control identified Terminal.app on the earlier macOS host but refused automation
of that application for safety reasons. OpenSSH 10.2p1 was installed during the
earlier probe, but no authorized local TCP port 22 listener was present. No
service was enabled or modified.

## Header correction impact

Source `0bc4b6384adf37dbdd9939e840d9c3e14e86d9e1` changes only TUI rendering
and its regression coverage relative to the prior production source. It reserves
space for freshness and moves Relayterm to the footer. All 24 TUI tests and
Clippy passed on Windows, including all six screens and eight freshness states
at 80 by 24. Existing daemon, persistence, and protocol behavior is unchanged.
The old physical observations remain valid only for their stated unchanged
boundaries. A new native artifact and affected header/footer observation are
required; the old Windows screenshot cannot certify the new layout.

| Evidence | Windows | macOS | Linux / SSH |
| --- | --- | --- | --- |
| Handover correction and retained history | Observed on d760554 | Observed on d760554 | Native journey pending |
| Prior session and editing journey | Observed, deviations recorded | Observed, source mapping retained | Native journey pending |
| Header/footer at exactly 80 by 24 on 0bc4b63 | Automated rendering and operator observation passed | Rebuild and physical retest pending | Include in new native and focused SSH observations |
| Command clipping after writer acquisition | Regression and physical retest passed on 9cf91f7, including read-only resize isolation | Rebuild and affected writer-transfer observation pending | Exercise differently sized clients with 9cf91f7 |

Use the updated [native continuation instructions](m12-native-continuation.md),
including the existing Linux task prompt. No separate prompt document is needed.

Source `9cf91f7164235d35beda81d9c8c2d4200b703aad` additionally calls the
existing lease-owned terminal resize operation after successful input acquisition.
It changes the client request sequence, not daemon authorization, persistence, or
protocol shape. Native regression reads actual snapshot dimensions through first
acquisition and transfers between differently sized clients, and verifies that
read-only and rejected clients retain the writer's dimensions. Affected native
and SSH manual observations remain required before global acceptance.

On 2026-09-19 the operator performed the [macOS manual observations](m12-macos-manual-observations.md). Session presentation, Unicode cursor editing, stale rename review, competing input transfer, and client reopen passed on the original local artifact. The installed quick start exposed a blocking handover refresh failure. The source correction passed automated checks. A separately identified local build from `d760554` subsequently passed the affected manual handover, second-session continuation, completion, backup recovery, and client-reopen retest. The operator subsequently verified absolute-path discovery and scoped removal of a disposable installation; independent checksum and task readback confirmed the retained candidate and state were preserved. Collision refusal was checked by the assistant. The local macOS journey is recorded with that evidence distinction; SSH, Linux, Windows, and the final M12 reconciliation remain open.

## Linux native continuation

The [Linux native record](m12-linux-manual-observations.md) tracks exact source
9cf91f7 on Ubuntu 24.04.5 x86-64. Two clean builds and normalized packages are
byte-identical; extracted smoke and dedicated installation passed. The installed
candidate passes all ten session presentation, TUI, backup/restore, and worktree
tests with one helper ignored after an isolated test-harness resize synchronization
correction. The record retains the original synchronization failure and the
loaded echo-budget failure, followed by the serial passing run.

The operator completed the required Linux observation attempts and authorized
focused loopback SSH journey. The [final Linux report](m12-linux-final-report.md)
records passing session, editing, conflict, writer-size, handover, backup/restore,
and normal-exit observations, plus failed interrupted-SSH visual restoration and
the documented Ctrl-Space detach shortcut. Additional UI findings remain open.
The implemented Ctrl-] release followed by Esc detach passed physically.

Linux acceptance and the SSH row remain open for corrective work and retesting;
they are no longer unattempted. Earlier pending statements and the historical
impact table above describe prior evidence stages. Windows and macOS source
mappings are unchanged. No global acceptance or artifact freeze is claimed.

### Corrected Linux physical results

The targeted physical Linux retests passed on source 15794ad: input release,
repeated release in navigation, disconnected indicators, no unintended Events
transition while disconnected, suppression of repeated diagnostics, clean exit,
and authoritative rename context with retained draft and safe discard. See the
[final report](m12-linux-final-report.md) for the exact executable identity and
independent readback. This supersedes the earlier pending Linux correction
checkpoints only. Cross-platform verification, the informational Error prefix,
and global acceptance treatment of SSH and resize limits remain open.
