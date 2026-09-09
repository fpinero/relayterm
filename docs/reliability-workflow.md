# M11 reliability workflow

## Purpose

This guide defines reproducible hardening evidence for the candidate. Use synthetic projects and identities in private disposable directories. Never commit runtime databases, captured terminal bytes, environments, native private paths or credentials.

## Automated regression entry point

Run the M11 gate twice as independent commands on every stable native runner:

```text
cargo test -p relayterm-cli --test hardening_gate --locked -- --nocapture --test-threads=1
```

The M11 gate compiles the retained real TUI and Git worktree scenarios into one candidate-level target, in addition to checking the public resource constants. It therefore uses the real `rt`, SQLite, authenticated native IPC, native PTY or ConPTY sessions, and installed Git. The original M08 and M10 targets remain in CI for focused regression evidence. Private backup and restore has its own required target:

```text
cargo test -p relayterm-cli --test backup_restore --locked -- --nocapture --test-threads=1
```

The native protocol fault target also runs twice as independent CI steps:

```text
cargo test -p relayterm-daemon --test protocol_gate --locked -- --nocapture --test-threads=1
```

The durable scale gate inserts 10,000 tasks plus 100,000 progress records and their matching durable events into private real SQLite. It then verifies 200-item task and progress pages, 200-event pages, stable follow-on cursors, stale revision rejection and preservation of all row counts. It runs twice on each stable native target:

```text
cargo test -p relayterm-persistence-sqlite --test resource_gate --locked -- --nocapture --test-threads=1
```

The composed scenarios cover the product boundaries but do not yet implement every step as one shared-state journey. The claim race barrier, combined worktree coordination, complete declared load, fault assertions and candidate evidence remain pending.

Each native CI command and each required gate repetition is a separate workflow step. Compiler installation, compiler reporting and Cargo reporting are also separate steps, so a failed native command on PowerShell cannot be hidden by a later successful command. The repository audit checks this workflow structure as part of candidate review.

Do not treat two passing invocations of the current entry point as evidence for the complete M11 journey. Candidate acceptance also requires every mapped retained gate, the fault and resource workloads, native offline observation and the manual matrix. Cleanup must remain bounded and target only fixture-owned children and private scratch paths.

## Current risk investigations

The M11 branch has reproduced two inherited resource concerns:

- R1: a Git child can exit while a descendant retains its stdout handle. On Unix, each Git invocation now runs in an isolated process group, both output readers use nonblocking descriptors, and completion terminates remaining members before joining bounded captures. Linux stable exposed `EIO` when the residual group was terminated after the Git parent exited. The reader now observes confirmed command completion before group cleanup and treats a subsequent pipe error as closure; errors while the command is live remain failures. On Windows, Git starts suspended inside a Job Object, resumes only after assignment, observes the primary exit independently and terminates the remaining job before joining bounded captures. Platform-specific regressions start a synthetic descendant that would write a delayed marker and verify prompt return and absence of that marker. Native Windows execution of the new regression remains pending, so R1 is not closed.
- R2: Git runs with a cleared and explicit environment, disabled system/global attributes and configuration, prompts, pagers, credentials, network transports and lazy fetch. Every Git operation overrides hooks, file-monitor commands and external attribute files. Worktree creation rejects filter configuration plus selected-commit nested attributes before checkout. Inert regressions verify that common and destination-specific included hooks, file monitors, a credential helper and an included filter do not execute. No Relayterm Git operation requests a remote. The complete native matrix remains pending, so R2 is not closed.
- R11: snapshot collection responses previously loaded all definitions, tasks, instances, claims, progress, handovers and worktrees before applying item and byte limits. Every public collection now uses revision-checked SQLite pages with `LIMIT + 1`, stable opaque-ID cursors and the existing response byte limit. Task and instance pages reconstruct their child records for at most the admitted page, and filtered worktree pages retain the same revision contract. The scale gate exercises 10,000 tasks plus 100,000 progress records and matching events, verifies complete durable row counts and measures repeated 200-item queries without reconstructing the full workspace. Mutation snapshots and single worktree-intent lookup still reconstruct larger workspace working sets, so R11 remains open until those paths and allocation accounting are bounded.
- Backup mechanism: a real WAL-mode SQLite writer now commits throughout a `VACUUM INTO` snapshot of an 8 MiB synthetic table. The resulting read-only database passes `PRAGMA integrity_check` and contains one committed point-in-time count. Backup and restore build private sibling staging directories and publish them with native no-replace renames only after database, manifest, registry and watermark validation. A failed operation cannot expose a partial final directory. The CLI gate copies `rt` away from the build tree and uses that copy to restore, reopen and stop the workspace. It rejects an existing restore home without changing its sentinel, redirected backup destinations and sources, unexpected members, newer manifest or schema versions, incomplete manifests, checksum-modified content, and a structurally corrupt database carrying a matching checksum. It verifies that rejected source material remains byte-for-byte unchanged and no failed final home appears. The public backup command still requires daemon ownership to be released, so live user-operated backup admission, cancellation, disk-full behavior and responsiveness remain pending.

These are candidate findings, not milestone completion evidence. Record native OS repetitions and measurements only after the complete candidate is available.

## Fault injection

Test-only barriers prove admission before triggering a failure. Production CLI and protocol contain no fault switches.

The real protocol gate sends partial prefixes and bodies, an oversized length header, invalid UTF-8 and JSON, an unknown envelope field, a wrong message version and a duplicate correlation ID over separate native IPC connections. A test-only completion counter acknowledges each rejected connection before the healthy control request. The gate asserts that every fault leaves the daemon responsive and does not change revision or event sequence. Typed unknown-operation handling, cancellation before delivery, cancellation after commit, dropped responses and cursor expiry remain in the same gate.

Remaining required boundaries include a nonreading subscriber through the real writer, SQLite write and notification loss, immediate child failure after spawn, daemon loss during a transaction, and the complete Git phase cancellation matrix.

Every fault test performs an independent healthy request or session action after the failure. A test fails if the daemon crashes, an unrelated session stops, a transaction becomes partial, a result is retried automatically, or a timeout leaves an unbounded reader, task, handle or child.

## Offline proof

Build with the locked dependency graph before the runtime observation. Run the integrated workflow without provider credentials under a process-scoped native network denial or observation mechanism. Validate that mechanism with a separate harmless synthetic probe.

The automated native baseline starts a real foreground `rt` daemon over real SQLite and local IPC, then samples its process network endpoints during repeated authenticated status calls. Linux correlates `/proc/<pid>/fd` socket inodes with the process network tables, macOS uses `lsof -a -p <pid> -i`, and Windows matches the owning PID in `netstat -ano`. Before inspecting Relayterm, the same monitor must detect a test-only loopback TCP listener. The target runs twice on each stable native runner:

```text
cargo test -p relayterm-cli --test offline_gate --locked -- --nocapture --test-threads=1
```

This baseline proves that the monitor is effective and that daemon startup plus local coordination do not open TCP or UDP endpoints. Candidate closure still requires the monitored Git, reconnect, backup and restore workflow and explicit DNS or outbound-attempt observation on every native OS.

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
