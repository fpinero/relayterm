# Relayterm MVP threat model

## Scope and assumptions

Relayterm coordinates local processes for one operating-system user. The local user and explicitly configured commands are trusted to use that user's permissions. Project content and terminal output are untrusted data. Relayterm is not a sandbox and does not protect the user from a malicious configured child, an administrator, compromised kernel or arbitrary same-user process.

The MVP has no TCP listener, hosted control plane, telemetry, provider API integration, automatic export or persistent terminal recording.

## Assets and boundaries

| Asset | Location and owner | Protection goal |
| --- | --- | --- |
| Workspace model and events | Private per-workspace SQLite database, daemon-owned | Atomicity, integrity, availability and current-user confidentiality |
| Registry | Private application data | Stable project-to-workspace identity and non-destructive recovery |
| IPC endpoint and locks | Private runtime directory | One daemon owner and authenticated current-user access |
| Terminal state | Bounded daemon memory | Correct reconstruction without durable transcript leakage |
| Definition arguments and task text | Authorized database fields and transient IPC/UI memory | Validate limits; omit from metadata events and diagnostics |
| Environment values | Resolved at launch only | Never persist or diagnose values |
| Git worktree intent | Durable database plus external Git state | Stable ownership, conservative partial-result recovery, no destructive compensation |
| Backup | Explicit private destination | Consistent, integrity-checked, no-overwrite recovery copy |

The TUI and administrative CLI are authenticated clients. The daemon alone opens workspace SQLite for normal mutations and owns supervised PTYs. Git, filesystem, SQLite and PTY adapters are external-effect boundaries. Public repository content and CI logs are separate publication boundaries.

## Threats and controls

| Threat | Current control | Residual risk |
| --- | --- | --- |
| Another OS user connects to IPC | Unix owner/mode and peer identity checks; protected Windows named-pipe DACL and handle inspection | Administrators and already-compromised same-user processes remain outside protection |
| Two daemons reconcile or mutate together | Runtime lock acquired before endpoint and recovery; startup lock serializes launch | Copies in unrelated private homes cannot be globally discovered |
| Malformed or oversized IPC exhausts memory | Length-prefixed limits, JSON depth, bounded connections/operations and typed rejection | Local same-user denial attempts can still consume permitted quotas |
| Sensitive content reaches logs/events | Metadata-only events, structured diagnostic allowlist, bounded sinks, synthetic marker tests | Heuristic secret detection cannot recognize arbitrary secrets |
| ANSI content changes non-terminal UI/logs | Terminal bytes stay in terminal parser; other views escape or sanitize controls | A terminal application can intentionally control its own pane |
| Environment leaks to a child or diagnostics | Explicit baseline and named allowlist assembled at launch; values not serialized | Child receives approved values and retains normal user permissions |
| Executable/path race changes launch | Canonical roots, neutral resolver and immutable admitted launch snapshot | Trusted user can intentionally modify files; filesystem guarantees vary |
| Git invokes hooks, filters, network or helpers | Argument arrays, explicit environment, disabled hooks/prompts/pager/lazy fetch, filter rejection and local operations | Git and filesystem are trusted native dependencies; configuration hardening requires continuing regression coverage |
| Git succeeds but SQLite fails | Durable prepared/applying intent, stable fingerprint, `needs_attention`, read-only reconciliation | Manual cleanup may be necessary; Relayterm never deletes or repairs automatically |
| Terminal flood exhausts daemon | Bounded raw/parsed state, frames, input, events, sessions and fair control paths | Configured children can consume CPU, memory and network outside Relayterm's own buffers |
| Backup overwrites or exposes data | Fresh private destination, create-new files, SQLite snapshot, checksum, exact inventory and fresh-home restore | Backup is not encrypted; operator controls retention and copies |
| Corrupt/newer storage is silently replaced | Preflight identity/version/integrity checks and preservation guidance | Manual recovery may require expertise; no automatic repair |
| Runtime contacts a hosted service | No product network service/client; source checks reject known network APIs | Native offline observation is still required; user-configured children and SSH test transport may use network independently |

## Privacy validation

Inject obvious synthetic credential-shaped markers, native path aliases, arguments, task text, environment values and ANSI/OSC bytes at their valid boundaries. Verify errors identify only field/category, metadata events omit content, diagnostics omit values and paths, terminal bytes are not durable, and project content receives no automatic artifact.

Authorized task and definition text and native workspace/worktree paths legitimately exist in private SQLite. A scan that asserts those databases contain no paths or narrative would test the wrong contract. Environment values, terminal streams and raw dumps remain prohibited.

## Security review policy

Run locked dependency advisory, license, ban and source checks; candidate and history secret scans; repository contract checks; and negative controls. Critical or high findings require a fix or an explicit maintainer-reviewed exception with impact, rationale, mitigation and follow-up. An implementation agent cannot approve its own exception. Behavioral acceptance defects remain blockers regardless of advisory severity.

Use GitHub private vulnerability reporting for sensitive findings. Do not test that route by sending a fabricated report and do not place private reproduction details in a public issue or pull request.
