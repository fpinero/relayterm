# Relayterm project vision and request for project

## Document status

- Status: Draft for public review
- Audience: Contributors, maintainers, users, and integration authors
- License: Apache-2.0, see [LICENSE](LICENSE)
- Companion document: [MVP technical specification](MVP_TECHNICAL_SPEC.md)

## Product identity

The initial product name is **Relayterm**. The name reflects its central purpose: relaying development work, context, and responsibility between terminal-based agents without binding the project to any one provider.

The user-facing executable is planned as **`rt`**, a short command intended to be easy to type during frequent local and SSH workflows. Initial checks on Linux and macOS found no broadly established general-purpose system command named `rt`. Packaging and installation documentation must still detect and report local command conflicts because no short executable name can be guaranteed to be unused in every environment.

The **`relayterm.com`** domain has been reserved for a future public landing page. The website is not part of the MVP, and Relayterm's local operation must never depend on that domain or any hosted service.

## Executive summary

Relayterm will create a persistent, agent-neutral development workspace where Claude Code, Codex, OpenCode, and future command-line agents can enter, leave, receive tasks, record progress, and hand work over without making the project dependent on any single provider.

Relayterm will be TUI-first, not TUI-only. Its initial user experience will run through `rt` in a terminal and work locally or over SSH on Linux, macOS, and Windows. The underlying core and daemon will remain independent from the terminal interface so that future desktop, web, mobile, and automation clients can use the same workspace model and protocols.

Relayterm is not intended to replace coding agents or merge them into a new model. It provides the durable environment around them: real terminal sessions, shared project state, explicit task ownership, structured handovers, and a provider-neutral coordination layer.

## The problem

Modern coding agents are powerful, but their operating context is fragmented:

- Each agent has its own session model, conventions, and context limits.
- Work often becomes tied to a specific terminal, provider, IDE, or chat history.
- A new agent cannot reliably determine what was attempted, what changed, what remains, or how the work was verified.
- Multiple agents can accidentally edit the same files or duplicate work.
- Remote development workflows are weakened when their control surface requires a graphical desktop or a provider-specific application.
- Provider access, supported models, pricing, and commercial relationships can change independently of user needs.
- Informal handovers may expose secrets, personal data, local paths, or private conversation content when committed to a public repository.

The missing layer is a persistent and inspectable workspace that belongs to the project rather than to an agent.

## Product premise

> A persistent and neutral workspace where Claude Code, Codex, OpenCode, and future agents can enter, leave, receive tasks, record context, and hand work over without the project belonging to any one of them.

The workspace is the system of record. Agents are interchangeable participants. Interfaces are clients. Provider-specific commands are adapters at the boundary, not foundations of the architecture.

## Purpose

Relayterm exists to make agent-assisted software development portable, resumable, inspectable, and resilient across tools and providers.

It should let a developer connect to a machine, open the workspace, inspect current activity, enter a real terminal session, assign or resume a task, and understand the state of the project without reconstructing it from private chat histories.

## Target users

The initial audience is technically experienced developers and operators who:

- Work comfortably in terminals and over SSH.
- Use more than one command-line coding agent.
- Develop across Linux, macOS, and Windows.
- Need long-running or remotely hosted development sessions.
- Want project continuity without dependence on one agent provider.
- Value plain files, inspectable state, and local control.

The initial product does not target first-time terminal users or attempt to hide the underlying development environment.

## Product principles

### Project ownership over agent ownership

Tasks, decisions, progress, and handovers belong to the workspace. An agent may contribute to them but must not become their exclusive source of truth.

### Neutrality by design

The core must not require a particular model provider, subscription, authentication method, or agent-specific transcript format. Provider support belongs in replaceable adapters.

### TUI-first, not TUI-only

The terminal interface is the first and primary client because it works locally and through SSH with minimal infrastructure. The domain model, persistence, process supervision, and coordination protocols must not depend on terminal widgets or rendering.

### Real terminals, not simulated command consoles

Agent tools must run inside real pseudo-terminal sessions with native shell behavior, resizing, signals, colors, and interactive authentication flows. The project must not ask users to copy credentials into its own configuration.

### Local-first and inspectable

Core functionality should work on a developer-controlled machine without a hosted service. State formats, logs, and generated coordination artifacts should be documented and recoverable.

### Explicit coordination

Task claims, status transitions, handovers, and conflicts should be visible and attributable. Coordination must not rely on agents inferring intent from an unstructured transcript.

### Progressive capability

The MVP should solve single-user, multi-agent continuity well before attempting autonomous swarms or distributed scheduling.

### Safe public defaults

No public artifact should contain personal data, secrets, private prompts, raw authentication material, or machine-specific identifying information. Collection must be minimal and opt-in where it is not essential.

### Cross-platform behavior

Linux, macOS, and Windows are first-class targets. Platform abstractions should isolate operating-system differences and make degraded behavior explicit.

### Open protocols and replaceable components

External clients and integrations should eventually be able to use a versioned, documented protocol. Internal interfaces should avoid unnecessary coupling to the initial TUI or storage implementation.

## Initial scope

The initial release will provide:

- A persistent workspace rooted in a software project.
- A background core or daemon that owns durable state and supervised terminal sessions.
- A TUI client that connects to the daemon.
- Multiple real terminal panes or sessions for shells and command-line agents.
- A provider-neutral registry of agents and their capabilities.
- A task queue with explicit lifecycle states and ownership.
- Structured progress notes and handovers that survive agent exit.
- Workspace events and an audit trail suitable for local troubleshooting.
- Optional Git worktree isolation for tasks that can safely run in parallel.
- Privacy controls, redaction boundaries, and public-safe defaults.
- Recovery after the TUI disconnects or the SSH connection drops, within the limits defined by the MVP specification.

## Out of scope for the initial release

The MVP will not include:

- A graphical desktop, browser, or mobile client.
- A hosted coordination service or mandatory cloud account.
- A custom AI model, inference service, or model gateway.
- Direct use of provider API keys as a requirement.
- Automated bypass of provider authentication, licensing, or subscription terms.
- Full semantic understanding of proprietary agent terminal output.
- Unattended autonomous task decomposition or agent swarms.
- Real-time collaborative editing.
- Remote multi-user tenancy or Internet-exposed daemon operation.
- Sandboxing strong enough to run untrusted code as a security boundary.
- A plugin marketplace.

These exclusions constrain the first deliverable, not the long-term architecture.

## Neutral multi-agent model

The workspace model separates four concepts:

1. **Agent definition:** A named, configurable command and declared capabilities, such as interactive terminal support or non-interactive invocation.
2. **Agent instance:** A concrete supervised process with a lifecycle, terminal session, working directory, and optional task claim.
3. **Task:** A provider-neutral unit of work with status, scope, dependencies, acceptance notes, and ownership history.
4. **Handover:** A structured record that explains current state, relevant files, decisions, unresolved questions, verification performed, and recommended next action.

No agent receives privileged ownership of the project model. Agent-specific instruction files such as `AGENTS.md` and `CLAUDE.md` may coexist with neutral workspace artifacts, but they do not replace shared task and handover state.

The system should prefer explicit adapters over pretending all agents behave identically. Neutrality means a stable common contract with documented differences, not a lowest-common-denominator terminal wrapper.

## Privacy and public open-source posture

The repository and its documentation are assumed to be public from the first commit.

### Data minimization

The project must store only the data required to coordinate work and restore sessions. Telemetry must be disabled by default in the MVP. Future telemetry must be opt-in, documented, and separable from core operation.

### Secret boundaries

The project must never request, persist, print, or transmit provider passwords, session cookies, access tokens, API keys, SSH private keys, or operating-system credential-store contents. Authentication remains inside the launched provider tool or the user's environment.

### Public-safe repository content

Examples, fixtures, screenshots, logs, and tests must use fictional identities, synthetic paths, and fake credentials. Documentation must not include author home directories, private hostnames, email addresses, account identifiers, real project names, or private conversation excerpts.

### Local state separation

Runtime databases, terminal scrollback, process logs, user configuration, and machine-specific metadata must live outside version-controlled project content by default. Generated files that may contain private context must be excluded through documented ignore rules.

### Explicit export

Sharing or exporting handovers, logs, transcripts, or diagnostic bundles must require an explicit user action. Exports should support preview and redaction before publication.

### Threat model transparency

The project must state what it protects and what it does not. A local agent process normally has the same filesystem permissions as the user who launched it. The workspace coordinates such processes but is not, by itself, a sandbox.

## Long-term objective

The long-term goal is an open, durable control plane for agent-assisted development that can run on a laptop, workstation, or remote development host.

A mature version may provide:

- TUI, desktop, web, and carefully scoped mobile clients over one versioned protocol.
- Local and remote workspace attachment with strong authentication and encryption.
- Cross-machine session discovery and recovery.
- Policy-driven task routing based on agent capability, cost, availability, and user preference.
- Rich collaboration using isolated Git worktrees, change ownership, dependency graphs, and merge assistance.
- Standardized handover and workspace event schemas that other tools can implement.
- Extension points for agent adapters, task sources, notifications, storage, and user interfaces.
- Human approval gates for sensitive or destructive operations.
- Searchable project memory with provenance, retention controls, and selective disclosure.
- Multi-user collaboration with roles, access control, and auditable actions.

This direction should be pursued incrementally. The system must remain useful as a local terminal workspace even if none of the distributed or graphical capabilities are built.

## Measures of success

The project succeeds when a user can:

- Start work with one supported command-line agent and continue with another without reconstructing project state manually.
- Disconnect an interface, reconnect, and recover the workspace and live sessions when the host process remains available.
- Understand who or what owns each task, what changed, what was verified, and what should happen next.
- use the same core workflow on Linux, macOS, and Windows.
- Add a new agent integration without changing the domain model or TUI architecture.
- Keep runtime-private data out of a public Git repository using safe defaults.
- Use the product locally and over SSH without a graphical environment or mandatory hosted account.
- Enter the workspace with the concise `rt` command, subject to normal local installation and path configuration.

## Governance expectations

As an open-source project, governance should favor transparent decisions and portability:

- Architectural decisions should be recorded in concise decision records.
- Protocol and persistence changes should be versioned and documented.
- Provider integrations should be reviewed for neutrality and compliance with provider terms.
- Security reports should have a private disclosure path before public discussion.
- Contributions should include tests appropriate to their risk and must not introduce personal or secret data.
- The governance model, license, code of conduct, and release policy should be selected before the first public release.

## Requested project outcome

Contributors are requested to design and implement the Relayterm MVP described in [MVP_TECHNICAL_SPEC.md](MVP_TECHNICAL_SPEC.md). The result should prove that a persistent, provider-neutral workspace can supervise real terminal agents, preserve task and handover state, and remain accessible through a cross-platform TUI while keeping the core independent from that interface.

The first milestone is not autonomous multi-agent programming. It is dependable continuity, clear coordination, and user-controlled interoperability.
