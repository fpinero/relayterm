# M07: Real PTY supervision and reattachment

## 1. Delivery and execution rules

This is the execution contract for the ten pending M07 tasks in [TODO](../TODO.md). It plans implementation; it establishes no new runtime evidence. The planning delivery changes documentation only. Do not close M07.01 or another implementation task when merging this document.

Implement on `feature/m07-pty` created from synchronized main, or continue an existing appropriate implementation branch without overwriting work. Read AGENTS.md, PROJECT_VISION.md, MVP_TECHNICAL_SPEC.md, README.md, this document, and recent avances.md entries first. Add the atomic tasks in section 10 to TODO before changing code. Remove each only after its stated evidence passes, appending that evidence to avances.md in the same change. Preserve every earlier logbook entry.

M07 is complete only when three independently supervised real interactive sessions, one shell and two neutral fixture agents, pass the native gate on Linux, macOS, and Windows. Client disconnection must preserve the children while the daemon and host remain alive. Correct full-screen reconstruction and bounded resources are mandatory.

Excluded: M08 TUI, provider templates from M09, worktree operations from M10, persistent terminal recordings, session migration, host-reboot survival, synthetic production sessions, and provider-output interpretation. A test executable running real OS processes is appropriate; fabricated production `running` observations are not.

## 2. Baseline and prerequisite review

Planning baseline: main after M06, commit `74eb386a6237b03287a0388d32f4fe2261f97bd0`. Re-read the actual implementation before applying this plan.

| Existing surface | M07 responsibility |
| --- | --- |
| `crates/relayterm-daemon` runtime and dispatcher | Preserve listener ownership before recovery; compose the real supervisor and implement reserved session operations |
| `crates/relayterm-protocol/src/message.rs` | Replace unavailable terminal capabilities with a versioned, bounded stream contract |
| `crates/relayterm-client` | Add typed session requests and streaming/resynchronization support |
| `crates/relayterm-platform` | Reuse native launch/lifetime primitives where appropriate; do not confuse daemon detachment with PTY child supervision |
| Domain/application instance observations | Preserve the single instance/session lifecycle and atomic final observation, claim closure, and task blocking |
| SQLite adapter | Persist lifecycle metadata and existing events, never terminal bytes or environment values |
| `crates/relayterm-cli/tests/architecture.rs` | Extend the explicit edge allowlist narrowly and retain negative controls |
| M06 durable-slice and console-lifetime tests | Keep passing as regressions; add actual PTY child lifetime evidence |

Inspect the complete admission, startup, shutdown, snapshot, and connection paths before writing the supervisor. Specifically reproduce or disprove: recovery before exclusive ownership; unbounded reads hidden behind paginated interfaces; blocking startup writes outside the deadline; timeout reuse instead of remaining-deadline accounting; pipe readers retained indefinitely by descendants; event and terminal queues sharing an unbounded producer. Correct reproduced defects required for M07, with regression tests. Record dismissed risks with the exact path and evidence, without claiming a general audit of unrelated milestones.

M06 console evidence establishes daemon lifetime below a Unix pseudo-terminal or a new Windows console. It does not establish real PTY child lifetime, GUI terminal behavior, SSH reconnection, or full-screen reconstruction. Keep these distinctions in the evidence guide.

## 3. Architecture and dependency gate

Introduce a narrow `relayterm-pty` adapter for native PTY handles and child control, and a provider-neutral `relayterm-terminal` component for bounded terminal state. A different module arrangement is acceptable only if the ADR explains equivalent boundaries and architecture tests enforce them. Daemon owns supervision, application owns durable coordination, protocol owns wire representations, and clients own presentation. Domain/application/protocol must not acquire concrete PTY, UI, OS process, Tokio, or SQLite dependencies.

The adapter exposes typed spawn, read, write, resize, termination, and wait results. It does not commit tasks or generate durable events. Terminal state accepts bytes and size changes and produces bounded state updates without provider knowledge or Ratatui types. Daemon translates observed process facts into application services. Extend only the necessary Cargo edges, including dev and target-specific edges; do not hide forbidden dependencies through reexports.

Prototype `portable-pty` and `vt100` first, as required by [ADR 0003](decisions/0003-terminal-state.md). Candidate documentation is [portable-pty 0.9.0 CommandBuilder](https://docs.rs/portable-pty/0.9.0/portable_pty/cmdbuilder/struct.CommandBuilder.html) and [vt100](https://docs.rs/vt100/latest/vt100/). These are investigation inputs, not approved dependency selections. Read their source and validate actual APIs, supported modes, Windows behavior, licenses, advisory status, and the repository's pinned MSRV before selecting exact compatible versions and updating Cargo.lock. Preserve the repository prohibition on local unsafe code. Prefer audited safe wrappers; do not silently relax lint or dependency policy to implement process groups or Job Objects.

Blocking PTY reads, writes, and process waits must not occupy Tokio executor threads. Define the ownership of each handle and bounded worker, how cancellation interrupts I/O, and how workers are joined. `spawn_blocking` alone does not make an uninterruptible read cancellable. Demonstrate finite teardown on every OS, including ConPTY output drainage and children retaining handles.

M07.01 is a prerequisite gate: commit the prototype tests and ADR evidence before relying on the chosen production design. If a candidate cannot pass reconstruction or native cleanup, amend the ADR with an alternative and repeat the gate. Do not replace a failing requirement with a smaller undocumented terminal subset.

## 4. Launch, lifecycle, and ownership

### 4.1 Launch validation

Support the existing generic shell and configured-definition launch kinds. Configured launches use the accepted immutable definition snapshot, not later edits. Disabled or missing definitions fail before spawn. Validate workspace membership, dimensions, capacity, optional task reference, working directory, argument limits, and authorization before reserving resources.

Resolve executables using the daemon's approved environment and native executable rules. Test absolute paths, PATH lookup, missing files, permissions, spaces, and Unicode. Never concatenate arguments into shell syntax. On Windows, batch scripts require an explicitly selected command interpreter and an audited argument contract; do not silently route arbitrary commands through `cmd /c`. Shell launch selects an available native shell using documented platform rules and returns a typed error when unavailable.

Canonicalize working directories through existing platform validation, reject missing/non-directory paths and escapes from the authorized workspace root, and preserve native path representations. Worktree references remain unavailable until M10. Document that lexical/canonical checks do not provide an OS sandbox or eliminate filesystem races.

Build environments by clearing inheritance and applying precisely [ADR 0005](decisions/0005-environment-privacy.md)'s platform baseline and explicitly allowed names. Resolve values from the daemon launch environment at child spawn; later client environment changes do not implicitly reconfigure the daemon. Handle Windows names case-insensitively. Derive truthful TERM and dimensions from the supported emulator profile. Test that an unapproved synthetic variable is absent, an approved one is present, and neither value reaches persistence, events, diagnostics, or Debug output. Do not expand the baseline silently or query credential stores.

### 4.2 Spawn transaction boundary

OS spawn and SQLite cannot share an atomic transaction. Implement an explicit launch state machine:

1. Validate and reserve a bounded supervisor slot.
2. Persist a `starting` instance with distinct session ID, accepted launch metadata, and a correlation identifier through application services.
3. Allocate the PTY and spawn exactly once for this admitted operation, outside an open database transaction.
4. Retain owned handles before reporting success. Persist `running` only after successful spawn and handle registration.
5. On allocation/spawn failure, persist `failed` with no invented exit code, release the reservation, and reap any partially created child.
6. If persistence fails after spawn, retain ownership, terminate and reap the child, and reconcile the nonfinal record. Do not abandon a running process because a request disconnected.
7. If the child exits immediately, serialize observations so a final state cannot be overwritten by a late running notification.

Return identifiers and operation status sufficient to query an uncertain response. A disconnected or timed-out create must not be blindly retried. Define a bounded, connection-independent operation receipt or idempotency key for launch admission; identical retries return the existing outcome while conflicting reuse fails. Retention and restart behavior must be explicit: after a daemon generation change, query durable IDs rather than assume an expired receipt permits another spawn. No claim of arbitrary exactly-once delivery across crashes is allowed.

A task launch context is not a claim. Launching must not implicitly activate a task. Claim acquisition remains explicit and requires an actually running instance. Attach does not confer task ownership. User attribution and neutral definition snapshots remain intact.

### 4.3 Exit, termination, and recovery

Use the existing lifecycle table. An observed normal wait result, including a nonzero exit code, is `exited`. A confirmed requested termination is `terminated`. Spawn/supervision failure is `failed` when its outcome is known; unreconstructable outcomes are `lost`. Unknown codes are absent, not zero. Final observations are idempotent only when identical.

Final lifecycle persistence, claim closure, and blocking an active owned task remain atomic. Never complete a task from terminal prose or process exit. Instances without claims must not affect tasks. Persist no raw stderr, process command line, or terminal transcript as failure detail.

Distinguish interrupt input from termination. A user input byte such as Ctrl-C is terminal input interpreted by the child's terminal settings. Explicit termination targets the owned process tree or session using the verified platform contract. Do not assume portable-pty's kill handle kills descendants. Prototype Unix process-group ownership and Windows Job Object/ConPTY behavior. Never signal a reused bare PID or an unrelated group. Document graceful request, bounded grace interval, escalation, drain deadline, and reap. A request alone must not mark `terminated`.

A disconnected client releases its input lease and subscriptions, not the child. Ordinary daemon stop must refuse with a typed busy result while live sessions exist. Add an explicit `--terminate-sessions` shutdown option for deliberate cleanup; stop admissions, finish admitted durable mutations, terminate owned sessions, persist final observations, then close storage and endpoint ownership. OS termination signals use the same bounded cleanup path where deliverable. Uncatchable daemon death cannot promise successful cleanup; startup marks unrecoverable nonfinal instances lost and never adopts a PID solely from storage. Test this honestly and document any platform-specific orphan cleanup limitation. Host-restart live-session recovery is excluded.

## 5. Terminal state and reconstruction

### 5.1 Authoritative state

Maintain daemon-owned screen state and bounded scrollback even with no attached clients. Output is opaque binary at the PTY boundary. UTF-8 decoding belongs to the terminal parser, must survive split characters, and must define invalid-byte handling. ConPTY can transform terminal output; assert the documented semantic behavior on Windows rather than require Unix byte identity.

A ring of bytes is not a reconstructable terminal. Snapshot state must cover both relevant screen buffers, cursor and saved cursor, attributes, wrap state, scrolling margins, tab stops, character width/combining behavior, and supported input/display modes. Retain parser continuation state for incomplete UTF-8 and escape sequences, or choose a server-parsed state-delta protocol that never requires clients to recreate hidden parser state. A formatted screen dump alone is insufficient unless continuation-equivalence tests establish the entire required contract.

The recommended wire representation is a versioned neutral screen snapshot plus ordered server-parsed screen deltas. Raw output may be exposed as a separately identified diagnostic stream, but must not become the only source of reattachment correctness. This avoids reconstructing a second parser's hidden state in every client. Document exact cell, attribute, mode, scrollback, and delta schemas in the implementation ADR before coding clients. M08 must consume these types without depending on the parser implementation.

Define the supported terminal profile through tested behavior: alternate-screen entry/exit, cursor addressing, erase, scrolling, SGR colors/attributes, saved cursor, wide/combining Unicode, bracketed paste, and resize. Test representative full-screen sequences and continuation after reconnect. Unknown sequences have bounded parsing and deterministic ignore behavior. Limit unterminated OSC/DCS payloads and excessive combining characters. Terminal parsing must never infer task changes or execute clipboard, hyperlink opening, shell commands, or other host side effects. Queries needing terminal replies are answered once by the daemon's emulator through the serialized input writer, never again by reconnecting clients.

### 5.2 Snapshot/live ordering

Use a daemon generation, session ID, attachment/stream ID, and monotonically increasing state revision. Keep raw byte offsets distinct from state revisions and durable workspace event sequences. Checked arithmetic must reject overflow; do not wrap counters or reuse a stream identity.

Attach reserves a bounded consumer and captures a snapshot at revision R together with its matching dimensions and retained history boundaries. Buffer only the bounded continuation after R while the snapshot is transferred. Deliver snapshot begin, bounded chunks, snapshot end, and contiguous deltas after R. A client applies the snapshot only when complete and valid. Interleaved resize participates in the same revision order. No bytes or deltas may be omitted or applied twice.

If the continuation overflows, invalidate that attachment generation and require a fresh snapshot. Signal history truncation explicitly, including the retained boundary. Never silently resume from the oldest raw ring byte. Cap repeated resnapshot work for a consumer that cannot keep up and disconnect that consumer with a typed reason. Other sessions and the child continue.

Detach cancels only that attachment. Final sessions may expose retained state until bounded retention eviction; after eviction return a precise terminal-state-unavailable result while durable lifecycle queries still work. Do not persist a screen to make restart attach appear successful.

### 5.3 Input and resize ownership

Allow multiple read-only observers and exactly one input lease per live session. Attach is read-only by default; acquiring input is an explicit operation on the authenticated connection. A second owner receives a conflict. Do not steal automatically. Disconnect, explicit release, and session finalization release the lease. A new owner receives a new lease generation; stale frames fail without writing bytes.

Only the lease owner may submit input or resize. Other observers do not resize the process when their windows change. Explicit termination is a local-user administrative operation, independent of the input lease. Route by workspace, session, connection, stream, and lease identity; a valid identifier alone is not sufficient authorization.

Input ordering uses a per-lease sequence or byte offset, with explicit acceptance semantics. Reject duplicates, gaps, unknown streams, final sessions, oversized payloads, and unauthorized leases before enqueueing. An acknowledgment means accepted into a bounded writer queue, not interpreted by the child. Report uncertain writes without replaying input after reconnect. Serialize emulator replies with user input. Apply resize to the PTY and terminal model coherently and expose a new revision only with a known outcome; failed resize cannot claim success.

## 6. Wire and administrative client contract

The baseline advertises protocol version 1 and terminal unavailable, with reserved create/attach/input/resize/terminate operations and an existing binary terminal frame. Inspect their exact schemas and limits. Because this plan changes attach and stream semantics, introduce protocol version 2 unless a documented compatibility proof shows all changes are additive and safely negotiated. Reject incompatible peers during hello with the supported version; update compatibility tests and specification deliberately. Event payload and database schema versions remain independent.

Define explicit typed requests/results for create, attach, detach, input lease acquire/release, input, resize, terminate, and existing list/query operations. Add terminal capability and negotiated limits only when production supervision is available. Keep JSON metadata and binary payload framing bounded before allocation. Existing decimal counters exclude zero; either start new revision counters at one or introduce an explicitly validated zero-capable offset type. Do not misuse the existing type for a zero initial byte offset.

Retain transport authentication, workspace isolation, request correlation, framing/depth limits, durable subscription replay, and control priority. Binary input is accepted only after stream authorization. Screen chunks are not durable workspace events. Unknown versions, malformed cells/deltas, oversized lengths, stale generations, and unexpected frame kinds fail closed with safe errors. Add fixtures covering old-client/new-server and new-client/old-server rejection.

Expose typed client APIs and narrow administrative `rt session` commands for create, list/status, resize, terminate, and bounded terminal diagnostics. Input must accept a bounded file/stdin byte source, not command-line text that leaks through process listings. Do not log its contents. An attach diagnostic can report safe snapshot metadata or a bounded explicit output destination selected by the caller; do not dump untrusted terminal control sequences into ordinary logs. Full interactive TUI rendering and keyboard focus remain M08. Exercise interactive attach and ownership through a dedicated test client using the production client library and native IPC, with no production simulation flags.

Document every final CLI spelling, JSON result, error category, timeout, and exit code. Preserve existing commands and unknown-command behavior. Add help/privacy tests and tests that a malformed session request cannot affect coordination operations.

## 7. Resource and scheduling contract

Use explicit defaults and validated hard ceilings. The following initial budgets are contractual starting values; a prototype-supported adjustment must update this table, ADR, tests, and public configuration documentation together.

| Resource | Default | Hard ceiling or behavior |
| --- | --- | --- |
| Live sessions per workspace daemon | 8 | Configurable 3 to 32; reject before spawn |
| Dimensions | Existing launch defaults | Each axis 1 to 1,000, plus combined cell budget below |
| Screen cells per session | At most 160,000 across both buffers | Reject size before allocating; enforce global budget |
| Global screen cells | 1,280,000 | Maximum 5,120,000; checked reservations |
| Scrollback per session | 1 MiB and 10,000 lines | Maximum 8 MiB and 100,000 lines; first reached bound evicts |
| Global scrollback | 16 MiB | Maximum 64 MiB; explicit fair eviction |
| Raw output staging per session | 256 KiB | Bounded independently from retained history |
| Pending input per session | 64 KiB | Reject excess without partial admission |
| Queued terminal data per connection | 1 MiB | Invalidate/resynchronize or disconnect slow consumer |
| Snapshot bytes | 16 MiB | Incremental bounded chunks, never one giant wire frame |
| Incomplete control sequence | 8 KiB | Deterministic discard/reset policy |
| Combining scalars per cell | 16 | Deterministic bounded replacement policy |
| Attachments per session | 8 | Also bounded by existing daemon connection limit |
| Retained final screens | 8 for up to 5 minutes | Global budgets still apply; oldest final state evicted first |

Bound object overhead as well as payload length. Snapshot construction must not secretly double unbounded parser allocations; reserve memory before copying or stream from a bounded immutable representation. A rows/columns-valid request can still exceed the cell budget and must fail explicitly. Output ingestion, history, pending snapshots, input, and connection queues require separate accounting.

Reserve control capacity independent of terminal data. Use bounded scheduling rounds, for example at most 64 KiB output per session per turn, then service control and other sessions. A blocked child stdin must not block resize, termination, other sessions, or SQLite work. A slow observer may lose its stream but may not impose global backpressure. Continued PTY drainage with parser/history eviction is preferred to allowing an absent client to stall a child. Resource exhaustion returns safe typed errors and releases reservations.

Use monotonic deadlines for all blocking tests and cleanup paths. Start with 5 seconds for ordinary fixture readiness/control, 2 seconds graceful termination, 3 seconds escalation/reap, and 30 seconds whole scenario. CI may use an explicit larger scenario deadline with a documented reason. Do not prove fairness using fragile submillisecond assertions. Instrument queue/cell counts in tests; combine deterministic bound assertions with process-level completion deadlines. RSS alone is not a proof of a hard bound.

## 8. Real fixture and required scenarios

Create a separately compiled test-only interactive child executable. It must use real PTYs and real OS behavior, require no credentials/network, and offer deterministic modes for echo, queried dimensions, full-screen redraw, alternate-buffer restoration, split escape/UTF-8 output, high output, blocked stdin, delayed exit, nonzero exit, crash, and a descendant retaining handles. Use a private test control channel or ordered markers for readiness; arbitrary sleeps are not synchronization. Bound harness output and clean up only owned resources on failure.

| Scenario | Required assertions |
| --- | --- |
| Launch validation | No child on invalid definition/path/env/size/capacity; snapshot unaffected by later definition edit |
| Spawn failure | Starting becomes failed, code absent, no false running record, no leaked reservation |
| Immediate exit and commit failure | Correct ordered observations; child owned and reaped after persistence failure |
| Real three-session gate | One native shell and two fixture agents concurrently accept target-specific input |
| Full-screen reconstruction | Reattached state equals uninterrupted state, then remains equal under additional output |
| Split/truncated data | Attach at every split of representative UTF-8/CSI/OSC sequences; correct state after ring eviction |
| Resize race | Child reports native dimensions; snapshot/delta revision ordering matches resize outcome |
| Multiple clients | Two observers, one writer; stale lease/input rejected; disconnect transfers no task claim |
| Flood and stalled consumer | All counters bounded; other sessions and control complete; resync is explicit |
| Blocked stdin | Input queue bounded; terminate and unrelated requests still complete |
| Child termination | Confirmed final status, descendants handled per ADR, unrelated sentinel process survives |
| Claim lifecycle | Real instance claims, adds progress, prepares atomic handover; second real instance continues |
| Child loss | Active claim closes and task blocks atomically; repeated observation produces no duplicate effect |
| Console disconnect | Close actual owning console/PTY while daemon and three real children remain; independent client reconnects |
| Daemon stop/restart | Busy refusal; explicit cleanup; abrupt loss reconciled honestly; no raw terminal persistence |
| Security/privacy | Unauthorized streams and cross-workspace references rejected; synthetic canaries absent from database/events/logs |

For reconstruction compare cells, attributes, cursor, modes, both applicable buffers, history boundary, and future continuation, not just visible text. Test entering and leaving alternate screen on either side of attachment. Include CR/LF, wrapping at the last column, wide characters at the edge, combining marks, scrolling regions, saved cursor, and rapid resize. Exercise unknown/unterminated escape floods under the parser limits.

Native tests must run actual Unix PTYs on Linux/macOS and ConPTY on Windows. Cross-compilation and mock handles are supplementary only. Run the gate twice independently per native stable target, with fresh isolated workspaces, plus the ordinary parallel workspace suite. Ensure process fixtures cannot collide through shared paths or global environment mutation.

For console lifetime, keep the host/daemon alive, close the owner of the originating terminal, confirm the same child identities remain alive, then interact through an independent client. Killing only the initial shell or detaching a synthetic session is insufficient. Record what terminal mechanism was actually used. Exercise an SSH connection where an authorized SSH environment is available; do not add hosted infrastructure or credentials. If required terminal behavior cannot be established automatically, obtain the corresponding native manual evidence and keep its task open until available. Explicitly distinguish optional unavailable SSH coverage from missing mandatory native console/PTY evidence.

## 9. Verification and evidence

During implementation run focused adapter, parser, client, daemon, and CLI tests for each change. Add named integration targets such as `pty_gate` and `terminal_reconstruction`; their exact committed names must match the guide and CI. Preserve the M06 lifecycle/durable gates.

Before completion run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
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

Also execute the new native gate twice and the reconstruction/adversarial tests explicitly. An ignored test, zero matched tests, cross-build, synthetic lifecycle observation, or skipped platform is not a passing acceptance result. CI must retain Linux, macOS, Windows stable and the pinned MSRV job, Quality and Security, with bounded dedicated gate steps. Adjust job deadlines based on measured runtime without removing controls.

Publish `docs/pty-supervision.md` with reproducible commands, supported terminal behavior, limits, ownership, shutdown, and honest recovery semantics. Update platform evidence with candidate SHA, OS/architecture/toolchain, fixture and terminal mechanism, repeat count, command/run references, result, and limitations. Do not upload raw terminal captures, environment dumps, personal paths, or credentials. Safe aggregate assertions and synthetic fixture labels are sufficient.

Update ADR 0003 with measured design results, ADR 0005 only for deliberate reviewed environment changes, the relevant specification sections, README status, architecture docs, and protocol compatibility documentation. State that TUI acceptance remains M08. A failing native acceptance item keeps M07 open regardless of green unrelated checks.

## 10. Atomic implementation queue

Copy these child tasks into TODO before coding, retaining the existing ten parents until all their children and acceptance conditions pass. Each child is a coherent implementation and verification unit, not a checkbox to remove after writing code.

| ID | Deliverable and required verification |
| --- | --- |
| M07.01a | Audit baseline launch, admission, reads, deadlines and cleanup; reproduce/disprove inherited risks and add regressions for fixes |
| M07.01b | Native portable-pty prototype on three targets; verify handle ownership, spawn, read/write, resize, wait and finite cleanup |
| M07.01c | vt100-first reconstruction prototype; compare snapshots plus continuation for all section 5 cases and resource boundaries |
| M07.01d | Amend ADR with dependency, terminal profile, wire state, leases, signals and overflow decisions backed by native results |
| M07.02a | Build real test-only child and bounded harness; verify deterministic modes and no production simulation entry |
| M07.02b | Verify crash, blocked input, descendant handles, teardown and concurrent isolation on each OS |
| M07.03a | Implement native shell/definition resolution and immutable launch snapshot; test missing/disabled/edited definitions |
| M07.03b | Implement cwd, argument and ADR environment assembly; test Unicode, spaces, escapes, case handling and privacy canaries |
| M07.04a | Add supervisor adapter composition and ownership state machine; enforce architecture tests and capacity reservations |
| M07.04b | Implement starting/spawn/running/failure saga and receipt semantics; inject commit failures and immediate exits |
| M07.04c | Implement wait/final observation and cleanup; verify atomic claims/tasks/events and duplicate observation handling |
| M07.05a | Add bounded binary ingestion and parser state; test invalid bytes, split sequences, parser limits and no persisted output |
| M07.05b | Add bounded scrollback, final retention and global accounting; verify exact boundaries and allocation reservations |
| M07.06a | Implement input lease and ordered bounded writer; test conflicts, stale identity, duplicates and blocked stdin |
| M07.06b | Implement coherent resize; test native child dimensions, limits, failure and output races |
| M07.06c | Implement verified native termination and descendant policy; assert confirmed state and unrelated child survival |
| M07.07a | Implement versioned snapshots/deltas and attachment epochs; test complete reconstruction and future continuation |
| M07.07b | Implement snapshot/live handoff, truncation and resync; force overflow and race attach with resize/output |
| M07.07c | Implement detach and observer/writer lifetime; test independent clients and unchanged task ownership |
| M07.08a | Enforce all per-session/global budgets and reservation rollback; test repeated rejected launches and final eviction |
| M07.08b | Implement fair output/control scheduling and slow-client policy; test flood, disconnected readers and blocked writers |
| M07.08c | Bound workers and teardown; stress repeated create/attach/exit cycles and verify no retained handles or queues |
| M07.09a | Implement versioned session wire operations and client APIs; test malformed/stale/cross-workspace frames and compatibility |
| M07.09b | Add administrative CLI commands and safe diagnostics; test help, byte input bounds, results and privacy |
| M07.09c | Integrate busy/explicit shutdown and startup reconciliation; test real sessions, pending transactions and abrupt daemon loss |
| M07.09d | Exercise real-instance coordination continuity and child failure isolation through separate clients and SQLite |
| M07.10a | Commit named real three-session gate and reconstruction matrix, passing twice on each native stable target |
| M07.10b | Establish console-close child survival and independent reconnect, plus SSH where available; record precise evidence |
| M07.10c | Publish guide, compatibility/ADR/spec updates and native evidence; run all repository/security/architecture controls |
| M07.10d | Deliver reviewed PR only after acceptance passes; merge, synchronize main, and verify post-merge Quality/Security |

Recommended order: 01 and 02 prototypes together, then 03/04, 05, 06/07, 08, 09, 10. Do not defer parser correctness or Windows cleanup until after production APIs become fixed. Tests can use in-memory fault injection for rare boundaries, but the final gate must traverse real processes, production composition, SQLite, and native IPC.

## 11. Delivery and stop conditions

Create focused English commits and a PR describing the final behavior, protocol changes, native evidence, and limitations. Inspect required checks and reviews rather than assuming workflow success alone grants merge eligibility. Correct in-scope failures, rerun affected controls, and update evidence for the final candidate. Never force-push, discard existing changes, delete branches, bypass controls, or publish releases under this task.

Merge only after all mandatory criteria pass. Synchronize local main with origin/main and monitor post-merge Quality and Security. Record the PR, merge SHA, and CI results. If native access, mandatory manual evidence, dependency capability, or repository approval is unavailable, report the concrete blocker, retain affected TODO tasks, and leave the implementation PR unmerged. Do not reinterpret a difficult acceptance condition as optional.

The implementation handoff must state: M07 proves real daemon-owned terminal supervision and reconstruction while the host/daemon survive; M08 still owns the user-facing TUI, and restarting a daemon does not restore a lost terminal screen or revive a child process.
