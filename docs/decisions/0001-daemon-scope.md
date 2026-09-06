# 0001: Daemon scope and lifetime

Status: Accepted architecture; M05 lifecycle implemented with native verification recorded separately.
Task: M01.01.
Requirements: Sections 4.1, 6.1, 9.2, 11; FR-1, FR-4, FR-9.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

One daemon owns each workspace. Multiple clients share that daemon; another workspace has an independent failure boundary. The private user registry maps canonical roots to opaque workspace IDs. Serialize registration under a user-registry OS lock and enforce one identity per canonical root before attempting the per-workspace daemon lock. Canonicalization resolves symlinks; do not lowercase arbitrary paths or use display names as identity.

## Invariants and behavior

Hold a per-workspace OS lock for the daemon lifetime. Derive endpoint names from opaque IDs and restrict their parent directories. PID metadata is diagnostic only. The registry lock is short-lived and must never be held while waiting for a daemon to become ready. Only the per-workspace lock holder may validate and remove its stale endpoint. Ownership mismatch or an inaccessible endpoint fails closed. Startup readiness has a bounded timeout and returns actionable errors.

The launcher detaches the daemon using a narrow platform adapter; no client lifetime owns it. Explicit shutdown rejects new mutations, finishes or rolls back pending transactions, terminates supervised children with a bounded grace period, records observed results, and closes IPC. Unknown final child outcomes become lost on restart. A disconnect or incompatible client never triggers shutdown or replacement.

Before a workspace endpoint exists, the public CLI invokes a finite hidden mode of the same executable over captured standard pipes. That daemon-side bootstrap component performs explicit initialization or non-creating lookup and starts the long-lived workspace runtime. A private per-workspace startup lock serializes endpoint probing, process creation, and readiness checks within the caller's deadline. The runtime receives only derived workspace identity and private-location inputs, closes standard streams, creates a new Unix session or detached Windows process group, and proves readiness through authenticated IPC. There is no persistent per-user manager or public bootstrap listener.

The platform adapter starts the absolute current executable with argument arrays and null standard streams. On Unix the runtime calls `setsid` before opening workspace resources. On Windows creation uses `DETACHED_PROCESS` and `CREATE_NEW_PROCESS_GROUP` without requesting unconditional job breakaway. These choices cover ordinary launcher and console closure while the user session and host remain available. They do not promise survival of host shutdown, logout policy, administrator termination, or a containing job configured to kill descendants.

Every runtime has an opaque generation. Status reports it, and shutdown must target it. The server acknowledges draining before closing admission, retains ownership of accepted application operations after client disconnect, joins connection tasks, and releases storage and endpoint ownership last. M05 production supervision is intentionally empty. M07 supplies real child handles and termination observations.

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

M03 tests registry uniqueness, M04 endpoint access, and M05 tests simultaneous startup, detached lifetime, generation-safe shutdown, durable drain, and idempotent lost-session reconciliation. M07 still owns real child shutdown and terminal recovery.
