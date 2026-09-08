# Daemon and administrative CLI

Relayterm M05 provides one independent local daemon per initialized workspace. The daemon owns SQLite state and serves current-user local IPC. Administrative commands are clients of that daemon. Real process and PTY creation starts in M07, and the default TUI starts in M08.

## Private state and workspace selection

Commands select `--workspace <path>`, defaulting to the current directory without ancestor search. The path must name an existing directory. Relayterm canonicalizes it and stores the registry, database, endpoint, locks, and logs in operating-system private locations outside the project tree.

For disposable tests, `--home <absolute-path>` selects an explicit private root. `RELAYTERM_HOME` is the equivalent environment override, and the explicit option wins. These overrides are the only supported way to place private data under a project. Help and version do not resolve locations or create state.

Initialize once and reconnect later:

```text
rt --workspace ./example workspace init --name "Example workspace"
rt --workspace ./example workspace status
rt --workspace ./example daemon stop
rt --workspace ./example daemon start
rt --workspace ./example workspace open
```

`workspace init` is the only command that creates a registration. `workspace open` and `daemon start` start an existing workspace. Status and stop never initialize or start one. Closing the starter does not stop a ready daemon. Orderly stop drains accepted mutations and verifies that the same daemon generation has stopped.

After a daemon crash or host restart, persisted `starting` and `running` instances become `lost` before the daemon reports ready. Their open claims close and active tasks become blocked atomically. Final historical exits remain unchanged. Relayterm does not attach to an operating-system process from a persisted PID.

## Machine-readable output

Use `--format json` for scripts. A finite command emits one object with `schema_version`, `command`, `ok`, and either `result` or `error`. Revisions and event sequences are decimal strings. JSON mode does not contain terminal color codes. Event watch emits JSON Lines records until interrupted.

| Code | Meaning |
| --- | --- |
| 0 | Success, including already running or already stopped |
| 1 | Internal or currently unavailable default TUI |
| 2 | Usage or local input error |
| 3 | Workspace or entity not found |
| 4 | Daemon, startup deadline, or transport failure |
| 5 | Authoritative conflict, state, reference, or actor rejection |
| 6 | Access, protocol, schema, integrity, or recovery failure |
| 7 | Mutation outcome unknown after possible dispatch |
| 8 | Definitive storage or service failure |
| 130 | User interruption before a mutation has an uncertain result |

Code 7 requires reading authoritative state, history, or events before deciding whether to retry. Relayterm never automatically repeats a possibly dispatched mutation.

## Administrative input

Narrative and structured mutation input comes from exactly one of `--file <path>` or `--stdin`. JSON files contain operation content. Identity and revision arguments supplied on the command line are inserted by the client and cannot also appear in the file.

```text
rt --workspace ./example task create --expected-revision 1 --file ./task.json
rt --workspace ./example task list --limit 50
rt --workspace ./example task get 00000000-0000-4000-8000-000000000601
```

Example `task.json`:

```json
{
  "title": "Verify the synthetic workflow",
  "description": "Exercise durable administrative state.",
  "priority": "normal",
  "scope_paths": ["src"],
  "acceptance_notes": "The state survives daemon restart.",
  "dependency_ids": []
}
```

Registering or updating an agent definition stores a neutral command and argument array. It does not execute the command or authenticate with a provider. Import an existing version-1 TOML candidate explicitly:

```json
{
  "display_name": "Synthetic agent",
  "command": "synthetic-agent",
  "arguments": ["--interactive"],
  "environment_allowlist": ["PATH"],
  "capabilities": ["terminal"],
  "enabled": false
}
```

```text
rt --workspace ./example agent import --file ./agents.toml --expected-revision 2
```

Source edits have no effect until another explicit import. Imports are atomic, preserve omitted definitions, never rewrite the source, and reject a stale expected revision even when values match. The daemon stores environment variable names only. It does not store their values.

The embedded catalog is read-only and never imports definitions automatically. The TUI copies a template into a new definition with a fresh ID, while explicit TOML import creates or updates the stable ID in that file. See [agent templates and custom CLIs](agent-templates.md).

Check one saved definition without executing it or inspecting provider credentials:

```text
rt --workspace ./example agent check 00000000-0000-4000-8000-000000000950 --expected-revision 4
```

The result separates enabled state from the bounded availability status. It contains no resolved path, PATH value, argument content, environment value, or native error text. Availability does not prove authentication or provider health. A launch always resolves again against the admitted definition revision.

Task lifecycle, claim, progress, handover, history, session metadata, and event commands are visible under `rt <group> --help`. M05 does not fabricate running instances, so a production claim that targets no real running instance fails honestly until M07 provides launch operations. Test fixtures may create synthetic lifecycle records without adding a production control.

Progress input records an attributed summary and explicit verification:

```json
{
  "summary": "Implemented the bounded parser.",
  "verification": "cargo test -p relayterm-cli --locked"
}
```

A handover input carries the complete continuation context. Creating one also requires `--expected-revision` and an active task owned by a running instance:

```json
{
  "summary": "The parser and daemon dispatch are connected.",
  "decisions": "Keep JSON content separate from command arguments.",
  "changed_paths": ["crates/relayterm-cli/src/main.rs"],
  "verification_performed": "cargo test -p relayterm-cli --locked",
  "open_questions": "None.",
  "recommended_next_action": "Review the native CI matrix."
}
```

## Diagnostics and recovery

Daemon diagnostics use stable event names and allowlisted fields. They are stored under the private workspace data directory. Records are limited to 4 KiB. The active file is limited to 1 MiB and retains at most three rotated files. Logs exclude requests, response bodies, task prose, configuration contents, command arguments, environment values, terminal output, and raw paths.

An inaccessible endpoint, incompatible live daemon, missing ready database, unsafe private path, or invalid configuration fails closed. Preserve the private data before investigating a recovery-required error. Status does not delete endpoints, create a replacement database, or stop another generation. Relayterm is local-first and opens no TCP listener.

An optional private `config/config.toml` is read at daemon startup. Its validated storage settings apply to that process. Definition entries are validated but are never imported automatically. An absent file uses defaults. Invalid or inaccessible configuration prevents readiness.

## Complete command groups

The administrative surface comprises `workspace init/open/status`, `daemon start/status/stop`, `agent list/register/update/import/check`, `task create/list/get/update/transition/claim/release/history/claims`, `progress append`, `handover create/get`, `session list`, and `event list/watch`. Pagination defaults to 50 and rejects zero or values above 200. Use each command's help for exact arguments.
