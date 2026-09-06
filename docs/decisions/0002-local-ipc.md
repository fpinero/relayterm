# 0002: Local IPC and synchronization

Status: Accepted architecture; implemented by M04 with native verification recorded separately.
Task: M01.02, refined by M04.01.
Requirements: Sections 6.2, 9.2, 9.5; NFR-2, NFR-4, NFR-8.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Use Unix domain sockets on Linux/macOS and local named pipes on Windows. Protocol version 1 is the initial compatibility boundary. Control requests, responses, and events use JSON; terminal bytes use a distinct binary frame kind.

## Invariants and behavior

Frames use a seven-byte header containing a big-endian `u32` payload length, a big-endian `u16` protocol version, and a one-byte kind. Kind 1 carries strict UTF-8 JSON and kind 2 reserves opaque terminal bytes. Reject unsupported versions, kinds, invalid terminal metadata, and oversized lengths before allocating payload buffers. JSON frames are limited to 8 MiB and terminal data to 16 KiB plus its 32-byte metadata prefix.

The first request is `protocol.hello`, which binds the connection to the server's fixed workspace. Request IDs are nonzero decimal `u64` strings and increase on each connection. Control envelopes reject duplicate and unknown fields. Unknown operation names receive a correlated `unknown_operation` response without echoing the supplied value. Every durable workspace event has a strictly increasing sequence.

Take each relational snapshot page with its workspace revision and event watermarks. A client stages all collections against the same revision and installs them together. Revision movement restarts the complete refresh at most three times. Subscribe after the snapshot watermark; if retained events no longer cover it, return an explicit resnapshot requirement. Durable polling prevents a missed notifier from creating an unnoticed event gap. Bound event and terminal queues separately. A slow subscriber is disconnected without blocking other clients.

M04 has no durable request receipt table. A client never automatically retries a mutation after writing begins. A timeout, disconnect, or lost response after that point produces an explicit unknown result and requires an authoritative refresh before the user decides what to do.

Restrict Unix endpoint parents to mode `0700`, sockets to `0600`, and validate credentials symmetrically against the current UID. The endpoint lock remains held for the listener lifetime, and stale cleanup checks type, ownership, and socket identity. On Windows create a byte-mode pipe with a protected owner-only DACL, disabled remote access, non-inheritable handles, a bounded instance count, and first-instance collision protection. Do not use default permissive descriptors or any TCP fallback.

Public clients always act as `LocalUser`. They may request a claim for a concrete running instance while retaining user attribution. Public dispatch cannot select `System`, inject lifecycle observations, or fabricate an active process.

## Alternatives

TCP would expand the trust boundary. Newline-only text framing is unsuitable for opaque terminal bytes. Unbounded queues and blind retries can corrupt perceived state or exhaust memory.

## Consequences

Clients must handle reconnect and cursor expiry. IPC access protects against other ordinary users, not an administrator or malicious process with the same user permissions.

## Verification and ownership

M04: fragmented/coalesced frames, rejected peers, unknown operations, version mismatch, oversized/truncated input, concurrent claims, snapshot/subscription races, expired cursors, uncertain mutation recovery, slow-client isolation, and terminal/control queue fairness. M07 still owns real terminal stream behavior. M01 tests every `u16` version against version 1.

Reference: [Windows pipe access control](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights).
