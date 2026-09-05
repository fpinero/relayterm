# M01 implementation details

## Purpose and execution contract

This document expands M01, repository foundations and bootstrap decisions, into an implementation guide for multiple coding sessions. It follows the [MVP technical specification](../MVP_TECHNICAL_SPEC.md), [project vision](../PROJECT_VISION.md), [repository instructions](../AGENTS.md), and [pending roadmap](../TODO.md).

The selected architectural defaults are one daemon per workspace and GitHub Private vulnerability reporting as the private security reporting channel. The latter must be verified as enabled before M01.12 closes.

Saving this plan does not complete M01.01 through M01.14. Those tasks remain pending until their individual deliverables and checks pass. This document describes the implementation contract, `TODO.md` owns the pending queue, and `avances.md` owns completion evidence. Do not maintain another progress checklist here.

M01 must produce eight architecture decision records (ADRs), a minimal six-crate Rust workspace, the `rt` executable with explicit placeholder behavior, public documentation, cross-platform CI, and verified quality controls. Actual SQLite persistence, IPC transport, child supervision, terminal widgets, provider templates, and Git worktrees remain in their later milestones.

Before implementation:

1. Read the repository instructions and relevant specification sections.
2. Inspect the active branch and working tree. Use an appropriate feature branch before editing; preserve unrelated changes.
3. Read `avances.md` to identify already verified tasks.
4. Check for Rust tooling both in `PATH` and conventional installation locations. Missing commands in `PATH` alone do not prove Rust is absent.
5. Install or expose the required toolchain only within the environment's permissions. Report unavailable tooling as a concrete limitation, not a failed product test.
6. Execute the earliest unblocked task below. Keep maintainer-dependent security and remote CI checks open while completing independent local work.

Publishing a branch, opening a PR, merging, changing GitHub settings, tagging, or releasing requires explicit authorization. Merely preparing this plan or implementing local files does not authorize those actions.

## Deliverables and task dependencies

| Task | Deliverable | Prerequisite within M01 | Verification boundary |
| --- | --- | --- | --- |
| M01.01 | ADR 0001: Daemon scope and lifetime | None | Architecture scenarios now; real lifecycle in M05-M07 |
| M01.02 | ADR 0002: Local IPC and synchronization | M01.01 | Contract review now; transport tests in M04 |
| M01.03 | ADR 0003: Terminal state and reattachment | M01.01, M01.02 | Reconstruction design now; prototype in M07 |
| M01.04 | ADR 0004: SQLite and recovery | M01.01, M01.02 | Durability contract now; storage tests in M03 |
| M01.05 | ADR 0005: Environment and privacy | M01.01 | Data-flow review now; runtime checks in M03-M11 |
| M01.06 | ADR 0006: Neutral agent configuration | M01.04, M01.05 | Precedence contract now; implementation in M03/M09 |
| M01.07 | ADR 0007: Worktree ownership and recovery | M01.01, M01.04 | Failure-sequence review now; Git tests in M10 |
| M01.08 | ADR 0008: Rust, platforms, and dependencies | None | Toolchain/dependency selection and recorded evidence |
| M01.09 | Six-crate skeleton and CLI contracts | M01.01-M01.08 | Local builds, behavioral tests, dependency graph |
| M01.10 | Runtime-private ignore rules | None | Positive and negative ignore checks |
| M01.11 | Contribution, privacy, platform, architecture documentation | M01.01-M01.10 | Documentation matches actual skeleton |
| M01.12 | Verified private security policy | Reporting-channel access | Enabled channel and accurate policy |
| M01.13 | Quality and security workflows | M01.08-M01.10 | Local equivalents and actual CI runs |
| M01.14 | Phase-0 exit evidence | All preceding tasks | Same candidate passes all required gates |

Create ADRs under `docs/decisions/`, named with the four-digit number and a descriptive lowercase slug. Each ADR must include context, decision, rejected alternatives, consequences, invariants, failure scenarios, and the milestone responsible for implementing and empirically verifying the behavior. Maintain an index under `docs/architecture/`.

An ADR may close after its documentation review succeeds. That does not establish that the future transport, lock, terminal, or database behavior already works.

## M01.01: Daemon scope and lifetime

### Decision

Use one daemon per workspace. A project failure must not stop another project's sessions. Multiple clients of one workspace share its daemon, database, and authoritative state.

### Implementation instructions for ADR 0001

1. Define workspace discovery using operating-system path canonicalization. Symlink aliases of the same root must resolve to the same workspace; the directory's display name is not its identity.
2. Define a private user registry associating canonical project roots with opaque workspace identifiers. Keep this registry outside project content.
3. Derive endpoint identity from the workspace identifier rather than a visible project path.
4. Require an operating-system lock held for the daemon's lifetime. A recorded PID alone is not proof of ownership.
5. Specify simultaneous startup: one starter becomes owner, the other connects to that owner. Never remove a live endpoint to resolve a startup race.
6. Permit stale endpoint removal only after acquiring the lock and validating ownership. Define errors for inaccessible or suspicious endpoints.
7. Separate client exit from daemon shutdown. Closing the TUI or losing SSH must not request daemon termination.
8. Define explicit daemon shutdown as ending supervision and applying the documented child termination policy. Survival of live processes across daemon restart is outside the MVP.
9. Reject incompatible daemons with actionable errors. Clients must not silently replace or restart them.
10. Explain the tradeoff against a per-user daemon: additional processes are accepted in exchange for simpler workspace failure isolation and recovery.

### Review and later tests

Include sequences for first open, reconnect, simultaneous starters, stale endpoint recovery, incompatible versions, and two unrelated workspaces. Ensure the registry/startup design cannot allocate two active identities for the same canonical root. Assign real lock, process independence, and recovery tests to M04-M07.

## M01.02: Versioned local IPC

### Decision

Use Unix domain sockets on Linux/macOS and named pipes on Windows. Restrict endpoints to the current user; reject remote pipe clients. Never fall back to TCP.

### Implementation instructions for ADR 0002

1. Define explicit protocol versioning from the first implementation, JSON control messages, request identifiers, and separate frames for opaque terminal bytes.
2. Adopt length-delimited framing with a frame-kind discriminator. M04 must finalize complete payload schemas, numeric limits, and retry/idempotency details before implementing transport.
3. Require bounded allocation before accepting payloads, request correlation, and ordered events per workspace.
4. Define initial synchronization as a snapshot with an event position followed by subscription from that position. Require detection of expired cursors and stream gaps.
5. Keep slow subscribers from blocking control processing or other clients. Require separate bounded handling of terminal traffic and control/event traffic.
6. Prohibit automatic mutation retries after an ambiguous response loss until the idempotency contract is implemented.
7. Document restrictive Unix permissions and peer validation where supported.
8. Document an explicit Windows DACL, current-user identity validation where available, and rejection of remote clients. Default named-pipe permissions are not sufficient.
9. Define incompatible-version and malformed-request outcomes as typed, actionable failures rather than crashes.
10. Introduce only the protocol version constant and pure compatibility check in M01 code. Do not publish an incomplete wire schema as a stable API.

[Microsoft's named-pipe security documentation](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights) explains why default descriptors must not be assumed to restrict access to the intended user.

### Review and later tests

Map fragmented/coalesced frames, oversized lengths, unknown operations, incompatible versions, rejected peers, client disconnects, and snapshot/subscription races to M04 tests. Review how terminal backpressure preserves control responsiveness in M07.

## M01.03: Terminal state and reattachment

### Decision

The daemon owns bounded in-memory scrollback and the terminal state required to reconnect. There is no persistent terminal recording. A circular byte buffer alone is not accepted as proof of correct full-screen reconstruction after truncation.

### Implementation instructions for ADR 0003

1. Require a provider-neutral representation of screen contents, cursor, and interaction modes sufficient for the supported terminal behavior.
2. Define attachment as a terminal-state snapshot followed by an ordered continuation of output. A detected discontinuity requires explicit resynchronization.
3. Keep the representation independent of TUI widgets. The client renders it; the daemon retains the state required while no client exists.
4. Keep output parsing limited to terminal behavior. It must never infer authoritative tasks, claims, or progress from provider prose.
5. Prevent parsing from executing external side effects such as writing to the client clipboard.
6. Retain memory bounds for parser state, dimensions, history, and queued output, not just the raw byte ring.
7. Select `vt100` as the first M07 prototype candidate, without adding it as an M01 dependency. Its suitability remains subject to evidence, not assumed from its API.
8. Assign input ownership, resize authority among attached clients, exact reconstruction format, and overflow policy to the M07 planning gate before production implementation.

The [vt100 documentation](https://docs.rs/vt100/latest/vt100/) describes terminal parsing and in-memory screen representation. It does not itself demonstrate Relayterm's complete reconnection requirements.

### Review and later tests

Require the M07 prototype to exercise alternate-screen applications, cursor movement, split escape sequences, Unicode, resize, and reconnect after retained history has been truncated. If the candidate fails, record evidence and revise the ADR at that gate before implementing the production adapter.

## M01.04: SQLite persistence and recovery

### Decision

Use one private database per workspace with checked-in migrations. Relational state is authoritative; state changes and corresponding events commit atomically. Keep the user workspace registry separate from individual workspace databases.

### Implementation instructions for ADR 0004

1. Document logical configuration, data, runtime, and cache locations using OS conventions and the future explicit test/portable override.
2. Keep runtime-private state outside project roots by default and require restrictive permissions or ACLs.
3. Enable foreign-key integrity and coordinate writes through the owning daemon.
4. Choose WAL as the initial intended journal mode, subject to M03 locking and durability tests.
5. Version schema and migrations from the first implementation. Reject newer schemas and unsupported downgrades.
6. Preserve original files on corruption or failed recovery. Never silently recreate an unreadable database.
7. Require consistent backups through SQLite facilities, never a copy of only the main file of a live database.
8. Use stopped-daemon restoration into a new destination as the initial recovery procedure. Never overwrite the original as an automatic fallback.
9. Mark unrecoverable former sessions honestly after restart; do not fabricate exit codes or promise live-process recovery.
10. Assign detailed repository schemas, transaction tests, and the selected backup mechanism to M03/M11, following the adapter's actual supported APIs.

SQLite documents both its online backup API and `VACUUM INTO` as backup mechanisms in the [backup reference](https://www.sqlite.org/backup.html).

### Review and later tests

Cover partial writes, transaction rollback, schema mismatch, corrupt storage, backup consistency, and repeated restart reconciliation. Check that loss of a client cannot become loss of a committed mutation. Real process-restart and migration tests belong to M03-M06.

## M01.05: Environment inheritance and privacy

### Decision

Construct child environments explicitly from documented platform needs and approved variable names. Resolve values at launch time; do not persist environment values. Provider authentication remains inside the provider CLI or operating-system facilities.

### Implementation instructions for ADR 0005

1. Document a per-platform table of baseline variable categories, purposes, and treatment for normal shell/tool behavior.
2. Permit additional variable names through `environment_allowlist`. Do not store their values in definitions, SQLite, events, or logs.
3. Do not introduce provider password/token fields or credential discovery.
4. Treat arguments, user text, and child output as potentially sensitive.
5. Construct diagnostics from an allowlist of safe fields rather than serializing complete objects and attempting redaction afterward.
6. Use opaque identifiers and path aliases in ordinary messages. Do not log complete requests, raw environments, terminal output, or sensitive arguments.
7. Escape untrusted text outside terminal panes.
8. Document controls for secret-bearing input and the limitations of pattern-based detection. Do not promise universal secret detection.
9. Include synthetic examples only, with variable names and fictional values where needed. Never copy the implementation machine's environment into an ADR or fixture.

### Review and later tests

Trace configuration-to-launch and request-to-diagnostic data flows. Identify where raw values may exist transiently and where they are forbidden. Assign injected synthetic secret/path/ANSI checks to M03-M11. Keep the no-sandbox limitation explicit.

## M01.06: Neutral agent configuration

### Decision

Use TOML for editable user configuration. Definitions have stable IDs, separate command/argument fields, declared capabilities, and no provider-specific domain types. Built-in templates and unknown custom commands use the same contract.

### Implementation instructions for ADR 0006

1. Define explicit template import, validated user configuration import, and daemon persistence of accepted definitions.
2. Make SQLite the active definition state observed by clients after import.
3. Require explicit reload to apply file edits. Do not silently change running processes.
4. Route future TUI definition edits through IPC into active state; do not automatically rewrite the user's TOML file.
5. Resolve executables at launch from a validated explicit path or the permitted `PATH`.
6. Preserve IDs and history for disabled definitions and reject launches from disabled definitions.
7. Keep active instances bound to their launch configuration when a definition is edited.
8. Define warnings for unknown configuration fields and closed failure for invalid security-sensitive fields, consistent with specification section 11.
9. Document environment variable names only, following ADR 0005.
10. Assign configuration implementation to M03 and template/editor integration to M09. Do not add provider templates or authentication helpers in M01.

### Review and later tests

Walk through first import, file edit without reload, explicit reload, concurrent client editing, disabled definition launch, missing executable, and a custom CLI. Ensure none requires a domain or TUI provider branch. Detailed conflict handling belongs to the configuration implementation before writes are enabled.

## M01.07: Explicit and recoverable worktrees

### Decision

Invoke installed Git with argument arrays behind a narrow adapter. Worktree creation is explicit and recoverable; deletion, automatic commits, merges, and rebases remain excluded.

### Implementation instructions for ADR 0007

1. Place the default worktree root under private application data, separate from the original project root. Permit additional roots only through explicit selection and validation.
2. Generate stable names from task identifiers, not free-form task titles. Use `rt/task-` plus the task identifier for proposed branch names.
3. Require an explicit base reference with `HEAD` as the default. Validate references using Git and paths in the adapter.
4. Register created worktrees as allowed roots for task session launch.
5. Describe creation as a recoverable operation spanning Git and SQLite rather than a fictitious shared transaction.
6. Preserve the resulting directory if Git succeeds but persistence fails. Expose a partial result that can be reconciled.
7. Reconcile the same operation before retrying creation. Never blindly duplicate a worktree or overwrite a collision.
8. Prohibit automatic deletion as rollback, including branches or directories containing apparently unfinished work.
9. Keep non-Git workspaces and unavailable Git functional for all unrelated workflows.
10. Assign the concrete operation state machine and failure-injection tests to M10.

### Review and later tests

Cover invalid references, option-like arguments, symlink/path escapes, existing paths/branches, simultaneous requests, and failure after Git succeeds. Describe manual recovery without implying authorization to remove user data.

## M01.08: Rust, platforms, and dependencies

### Decision

Use Rust edition 2024 and a virtual workspace with explicit `resolver = "3"`. At bootstrap, select the then-current stable Rust release, pin its exact version, and use that same version as the initial minimum supported Rust version (MSRV). Do not promise older compiler support without tests.

### Implementation steps

1. Verify official current Rust release information and platform support when implementing this task. Record the selected version and sources in ADR 0008.
2. Configure `rust-toolchain.toml` with the exact selected version and minimal profile plus Rustfmt and Clippy.
3. Share project version, edition, license, and `rust-version` through workspace metadata. Use `publish = false` for initial packages.
4. Run CI on both the pinned toolchain and current stable. Record the resolved compiler versions in verification evidence.
5. Select stable dependency releases compatible with that compiler, verify licenses, and commit `Cargo.lock`. Do not invent future version numbers in advance.
6. Initially add only `uuid` for domain identifiers and `clap` for the CLI. Use `serde_json` as a CLI test-only dependency for dependency-graph inspection.
7. Defer SQLite, Tokio, PTY, rendering, and provider dependencies until code actually needs them. Do not add empty adapter crates merely to mirror the eventual architecture.
8. Document native CI targets for Linux GNU, macOS, and Windows MSVC. Record actual runner OS and architecture; do not infer coverage of untested architectures from an OS label.
9. Distinguish the planned shell/terminal matrix from proven bootstrap build support. Actual terminal behavior is verified in later milestones.

Cargo requires an explicit resolver setting for a virtual workspace because it has no package edition from which to infer it. See the [Rust 2024 resolver guide](https://doc.rust-lang.org/stable/edition-guide/rust-2024/cargo-resolver.html).

### Verification

Confirm toolchain selection, dependency licensing, metadata inheritance, and locked resolution. M01.09 validates the resulting compilation and package boundaries; M01.13 adds the native CI matrix.

## M01.09: Rust skeleton and executable contract

### Crates and dependency direction

Create the following packages under `crates/`. The table is an allowlist, not a requirement to add an unused dependency.

| Crate | M01 responsibility | Allowed internal dependencies |
| --- | --- | --- |
| `relayterm-domain` | Distinct opaque `WorkspaceId` and `TaskId` types | None |
| `relayterm-application` | Documented boundary for future use cases | Domain |
| `relayterm-protocol` | Initial version constant and pure compatibility check | None |
| `relayterm-daemon` | Explicit placeholder daemon entry point | Application, domain, protocol |
| `relayterm-tui` | Explicit placeholder client entry point | Protocol |
| `relayterm-cli` | Argument parsing and executable composition | Daemon, TUI |

### Atomic implementation steps

1. Create the root workspace manifest, inherited metadata, package manifests, and library targets. Only the CLI package provides a user-facing binary, named `rt`.
2. Implement distinct UUID wrappers for `WorkspaceId` and `TaskId`, with explicit construction, equality, display, and validated parsing. Avoid implicit cross-type conversion. M02 will extend the domain rather than replace these identifiers.
3. Leave task entities, transitions, claims, repositories, and orchestration for M02. Keep the application boundary documented without inventing no-op use cases or unused ports.
4. Define initial protocol version 1 and a pure compatibility check that accepts exactly that version. Return a typed incompatibility result without opening a transport.
5. Provide separate daemon and TUI entry functions returning explicit not-implemented errors. Avoid `todo!()` panics or successful fake startup.
6. Use Clap for help, version, daemon subcommand dispatch, and invalid-argument handling. Map placeholder errors to safe English messages and nonzero exits.
7. Add CLI integration tests that execute the built `rt` binary using argument arrays in isolated temporary directories.
8. Add a dependency-boundary integration test in the CLI package using `cargo metadata --format-version 1 --locked` and test-only JSON parsing. Inspect declared workspace edges and the resolved core dependency closure, including relevant target-specific edges.
9. Reject forbidden internal edges and prevent TUI, SQLite, PTY, or provider dependencies from entering domain/application/protocol closures. Test the checker against synthetic graph mutations without modifying actual manifests.

### Observable command behavior

| Invocation | Required M01 behavior | Exit code |
| --- | --- | --- |
| `rt --help` | Print usage and available commands | 0 |
| `rt --version` | Print the package version | 0 |
| `rt daemon --help` | Print daemon subcommand usage | 0 |
| `rt` | Print that the TUI is not implemented yet | 1 |
| `rt daemon` | Print that the daemon is not implemented yet | 1 |
| Invalid arguments | Print an actionable usage error | 2 |

Placeholder commands must terminate promptly. They must not create databases, runtime directories, sockets, child processes, or terminal raw-mode state. Capture both output streams in tests and verify they contain no environment dumps or machine-specific paths.

### Verification

Test identifier round trips, malformed identifiers, compile-time distinction of ID types, matching and mismatching protocol versions, all six CLI outcomes, and absence of generated runtime state in the disposable project directory. Run the core tests without selecting the CLI or TUI packages. Do not claim that a successful placeholder test proves daemon or TUI functionality.

## M01.10: Repository hygiene

1. Preserve existing JetBrains and Cargo ignore rules.
2. Add rules for SQLite databases and sidecars, sockets, logs, terminal captures, coverage output, and Relayterm-specific private configuration.
3. Avoid broad rules that hide arbitrary project configuration or all data fixtures.
4. Keep migrations, synthetic fixtures, manifests, `Cargo.lock`, and public example configuration visible to Git.
5. Use synthetic paths with `git check-ignore` to verify positive cases. Also test negative cases that must remain trackable.
6. Do not create sensitive runtime data to test an ignore pattern.
7. Document that ignore rules are a secondary repository safeguard; default runtime locations still belong outside project roots.

Close this task only after positive and negative path checks pass and `git diff --check` is clean.

## M01.11: Public documentation

### Required documents

- `CONTRIBUTING.md`: toolchain preparation, validation commands, dependency direction, synthetic fixtures, and the queue-to-logbook workflow.
- `docs/privacy.md`: durable versus ephemeral data, private runtime locations, sensitive arguments, no telemetry, explicit export exclusions, and the non-sandbox boundary.
- `docs/supported-platforms.md`: actual compiler/runner/architecture evidence, intended shells/terminals, and the future manual test matrix.
- An architecture index linking all eight ADRs and explaining adapter boundaries.
- README updates describing the real placeholder behavior and current build instructions.

### Atomic implementation steps

1. Write all public text and code comments in English with sentence-case headings and no em dash or technical-documentation emojis.
2. Use fictional paths and identities only. Do not copy hostnames, usernames, terminal transcripts, or local environment details.
3. Reconcile the vision's pending license statement with the existing Apache-2.0 license. Preserve `LICENSE` and avoid unrelated product changes.
4. Explain local/SSH intent while distinguishing unverified terminal behavior from successful native compilation.
5. Check that each documented command exists and has the stated behavior in M01.
6. Validate relative links and review consistency with the ADRs and executable help.

The language must not imply that persistence, sessions, or a real TUI are already available.

## M01.12: Private security reporting

### Decision

Use GitHub Private vulnerability reporting. Selection of this channel does not establish that the repository setting is enabled.

### Atomic implementation steps

1. Read the repository's reporting setting when GitHub access is available.
2. Confirm an enabled private channel before claiming that users can submit reports.
3. Write `SECURITY.md` with navigation to the repository's security area and the Report a vulnerability action.
4. Ask reporters not to publish vulnerabilities, credentials, or sensitive reproductions in public issues.
5. Do not invent contact addresses or response-time commitments.
6. Verify the setting and entry point without submitting an unnecessary test vulnerability report.
7. If disabled, retain an explicit maintainer activation dependency. Do not silently change repository settings.
8. Keep M01.12 open if network access, repository access, or activation is unavailable. Continue independent local tasks.

GitHub requires the feature to be enabled before private reports can be submitted. See [private vulnerability reporting](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

## M01.13: Quality and security CI

### Quality workflow

1. Add `.github/workflows/ci.yml` for push, pull request, and manual runs.
2. Run native Linux, macOS, and Windows jobs on current stable Rust. Use Linux GNU and Windows MSVC targets; record the macOS runner architecture.
3. Add a Linux job for the exact pinned bootstrap toolchain to verify the initial MSRV. Current stable and pinned-version checks must both remain visible when their versions differ.
4. Run builds and tests with `--locked`, formatting checks, Clippy with warnings denied, CLI contract tests, and the dependency-boundary test.
5. Set matrix `fail-fast: false` so one platform failure does not hide the others.
6. Use minimal read permissions and no repository secrets for contributed code. Do not execute untrusted PR code through privileged `pull_request_target` workflows.
7. Pin third-party actions to full commit SHAs with a readable release annotation. Verify release provenance when selecting those SHAs.
8. Print compiler and runner versions as evidence without dumping environment values.

[GitHub's secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use) recommends full commit SHA pins for actions.

### Security workflow

1. Add `.github/workflows/security.yml` for pushes, pull requests, manual runs, and a weekly schedule.
2. Use version-pinned `cargo-deny` for advisories, licenses, dependency restrictions, and sources. Add `deny.toml` with explicit policy.
3. Start the license allowlist with Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, and Unicode-3.0. Reject unlicensed or unapproved dependencies; do not silently broaden policy to pass CI.
4. Reject known vulnerabilities and unjustified Git dependencies. Any exception requires a specific recorded rationale, review, and follow-up rather than a broad suppression.
5. Run a pinned Gitleaks CLI against Git history and a tracked-source scan input, with findings fully redacted. Ensure the scan includes the candidate changes and is not polluted by build caches.
6. Fetch sufficient Git history for the history scan. Do not exclude the complete synthetic-fixture directory.
7. Use narrowly scoped exceptions only for demonstrated false positives. Keep fake secret fixtures obvious and never introduce real credentials.
8. Verify control failures using disposable synthetic scan inputs and graph/configuration fixtures outside tracked source. Do not commit a deliberately failing secret to exercise CI.

References: [cargo-deny checks](https://embarkstudios.github.io/cargo-deny/checks/index.html) and [Gitleaks CLI documentation](https://github.com/gitleaks/gitleaks/blob/master/README.md).

### Verification boundary

Execute local equivalents before requesting remote evidence. A workflow file that parses is not proof that the jobs pass. Without authorized publication or access to native runners, retain the remote/platform gate as pending and record the limitation.

## M01.14: Verification and phase-0 exit gate

### Local command baseline

After the workspace and tools exist, run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo deny check advisories licenses bans sources
git diff --check
```

Also run the pinned Gitleaks CLI according to its verified command syntax, positive/negative ignore checks, and the built binary's contract tests. Use the same documented commands in CI where applicable. Do not report commands as passed merely because they appear in this guide.

### Required test matrix

| Area | Required scenario | Passing evidence |
| --- | --- | --- |
| IDs | Valid round trip and malformed input | Value preserved or typed rejection |
| ID boundaries | Task and workspace IDs cannot be interchanged | Compile-fail or equivalent type-level test |
| Protocol | Version 1 and a different version | Acceptance and typed rejection respectively |
| CLI | All six documented invocations | Expected output class and exit status |
| Placeholder safety | Run in an isolated project | Prompt exit and no generated runtime state |
| Layering | Real dependency graph | Allowed edges and no forbidden core closure |
| Layering checker | Synthetic forbidden edge | Checker fails as expected |
| Ignore policy | Private runtime and public source examples | Runtime ignored; sources remain trackable |
| Documentation | Links, style, examples, command behavior | No broken local links or inaccurate claims |
| Scanners | Clean candidate and synthetic rejected inputs | Clean result and expected negative-control failure |
| Platforms | Linux, macOS, Windows native jobs | Actual job results for the candidate |
| Security channel | Repository setting and entry point | Enabled private reporting without a test submission |

### ADR consistency review

For each of the eight ADRs, record its source requirement, decision, rejected alternative, failure scenario, implementation milestone, and future empirical test. Confirm that daemon ownership, configuration precedence, event ordering, private storage, and reconstruction contracts agree across documents.

Documentation review does not establish actual mutual exclusion, permissions, persistence, or terminal reconstruction. Record these as later tests rather than pretending M01 proved them.

### Completion conditions

M01 closes only when all of the following are evidenced:

1. All eight ADRs exist and are internally consistent.
2. The workspace builds and tests pass.
3. The provisional `rt` contracts pass.
4. Core tests run without TUI modules.
5. Native Linux, macOS, and Windows jobs pass.
6. Formatting, linting, audit, license, and secret checks pass.
7. Positive and negative ignore checks pass.
8. Public documentation describes the actual skeleton.
9. The private security channel is verified.
10. Evidence identifies the tested candidate revision or exact working-tree content and does not borrow success from a different candidate.

Append the exact commands, result summaries, native CI references, and documented limitations to `avances.md` as tasks close. Remove only completed tasks from `TODO.md`. Keep M01.14 open if any required evidence is missing; local implementation may be ready while the milestone remains unverified.

## Session sequence and handoff

| Session | Reviewable outcome |
| --- | --- |
| 1 | Detailed guide and roadmap link |
| 2 | Daemon, IPC, and terminal ADRs |
| 3 | Persistence, environment, agent, and worktree ADRs |
| 4 | Rust/platform ADR and buildable workspace |
| 5 | CLI contracts, identifiers, and dependency enforcement |
| 6 | Ignore rules, public documentation, and security policy |
| 7 | Workflows and verification of their controls |
| 8 | Native platform evidence, corrections, and final gate |

These are work groupings, not duration estimates. Split a task into stable-ID subtasks in `TODO.md` if it cannot be reviewed coherently in one session. Consult the logbook before resuming so completed work is not repeated.

At each handoff, distinguish implemented behavior, executed checks, unavailable evidence, and the next concrete action. Preserve this guide as design/procedure documentation, the roadmap as the pending queue, and the logbook as the append-only evidence record.
