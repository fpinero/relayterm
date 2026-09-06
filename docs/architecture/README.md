# Architecture decisions

Relayterm keeps domain and application logic independent from adapters. The bootstrap created six core and composition crates. M03 adds platform, configuration, and SQLite adapter crates when their production behavior is implemented. Future empty adapters and fake use cases remain absent.

## Decision index

- [0001: Daemon scope and lifetime](../decisions/0001-daemon-scope.md), M01.01.
- [0002: Local IPC and synchronization](../decisions/0002-local-ipc.md), M01.02.
- [0003: Terminal state and reattachment](../decisions/0003-terminal-state.md), M01.03.
- [0004: SQLite persistence and recovery](../decisions/0004-sqlite-recovery.md), M01.04.
- [0005: Environment inheritance and privacy](../decisions/0005-environment-privacy.md), M01.05.
- [0006: Neutral agent configuration](../decisions/0006-agent-configuration.md), M01.06.
- [0007: Worktree ownership and recovery](../decisions/0007-worktree-ownership.md), M01.07.
- [0008: Rust, platforms, and dependency policy](../decisions/0008-rust-platforms.md), M01.08.

Each ADR records requirements, invariants, alternatives, consequences, and later empirical tests. Acceptance of an ADR is a design decision, not evidence that a future subsystem already works. The [M01 guide](../M01_details.md) defines the bootstrap gate.

## Dependency boundary

Domain, application, and protocol cannot depend on UI, SQLite, PTY, Git, or provider adapters. Daemon composes future services/adapters; TUI consumes the protocol; CLI selects separate runtime entries. The CLI integration suite inspects Cargo declarations and the resolved core graph, including target-specific dependencies, and tests synthetic violations.

M02 introduces domain state machines and use-case ports. M03-M06 prove durable coordination through SQLite and IPC. M07 adds the `relayterm-pty` native adapter and provider-neutral `relayterm-terminal` state component. M08 adds terminal presentation.

M03 preserves these exact project edges: application to domain; platform to application and domain; configuration to domain; SQLite persistence to application, domain, and platform. M04 adds IPC to platform and protocol, and client to IPC and protocol. M05 composes the daemon and CLI. M07 adds two leaf components: `relayterm-pty` depends only on external native PTY/process libraries, and `relayterm-terminal` depends only on the external parser and serialization. The daemon composes both. Domain, application, and protocol have no transitive adapter, SQLx, SQLite, Tokio, configuration-parser, PTY, parser, or UI dependency.

`relayterm-protocol` owns pure wire scalars, strict JSON DTOs, framing, errors, and limits. `relayterm-ipc` owns authenticated local streams and bounded output scheduling. `relayterm-client` owns correlation, explicit uncertain outcomes, staged snapshots, and event consumption. The daemon library maps DTOs to application services and owns bootstrap, composition, recovery, lifecycle, and bounded diagnostics. The CLI parses public commands and reaches durable state through daemon-owned bootstrap or authenticated workspace IPC.
