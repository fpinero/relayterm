# PTY supervision and reattachment

Relayterm M07 adds daemon-owned real pseudo-terminals for generic shells and neutral configured commands. The TUI remains M08 work. Administrative session commands and the shared client library provide the current diagnostic surface.

## Ownership model

The daemon owns PTY masters, child handles, input writers, parsed terminal state, and bounded history. A client attachment observes a versioned replacement snapshot. Disconnecting a client releases its input lease and leaves the child running. Attaching does not acquire a task claim or change durable coordination state.

Each live session permits up to eight observers and one input owner. `session acquire-input` grants a generation-scoped lease. Input frames carry a contiguous sequence starting at one. Input and resize require the lease, and duplicate, missing, or stale sequences fail before bytes enter the writer queue. Termination is an explicit local-user administrative action and does not require the input lease.

The normal daemon stop command refuses while a real session is live. `rt daemon stop --terminate-sessions` deliberately terminates owned sessions before shutdown. An uncatchable daemon failure cannot preserve its in-memory screen or safely adopt a process from a stored PID. Startup therefore marks unreconstructable nonfinal instances `lost` through the existing atomic recovery operation.

## Launch contract

`rt session create` starts the platform default shell. `--definition-id ID` starts an enabled provider-neutral definition and captures its accepted command, argument array, allowed environment names, and capabilities in durable instance metadata before spawn. `--task-id ID` records launch context only. It does not claim or activate the task.

Commands are passed as executable plus argument array without shell interpolation. The working directory is canonicalized and must remain inside the workspace root. M10 will add worktree destinations. The child environment is cleared, rebuilt from the platform baseline in ADR 0005, extended only by configured variable names, and given `TERM=xterm-256color`. Values remain transient and are excluded from SQLite, events, diagnostics, and errors.

The launch sequence records `starting`, spawns exactly once, retains the native handles, and then records `running`. Allocation or spawn failure records `failed` without an invented exit code. Every request has a client-selected launch receipt. Repeating the same receipt and launch parameters during one daemon generation returns the existing outcome, while conflicting reuse fails. Receipts are bounded and generation-local. After restart or expiry, clients query durable session identifiers before deciding whether to submit a new receipt. `rt session create --receipt-id UUID` exposes this recovery contract when a caller needs to control the receipt.

## Terminal state

`relayterm-terminal` uses `vt100` behind a provider-neutral cell snapshot. A snapshot contains dimensions, cursor, relevant modes, cell content and attributes, a state revision, raw output position, and the retained-history boundary. Attach returns a daemon generation, stable connection attachment, nonzero stream identity, and an atomic snapshot. The typed client can then request output after that snapshot's zero-capable raw offset. Each bounded continuation arrives as an actual terminal frame with matching session, stream, and offset metadata. A cursor older than retained history returns `resnapshot_required`, so the client obtains a new authoritative replacement instead of rendering an incomplete byte suffix.

Parsing accepts split UTF-8 and escape sequences. Alternate-screen entry and exit preserve the parser's normal and alternate grids. The daemon produces each replacement from its authoritative parser, so a client never needs to recreate hidden parser state. Terminal control data never mutates tasks and Relayterm does not execute clipboard, hyperlink, or shell side effects from output.

| Resource | Limit |
| --- | --- |
| Live sessions | 8 per workspace daemon |
| Terminal dimensions | 1 to 1,000 per axis and 160,000 cells |
| Retained raw output | 1 MiB by default, 8 MiB implementation ceiling |
| Input frame | 64 KiB |
| Pending input | 64 KiB per session |
| Snapshot | Half the negotiated JSON frame limit at the diagnostic boundary |
| Attached clients | 8 per session, also bounded by the daemon connection limit |

Input uses a bounded nonblocking queue. PTY reading and writing run on owned blocking threads rather than Tokio workers. Slow or absent clients do not stop PTY drainage. An oversized snapshot fails with `resource_limit` instead of sending an incomplete frame.

## Administrative commands

```text
rt session list
rt session create [--receipt-id UUID] [--definition-id ID] [--task-id ID] [--working-directory PATH] [--rows N] [--columns N]
rt session attach SESSION_ID
rt session detach SESSION_ID
rt session input SESSION_ID --file PATH
rt session input SESSION_ID --stdin
rt session resize SESSION_ID ROWS COLUMNS
rt session terminate SESSION_ID
rt daemon stop --terminate-sessions
```

Input bytes come from a bounded file or standard input so they do not appear in process arguments. Human and JSON diagnostics never print input data. `session attach` returns the neutral state snapshot and continuation identity for diagnostics and future clients. The shared client exposes bounded binary continuation reads. M08 owns interactive rendering, focus, keyboard translation, pane switching, and the continuous user-facing read loop.

## Verification

Run the native process gates outside a filesystem-restricted sandbox:

```text
cargo test -p relayterm-terminal -p relayterm-pty --locked
cargo test -p relayterm-daemon --test pty_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-daemon --test pty_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test durable_slice closing_launch_console_keeps_daemon_reachable --locked -- --exact --nocapture --test-threads=1
```

The PTY gate uses one actual platform shell and two separately launched instances of a test-only interactive executable, then fills the eight-session admission limit and rejects a ninth launch. It verifies idempotent launch receipts, ordered input, full-screen state, Unicode, resize, exclusive writer ownership, client disconnect, stable attachment identities, binary continuation frames, truncation resynchronization, reconstruction from another connection, explicit termination, daemon shutdown, and absence of terminal capture files. The console gate launches three real shell PTYs through the detached daemon, closes the Unix pseudo-terminal owner or Windows console owner, reconnects from an independent `rt` process, and reads every session before explicit cleanup.

Native CI is the authority for Linux, macOS, and Windows behavior. Cross-compilation is supplementary. SSH coverage is recorded only where an authorized SSH environment exists; Relayterm does not provision hosted machines or credentials for this gate.

## Platform termination

On Unix, the PTY adapter reports the controlling process-group leader. Relayterm signals that exact owned process group through safe `rustix` APIs and then observes the leader exit. It never signals a stored or unrelated PID.

On Windows, Relayterm uses `portable-pty-psmux` without ConPTY cursor inheritance. It retains the child handle and first sends bounded `exit` input to a platform default shell. Configured commands and forced cleanup use the native `taskkill.exe /T /F` tree operation only for the retained live handle's current process identifier. Relayterm confirms exit through the retained handle. The writer is closed and joined before the ConPTY master and reader are reaped outside request handling. Bulk shutdown performs this cleanup in sequence because closing a pseudo-console can block on older Windows implementations. The native gate starts a descendant and verifies it is gone before accepting cleanup. Relayterm does not claim a general Windows sandbox or containment of processes that deliberately escape their original tree.

Natural exit, including a nonzero code, becomes `exited`. Confirmed explicit termination becomes `terminated`. Either final observation atomically closes an active claim and blocks its task. Process output never completes work automatically.
