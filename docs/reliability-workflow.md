# M11 reliability workflow

## Purpose

This guide defines reproducible hardening evidence for the candidate. Use synthetic projects and identities in private disposable directories. Never commit runtime databases, captured terminal bytes, environments, native private paths or credentials.

## Automated regression entry point

Run the M11 gate twice as independent commands on every stable native runner:

```text
cargo test -p relayterm-cli --test hardening_gate --locked -- --nocapture --test-threads=1
```

The current M11 gate uses the real `rt`, SQLite and authenticated native IPC to verify resource contracts and private backup/restore. The retained M06 through M10 gates separately cover durable coordination, native PTY or ConPTY sessions, the real TUI, unknown executables and Git worktrees. The complete combined journey described in `docs/M11_details.md` remains pending until one candidate-specific harness joins those boundaries and adds the declared load and fault assertions.

Do not treat two passing invocations of the current entry point as evidence for the complete M11 journey. Candidate acceptance also requires every mapped retained gate, the fault and resource workloads, native offline observation and the manual matrix. Cleanup must remain bounded and target only fixture-owned children and private scratch paths.

## Current risk investigations

The M11 branch has reproduced two inherited resource concerns:

- R1: a Git child can exit while a descendant retains its stdout handle. On Unix, each Git invocation now runs in an isolated process group, both output readers use nonblocking descriptors, and completion terminates remaining members before joining bounded captures. A regression starts a synthetic descendant that would write a delayed marker and verifies that the command returns promptly and the marker is never created. Native Windows descendant cleanup remains pending, so R1 is not closed.
- R11: snapshot collection responses previously loaded all progress and handover rows before applying item and byte limits. These two append-only collections now use revision-checked SQLite pages with `LIMIT + 1`, stable opaque-ID cursors and the existing response byte limit. Persistence and real IPC regressions cover the page boundary, next cursor and stale revision. Other collection reads and mutation snapshots still reconstruct larger workspace working sets, so R11 and the 10,000-task or 100,000-history workload remain pending.
- Backup mechanism: a real WAL-mode SQLite writer now commits throughout a `VACUUM INTO` snapshot of an 8 MiB synthetic table. The resulting read-only database passes `PRAGMA integrity_check` and contains one committed point-in-time count. The public backup command still requires daemon ownership to be released, so live user-operated backup admission and responsiveness remain pending.

These are candidate findings, not milestone completion evidence. Record native OS repetitions and measurements only after the complete candidate is available.

## Fault injection

Test-only barriers prove admission before triggering a failure. Production CLI and protocol contain no fault switches.

Required boundaries include malformed and truncated framing, wrong protocol versions, oversized requests, client loss before admission and after commit, slow subscribers, cursor gaps, SQLite write failure, notifier loss, missing executable, immediate child failure, PTY output overflow, daemon loss during a transaction, and Git failure before and after an external effect.

Every fault test performs an independent healthy request or session action after the failure. A test fails if the daemon crashes, an unrelated session stops, a transaction becomes partial, a result is retried automatically, or a timeout leaves an unbounded reader, task, handle or child.

## Offline proof

Build with the locked dependency graph before the runtime observation. Run the integrated workflow without provider credentials under a process-scoped native network denial or observation mechanism. Validate that mechanism with a separate harmless synthetic probe.

Local IPC must remain functional. Record no product TCP listener, DNS lookup, telemetry, update request or hosted dependency during init, session coordination, Git worktrees, reconnect, backup and restore. Build-time package downloads, user-configured child networking and SSH test transport are separate and must not be attributed to Relayterm runtime.

Do not change a host firewall, install an SSH server, or expose a port as part of this procedure without operator authorization.

## Manual observation procedure

Use only synthetic content. Record date, candidate SHA, OS/build/architecture, terminal and shell versions, connection type, expected behavior, actual behavior and observer result. Screenshots are optional and must be sanitized.

For each cell in `docs/acceptance-matrix.md`:

1. Start `rt` in the named terminal or SSH connection and initialize a synthetic workspace.
2. Launch a shell and two synthetic interactive commands. Confirm all three remain independently responsive.
3. Exercise an alternate-screen application, wide and combining Unicode, rapid resize, small-window guidance, keyboard help and color-independent state.
4. Switch sessions, acquire/release input, complete a form, and observe an intentional revision conflict without losing the draft.
5. Detach and reattach from a new client. Confirm authoritative rendered state and the same live child identities.
6. Close the actual local terminal window or SSH connection. Reconnect through a new window or connection and confirm the daemon and children remained alive.
7. Exit normally and force one recoverable client failure. Confirm terminal modes, cursor and screen are restored where restoration code can run.
8. Stop the daemon explicitly, reopen it, and confirm durable state plus honest one-time loss reconciliation.

Exiting only a shell does not prove terminal or connection closure. A forced client kill cannot guarantee that the dead process restored terminal modes, so record terminal recovery separately.

## Evidence and blockers

Automated console PTY or ConPTY results do not satisfy Terminal.app, Windows Terminal or SSH observations. A missing required OS or connection stays `Pending` or `Blocked`; it is not an optional limitation. Repeat affected manual cells after a behavioral fix.

The candidate cannot merge while any required behavior, resource threshold, offline result, security review or manual cell is missing. M12 receives only installation, packaging, PATH collision and final release work.
