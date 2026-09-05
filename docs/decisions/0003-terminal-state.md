# 0003: Terminal state and reattachment

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.03.
Requirements: Sections 6.3, 8, 14; FR-3, FR-4.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

The daemon retains bounded terminal screen state and scrollback in memory while clients are absent. It does not persist terminal recordings. A truncated ring of raw bytes alone cannot establish correct full-screen reattachment.

## Invariants and behavior

Use a provider-neutral parser representation containing screen cells, cursor, relevant modes, and bounded scrollback independent of Ratatui types. Attach by taking a state snapshot at a stream position and continuing output after that position. Detect discontinuities and resynchronize instead of rendering silently incomplete state.

Terminal parsing must never mutate authoritative tasks based on prose or execute clipboard side effects. Bound screen dimensions, parser/history allocations, frames, and per-client queues. Client rendering is an adapter, not the source of surviving terminal state.

M07 prototypes vt100 first and records correctness evidence before dependency selection. That gate also fixes input ownership, resize authority, stream format, overflow behavior, and supported escape-sequence behavior. A failing prototype requires an ADR amendment before production implementation.

## Alternatives

Client-only state disappears on detach. A raw ring can begin inside an escape sequence or omit alternate-screen initialization. Provider-specific transcript parsers violate neutrality.

## Consequences

Terminal parsing introduces compatibility and resource-limit work. No promise of arbitrary terminal emulation or host-restart session survival is made. Disk scrollback remains excluded.

## Verification and ownership

M07 must test alternate-screen applications, cursor movement, split escapes, Unicode, resize, truncation, attach while output is flowing, multiple clients, and high-volume output. Compare reconstructed screen/modes with uninterrupted state.

Reference: [vt100 parser documentation](https://docs.rs/vt100/latest/vt100/).
