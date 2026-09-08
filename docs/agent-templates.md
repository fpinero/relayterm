# Agent templates and custom CLIs

Relayterm stores every agent as the same neutral definition. The built-in Claude Code, Codex, and OpenCode templates are editable starting points. They do not grant special domain behavior, install a command, authenticate an account, or select a model.

## Built-in templates

The embedded catalog is version 1 and contains these disabled defaults:

| Template | Command | Arguments | Environment names | Capability |
| --- | --- | --- | --- | --- |
| Claude Code | `claude` | None | None | `terminal` |
| Codex | `codex` | None | None | `terminal` |
| OpenCode | `opencode` | None | None | `terminal` |

The commands were verified on 2026-09-08 against the [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference), [official Codex CLI documentation](https://developers.openai.com/codex/cli/), and [OpenCode CLI reference](https://opencode.ai/docs/cli/). Provider behavior can change. Check current provider documentation before adding optional arguments.

On the Agents screen, use `[` and `]` to select a template and `p` to copy it into a new definition form. Review every field, save the disabled definition, check availability with `v`, and enable it explicitly with Space. Saving or enabling never launches a process. Use `a` for a separate launch action.

Copying a template creates a fresh definition ID. Repeating the action intentionally creates another definition, even when names match. Importing a TOML file in `templates/agents` follows a different contract: its stable ID creates or updates that exact definition through explicit import. An import never deletes omitted definitions and source-file changes have no automatic effect.

## Generic definition schema

The version-1 TOML schema and TUI form expose the same fields:

```toml
format_version = 1

[[definitions]]
id = "00000000-0000-4000-8000-000000000950"
display_name = "Example interactive CLI"
command = "example-agent"
arguments = ["--mode", "interactive"]
environment_allowlist = ["EXAMPLE_CONFIG_HOME"]
capabilities = ["terminal"]
enabled = false
```

`command` is one executable name or path. Relayterm never splits it as a shell command. `arguments` is an ordered array. Spaces, commas, duplicate values, and empty values are preserved. In the TUI argument editor, enter one argument per line and use the literal marker `<empty>` for an empty argument. Ctrl-U clears the active field before replacement. Environment entries are names only, without assignments or values. Capabilities are neutral informational names. Relayterm does not interpret a provider name or capability as permission to mutate tasks.

The active definition lives in SQLite. TUI edits use authenticated local IPC and do not rewrite TOML. A form captures its target and workspace revision when opened. Concurrent edits cause a conflict and require review. If delivery becomes uncertain, Relayterm does not resubmit automatically. Press Ctrl-R to reconcile state, review the retained draft, and decide explicitly whether another submission is appropriate.

Definitions can remain disabled or unavailable while being edited. Disabling affects future launch admission only. Existing children, claims, terminal leases, history, and immutable launch snapshots remain unchanged.

## Executable availability

Use `v` on the Agents screen or the administrative command:

```text
rt --workspace ./example agent check 00000000-0000-4000-8000-000000000950 --expected-revision 4
```

JSON mode returns a bounded result containing only definition ID, observed revision, enabled state, status, and guidance code. It never returns the resolved path, PATH contents, arguments, environment values, or native operating-system error text.

`agent check` exits with status 0 for `available`, status 3 for a completed check whose result is not available, and the established administrative error status for transport, validation, conflict, or protocol failure. Both human and JSON output retain the typed result.

Availability checks inspect filesystem metadata. They do not run the command, invoke `which`, `where`, a shell, `--version`, or an authentication command. They do not read provider credential files. A positive result means the configured executable can be resolved under Relayterm's launch policy. It does not prove correct installation, authentication, licensing, network access, or dependency loading.

Bare names search the daemon's PATH. Only absolute, nonempty PATH entries are considered, with a maximum of 256 entries and 64 KiB. Empty or relative PATH entries do not imply the workspace directory. An explicit relative path is resolved from the validated workspace working directory. Relayterm performs no tilde, variable, glob, quote, or shell expansion.

On Unix, the target must be a regular executable file. On Windows, native `.exe` and `.com` targets are supported by the neutral resolver. Relayterm does not silently wrap `.cmd`, `.bat`, or `.ps1` in a command shell. Configure a native executable or an explicit interpreter as `command`, with the script path as a separate argument. PATH lookup is case-insensitive on Windows and checks native executable suffixes.

The statuses are `available`, `not_found`, `not_executable`, `unsupported_launcher`, `invalid_command`, and `unavailable`. Enabled state is independent. Missing-command guidance asks the user to install the tool manually, correct the definition, check the daemon environment, or configure an explicit executable or interpreter. Relayterm never installs or opens authentication flows.

Availability is a preflight, not a reservation. Launch resolves the command again and admits a particular definition revision. The durable instance snapshot and actual argument vector come from that admitted definition. A definition changed or disabled before admission causes a conflict or rejection. A change committed after admission applies to later launches only.

## Adding an arbitrary interactive CLI

An unknown CLI uses the same route as a built-in template:

1. Install or provide the CLI independently. Relayterm does not download it.
2. Open Agents, press `n`, and enter a display name, one command, arguments one per line, optional environment names, neutral capabilities, and `false` for enabled.
3. Save the definition, select it, and press `v`. Correct missing or unsupported command guidance without exposing private paths in diagnostics.
4. Enable with Space, then launch with `a`.
5. Assign work through explicit task claims. Record progress and verification, detach without terminating the child, and prepare a structured handover when another instance should continue.

The provider owns authentication. Sign in inside its own terminal UI or according to its official instructions. Relayterm does not store provider keys, passwords, tokens, cookies, or authentication state. Environment allowlisting controls which named values may be present in the launched process, but values are resolved transiently at launch and are not persisted in definitions or diagnostics.

An agent process normally has the launching user's operating-system permissions and may read its own configuration or contact external services. Relayterm coordinates the process and is not a security sandbox. Core operation and automated acceptance require no provider account or hosted Relayterm service.

Child output never changes a task automatically. Text resembling a completion, claim, or handover remains terminal content. `LocalUser` actions explicitly claim, append progress, prepare handovers, transition, and complete tasks.

## Limits and troubleshooting

Names are limited to 256 UTF-8 bytes. Commands and individual arguments are limited to 4 KiB. Definitions accept at most 128 arguments and 32 KiB of argument data, 128 environment names, and 128 capabilities. The complete TUI draft is limited to 256 KiB. Lists representing sets reject duplicates. Windows environment-name comparison is case-insensitive.

Availability work admits at most four concurrent filesystem jobs plus sixteen bounded waiters. Further requests fail with a resource result. Timed-out blocking filesystem work retains both permits until it actually finishes, so repeated checks cannot create unbounded workers. UI results are scoped to definition ID and revision, expire after 30 seconds, and are cleared by a revision or selection change. A launch always checks again.

If a command is available in a newly opened shell but missing to the daemon, compare how the daemon was started and use an explicit path when appropriate. Restarting refreshes its inherited environment, but a daemon with live children follows the documented shutdown policy. Review that consequence before stopping it.

Provider authentication and named terminal application smoke tests are optional and use user-owned installations. The mandatory M09 gate uses a separately built unknown interactive fixture through real `rt`, SQLite, authenticated native IPC, and PTY or ConPTY. It proves the generic route without network access or provider credentials.

## Native verification

Run the dedicated gate twice as separate commands so either result independently fails the job:

```text
cargo test -p relayterm-cli --test agent_templates_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test agent_templates_gate --locked -- --nocapture --test-threads=1
```

The gate starts with an empty catalog application state, previews all templates, configures the unknown test CLI through the TUI, proves unavailable and available checks, preserves its exact argument vector, coordinates a task across two instances, and reopens durable state after daemon restart. It reports bounded counts and check latency only. It never publishes terminal capture, paths, arguments, environment values, or credentials.

Local macOS arm64 validation on 2026-09-08 passed two consecutive invocations. Provider account tests were not run because they are optional and would require user-owned installations and authentication. Hosted Linux, macOS, and Windows results belong in [supported platform evidence](supported-platforms.md) only after the candidate CI runs complete.
