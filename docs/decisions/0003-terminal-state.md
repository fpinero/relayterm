# 0003: Terminal state and reattachment

Status: Accepted and implemented by M07; native evidence is tracked in the platform guide.
Task: M01.03.
Requirements: Sections 6.3, 8, 14; FR-3, FR-4.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

The daemon retains bounded terminal screen state and scrollback in memory while clients are absent. It does not persist terminal recordings. A truncated ring of raw bytes alone cannot establish correct full-screen reattachment.

## Invariants and behavior

Use a provider-neutral parser representation containing screen cells, cursor, relevant modes, and bounded scrollback independent of Ratatui types. Attach by taking a state snapshot at a stream position and continuing output after that position. Detect discontinuities and resynchronize instead of rendering silently incomplete state.

Terminal parsing must never mutate authoritative tasks based on prose or execute clipboard side effects. Bound screen dimensions, parser/history allocations, frames, and per-client queues. Client rendering is an adapter, not the source of surviving terminal state.

M07 selected `vt100` 0.16.2 after split-sequence and alternate-screen prototypes. The daemon keeps the parser authoritative and publishes versioned provider-neutral replacement snapshots containing cells, attributes, cursor, modes, revision, raw offset, and truncation boundary. Attach binds the snapshot to a daemon generation, connection attachment, and stream identity. Bounded raw continuations use terminal frames with exact offsets. Cursor expiry requires a replacement snapshot. Replacement snapshots preserve future correctness when retained raw bytes no longer include terminal initialization.

Attachments are read-only by default. One connection owns a generation-scoped input lease, and only that owner can send contiguous input sequences or resize. Disconnect and explicit release drop the lease without stopping the child. Snapshot responses and binary continuations have explicit frame budgets. Input uses a separate bounded writer queue so a blocked child cannot allocate unbounded memory.

The session operations activate previously unavailable, capability-negotiated protocol version 1 names. Their strict request bodies were not accepted by a version 1 production server before M07, so adding receipts, leases, attachment identities, snapshots, and output cursors does not reinterpret a formerly successful request. Older servers omit these operations from hello and reject them. New clients require the advertised operation set before use. A protocol version increase remains required if a successful published operation changes incompatibly.

## Alternatives

Client-only state disappears on detach. A raw ring can begin inside an escape sequence or omit alternate-screen initialization. Provider-specific transcript parsers violate neutrality.

## Consequences

Terminal parsing introduces compatibility and resource-limit work. No promise of arbitrary terminal emulation or host-restart session survival is made. Disk scrollback remains excluded.

## Verification and ownership

Committed parser tests compare cells, cursor, and modes at every split point of representative UTF-8 and CSI input. They verify alternate-screen restoration after raw-history truncation. The native `pty_gate` uses a shell and two test-only interactive children to verify input, resize, full-screen state, exclusive ownership, disconnect, reattachment, termination, and no persistent terminal capture. The console-lifetime gate reconnects to a real shell session after its originating console owner closes. CI runs these gates on Linux, macOS, and Windows.

Reference: [vt100 parser documentation](https://docs.rs/vt100/latest/vt100/).
