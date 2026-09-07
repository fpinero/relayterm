# M08: Complete TUI workflow execution contract

## 1. Delivery and authority

Implement the ten pending M08 parent tasks in [TODO](../TODO.md). This document is an execution contract for a subsequent implementation agent. Creating and merging this plan completes only M08-DOC, not any M08 implementation task. Read [project instructions](../AGENTS.md), [vision](../PROJECT_VISION.md), [specification](../MVP_TECHNICAL_SPEC.md), [README](../README.md), and recent [logbook entries](../avances.md) first.

M08 delivers an interactive `rt` client through which a user opens or initializes a workspace, launches real shells or existing neutral agent definitions, coordinates tasks, records progress and handovers, changes session, disconnects, and resumes. All authoritative mutations go through authenticated IPC. The daemon retains ownership of SQLite, lifecycle observations, terminal parsers, and children.

Read ADRs [0001](decisions/0001-daemon-scope.md), [0002](decisions/0002-local-ipc.md), [0003](decisions/0003-terminal-state.md), [0005](decisions/0005-environment-privacy.md), [0006](decisions/0006-agent-configuration.md), and [0008](decisions/0008-rust-platforms.md), plus the [PTY guide](pty-supervision.md), [daemon guide](daemon_cli.md), [platform evidence](supported-platforms.md), and [M07 contract](M07_details.md). Resolve contradictions against implemented behavior and these contracts, record the resolution, and add regression evidence. Historical completion statements are not substitutes for reproducing a prerequisite.

Do not implement provider templates, definition editing/import screens assigned to M09, worktrees, TUI multiplexing over TCP, remote accounts, automatic task orchestration, persistent terminal recordings, credential forms, exports, telemetry, or releases. Existing administrative commands remain usable. A generic shell supports the complete account-free coordination journey; configured agents use existing definitions. Tests may register synthetic definitions in fixture setup through public IPC, but the actual M08 user journey must use the real TUI for every in-scope action.

## 2. Inspected baseline and prerequisite work

The baseline is main after M07 PR #13, merge `800ac87acb7f17d01845c2c4f84c65e24c3b63ae`. Recheck the target revision when implementation starts.

| Location | Current behavior | M08 consequence |
| --- | --- | --- |
| `crates/relayterm-tui/src/lib.rs` | Only `NotImplemented` and a placeholder `run` | Build presentation, effects, and lifecycle from an explicit model |
| `crates/relayterm-cli/src/main.rs` | No-command branch invokes that placeholder; bootstrap and route handling already exist | Share bootstrap composition, retain global options and administrative output contracts |
| `crates/relayterm-client/src/lib.rs` | Typed session calls; generic coordination calls; staged snapshots; explicit delivery uncertainty | Add typed presentation adapters without copying domain rules into widgets |
| Same client | `next_event` holds the stream mutex while waiting; requests use that mutex too | Prove idle subscription cannot stall input, resize, mutation, cancellation, or reconnect |
| Same client | Full workspace refresh collects definitions, tasks, instances, claims, progress and handovers | Measure existing staging cap, allocations and startup latency; avoid repeated full-history refresh on each event |
| `crates/relayterm-terminal/src/lib.rs` | Server parser owns hidden state; snapshot exports visible cells and selected modes | Do not initialize a new client parser from visible cells and feed raw suffixes |
| Protocol and supervisor | Attach replacement snapshot, raw-offset output reads, generation and input lease | Supply an authoritative live display path with coherent state revisions |
| Daemon snapshot serialization | Frame budgets can reject a large valid screen | Reproduce worst-case 160,000-cell display; provide bounded transfer without reducing accepted dimensions silently |
| `.github/workflows/ci.yml` | Multiple native commands share steps; Windows default shell differs from Unix | Verify every individual command exit, not just the last command or green step title |

M07 calls for ordered parsed updates, bounded scrollback, terminal replies and parser bounds. Inspect actual implementation for missing visible history, unbounded combining sequences, control-string parsing, terminal queries and concurrent resize. Create focused failing tests before fixes. Restrict fixes to prerequisites for correct M08 operation and inherited regression coverage. No production fake sessions or skipped Windows assertions are acceptable substitutes.

The current workflow's first durable-slice step contains one invocation despite its title; another invocation occurs in the PTY step. Keep each required repetition explicit and independently failing. Do not edit old logbook entries. If evidence needs correction, append an exact correction with candidate SHA, command and result.

## 3. Architecture and execution model

### 3.1 Boundaries and modules

Use Ratatui and Crossterm as specified. During implementation select versions compatible with the pinned Rust/MSRV and native matrix, consult their official documentation for selected APIs, minimize features, update Cargo.lock, and run dependency/license checks. This plan does not authorize a compiler upgrade or blanket architecture exception.

Suggested TUI modules: `model`, `action`, `reduce`, `effects`, `screens`, `forms`, `terminal_view`, `input`, `lifecycle`, and `safe_text`. Adapt names to repository conventions. Keep a pure reducer accepting model plus action and producing model changes and bounded effect requests. Rendering consumes immutable presentation state. Tests inject effects and time without making test controls available in production.

Allow `relayterm-tui -> relayterm-client, relayterm-protocol`. CLI remains the executable/composition boundary and provides a narrow bootstrap callback or channel for workspace open/init. TUI must not import daemon, domain entities, SQLx, configuration loaders, PTY handles or process launch APIs. Retain the existing client-to-IPC boundary and core dependency closure. Do not add a client-side terminal parser merely to simulate daemon state. Update the graph test with positive and negative tests for new legitimate edges.

Use one screen writer and one keyboard reader. Blocking terminal event reads must not run on an async worker responsible for IPC. Network/effect workers return typed completions tagged with workspace, connection generation, selection ID and local request ID. Reject stale completions after switching workspaces or selections. Never hold a rendering/model lock across I/O. Cancel obsolete reads; admitted mutations retain their delivery outcome even when the user closes a modal.

A shared-stream reader must own frame demultiplexing or separate bounded connections must isolate event subscription, controls and active terminal traffic. Choose and document one after the idle-subscription test. With separate connections, terminal attachment, lease, input and resize must remain on the same owning connection. Budget total connections within daemon limits. Do not race two consumers reading one framed stream or cancel a partial read and then reuse a desynchronized stream.

### 3.2 Client resource contract

Initial policy: render at most 30 frames/second; process keyboard and control completions before bulk display refresh; no busy polling. Coalesce redraw and resize notifications to the latest value. Keep at most one terminal display transfer in flight per visible session and one workspace refresh. Schedule at most one visible terminal pane initially, with tabs for at least three concurrent sessions. Inactive sessions stay alive in the daemon and need no full-rate rendering.

Set explicit constants and accounting tests: 256 queued UI actions, 64 queued control completions, 64 KiB unsent terminal input, 256 KiB total form drafts, 1,000 diagnostic entries and 1 MiB diagnostic text, and history pages of 50 with the protocol maximum of 200. Count bytes as well as entries. Large snapshot staging must reuse the protocol's current cap with separately bounded decoded state, rather than multiply it through unlimited clones. Terminal transfer staging is specified in section 6. Never silently drop keystrokes, paste fragments, mutations or event gaps: reject new input visibly or invalidate the affected display and resynchronize. Dropping redundant redraw requests is permitted.

Measure CPU, queue high-water marks and bounded memory under output load. Document actual peak allocation accounting, including parser, serialized and decoded snapshots. If a limit needs adjustment, justify it with native measurements and explicit tests. No unbounded channels, caches, histories or task-per-byte spawning.

## 4. Screen, keyboard and startup contract

### 4.1 Screen model

| Screen | Required content and actions |
| --- | --- |
| Workspace entry | Current or explicitly selected directory, open, initialize after confirmation, choose another path, actionable failure |
| Overview | Workspace label, connection freshness, task counts, active owners, session summaries, shortcuts |
| Tasks | Filter by status, stable selection by task ID, owner and priority labels, create/edit/state/claim/release, detail |
| Task detail | All editable content, informational dependencies, claim history, paginated progress and handovers, append and prepare handover |
| Sessions | Session and instance IDs, shell/definition label, launch context versus current claim, actual lifecycle, attach, launch, confirmed terminate |
| Terminal | Daemon-authoritative grid, cursor, session label, read-only/writer status, connection status, focus escape hint, history position |
| Agents | Existing definition names, enabled state and capabilities, associated instances; launch enabled definitions |
| Events/help | Safe structured events, typed errors, sequence/freshness, key bindings and recovery instructions |

All screens need loading, empty, stale/disconnected, permission/validation, resource-limit and retry states. IDs distinguish duplicate labels. Selection follows IDs across refresh and falls back deterministically when an entity disappears. Lists use bounded paging/virtual rendering. A task board is optional; a complete keyboard task list is sufficient. Worktree controls are absent until M10.

### 4.2 Keyboard routing

In navigation focus: Tab/Shift-Tab cycle focus, arrows move selection, Enter opens/activates, Escape closes a modal or backs out, `?` opens context help, `q` requests client exit. Use a discoverable action menu for all task and session operations, avoiding undocumented overloaded single letters. Forms treat printable keys literally; Enter inserts a newline in multiline fields and submits only through a focused submit button. Support keyboard editing, Home/End, deletion, horizontal/vertical scrolling and paste without byte-boundary corruption.

Terminal input focus captures keys for the child, including Ctrl-C, Ctrl-D, Escape, Tab and printable `q`. Reserve Ctrl-] to return to Relayterm navigation. In navigation, a documented "Send Ctrl-]" action forwards the literal control byte after reacquiring writer focus. There is no timing-dependent double-key escape rule. The focus hint remains visible, including during high-volume output. Leaving input focus releases the lease and enters observation mode; detach releases the attachment too. Quitting the client never terminates sessions. Unsaved drafts require discard confirmation; termination has its own explicit confirmation naming the session and warning about its active task becoming blocked.

Handle key press/repeat/release according to the selected Crossterm API so Windows does not submit twice. Ctrl-C in navigation requests ordinary client exit; in a form it cancels with draft protection. Mouse support is optional and cannot be required for any action. Do not forward mouse or focus sequences unless the negotiated terminal profile and implementation support them.

### 4.3 Startup and small terminals

No-command `rt` uses `--workspace` if present, otherwise the current directory, preserving established root semantics rather than inventing recursive workspace discovery. Reuse M05 bootstrap with native paths, validated root and private runtime storage. When no workspace exists, display an explicit initialize/open choice; do not silently create one. "Choose path" is enough for workspace selection, no global registry browser is required. Errors preserve the entered path in local UI memory but diagnostics use aliases. Respect `--home` and bounded `--timeout` through composition. Do not convert native paths to lossy strings and then submit the altered path.

Check stdin/stdout terminal capability before entering raw mode or starting bootstrap side effects. Piped no-command invocation and `--format json` without an administrative command return an actionable bounded error and no escape sequences. `--help`, `--version` and existing administrative JSON output remain unchanged. Supported-terminal detection must report limitations without relying on the mere presence of TERM on Windows.

Normal layout target is 100x30; functional compact layout is 80x24. Below 80x24 show minimum-size guidance with resize, help and exit available, preserve drafts and do not resize a child to zero. Larger layouts may show side detail, but all functions remain reachable at 80x24. Compute child size from the drawable pane after borders/status, validate M07 cell limits, and clip read-only observers without changing the shared terminal. Only the writer may resize; debounce latest dimensions and ignore stale acknowledgements. Keep navigation available if an oversized layout requires clipping or an explicit bounded viewport.

### 4.4 Terminal lifecycle

Create an idempotent RAII lifecycle guard before the first mode change; track which changes succeeded. Restore raw mode, alternate screen, cursor visibility and any enabled paste/mouse/focus modes on normal exit, initialization failure, render/I/O error and catchable panic. Coordinate panic-hook restoration with the previous hook and do not dump sensitive panic payloads while raw mode is active. Preserve unrelated global hooks when ownership ends. Test failure after each initialization step.

Deliverable OS signals should use graceful client cleanup where available. SIGKILL, power loss and terminal emulator destruction cannot promise terminal restoration; document that boundary. Those paths must still not request session termination. After teardown, administrative-style output must not print an unsolicited success JSON object for a successful TUI exit. Error reporting occurs after restoration. Test terminal usability by sending and observing input through the outer native terminal after the TUI exits, not only by asserting a guard flag.

## 5. Coordination forms and authoritative state

Use wire DTOs and existing operations, not domain repositories. The UI may prevalidate to help users, but server validation remains authoritative. Public actions always use LocalUser attribution, including claims on behalf of a selected running instance. Never fabricate an instance actor or expose System.

Task form fields are title, description, priority, scope paths, acceptance notes and dependency IDs. Keep identity/status/ownership outside generic edits. Defaults are backlog and normal priority. Display dependencies as informational; reject duplicate, self, absent and cross-workspace references through the existing contract, allow cycles between distinct tasks and do not block claims based on dependency state.

| Source | Allowed destinations and UI operation |
| --- | --- |
| backlog | ready or cancelled through transition |
| ready | active only through claim; cancelled through transition |
| active | blocked/done/cancelled through transition, blocked through release, handover_ready only through structured handover |
| blocked | ready or cancelled through transition |
| handover_ready | active only through a new claim; cancelled through transition |
| done / cancelled | No state or content edits; user historical annotations remain available |

Do not expose same-state transitions, direct active-to-ready, generic activation or a generic handover_ready transition. A claim requires a running instance with no open claim and a claimable task; another client may race after the selector was populated. On conflict retain the draft and selection, refresh authoritative task/claim state, explain the conflict, and require deliberate resubmission. Release explicitly blocks; reopening is a separate action. Show immutable claim history with requester, closer, reason and timestamps.

Progress fields are summary and verification. Handover fields are summary, decisions, changed paths, verification performed, open questions and recommended next action. Display all stored fields without collapsing verification or next action out of reach. Require handover summary, verification and next action in the form; verification may explicitly say `Not run: reason`. Keep stricter user guidance separate from server rules where existing progress contracts permit empty verification. Handover creation, transition and claim closure remain one server operation. Do not implement three client mutations. A second instance explicitly claims handover_ready and reads context before work continues. User-authored progress/handover preserves optional instance attribution as None.

Mirror domain byte limits: names/title 256, description/acceptance 16 KiB each, summaries 8 KiB, verification/next action 8 KiB each, decisions/open questions 16 KiB each, complete handover 64 KiB, relative paths at most 128 entries and 4 KiB each, dependencies at most 128. Confirm the exact aggregate formula from the domain validator before coding. Show byte counters and field-specific errors without echoing rejected sensitive values. No silent truncation, trimming, normalization or splitting quoted commands. Narrative fields allow admitted newline/tab controls; single-line fields reject them. Scope and changed paths remain workspace-relative with no escape components.

Keep drafts only in bounded memory. Explicit save submits once and disables duplicate submission while pending. Closing a form does not imply cancellation of a sent mutation. Failed validation retains content. Changed server state during editing shows a refresh/conflict decision; never overwrite drafts with a background snapshot. Inspect whether TaskUpdate supports an expected revision. If it lacks a stale-edit guard, reproduce two-client overwrite and add a narrow capability-negotiated compare-and-set contract before promising conflict-safe edits. A client-side pre-read alone is not atomic protection. Do not silently change accepted version-1 operation semantics.

History is ordered using server cursors and durable sequence/record ordering, not timestamp sorting alone. Load pages on demand, cap retained pages, preserve reading position and expose truncation/load-more. Background notifications indicate newer history without jumping the user's view.

## 6. Real terminal display and input

### 6.1 Prerequisite reconstruction contract

Visible cells are display state, not a serialized parser. Hidden primary/alternate buffers, saved cursors, wrap/margins, tab stops and partial UTF-8/escape sequences affect future output. M08 must render authoritative server-parsed state and never apply raw continuations to an incompletely restored local parser or write child escape sequences directly to the host terminal.

Implement a bounded capability-negotiated display read contract, reusing attach identity. It returns a replacement snapshot or parsed delta with daemon generation, session, attachment/stream identity, base and resulting state revision, dimensions, cursor, modes and retention boundaries. Prefer changed-cell runs plus explicit replacement when history is unavailable. Full replacement on each changed revision is acceptable only if it passes measured bandwidth, memory and input latency gates under sustained output. Polling must be bounded, cancellable and avoid generating revisions when state has not changed.

Define exact DTOs and examples in ADR 0003 before integration: snapshot begin/chunks/end, transfer identity, immutable revision, expected count/byte totals, chunk ordinal or cell range, completion validation and expiry. Support valid admitted screens up to 160,000 cells without exceeding the 8 MiB JSON frame limit. Freeze a bounded consistent snapshot or reject/restart when the revision changes; do not mix chunks from different screen states. Bound transfer allocation before trusting declared counts, cap outstanding snapshots per connection and globally, expire abandoned transfers and validate unknown schema versions. Preserve existing successful wire operation semantics through new advertised operations; increase protocol version if an incompatible change is unavoidable and test old/new rejection paths.

Install only complete validated snapshots. Apply deltas only to the exact base revision; duplicates, gaps, stale generations, resize races, truncation or invalid cell ranges trigger replacement. Coalesce intermediate revisions only through an explicit resulting snapshot or delta relative to the retained base. Raw offsets, screen revisions and workspace event sequences are separate counters. Raw diagnostics stay separate from display correctness.

Expose bounded parsed scrollback for navigation if the current wire model lacks it. History must not be reconstructed from a ring beginning inside a control sequence. Specify line IDs, retention boundary, byte/cell caps, page cursor invalidation, primary versus alternate behavior and resize policy. Leaving scrollback returns to the live screen. A reader cannot change the daemon's shared viewport by scrolling. No persistent history or recordings.

### 6.2 Rendering and input encoding

Map neutral cells to Ratatui cells with tested indexed/RGB colors, inverse and supported attributes, cursor visibility and wide/continuation-cell semantics. Prevent double-width glyphs from overwriting borders; handle combining text and malformed cell lengths within bounds. The host's alternate screen belongs to the TUI, independent of the child's alternate screen. Render malicious control characters as safe placeholders, even if a peer violates expected cell content. Never execute OSC clipboard, hyperlinks, title changes or arbitrary escape sequences from a child on the host.

Encode child input from key events using the authoritative supported terminal profile: UTF-8 text, Enter, Backspace, Delete, Tab, Escape, arrows, Home/End, PageUp/PageDown, function keys, Ctrl combinations and supported Alt combinations. Application cursor/keypad modes must affect encoding correctly. Write a tested key-to-byte table in the terminal guide. Do not depend on host terminal echo or send host escape sequences without decoding. Unsupported modifiers have a documented safe behavior.

Bracketed paste wraps once only when the child mode requests it; otherwise send literal admitted text. Cap paste before encoding/enqueueing, account for wrappers, reject an oversized paste without partial delivery, and do not log pasted content. Input sequence advances only according to the accepted lease contract. Uncertain input is never replayed automatically. On reconnect discard unsent keystrokes and invalidate old lease and sequence; request writer ownership explicitly. Mode updates, input and resize require a defined order so keys are not encoded against a stale resized/replaced screen.

Launch form chooses default shell or enabled definition, optional task launch context and validated working directory. Commands/arguments are not shell-concatenated. Launch does not claim. Keep a generation-scoped receipt per deliberate launch and prevent duplicate clicks. If the response is uncertain, query receipts/session state under the existing guarantees; after generation change do not reuse it as permission to spawn again.

Attach starts read-only. Entering terminal input explicitly acquires the writer lease; conflicts preserve read-only observation. Release input on leaving focus; detach and quit release attachments best-effort with bounded cleanup. Connection loss releases ownership server-side. Retain the actual exit status, distinguish exited/failed/terminated/lost and unknown exit code, and do not infer task completion. Termination is confirmed only after lifecycle observation, not after sending the request.

## 7. Synchronization, uncertainty and privacy

Model connecting, loading snapshot, current, stale, disconnected, reconnecting, incompatible and retryable states separately. Show stale data as stale, disable new state-dependent mutations while disconnected, preserve drafts and offer exit/retry. Use bounded backoff with an explicit manual retry after exhaustion. Do not create a reconnect storm or relaunch a daemon indefinitely. Missing workspace, wrong peer, authentication failure and version mismatch require explicit actionable handling, never relaxed authentication or TCP fallback.

Install a coherent workspace snapshot with its watermark, subscribe after that watermark and apply only ordered events from the correct workspace/subscription. Events are metadata invalidations, not full entity snapshots. Refresh affected entities with bounded coalescing; use a fresh complete snapshot when a gap or revision conflict demands it. Do not claim the screen is current merely because it received an event while its entity fetch remains pending. Reject responses from an old connection epoch.

For every mutation distinguish not sent, rejected, committed and unknown. Never auto-retry unknown task creation, progress, handover, claim, release, transition, termination or input. Refresh and present the actual state plus the still-uncertain operation. Matching narrative content is not proof of identity for append-only writes. Let the user review and deliberately submit a new operation only after an explicit duplication warning. Correlation IDs are not durable idempotency guarantees. Test disconnect before send, during write, after commit before response, and while closing a draft.

Daemon restart requires new session/attachment discovery. Persisted lost instances do not provide live terminal state or claim ownership. The TUI must show the active task blocked and historical claim closed, retain durable progress/handover, and require explicit reopen and new claim. Same-daemon client reconnect must preserve children and recover live display even after the raw ring has wrapped.

Outside terminal panes sanitize all untrusted names, paths, narratives and errors before display. Escape ESC/C0/C1 and unsafe bidirectional controls, expand allowed tabs/newlines only in designated narrative layout, and cap transformed output. Test titles, list rows, form errors and diagnostics independently. Monochrome displays use text/status indicators, selection markers and borders rather than color alone. Do not log drafts, frames, terminal cells, arguments, raw exception chains, environment values or native private paths. The diagnostic view shows allowlisted events, categories, opaque IDs and sequence only. Ordinary runtime state remains outside source control; no crash transcript is persisted.

## 8. Verification and acceptance evidence

### 8.1 Layered tests

Pure reducer tests cover every screen/action, focus and modal routing, stable selections, async completion invalidation, draft preservation, all task transitions, state freshness and delivery outcomes. Ratatui buffer tests cover 100x30, 80x24, subminimum, monochrome, empty/error/loading, long text, wide/combining Unicode and control injection. Do not rely only on golden snapshots that mirror implementation.

Client/protocol tests reproduce idle subscription starvation, interleaved terminal/control/event frames, partial-read cancellation, event gaps, concurrent edits, snapshot chunk corruption/overflow/expiry, generation mismatch, resize during transfer, slow consumers and unknown capabilities. Continuation-equivalence tests attach at every byte split of representative UTF-8/CSI/OSC sequences, continue output and compare displayed cells/cursor/modes with the authoritative server. Include alternate-buffer restore after ring truncation and avoid assuming every Windows terminal reports the same alternate flag.

Use a native outer PTY/console harness to drive the actual `rt` executable and inspect its rendered screen and terminal modes. Test executables may provide deterministic full-screen children and fault injection through test-only composition. They cannot bypass the real TUI with reducer calls or replace the production daemon with a synthetic lifecycle host. Use real SQLite and native IPC, isolated temporary homes and roots with spaces/Unicode, bounded output capture in temporary storage, monotonic deadlines and cleanup of only owned test processes.

### 8.2 Mandatory end-to-end journeys

1. From a terminal run no-command `rt`, initialize a fresh workspace via UI, reach overview, create a task with every field, mark ready and launch a real shell.
2. Launch two neutral configured interactive fixtures through the UI. Observe three concurrent real sessions, switch between them and verify independent content and input targets.
3. Claim with the first real instance. A second TUI tries the competing claim and sees a contextual error without losing its draft. Append progress and verify history from the other client.
4. Prepare a valid structured handover; an invalid attempt first leaves state unchanged. Verify the handover, handover_ready and closed claim atomically. Claim with the second instance, read full context and complete explicitly.
5. Exercise release, blocked-to-ready, new claim, cancellation, final-task historical annotation, disabled/missing agent launch and natural nonzero exit. Confirm launch context never substitutes for a claim.
6. In a real full-screen child test ordinary and application-mode keys, Ctrl-C, literal Escape, Unicode, bracketed paste, focus escape, read-only competing attachment, resize and scrollback truncation. Compare display after switching away/back and after reconnect with ongoing partial-sequence output.
7. Close the actual TUI normally, reopen from another process and recover all three live sessions. Separately destroy the outer controlling terminal/console and reconnect independently. Prove child survival by fresh input/output and stable instance IDs, not only daemon liveness or a shell-exit assertion.
8. Disconnect IPC during editing and after an admitted mutation but before its response. Preserve drafts, show uncertainty, resynchronize and prove no automatic duplicate write/input or wrong-session completion.
9. Kill/restart the daemon in an isolated recovery scenario with explicit owned-process cleanup. Show honest lost sessions and blocked task, preserve durable context, then explicitly launch/reopen/claim. Never claim process adoption or terminal persistence across daemon restart.
10. Exercise ordinary exit, startup failure, render error and catchable panic; verify outer terminal mode/cursor and subsequent typed input. Verify noninteractive commands produce no terminal controls or bootstrap side effects.

### 8.3 Native matrix, performance and reports

Run the native TUI gate twice consecutively on ubuntu-24.04/stable, macos-14/stable and windows-2022/stable. Retain the pinned Linux toolchain and ordinary workspace/core suites. Each invocation must propagate its own failure, including PowerShell `$LASTEXITCODE`; separate CI steps are preferred. Do not call retries "two passes" unless two final candidate invocations actually pass. Review individual logs, counts and exit codes before trusting green job status.

Automated native PTY/ConPTY evidence is mandatory and cannot be replaced by mock rendering, compilation or cross-compilation. Record actual host, compiler, terminal harness, shell, candidate SHA, run URLs, repetition counts and limitations. Attempt local xterm-compatible terminal, macOS Terminal.app, Windows Terminal and authorized SSH where available; clearly separate named-emulator/manual/SSH evidence from automated native evidence. Missing optional access remains recorded for M11, but missing native three-platform TUI acceptance blocks M08 merge. Do not claim a named emulator was tested from a hosted console harness.

Measure connection-to-first-usable-screen, excluding new daemon startup, on an already running local daemon with a documented representative workspace (100 tasks, 3 live sessions and 100 history entries). Run 20 samples after one warm-up, report median, p95, maximum and input method; target p95 <= 2 seconds on the reference environment. Separately measure cold bootstrap and large bounded workspace degradation without including them in the two-second metric. A spinner alone is not an interactive workspace.

Under one noisy session producing at least 1 MiB/s of synthetic output, a quiet interactive session and a third live session, measure 100 navigation responses and 100 echoed child inputs. Target p95 <= 100 ms for local navigation and <= 250 ms for input-to-rendered-echo on the reference native environment. Report actual throughput, redraw rate, queue high-water marks, memory bounds and measurement overhead. These are M08 operational test targets, not universal hardware guarantees; a miss requires investigation and a documented resolution before sign-off, not changing thresholds silently. Run the load long enough to wrap retention at least twice and demonstrate other sessions and control operations progress.

### 8.4 Required commands

During iteration run the narrow affected test targets. Before closing run all of:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p relayterm-tui --locked
cargo test -p relayterm-client -p relayterm-terminal -p relayterm-protocol --locked
cargo test -p relayterm-daemon --test pty_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test durable_slice --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test tui_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny --locked check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner .
git diff --check
```

`tui_gate` is a new required integration target to create during implementation. Name any additional focused targets in the evidence guide. Run both Quality and Security, retain existing negative controls, and never bypass a failing check by excluding a test or weakening assertions. Documentation-only planning does not need to claim these runtime checks were executed.

## 9. Atomic queue and implementation order

Add the following 36 child tasks beneath their existing M08 parents in TODO before coding. Each task closes only with its stated evidence and an append-only logbook entry in the same change. Parent closure requires every child and all parent acceptance conditions.

| ID | Work | Required evidence |
| --- | --- | --- |
| M08.01a | Finalize screen/action/focus matrix, small layout and coordination mapping from sections 4-5 | Reviewed UI contract and complete action-to-operation table |
| M08.01b | Reproduce inherited client starvation, stale edits and terminal continuity/size/history risks | Focused failing regression cases or explicit negative findings per risk |
| M08.01c | Decide bounded architecture, display protocol and dependency versions; update ADRs | Capability examples, memory accounting and architectural graph checks |
| M08.02a | Extract shared bootstrap composition and no-command entry without changing admin output | Existing CLI regression suite plus option-routing tests |
| M08.02b | Implement workspace choose/open/init and failure flows | Real first launch/open and missing/invalid root tests |
| M08.02c | Reject unsupported/noninteractive terminal modes before side effects | Piped input/output, JSON, help/version and native path tests |
| M08.03a | Implement idempotent terminal lifecycle and independent event reader | Failure at each acquisition step and cleanup unit tests |
| M08.03b | Restore terminal on errors/panic/signals and bound teardown | Native outer-terminal usability after tested exit paths |
| M08.04a | Implement reducer, typed effects and bounded fair scheduler | Stale completion, queue limit, cancellation and idle-subscription regressions |
| M08.04b | Build overview/tasks/agents/instances from coherent snapshots | Loading/empty/stale and stable-selection rendering tests |
| M08.04c | Apply ordered event invalidations and bounded refresh | Snapshot/subscription race, gap, rapid-event and paging tests |
| M08.05a | Implement task create/edit fields and byte-aware validation | Boundary/Unicode/path/dependency tests and real IPC submission |
| M08.05b | Implement state, claim/release and claim-history actions | All allowed/rejected paths, real two-client claim race and attribution |
| M08.05c | Protect drafts and stale edits with verified server semantics | Two-client edit conflict and no silent overwrite regression |
| M08.06a | Implement task detail and bounded ordered history navigation | Page boundaries, retained reading position and every content field |
| M08.06b | Implement progress and structured handover forms | Invalid no-effect submission, atomic valid handover, explicit verification |
| M08.06c | Implement contextual resume and historical annotations | Two actual instances continue and final task remains closed |
| M08.07a | Implement validated launch/receipt, session tabs and lifecycle reporting | Shell/configured agent, duplicate launch, disabled/missing command and exit tests |
| M08.07b | Complete authoritative bounded display transfer and parsed history | Chunk boundaries, malformed/gapped transfers, maximum screen and scrollback tests |
| M08.07c | Render neutral terminal cells, cursor and modes | Split-sequence continuation, alternate restoration and native full-screen equality |
| M08.07d | Implement key/paste encoder, input leases and focus escape | Byte table, application modes, competing writer, oversized paste and no replay |
| M08.07e | Implement resize, read-only attach, detach, switch and confirmed termination | Resize ordering, clip behavior, stable child IDs and explicit confirmation tests |
| M08.08a | Implement connection state/backoff and generation-safe refresh | Same-daemon and replacement-daemon recovery with fresh attachments |
| M08.08b | Implement pending mutation uncertainty and draft preservation | Before-send/after-send/after-commit disconnects without duplicate effects |
| M08.08c | Verify reconnect under terminal load and unrecoverable session loss | Truncation recovery, no old input replay and explicit reopen/new claim |
| M08.09a | Implement discoverable keyboard help and complete compact navigation | Keyboard-only reachability at 80x24 and safe subminimum behavior |
| M08.09b | Implement safe text and bounded redacted diagnostics | Control/bidi/secret canaries, monochrome and Unicode tests |
| M08.09c | Publish keyboard/profile/lifecycle/privacy guide and update public status | Exact behavior-to-document cross-review, no unsupported claims |
| M08.10a | Create actual rt native outer-terminal integration harness | Native terminal input/render observation and bounded owned cleanup |
| M08.10b | Automate complete coordination and three-session TUI journeys | Section 8.2 account-free gates through real TUI, IPC and SQLite |
| M08.10c | Verify normal close, terminal destruction and independent reattachment | Fresh child input/output after actual client terminal loss on each OS |
| M08.10d | Make CI repetitions independently fail and collect native evidence | Two passing TUI gates per native OS and exact logs for inherited gates |
| M08.10e | Measure startup, latency, fairness and resource bounds | Reproducible section 8.3 samples and reported limits |
| M08.10f | Record available named-terminal/SSH checks and explicit gaps | Honest evidence matrix without equating harness to emulator coverage |
| M08.10g | Run full verification, audits and candidate documentation review | Section 8.4 checks, Quality and Security on final candidate |
| M08.10h | Deliver PR, required reviews, merge and synchronized main | Merge SHA, matching local/remote main and post-merge CI |

Execution order: M08.01, bootstrap/lifecycle, state and coordination, terminal prerequisite/display/input, reconnect/security, native journey/performance and final evidence. Terminal protocol experiments may precede form work; freeze their compatibility contract before UI integration. Do not wait until the end to try Windows input and ConPTY restoration.

## 10. Delivery and stopping conditions

Before editing, inspect branch/status, verify this document's PR is merged, fetch and synchronize main without discarding work, then create `feature/m08-tui` or continue an appropriate existing implementation branch. Keep documentation and comments in English; user communication in Spanish. No force-push, branch deletion, hidden stashes or inclusion of unrelated local edits.

Update README, specification section 6.6, the PTY/daemon guides, platform evidence and architecture documentation to describe actual implemented behavior. Publish `docs/tui-workflow.md` with reproducible setup, key map, input mode table, all journeys, test commands, measurement procedure and limitations. Update protocol/terminal ADRs for additions and compatibility, without changing prior logbook entries. Add only necessary configuration and no automatic source-file rewriting.

Create coherent commits, push and open a PR describing the concrete workflow, prerequisite fixes, protocol/dependency changes and validation. Monitor Quality and Security and resolve related failures. Confirm required reviews and branch protection through GitHub; do not bypass them. Merge only when all mandatory native evidence and tests pass. Synchronize local main to origin/main, verify exact SHA equality, and monitor post-merge CI. Correct in-scope regressions through the same protected workflow.

Keep M08 open if required terminal correctness, terminal restoration, three-platform native evidence, data privacy, uncertain mutation safety or review/access requirements remain unmet. Report the exact blocking condition and retain its TODO tasks. Do not substitute a simulation, a green step with a failed internal command, a rendering screenshot or historical M07 evidence for the new acceptance gate. Missing optional SSH/named-emulator access must be explicitly recorded without representing it as completed M11 validation.

Final implementation report must identify PR, merge commit, exact checks, native runs and repetitions, performance measurements, post-merge CI and remaining limitations. Then stop before M09 or M10.
