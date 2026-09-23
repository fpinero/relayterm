# Relayterm 0.1.0 candidate notes

Relayterm 0.1.0 is the first local MVP release candidate. It has not been tagged or published. Candidate status applies only to artifacts whose source commit, target, hash, native checks, and manual evidence appear in the final handoff.

## Included behavior

- One `rt` executable provides the interactive TUI and administrative CLI.
- One detached daemon per workspace owns private SQLite state, current-user local IPC, real PTY or ConPTY sessions, and optional Git worktree operations.
- Tasks support explicit lifecycle transitions, exclusive claims, append-only progress, and structured handovers.
- Neutral editable agent definitions include Claude Code, Codex, and OpenCode starting templates without installing or authenticating a provider.
- Sessions retain stable IDs, immutable launch snapshots, optional private display names, creation order, bounded paging, terminal state, and exclusive input leases while the daemon remains alive.
- Forms render a cell-aware cursor. Stale edits preserve the draft for explicit review, and competing terminal input shows bounded inline guidance.
- Backup and restore preserve validated coordination state through a fresh private home without copying live terminals, source files, Git objects, environment values, or credentials.
- Worktree creation is explicit, bounded, local-only, and conservatively recoverable. Relayterm does not delete worktrees or branches automatically.
- Portable target archives contain the executable, Apache-2.0 project license, target-specific third-party notices and license texts, install and recovery guides, collision-safe install helpers, and a machine-readable manifest.

## Supported candidate targets

- `x86_64-unknown-linux-gnu`, tested initially on Ubuntu 24.04.
- `aarch64-apple-darwin`, tested initially on macOS 14 or newer native evidence.
- `x86_64-pc-windows-msvc`, tested initially on Windows 10 22H2 for desktop behavior and Windows Server 2022 for automation.

No musl, other Linux distribution, macOS Intel or universal, or Windows arm64 artifact is claimed. The tested baseline is the initial minimum until an older runtime is tested.

## Security and privacy boundaries

Relayterm is local-first, opens no product TCP listener, has no telemetry, and requires no hosted Relayterm account. It is not a sandbox. Supervised commands normally have the operating-system permissions of the launching user. Session names, task text, terminal content, command arguments, paths, and environment values are excluded from public logs and release evidence. Secret-pattern scanning cannot detect every arbitrary secret.

The local candidate is unsigned. Checksums detect corruption but do not independently authenticate a publisher. Signing, notarization, tags, uploads, and a public release require a separate maintainer decision.

## Known limits

- Live processes and terminal contents survive client closure only while the owning daemon remains alive. Daemon or host restart records unrecoverable sessions as lost.
- Terminal scrollback is bounded in daemon memory and is not a persistent recording.
- Fixed-grid terminal history does not reflow after resize and narrowing can discard historical cells. Fresh output follows the current writer dimensions. This limit was explicitly accepted on 2026-09-23.
- Forced termination of the local SSH client can prevent terminal restoration sequences from arriving. A local `reset` may be required; normal exit and child survival remain separate requirements. This limit was explicitly accepted on 2026-09-23.
- Git is required only for worktree features. Provider commands and authentication remain user-managed prerequisites.
- The Windows x64 artifact dynamically requires the supported Microsoft Visual C++ v14 Redistributable for x64. Relayterm does not install it.
- Unix socket paths are bounded. An excessively long explicit private home can make the daemon endpoint unavailable; use a shorter private location and preserve the original state.
- The two supplemental Windows throughput misses retained from M11 remain documented. They did not change the fixed acceptance thresholds and are not a waiver for candidate regressions.

Read [installation](install.md), [installed quick start](quick-start.md), [upgrade and rollback](upgrade.md), [privacy](privacy.md), and [supported platform evidence](supported-platforms.md) before using a candidate.

## Acceptance still pending

The native physical correction rounds and final hosted Quality/Security checks
pass for the mapped candidate. Global M12 acceptance remains open for the
changed SSH presentation, independent Windows runtime installation and
the local Windows debug input-p95 measurements of 290/294 ms against 250 ms.
Passing installed release and hosted results do not waive those local failures.
See the [closure audit](m12-closure-audit.md) for the exact evidence boundaries
and remaining procedures. No stable-release readiness is claimed.
