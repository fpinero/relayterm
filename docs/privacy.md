# Privacy and runtime boundaries

## Current local runtime

The `rt` executable connects to one detached daemon per initialized workspace through current-user IPC. The daemon owns private SQLite state, real PTY or ConPTY sessions, bounded terminal state, and optional explicit Git worktree operations. The interactive client and administrative commands use the same authenticated protocol. Help and version remain side-effect free. Relayterm has no telemetry, hosted dependency, or automatic export.

## Durable state

The daemon stores workspace identities, definitions, task and claim history, progress, handovers, session metadata, immutable launch snapshots, worktree intents, approved worktree roots, and task worktree associations in private OS application data. Native paths are necessary for workspace, process, and worktree recovery. They are not written into project files automatically. Configuration, application data, runtime endpoints, and cache have separate logical locations.

Worktree events contain IDs, phases, reason enums, and changed field names. They exclude checkout paths, branch labels, base expressions, and Git output. Authorized worktree detail queries include the local path and branch needed by the current user. Runtime databases, sockets, logs, and worktree metadata are excluded from version control.

## Ephemeral state

Terminal screen state and bounded scrollback remain in daemon memory. Environment values are resolved at launch and passed to the child, not serialized into definitions, events, databases, or logs. Command arguments and user text may be sensitive; diagnostics use approved fields and path aliases rather than full-object dumps.

Secret-pattern checks cannot identify every arbitrary secret. Users must not place credentials in task or handover text. Authentication stays in provider CLIs or OS facilities. Any future optional project export requires exact-content preview and explicit destination confirmation; it is deferred from this roadmap.

## Trust boundary

Relayterm coordinates processes and is not a sandbox. Children normally have the user's filesystem and network permissions. Same-user IPC and private runtime locations do not defend against a malicious administrator or arbitrary processes already running as that user.

Explicit workspace backups contain sensitive coordination data and remain outside the project. They use a private, new destination and an integrity-checked manifest, but are not encrypted. Restore writes only to a fresh private Relayterm home and does not copy source files, credentials, terminal state, or live processes. See [private backup and restore](backup-restore.md) and the [MVP threat model](threat-model.md).

Repository fixtures use synthetic identities and fake values. Runtime capture, raw environments, private conversations, automatic crash uploads, and hosted dependencies are excluded. See the [environment ADR](decisions/0005-environment-privacy.md) for planned launch and diagnostic data flows.
