# Relayterm

**A persistent, agent-neutral development workspace for the terminal.**

Relayterm is an open-source workspace where Claude Code, Codex, OpenCode, and future command-line agents can enter, leave, receive tasks, record progress, and hand work over without making the project dependent on any single provider.

The planned user-facing command is `rt`, designed for frequent use in local shells and over SSH.

> [!NOTE]
> Relayterm is in its initial design and implementation stage. The documents in this repository define the intended product and MVP, not an already released tool.

## Why Relayterm

Coding agents are powerful, but their context is often fragmented across terminals, providers, IDEs, and private chat histories. When one agent stops and another takes over, the new participant may not know what changed, what was verified, what remains, or which decisions matter.

Relayterm makes the workspace the system of record. Agents are interchangeable participants, and provider-specific tools remain replaceable adapters at the boundary.

The goal is dependable continuity:

- Run multiple coding agents in real terminal sessions.
- Keep tasks, claims, progress, and handovers in shared durable state.
- Disconnect and reconnect without losing the workspace.
- Resume work with a different agent without reconstructing context manually.
- Work locally or through SSH on Linux, macOS, and Windows.
- Preserve user control without requiring a hosted account or provider API keys.

## TUI-first, not TUI-only

Relayterm will begin with a terminal user interface because the target audience already works in shells, remote hosts, and SSH sessions. The TUI will be a client, not the product's architectural foundation.

The core daemon will independently own:

- Persistent workspace state.
- Task and handover workflows.
- Real pseudo-terminal sessions.
- Agent process supervision.
- Local inter-process communication.
- Optional Git worktree coordination.

This separation will allow future desktop, web, mobile, and automation clients to use the same domain model and versioned protocol.

## MVP direction

The initial MVP is designed for one local operating-system user and will focus on:

- A persistent workspace rooted in an existing software project.
- A background daemon with SQLite-backed state.
- A cross-platform TUI built with Rust, Ratatui, and Crossterm.
- Concurrent real terminal sessions managed through pseudo-terminals.
- Provider-neutral agent definitions, including templates for Claude Code, Codex, and OpenCode.
- Explicit task ownership, lifecycle states, progress entries, and structured handovers.
- Detach and reattach behavior while the daemon remains active.
- Optional Git worktrees for isolated parallel tasks.
- Safe public defaults, bounded diagnostics, and no telemetry by default.

The MVP will not provide autonomous agent swarms, remote multi-user hosting, cloud synchronization, graphical clients, provider API integrations, or a security sandbox for untrusted code.

## Guiding principles

- **Project ownership over agent ownership:** Work state belongs to the workspace, not to a provider or chat session.
- **Neutrality by design:** No agent receives privileged status in the domain model.
- **Real terminals:** Agent CLIs run inside real interactive pseudo-terminal sessions.
- **Local-first operation:** Core workflows do not require a hosted service.
- **Explicit coordination:** Claims, transitions, handovers, and conflicts remain visible and attributable.
- **Cross-platform support:** Linux, macOS, and Windows are first-class targets.
- **Privacy by default:** Runtime-private data stays outside the Git repository, and secrets are never intentionally captured.
- **Open and replaceable interfaces:** The daemon, clients, storage, and provider adapters remain separable.

## Technology direction

Relayterm is planned in Rust, using:

- Ratatui and Crossterm for the terminal client.
- `portable-pty` for cross-platform pseudo-terminal management.
- Tokio for asynchronous orchestration.
- SQLite for transactional local persistence.
- Git worktrees when task isolation is useful.

The canonical user-facing executable will be `rt`. The reserved `relayterm.com` domain is intended for a future landing page, but the local product will not depend on that website or any hosted Relayterm service.

## Documentation

- [Project vision and request for project](PROJECT_VISION.md) explains the problem, purpose, principles, scope, privacy posture, and long-term direction.
- [MVP technical specification](MVP_TECHNICAL_SPEC.md) defines the proposed architecture, domain model, requirements, implementation phases, exclusions, and acceptance criteria.
- [Daemon and administrative CLI](docs/daemon_cli.md) documents the implemented M05 lifecycle and script interface.

## Current status

Relayterm now has a deterministic coordination core, private SQLite persistence, a versioned local protocol, current-user IPC transports, an independent per-workspace daemon, and an administrative CLI. Integration tests exercise detached startup, two clients, ordered events, claims, handovers, restart reconciliation, coherent snapshot refresh, and uncertain mutation recovery. Real PTY sessions and the TUI remain unimplemented. See the [daemon and CLI guide](docs/daemon_cli.md), [M04 protocol plan](docs/M04_details.md), and [M03 storage contract](docs/M03_storage_contract.md) for implemented boundaries and later handoffs.

Build with the pinned Rust toolchain:

```sh
cargo fetch --locked
cargo build --workspace --locked
cargo run -p relayterm-cli --bin rt -- --help
cargo run -p relayterm-cli --bin rt -- --version
```

Running `rt` without a subcommand still reports that the TUI is unavailable and exits with code 1. Use `rt workspace init`, `rt daemon status`, and the administrative command groups before M08. Help/version exit successfully and create no runtime-private state.

See [contributing](CONTRIBUTING.md), [architecture decisions](docs/architecture/README.md), [privacy](docs/privacy.md), and [platform evidence](docs/supported-platforms.md). The [pending queue](TODO.md) distinguishes implemented foundations from the remaining daemon, PTY, TUI, agent-adapter, and worktree milestones.

Contributions and technical discussion are welcome, but interfaces and behavior should be considered unstable until the first working release and stable protocol are defined.

## License

Relayterm is licensed under the [Apache License 2.0](LICENSE).
