# 0004: SQLite persistence and recovery

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.04.
Requirements: Sections 5.7, 6.5, 11; FR-1, FR-9; NFR-2, NFR-4.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Store one SQLite database per workspace in private application data. A separate private registry identifies workspaces. Relational state is authoritative; related state and event writes share one transaction. Use checked-in migrations, foreign keys, and initially WAL, verified in M03.

## Invariants and behavior

Separate user configuration, durable application data, runtime endpoints/locks, and disposable cache using OS directory conventions. Define an explicit override for isolated tests and portable development in M03. The project contains source and explicitly exported artifacts only; export is not implemented in the MVP roadmap.

The daemon serializes mutation use cases; database constraints still enforce identities, references, and claim exclusivity. Migrations are ordered and transactional where supported. Refuse newer schemas or unsupported downgrade. On corruption or migration failure, preserve originals and return safe recovery guidance, never silently recreate storage.

Back up with SQLite facilities such as the online backup API or VACUUM INTO, selected against the actual adapter in M03/M11. Never copy only an active database's main file. Restore initially with the daemon stopped into a fresh destination. Reconcile former live sessions as lost without inventing an exit code; related events commit atomically.

## Alternatives

Full event sourcing is unnecessary for the MVP. Project-local private state risks publication. Raw live-file copies can omit WAL content. Destructive automatic recovery can erase the only usable evidence.

## Consequences

Backups contain sensitive coordination data and require private storage. Migration and recovery code must handle both registry and workspace state without assuming cross-database atomicity.

## Verification and ownership

M03-M06 test write/reopen in separate processes, initial-schema migrations, claim races, failed transaction rollback, newer schemas, locked/corrupt databases, and idempotent reconciliation. M11 tests consistent backup and non-destructive restore.

Reference: [SQLite backup mechanisms](https://www.sqlite.org/backup.html).
