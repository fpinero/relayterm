# Privacy and runtime boundaries

## Current bootstrap

The current `rt` binary provides help/version and explicit unavailable-runtime errors. It opens no database or IPC endpoint, launches no child process, enters no terminal raw mode, and makes no network requests. No telemetry or automatic export exists.

## Planned durable state

The daemon will store workspace identities, definitions, task/claim history, progress, handovers, and session metadata in private OS application data. Local paths may be necessary in that private state. They must not be written to project files automatically. Configuration, application data, runtime endpoints, and cache have separate logical locations.

## Planned ephemeral state

Terminal screen state and bounded scrollback remain in daemon memory. Environment values are resolved at launch and passed to the child, not serialized into definitions, events, databases, or logs. Command arguments and user text may be sensitive; diagnostics use approved fields and path aliases rather than full-object dumps.

Secret-pattern checks cannot identify every arbitrary secret. Users must not place credentials in task or handover text. Authentication stays in provider CLIs or OS facilities. Any future optional project export requires exact-content preview and explicit destination confirmation; it is deferred from this roadmap.

## Trust boundary

Relayterm coordinates processes and is not a sandbox. Children normally have the user's filesystem and network permissions. Same-user IPC and private runtime locations do not defend against a malicious administrator or arbitrary processes already running as that user.

Repository fixtures use synthetic identities and fake values. Runtime capture, raw environments, private conversations, automatic crash uploads, and hosted dependencies are excluded. See the [environment ADR](decisions/0005-environment-privacy.md) for planned launch and diagnostic data flows.
