# TUI workflow and verification

## Scope

M08 makes `rt` without a subcommand the interactive Relayterm client. The TUI is a replaceable IPC client. It does not open SQLite, own PTY handles, launch child processes directly, or infer task state from terminal text. The daemon remains authoritative for workspace data, claims, events, process lifecycle, terminal parsing, input leases, and resource limits.

Administrative subcommands and their human or JSON output remain available. `rt --help`, `rt --version`, and every administrative command avoid raw mode and alternate-screen output.

## Startup

Run `rt` in a terminal from a project directory, or select a project explicitly:

```text
rt
rt --workspace /path/to/project
```

If the directory is not registered, `rt` displays its escaped path and asks before initialization. Declining makes no workspace changes. A noninteractive stdin or stdout, or `--format json` without an administrative command, fails before bootstrap and emits no terminal controls. Opening an existing workspace discovers or starts its detached daemon, connects over current-user local IPC, obtains one coherent snapshot, and then enters the alternate screen.

## Screens and keys

Essential state always has a text label. Color is supplementary.

| Context | Keys | Result |
| --- | --- | --- |
| Global | `1` to `6`, `Tab`, `Shift-Tab` | Select Overview, Tasks, Sessions, Agents, Events, or Help |
| Global | `?` | Open keyboard help |
| Global | `R` | Request an authoritative refresh or reconnect |
| Global | `q`, `Ctrl-C` | Close this client without terminating sessions |
| Lists | `j`, `k`, Up, Down | Move the stable selection |
| Tasks | `n`, `e` | Create or edit the selected task |
| Tasks | `r` | Move backlog or blocked to ready, or explicitly release active work to blocked |
| Tasks | `c`, `b`, `d`, `x` | Claim for the selected running instance, block, complete, or cancel |
| Tasks | `p`, `h` | Append progress or prepare an atomic structured handover |
| Task detail | PageUp, PageDown | Read ordered claim, progress, and handover history |
| Sessions | `s`, `a` | Launch the default shell or selected enabled definition |
| Sessions | Enter, Escape | Attach read-only or detach |
| Terminal navigation | PageUp, PageDown | Browse the daemon's parsed, bounded in-memory history and return toward live output |
| Terminal navigation | `i` | Acquire the exclusive input lease and return to live output |
| Terminal input | `Ctrl-]` | Release the input lease and return to Relayterm navigation |
| Sessions | `t` | Open explicit termination confirmation |
| Forms | Tab, Shift-Tab | Select a field |
| Forms | Left, Right, Home, End, Backspace, Delete | Edit at UTF-8 character boundaries |
| Multiline forms | Enter | Insert a newline |
| Forms | `Ctrl-S` | Submit exactly once |
| Forms | Escape, `Ctrl-C` | Ask before discarding the in-memory draft |

Task claims use the instance represented by the selected session. Launch context does not claim a task. Task dependencies remain informational. A release blocks active work; making it available again is a separate ready action. Task detail exposes immutable claim attribution and close reasons, progress verification, and every structured handover field, including verification and recommended next action.

Termination requires typing `TERMINATE`. Closing the TUI, detaching, losing a read-only attachment, or releasing input never terminates a child.

## Child terminal input profile

The input lease owner can send UTF-8 text and the following decoded keys. Relayterm never copies child escape sequences directly to the host terminal.

| Key | Child bytes |
| --- | --- |
| Enter | `0d` |
| Backspace | `7f` |
| Tab | `09` |
| Escape | `1b` |
| Delete | `1b 5b 33 7e` |
| Home, End | `1b 5b 48`, `1b 5b 46` |
| PageUp, PageDown | `1b 5b 35 7e`, `1b 5b 36 7e` |
| Arrows | CSI arrows, or SS3 arrows while application cursor mode is active |
| F1 to F4 | SS3 `P` to `S` |
| F5 to F12 | Standard CSI tilde sequences |
| Ctrl plus ASCII | The corresponding C0 byte |
| Alt plus a supported key | Escape followed by that key's bytes |

Bracketed paste wraps one complete, admitted paste in `ESC[200~` and `ESC[201~` only when the daemon's terminal state reports that mode. One paste and one input frame are limited to 64 KiB. An oversized paste is rejected without partial delivery. Unsupported key combinations have no effect.

## Authoritative terminal display

`session.read_display` is an additive protocol-version-1 capability advertised by the daemon. It binds reads to the existing daemon generation, connection attachment, and stream identity. Each response describes one coherent viewport with source dimensions, parsed cells, cursor, modes, terminal revision, raw retention boundary, parsed scrollback offset, and retained scrollback rows. An unchanged revision and scrollback offset returns no replacement cells.

The wire viewport is limited to 16,000 cells. Each cell is a compact tuple containing safe text, neutral foreground and background colors, and attribute bits for bold, dim, italic, underline, inverse, wide, and wide-continuation state. This format keeps the maximum viewport below half the JSON frame budget. It is a parsed display representation, not a serialized parser and not a byte continuation. The TUI maps it to Ratatui cells, skips continuation cells, respects double-width boundaries, and renders control or bidirectional formatting characters as placeholders.

PageUp and PageDown request a private parsed-history view. The daemon takes that view under the session lock and restores its live parser view before releasing the lock, so one reader never changes another reader's viewport. Input acquisition always returns to live output. Parsed history and the 1 MiB raw continuation ring are bounded in daemon memory and are never written to SQLite.

## Synchronization and uncertainty

Control requests and event subscription use independent connections. The event reader queues at most 64 invalidations for the UI, and the protocol client retains at most 256 events or 1 MiB. It reconnects with bounded delays of 100, 250, 500, 1,000, and 2,000 ms. A gap, expired cursor, incompatible generation, or changed attachment requires an authoritative replacement.

Reads may reconnect automatically. Mutations and terminal input never retry automatically after delivery becomes uncertain. The TUI labels the state stale, retains a form draft where applicable, refreshes authoritative state, and requires a deliberate user decision before another submission. Form submission is disabled while its request is pending. Revision-bearing task edits use the revision captured by the coherent snapshot, so a concurrent edit is rejected instead of overwritten.

Diagnostics contain allowlisted categories and sequence or opaque identity context. They do not retain drafts, terminal cells, child output, command arguments, environment values, raw errors, or native paths. Diagnostics retain at most 1,000 entries and 1 MiB. All form drafts together are limited to 256 KiB and exist only in client memory.

## Terminal lifecycle

The client acquires raw mode, alternate screen, and hidden cursor in order. A guard tracks each acquired step. Normal return, initialization failure, render failure, catchable panic unwinding, Ctrl-C navigation, SIGINT, and supported termination signals run idempotent restoration for acquired steps. Restoration shows the cursor, leaves the alternate screen, disables raw mode, and asks Ratatui to show the cursor again.

The TUI requires at least 80 columns by 24 rows for the complete layout. Smaller terminals show bounded resize guidance and still accept help and exit. Resize changes the attached PTY only while this connection owns its lease; a read-only observer cannot resize shared child state.

## Native gate

Run the complete gate outside a filesystem-restricted sandbox because it creates local IPC endpoints, a detached daemon, SQLite files, native PTYs or ConPTY instances, and child processes:

```text
cargo test -p relayterm-cli --test tui_gate --locked -- --nocapture --test-threads=1
```

The gate drives the actual `rt` screen through a native outer PTY or ConPTY. Its test-only executable provides two neutral interactive fixtures through ordinary agent definitions, while the third session is the platform default shell. The daemon and all product operations use production composition. The gate initializes a Unicode path, creates and coordinates a task, records progress, performs a handover between selected instances, closes and reopens a client, verifies stable live sessions, exercises an input lease, wraps terminal retention more than twice, browses parsed history, and performs bounded owned cleanup.

The same target includes a representative startup workload with 100 tasks, 3 live sessions, and 100 history entries. It performs one warm-up and 20 measured client launches. Under a separate 2.7 MiB fixture flood it measures 100 navigation responses and 100 one-character line echoes in a quiet session. Every CI invocation runs as its own step so a failing first pass cannot be hidden by a later command.

Local macOS candidate runs on 2026-09-07 used the native PTY harness and a debug test build. The table reports the latest complete measurement for each workload:

| Measurement | Samples | Median | p95 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| Existing-daemon connection to usable overview | 20 after warm-up | 169 ms | 195 ms | 198 ms |
| Navigation while another session emits output | 100 | 38 ms | 49 ms | 51 ms |
| Input to parsed rendered echo | 100 | 168 ms | 174 ms | 199 ms |

The measured fixture output rate was 4,745,853 bytes per second. The TUI polling ceiling is approximately 30 frames per second. Its bounded queues are 64 UI invalidations, 256 protocol events or 1 MiB, 64 KiB pending input per session, 16,000 cells per viewport, 1,000 diagnostics or 1 MiB, and 256 KiB per complete draft.

CI run links, runner versions, and both native repetitions are recorded in `docs/supported-platforms.md` after the final candidate passes. Hosted console automation is native PTY or ConPTY evidence, but it is not a claim that Terminal.app, Windows Terminal's graphical interface, xterm, or SSH was manually tested. Those optional named-terminal checks remain explicit gaps unless an authorized environment is available.

## Limitations

Relayterm does not persist terminal history or recordings. A daemon restart cannot adopt existing PTYs and honestly marks unreconstructable sessions lost. The TUI does not provide provider templates, Git worktrees, mouse input, OSC clipboard, hyperlinks, or automatic task updates from child output. Those boundaries belong to later milestones or remain excluded from the MVP.
