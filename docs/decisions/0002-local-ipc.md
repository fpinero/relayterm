# 0002: Local IPC and synchronization

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.02.
Requirements: Sections 6.2, 9.2, 9.5; NFR-2, NFR-4, NFR-8.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Use Unix domain sockets on Linux/macOS and local named pipes on Windows. Protocol version 1 is the initial compatibility boundary. Control requests, responses, and events use JSON; terminal bytes use a distinct binary frame kind.

## Invariants and behavior

Frames have an explicit length and kind. Reject unsupported versions and oversized lengths before allocating payload buffers. Every request has an ID; every workspace event has a sequence. M04 fixes the complete envelope, length encoding, limits, and idempotency policy before transport code is introduced. M01 publishes only a pure version check, not a stable wire API.

Take each relational snapshot with an event watermark from a consistent read. Subscribe after that watermark; if retained events no longer cover it, return an explicit resnapshot requirement. Bound event and terminal queues separately. A slow subscriber is resynchronized or disconnected without blocking other clients. Do not automatically retry mutations after uncertain response loss until duplicate-safe request semantics exist.

Restrict Unix directories to the owner and validate peer credentials when available. On Windows use an explicit owner-restricted DACL, reject remote pipe clients, and validate peer identity where supported. Do not use default permissive descriptors or any TCP fallback.

## Alternatives

TCP would expand the trust boundary. Newline-only text framing is unsuitable for opaque terminal bytes. Unbounded queues and blind retries can corrupt perceived state or exhaust memory.

## Consequences

Clients must handle reconnect and cursor expiry. IPC access protects against other ordinary users, not an administrator or malicious process with the same user permissions.

## Verification and ownership

M04: fragmented/coalesced frames, rejected peers, unknown operations, version mismatch, oversized/truncated input, concurrent claims, snapshot/subscription races, expired cursors, and slow versus fast clients. M07: terminal/control fairness. M01 tests every u16 version against version 1.

Reference: [Windows pipe access control](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights).
