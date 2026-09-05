# 0006: Neutral agent configuration

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.06.
Requirements: Sections 5.2, 6.4, 11; FR-2.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

TOML is the editable import format. Definitions have stable IDs, command plus argument arrays, declared capabilities, enabled state, and environment_allowlist names. Templates and custom commands use the same model. SQLite stores the accepted active definition.

## Invariants and behavior

Precedence is explicit: offer template import; validate user TOML; persist the accepted definition through the daemon. File edits do not automatically change active state. An explicit reload applies a validated candidate. TUI edits go through IPC and do not rewrite the source TOML automatically. A subsequent explicit reload may replace active fields and must present conflict semantics defined before M03 write implementation.

Resolve a validated executable path or search permitted PATH at launch. An existing instance retains its launch configuration; edits affect subsequent launches only. Disabled definitions retain identity and historical attribution but cannot launch. Unknown fields warn; security-sensitive invalid fields fail closed. Store variable names, never environment values. No provider-specific domain branches or automatic authentication.

## Alternatives

Automatic bidirectional file/database synchronization creates hidden conflicts. Provider types in the domain make custom adapters second-class. Resolving only at registration misses later missing executables.

## Consequences

Users must explicitly reload imports and understand which active definition will launch. Configuration edit conflicts need a documented M03 contract before mutation is implemented.

## Verification and ownership

M03/M09 test first import, file edit without reload, reload, concurrent edit conflict, disabled and missing commands, preserved running instances, and arbitrary custom CLI parity. M01 adds no templates or configuration writes.
