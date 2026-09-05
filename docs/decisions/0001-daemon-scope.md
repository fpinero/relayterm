# 0001: Daemon scope and lifetime

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.01.
Requirements: Sections 4.1, 6.1, 9.2, 11; FR-1, FR-4, FR-9.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

One daemon owns each workspace. Multiple clients share that daemon; another workspace has an independent failure boundary. The private user registry maps canonical roots to opaque workspace IDs. Serialize registration under a user-registry OS lock and enforce one identity per canonical root before attempting the per-workspace daemon lock. Canonicalization resolves symlinks; do not lowercase arbitrary paths or use display names as identity.

## Invariants and behavior

Hold a per-workspace OS lock for the daemon lifetime. Derive endpoint names from opaque IDs and restrict their parent directories. PID metadata is diagnostic only. The registry lock is short-lived and must never be held while waiting for a daemon to become ready. Only the per-workspace lock holder may validate and remove its stale endpoint. Ownership mismatch or an inaccessible endpoint fails closed. Startup readiness has a bounded timeout and returns actionable errors.

The launcher detaches the daemon using a narrow platform adapter; no client lifetime owns it. Explicit shutdown rejects new mutations, finishes or rolls back pending transactions, terminates supervised children with a bounded grace period, records observed results, and closes IPC. Unknown final child outcomes become lost on restart. A disconnect or incompatible client never triggers shutdown or replacement.

## Alternatives

One per-user daemon would reduce processes but couple unrelated project failures and require more internal isolation. A TUI-owned process would violate detach/reattach. PID-only locks permit PID reuse and startup races.

## Consequences

More processes and private registry coordination are accepted. Live child recovery after daemon/host restart remains excluded. Administrator-level interference is outside the same-user trust boundary.

## Verification and ownership

| Scenario | Required outcome |
| --- | --- |
| First open | Canonicalize, register once, acquire workspace lock, start, wait for readiness |
| Reconnect | Resolve registry identity and connect without replacing the daemon |
| Two starters | Registry identifies one workspace; only one workspace lock owner starts |
| Stale endpoint | Lock owner checks ownership before cleanup; client cannot unlink it |
| Two roots | Independent IDs, locks, endpoints, and child lifetimes |
| Incompatible version | Explain matching-version requirement without restart |

M03 tests registry uniqueness, M04 endpoint access, M05 simultaneous startup and detached lifetime, and M07 child shutdown/recovery. These are future empirical tests, not proven by this ADR.
