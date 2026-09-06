# First durable slice

M06 is the process-level exit gate before real PTY and TUI work. It proves that Relayterm coordination state crosses actual process, IPC, SQLite, client, and daemon-restart boundaries. The synthetic lifecycle host is a test executable. It does not launch a command, expose a public operation, or claim that a real terminal session survived.

## Reproduce the gate

Install the pinned Rust 1.98.1 toolchain and native build tools, then run from the repository root:

```text
cargo test -p relayterm-cli --test durable_slice --locked -- --test-threads=1 --nocapture
cargo test -p relayterm-daemon --test runtime_gate --locked -- --nocapture
cargo test -p relayterm-daemon --test protocol_gate --locked -- --nocapture
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
```

Unix socket tests can fail with `operation not permitted` inside a restricted filesystem sandbox. Run them in a normal local shell with the current user's permissions. The tests create short, private temporary roots outside the checkout and remove them after stopping every owned daemon. They require no provider executable, account, credential, network service, TUI, PTY, or graphical desktop.

The CI Quality workflow runs the durable slice twice on native Linux, macOS, and Windows before the complete workspace suite. Cross-compilation is supplementary and is not native evidence.

## Scenario

The non-ignored parent test invokes the actual `rt` binary and a separate helper process. It exercises both Git and non-Git project roots containing spaces and Unicode.

1. `rt workspace init` creates private state outside the project and starts the production daemon.
2. Two neutral definitions are registered without executing their commands.
3. The production daemon stops. A test executable prepares the same production runtime composition, performs normal startup recovery, creates two synthetic running lifecycle records, and starts the ordinary IPC server.
4. Separate `rt` processes create and activate a task, establish one exclusive claim, reject competing and repeated claims, append verified progress, and create a structured handover.
5. The handover closes the first claim atomically. A fresh client reads its verification and recommended next action, a second instance claims the task, and the user completes it explicitly.
6. Additional tasks exercise a concurrent two-client claim race, explicit release to blocked, rejected repeated release, explicit reopen, reclaim, and one-task-per-instance exclusivity.
7. The parent terminates its owned synthetic host while a task is active. Starting the production daemon against the same private home marks the prior synthetic sessions lost, closes the active claim, blocks the task, and preserves completed state and the complete ordered event prefix.
8. A second production restart performs no duplicate reconciliation. Every daemon is stopped by generation and both project trees retain their original contents.

The runtime ownership regression acquires the local endpoint before recovery. A contender therefore fails before it can mark the live owner's instances lost or mutate claims, tasks, revisions, or events. Existing daemon and protocol gates retain deterministic admitted-write draining and unknown-result recovery tests.

## Bounds and failure interpretation

Finite CLI commands and helper readiness use 30-second monotonic deadlines. Captured stdout and stderr are capped at 1 MiB per command. The bootstrap exchange remains capped at 64 KiB and now bounds both response reading and child exit under one operation deadline. Test-owned children are killed and reaped only after their deadline. Production daemons are stopped through the exact generation reported by the server.

A rejected operation must leave revision and durable state unchanged. A response lost after commit remains an unknown result and must be resolved by reading authoritative state, never by automatically replaying the mutation. Recovery may append typed events, but the complete event prefix before restart must remain byte-for-byte equivalent at the JSON value level and all sequences must stay contiguous.

Failure reports should identify the test and safe stage. Do not publish temporary roots, database files, raw diagnostic logs, command input, environment dumps, or terminal output as CI artifacts.

## Privacy and limitations

Registry files, SQLite files and journals, endpoint objects, locks, and diagnostics remain below the explicit private test home. Git status alone is not the cleanliness oracle because ignored files can be hidden; the gate compares the project directory contents directly.

Events and default diagnostics do not contain task narratives, progress or handover content, environment values, command arguments, terminal data, or project paths. Authorized entity and history queries do return their requested structured content.

M06 uses synthetic metadata to test coordination. It does not provide a production fake-session switch and does not prove real process supervision, PTY I/O, terminal reconstruction, SSH reattachment, TUI behavior, provider execution, or host-restart recovery of live processes. Those behaviors remain assigned to later milestones.

The native console-lifetime test closes the process that owns the launch console or pseudo-terminal, then reconnects through a separate `rt` process and stops the exact daemon generation. Unix runs the launch command below `script`; Windows runs it in a new console process. This closes the inherited M05 lifetime-evidence gap on the automated native matrix. Hosted CI does not provide Terminal.app, Windows Terminal, or an SSH server, so the test does not claim behavior specific to those applications or connections.
