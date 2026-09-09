# Project log

## 2026-09-01: Ignore JetBrains project metadata

Added the `.idea/` directory to the repository ignore rules so local JetBrains project metadata remains outside version control.

Verification:

- `git check-ignore -v .idea/vcs.xml .idea/workspace.xml`
- `git diff --check`
- `rg -n '\x{2014}' .gitignore AGENTS.md CLAUDE.md TODO.md avances.md` (no matches)

## 2026-09-05: Design the initial MVP implementation roadmap

Completed ROADMAP-01. Expanded `TODO.md` into 12 dependency-ordered milestones and 100 individually identified pending tasks, grounded in `PROJECT_VISION.md` and `MVP_TECHNICAL_SPEC.md`. Added planning gates for unresolved architectural details, per-task verification expectations, milestone exit gates, session handoff rules, and coverage of all 16 MVP acceptance criteria. Preserved the first durable domain/SQLite/IPC slice before production PTY and TUI implementation, cross-platform validation, provider neutrality, privacy boundaries, and explicit MVP exclusions. Deferred optional export and persistent scrollback. Identified the existing license/status discrepancy and future maintainer decisions without changing the product specification.

Verification:

- Read the project vision, full MVP specification, README, repository agent instructions, existing queue, and logbook; manually cross-checked the roadmap against the six implementation phases, eight required bootstrap ADRs, domain fields, protocol operations, functional/non-functional requirements, exclusions, and acceptance criteria.
- Ran an inline Ruby document validator: confirmed 12 ordered milestones, 100 unique sequential task IDs, valid task references, existing earlier milestone dependencies, first tasks for planning gates, all 16 acceptance coverage rows, and valid local Markdown links. It also checked for completed checkboxes, prohibited punctuation, and private path markers. All checks passed after correcting the validator to parse only the dependency clause, excluding later coverage prose.
- `git diff --check` passed.
- This task changes planning documentation only. No Rust build or runtime behavior was claimed as verified; the repository has no Rust workspace yet.

## 2026-09-05: Document the detailed M01 implementation plan

Completed M01-DOC. Added `docs/M01_details.md` in English and linked it from M01 in `TODO.md`. The guide covers M01.01 through M01.14, eight ADR contracts, the six-crate skeleton, observable CLI behavior, documentation, CI, task dependencies, test scenarios, session sequencing, and final acceptance evidence. It records one daemon per workspace and GitHub Private vulnerability reporting as the selected defaults while keeping activation and native CI verification explicit. All 100 existing implementation tasks remain pending and unchanged.

Verification:

- Ran an inline Ruby document validator: confirmed all 14 task sections appear exactly once in order, all task references exist in the queue, all local Markdown links resolve, the destination filename is correct, and code fences are balanced. Punctuation, completed-checkbox, and private-path/identity marker checks passed.
- Compared every existing implementation task line in `TODO.md` with `git show HEAD:TODO.md`; all 100 remained unchanged.
- Reviewed the guide against the approved plan, including the separation between saving the plan and implementing the milestone, future runtime verification boundaries, and the requirement to leave unverified gates open.
- `git diff --check` passed. No Rust, runtime, remote CI, or GitHub reporting-setting verification is claimed by this documentation task.

## 2026-09-05: Implement the M01 local foundations

Completed M01.01 through M01.11. The detailed-plan branch was committed as `d896afd` and fast-forwarded into local `main` before implementation began on `feature/m01-foundations`.

Deliverables:

- M01.01-M01.07: Seven ADRs define per-workspace daemon ownership, local IPC, bounded terminal reconstruction, transactional SQLite recovery, environment/privacy policy, neutral configuration precedence, and recoverable worktree creation. Each identifies requirements, alternatives, invariants, consequences, and future empirical tests. Runtime behavior remains assigned to later milestones.
- M01.08: ADR 0008 pins the officially published stable Rust 1.98.1 as compiler/MSRV with edition 2024 and resolver 3. Added the workspace lockfile and verified selected package metadata: uuid 1.26.0, clap 4.6.6, and test-only serde_json 1.0.151 use approved MIT/Apache-2.0 licensing.
- M01.09: Six-crate skeleton with distinct UUID-based task/workspace IDs, a typed protocol version check, separate unavailable daemon/TUI entry points, and `rt` help/version/dispatch. Integration tests check six command outcomes, bounded termination, isolated-directory side effects, and dependency direction with synthetic forbidden edges. One compile-fail doctest proves IDs cannot be interchanged.
- M01.10: Added runtime-private ignore patterns and automated positive/negative checks that keep source, migrations, public fixtures, and lockfiles visible.
- M01.11: Added contribution, privacy, platform, and architecture documentation; updated README to describe actual placeholder behavior; reconciled the vision with the existing Apache-2.0 license without changing `LICENSE`.

Verification on native macOS aarch64 with Rust 1.98.1:

- `cargo fetch --locked` populated the complete graph, including other-target dependencies needed by offline architecture inspection.
- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed. The suite contains seven test functions and one compile-fail doctest; the CLI contract test covers six invocations.
- `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked` passed independently of UI selection.
- `python3 scripts/check_repository.py` passed: 21 Markdown files, eight ADR structures, candidate links/style, pending-queue checks, and positive/negative ignore cases.
- cargo-deny 0.20.2 fetched the public advisory database. After adding explicit versions to internal path dependencies, `cargo deny --offline --locked check advisories licenses bans sources` passed. Only unused license-allowance warnings remained; no vulnerability exception was added.
- Gitleaks 8.30.1: `gitleaks git --redact --no-banner .` found no leaks in the five-commit history. `python3 scripts/check_secrets.py` passed the current-source scan and rejected its disposable synthetic token. `python3 scripts/check_audit_controls.py` confirmed license rejection and dependency-ban failures using temporary policy copies.
- Official archive SHA-256 values were checked for the local audit tools. An inline Ruby validator parsed both workflow YAML files, checked full action SHA pins, and ran `bash -n` on explicit Bash steps. `git diff --check` passed.
- Reviewed ADR consistency against the detailed plan and specification. Documentation verification does not claim that future IPC, SQLite, PTY, or worktree behavior has been implemented.

Remaining gates, not completed: M01.12, M01.13, and M01.14. A read-only GitHub API check confirmed private vulnerability reporting is disabled. `SECURITY.md` states that limitation honestly. Quality/security workflows are prepared and locally checked, but no branch was pushed and no native remote CI run was claimed. Linux/Windows behavior and the phase-0 gate remain unverified. Rust and audit tools were installed into an isolated temporary tool directory without editing shell configuration.

## 2026-09-05: Enable private security reporting

Completed M01.12 after explicit maintainer authorization. Enabled GitHub Private vulnerability reporting and updated `SECURITY.md` to describe the operational channel. Published `feature/m01-foundations` to start the authorized native CI verification; M01.13 and M01.14 remain pending until their runs pass.

Verification: the GitHub repository private-vulnerability-reporting API returned `enabled: true` after activation. A read-only fetch of the repository security advisories page confirmed the Report a vulnerability link to the new-advisory entry point. No vulnerability report was submitted. `python3 scripts/check_repository.py` and `git diff --check` validate the policy/queue update before publication.

## 2026-09-05: Verify native CI and close M01

Completed M01.13 and M01.14. The authorized branch publication ran all quality and security workflows. The first native Windows test exposed executable-name inference in Clap: subcommand help displayed `rt.exe daemon`. Commit `0d14f05` sets `bin_name = "rt"`, preserving the canonical help contract across platforms. The existing six-invocation CLI regression test passed locally and on Windows after the correction.

Verification for candidate `0d14f05eb218f23d837e6f1cdcb73f5bc4e66cdd`:

- Quality run `33974825769` completed successfully. All four jobs passed: `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Each ran locked graph fetch, formatting, all-target checks, Clippy with denied warnings, workspace tests, core-only tests, build, repository contracts, and whitespace checks.
- Security run `33974825731` completed successfully: verified tool archives, dependency/advisory/license/source checks, full-history secret scan, audit negative controls, and candidate-source scanner negative control.
- Private reporting was enabled and its reporting entry point verified in M01.12. Eight ADRs, the private-state ignore policy, public documentation, and dependency boundaries were verified by the earlier local checks and the native repository-contract jobs.
- Updated the platform evidence and removed the completed milestone from `TODO.md`; M02 is the next pending milestone. The closure changes affect documentation only and will receive their own branch CI runs before handoff.

M01 establishes the bootstrap only. Durable coordination, live IPC, PTY sessions, and the real TUI remain unimplemented and require their later milestone acceptance tests. No PR, merge of the implementation branch, tag, or release was performed.

## 2026-09-05: Clarify merge-to-main authorization

Completed GUIDE-01. Updated the shared Git rules in `AGENTS.md` and `CLAUDE.md`: a request to merge into main includes pushing the resulting main branch to origin/main unless the user explicitly requests a local-only operation. The rule requires divergence checks, remote synchronization verification, and monitoring of triggered CI, without authorizing force-pushes, branch deletion, PR creation, or releases.

Verification: `python3 scripts/check_repository.py` and `git diff --check` passed. An inline Ruby comparison confirmed the shared policy from section 1 onward is identical in both guides; their existing tool-specific introductions remain intact.

## 2026-09-05: Save the detailed M02 plan

Completed M02-DOC. Added `docs/M02_details.md` with actor permissions, task and instance transition tables, claim and recovery rules, informational dependencies, model and validation contracts, transaction and event semantics, atomic tasks M02.01-M02.09, and required implementation verification. Linked the document from M02 in `TODO.md`, preserving its pre-existing first-line edit. Specification changes and all M02 implementation tasks remain pending; this entry records only the documentation delivery.

Verification: `python3 scripts/check_repository.py` passed for 22 Markdown files, candidate links/style, eight ADRs, queue rules, and ignore boundaries. An inline Python check confirmed all nine task sections and pending queue entries, the task and instance transition rows, validation limits, the milestone link, the preserved first-line edit, and the unchanged pre-existing logbook before appending this entry. Reviewed the document for English prose and sentence-case headings. `git diff --check` passed. No Rust behavior changed, and no implementation or cross-platform test result is claimed.

## 2026-09-05: Implement the M02 domain and application core

Completed M02.01-M02.08 on the existing M02 feature branch, preserving the pre-existing first-line queue edit and all earlier log entries.

- M02.01: Updated specification sections 5.3 and 5.4 with explicit instance/task transition tables, informational dependencies, cancellation of any non-final task, actor attribution, claim exclusivity, atomic handover/loss handling, and test-only pre-PTY simulation.
- M02.02: Added distinct IDs, checked UTC timestamps, validated boundary records and immutable entity views, neutral definitions, UTF-8/list/path limits, safe errors, and credential-pattern checks. Locked time 0.3.55 and enabled explicit Serde representations. Preserved M01 ID construction/parsing APIs.
- M02.03-M02.04: Implemented task editing and dedicated state commands, ownership guards, unique open claims per task/instance, immutable closure history, explicitly attributed user intervention, and informational dependency cycles with same-workspace validation.
- M02.05-M02.06: Implemented instance/session lifecycle with last-observation time, confirmed final observations, atomic task blocking on loss/exit, append-only progress and handovers, historical user corrections, and ordered bounded history queries. Production exposes no fake-process CLI or test adapter.
- M02.07: Added pending and confirmed typed events, payload version 1, safe decoding, typed field-change names, enumerated reasons, and content-free event serialization. IPC version remains 1.
- M02.08: Added generic asynchronous store/transaction ports with Send futures, injected clock/IDs, opaque write batches, revision conflicts, and post-commit notification outcomes. Added test-only in-memory storage, deterministic doubles, two-transaction races, and failure injection. Domain/application remain independent of concrete adapters; architecture checks additionally forbid Tokio and git2 in the core closure.
- Implemented the M02.09 continuity and recovery test journeys and the M03 record/reference/limit/transaction inventory in `docs/M02_storage_contract.md`. Updated README and linked the inventory from the plan and queue.

Verification on native macOS aarch64 with the pinned Rust 1.98.1 toolchain:

- `cargo test -p relayterm-domain --locked`, `cargo test -p relayterm-application --locked`, and `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked` passed.
- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed. The workspace now executes 30 test functions and three compile-fail doctests. Matrix tests exercise all 49 task pairs and 36 instance pairs with valid surrounding preconditions and unchanged state/events on rejection.
- Tests verified continuity across two instances, loss recovery and repeated observations, claim exclusivity and actor guards, immutable historical corrections, dependency cycles and foreign references, constructor/reconstruction failures, exact byte/list/handover limits, terminal dimensions, event variants/version rejection, safe decoding, pagination, reversed clocks, atomic rollback, duplicate event rejection, transaction races without sleeps, and notifier failure after commit. Windows environment-name comparison has a native-platform assertion; local execution verifies the macOS branch only.
- `cargo deny check advisories licenses bans sources` passed after an authorized public RustSec database refresh. Only unused license-allowance warnings remained. `python3 scripts/check_audit_controls.py` passed its license rejection and dependency-ban negative controls.
- `python3 scripts/check_repository.py` passed for 23 Markdown files, references/style, queue rules, eight ADRs, and ignore boundaries. `python3 scripts/check_secrets.py` passed candidate-source scanning and its synthetic negative control. `git diff --check` passed.
- Reviewed field coverage against specification section 5 and documented additional workspace/session/closure/last-observation metadata. Checked the queue's original first-line edit and append-only preservation of the existing logbook.

M02.09 remains open only for native Linux/Windows CI evidence for this candidate. The existing quality matrix includes those platforms, but no branch publication or remote CI run was authorized or claimed. M02 does not claim durable SQLite persistence, restart survival, live IPC, real process supervision, or a working TUI. No commit, push, PR, merge, tag, or release was performed in this implementation session.

## 2026-09-06: Verify native CI and close M02

Completed M02.09 after authorization to merge the current branch into main. Published implementation candidate `9a92d2714664e5b6b46b0a438e527747b2cfd643` from `feature/m02-details`, retaining the pre-existing first-line edit of `TODO.md` only in the local working tree.

Verification:

- Quality run `34019822893` completed successfully on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Every job passed locked fetch, formatting, all-target checks, Clippy, workspace/core-only tests, build, repository contracts, and whitespace validation.
- Security run `34019822888` completed successfully: dependency/advisory/license/source audits, Git-history and candidate-source secret scans, and negative controls passed.
- Before publication, `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `python3 scripts/check_repository.py`, and `git diff --check` passed again locally. The tested workspace contains 30 test functions and three compile-fail doctests, including the 49 task pairs and 36 instance pairs.
- Updated native platform evidence and the M02 contract, removed the completed milestone from the queue, and moved remaining sequencing/acceptance coverage to M03 onward. The closure documentation is checked with `python3 scripts/check_repository.py` and `git diff --check` before integration.

M02 is complete. Durable storage, live IPC, real process supervision, and TUI behavior remain the responsibilities of later milestones. The authorized main integration and its resulting CI will be verified separately after publication.

## 2026-09-06: Prepare the exhaustive M03 plan

Completed M03-DOC. Created `docs/M03_details.md` and linked it from M03 in `TODO.md`. The English plan covers M03.01-M03.09 with 53 atomic child tasks, private locations and native permissions, canonical identity, recoverable registry initialization, explicit configuration import/reload, required M02 application extensions, relational schemas and lossless codecs, atomic SQLite commits, bounded queries and watermarks, migrations, safe recovery, separate-process tests, native CI gates, and later-milestone handoffs. Consulted official SQLite, SQLx, and directory-library documentation and included supporting references, including the patched SQLite WAL requirement.

Verification: `python3 scripts/check_repository.py` passed for 24 Markdown files, candidate links/style, eight ADRs, queue rules, and ignore boundaries. An inline Python check confirmed all nine M03 sections, 53 unique child tasks, required contract topics, the milestone link, all nine implementation tasks still pending, and an unchanged existing logbook before this append. Reviewed the plan against the current M02 service, record inventory, specification, and accepted ADRs. `git diff --check` passed. No Rust code, dependency, migration, or existing architecture policy changed; implementation tests were not claimed. All M03 implementation tasks remain pending.

## 2026-09-06: Implement and verify M03 private persistence

Completed M03.01-M03.09.

- M03.01: Added side-effect-free private location resolution, explicit `RELAYTERM_HOME` overrides, native path codecs, canonical workspace identities, filesystem identity guards, bounded lock acquisition, Unix ownership and mode checks, and Windows protected ACL creation and validation. Windows ACL operations use file handles with explicit security access rights and reject unprotected or broadly accessible objects without exposing paths or SIDs.
- M03.02: Added bounded versioned TOML parsing, stable definition imports and updates, typed definition events, revision-aware atomic application, safe diagnostics, and immutable launch-definition snapshots.
- M03.03-M03.05: Added the SQLx SQLite adapter, pinned bundled SQLite support, checked migration ledgers, strict codecs, foreign keys and indexes, explicit new/reopen/read-only modes, durable registry coordination, complete validated reconstruction, revision conflicts, exclusive claims, atomic entity/event commits, and categorized failure results.
- M03.06-M03.07: Persisted append-only progress, corrections, handovers, claims, sessions, launch snapshots, and lifecycle observations. Added bounded ordered task history, consistent snapshot and event watermarks, strict event cursors, and repeatable final-observation no-ops.
- M03.08: Added migration checksum and version rejection, transactional rollback fixtures, concurrent initialization, cancellation and write-failure recovery, pool reuse, SQLite-consistent backup reopening, and privacy checks for diagnostics and serialized events.
- M03.09: Added real-file and separate-process continuity journeys for handover, competing claims, workspace aliases, lost-instance recovery, reopen behavior, and durable event ordering. Updated the architecture controls, specification, ADRs, storage contract, dependency policy, and platform evidence.

Local verification on native macOS aarch64 with Rust 1.98.1:

- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo test -p relayterm-domain --locked`, `cargo test -p relayterm-application --locked`, and `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked` passed.
- `cargo check -p relayterm-platform --target x86_64-pc-windows-msvc --all-targets --locked` and `cargo clippy -p relayterm-platform --target x86_64-pc-windows-msvc --all-targets --locked -- -D warnings` passed as cross-compilation checks. Native behavior is established separately by CI.
- `cargo deny check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, the verified Gitleaks full-history scan, and `git diff --check` passed.

Candidate `9b539b9768d658e55551ff9edd10177e5528d514` passed Quality run `34025433599` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Every job passed locked fetch, formatting, all-target checks, Clippy with denied warnings, workspace and core-only tests, build, repository contracts, and whitespace checks. Security run `34025433476` passed dependency, advisory, license, and source audits, full-history and candidate-source secret scans, and audit negative controls.

M03 is complete. Relayterm now provides durable private coordination state and deterministic SQLite recovery across process boundaries. Local IPC, daemon lifecycle, real process supervision, PTY behavior, and the TUI remain pending in M04 and later milestones.

## 2026-09-06: Implement and verify M04 local protocol and synchronization

Completed M04.01-M04.09.

- M04.01: Reconciled ADR 0002, specification section 6.2, architecture boundaries, public operation ownership, revision preconditions, synchronization, uncertainty, local transport security, and the M05/M07 handoffs.
- M04.02: Added explicit version-1 wire scalars, entity DTOs, stable operations and errors, strict bounded JSON with duplicate-key and nesting rejection, incremental seven-byte framing, lossless native paths, and the reserved binary terminal envelope.
- M04.03-M04.04: Added Tokio Unix sockets with private endpoint locking, mode and UID checks, stale-socket recovery, identity-safe cleanup, and no TCP fallback. Added Interprocess 2.4.4 Windows byte-mode named pipes with a protected current-user-only DACL, explicit owner, disabled remote access, non-inheritable handles, bounded instances, first-instance collision protection, actual-handle security validation, and native allow/deny tests.
- M04.05-M04.06: Added the reusable workspace server, complete public `LocalUser` dispatch, transaction-bound revision checks, compact mutation receipts, bounded reads, safe error mapping, validated unavailable session/worktree operations, and retained the M05 daemon entry point as unavailable.
- M04.07-M04.08: Added revision-checked bounded snapshot pages, ordered durable event polling and replay, subscription cleanup, shared client handshake and monotonic request IDs, staged atomic snapshots, visible stale/current state, bounded reconnect with subscription resumption, and explicit mutation uncertainty without automatic mutation replay.
- M04.09: Added real SQLite and local IPC integration gates. One test composes two clients against the reusable server; another launches separate server, handover client, and completion client processes. The suite verifies exclusive claims, progress, handover continuity, event order, response loss after commit, exact recovery without duplication, malformed-peer isolation, reserved-operation side-effect exclusion, storage reopen, and architecture/privacy controls.

Local verification on native macOS aarch64:

- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked` passed. The workspace includes 76 passing test functions and four compile-fail doctests, with two process-helper tests intentionally ignored except when invoked by their parent process gates.
- `cargo check -p relayterm-ipc --target x86_64-pc-windows-msvc --tests --locked` passed as an additional compile check. Native Windows behavior was established by CI.
- `cargo deny check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner .`, and `git diff --check` passed. The reviewed 0BSD license used by two Interprocess dependencies was added to the explicit allowlist.

Candidate `ec1953e` passed Quality run `34030632309` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Each job passed formatting, all-target checks, Clippy, workspace and core-only tests, build, repository contracts, and whitespace checks. Security run `34030632344` passed dependency, advisory, license, ban, and source checks, full-history and candidate-source secret scans, and negative controls. The first Linux candidate exposed a macOS-specific `/private/tmp` test path; commit `ec1953e` selected a short native temporary root per Unix platform, and the complete matrix then passed.

M04 is complete. Relayterm now provides authenticated current-user local IPC, a strict versioned protocol, reusable service dispatch, coherent client snapshots, ordered durable events, safe reconnect behavior, and explicit uncertain mutation recovery. Detached daemon lifetime and administrative CLI composition remain M05 work. Real process supervision, PTY streams, terminal reconstruction, and reattachment remain M07 work.

## 2026-09-06: Close M04 subscription acceptance gaps

Completed the M04.07c-M04.07g, M04.08c, and M04.09d follow-up audit.

- Connected coalesced application commit notifications to each subscription while retaining the 250 millisecond durable fallback poll.
- Added persistent per-subscription item and byte budgets, ordered `caught_up` and `resnapshot_required` controls, subscription and request correlation, and pending-frame cleanup before unsubscribe acknowledgement.
- Added explicit client cancellation before and after writer ownership. Mutations cancelled after writer ownership report an unknown result, complete server dispatch, replace the unusable connection, resubscribe from the last contiguous sequence, and never replay the mutation automatically.
- Added deterministic gates for snapshot-to-subscribe replay, live notification wakeups, cancellation after commit, exact durable recovery, cursor expiry, malformed parameter survival, queue exhaustion, frame cleanup, permit release, and continued service to unaffected clients.

Local verification on native macOS aarch64:

- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo test -p relayterm-protocol -p relayterm-client -p relayterm-daemon --locked` passed, including the real SQLite and local IPC protocol gates.
- `cargo check -p relayterm-protocol -p relayterm-client -p relayterm-daemon --lib --target x86_64-pc-windows-msvc --locked` passed. A whole-workspace cross-check could not compile bundled SQLite C without a Windows SDK, so native Windows behavior remained a CI requirement.
- `cargo deny --locked check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner .`, and `git diff --check` passed.

Candidate `14ed5ac` passed Quality run `34032866039` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Security run `34032866081` passed the dependency, advisory, license, ban, source, secret, and negative-control gates. An earlier Windows run exposed that a cancelled named-pipe write half could still deliver a late response to the next request. Commit `14ed5ac` makes every subsequent request replace a connection already marked disconnected, and both repeated native Windows jobs passed.

The M04 acceptance follow-up is complete. The next pending milestone is M05.

## 2026-09-06: Prepare the M05 execution contract

Completed M05-DOC by creating `docs/M05_details.md` and linking it from the pending M05 queue. The plan specifies daemon-side bootstrap IPC, non-creating discovery, singleton ownership, native detachment, readiness, transaction-preserving shutdown, restart reconciliation, administrative commands, explicit configuration import, safe bounded diagnostics, and atomic M05.01-M05.07 implementation tasks. It defines targeted concurrency, cancellation, privacy, recovery, and native acceptance evidence while preserving M06 and M07 scope.

Verification: manually cross-reviewed the plan against the specification, ADRs, M04 contract, and current registry, server, client, configuration, and CLI boundaries. `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. A focused Python check verified all seven M05 task groups and required contract sections, preserved pending implementation tasks, ASCII document content, and the unchanged existing logbook before this append. No Rust implementation was changed and no M05 implementation task was closed. Native runtime checks listed in the plan remain requirements for the implementing agent.

## 2026-09-06: Implement and verify M05 daemon lifecycle and administrative CLI

Completed M05.01-M05.07.

- M05.01-M05.02: Added the finite typed bootstrap exchange, non-creating workspace discovery, production daemon composition, private per-workspace startup serialization, exclusive endpoint ownership, opaque runtime generations, bounded readiness, and native detached launch through the absolute `rt` executable. Unix uses a new session. Windows uses a detached process group. The long-lived process receives argument arrays and null standard streams.
- M05.03: Added ready, draining, and stopped admission states; generation-targeted shutdown; writer-confirmed acknowledgements; joined connection and writer ownership; completion of admitted transactions after client disconnect; bounded client observation; persistent fragmented-frame readers; and cooperative signal handling. M05 production supervision is explicitly empty until M07 provides real process handles.
- M05.04: Reconciled persisted starting and running instances before readiness through the existing atomic application observation. Active claims close, owned tasks become blocked, final records remain unchanged, repeated recovery is a no-op, and invalid or failed recovery prevents readiness.
- M05.05: Implemented the complete administrative CLI inventory for workspaces, daemon lifecycle, definitions, explicit TOML import, tasks, claims, progress, handovers, histories, sessions, and events. Added native path selection, strict bounded file/stdin input, versioned JSON output, escaped human output, stable exit codes, revision preconditions, event streaming cancellation, and explicit uncertain-mutation guidance. Public commands act only as `LocalUser` through authenticated IPC.
- M05.06: Added private allowlisted JSON diagnostics with bounded records, 1 MiB rotation, one active plus three retained files, safe destination checks, and postcommit failure isolation. Logs exclude request bodies, entity narratives, commands, environment values, paths, terminal data, and raw error chains.
- M05.07: Added the daemon and CLI guide, updated the specification and architecture decisions, and added native process gates for disposable Git and non-Git roots with spaces and Unicode. The gate proves launcher exit, separate-client reconnection, durable restart, simultaneous-start convergence, exact-generation stop, and source-tree cleanliness without adding PTY, TUI, provider execution, or production fake sessions.

Local verification on native macOS aarch64:

- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo test -p relayterm-cli --locked -- --nocapture --test-threads=1` and the exact detached lifecycle gate passed with native file locks and local sockets enabled.
- `cargo deny --locked check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner .`, and `git diff --check` passed. Cargo Deny retained its existing non-failing duplicate-package and unused-license-allowance warnings.

Candidate `7a4b0f9b590cf98f493eed0e338b2493c159c91b` passed Quality run `34043677019` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Every job passed locked fetch, formatting, all-target checks, Clippy with denied warnings, the dedicated detached lifecycle test, the complete workspace suite, core-only tests, build, repository contracts, and whitespace checks. Security run `34043677020` passed dependency, advisory, license, ban, source, secret, and negative-control gates. Earlier Windows candidates exposed unbounded waits around inherited pipe EOF, connection handshakes, and simultaneous starts. The final candidate uses bounded single-response reads, a bounded complete handshake, deadline-bound launcher checks, and serialized startup.

M05 is complete. Relayterm can now operate durable coordination state through an independently running daemon and its administrative CLI. The M06 durable end-to-end slice, real process supervision, PTY behavior, terminal reattachment, and the TUI remain pending.

## 2026-09-06: Prepare the M06 execution contract

Completed M06-DOC by creating `docs/M06_details.md` and linking it from the pending M06 queue. The plan defines 18 atomic child tasks for a bounded process harness, test-only synthetic lifecycle observations, actual administrative CLI and SQLite/IPC continuity, exclusive claims, structured handover, explicit release, production restart, honest lost-session reconciliation, coherent reads, event history, privacy and native CI acceptance. It identifies prerequisite ownership and I/O deadline investigations and the difference between recorded M05 shell-exit evidence and its required terminal-close evidence. These are implementation and verification requirements, not newly established runtime results.

Verification: cross-reviewed the contract against specification sections 5 and 18, M05 requirements, platform evidence, and the current runtime, CLI, application and integration-test boundaries. `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. A focused Python check verified local document links, ASCII content, all 18 child IDs, all four pending M06 parent tasks and unchanged prior logbook bytes before this append. No Rust code was changed, no runtime test result is claimed, and no M06 implementation task was closed.

## 2026-09-06: Implement and verify M06 first durable slice

Completed M06.01-M06.04 and all 18 atomic child tasks.

- M06.01: Moved endpoint ownership ahead of storage opening and startup recovery, so a second runtime cannot mutate a live owner's persisted sessions, claims, tasks, revisions, or events. Bounded the complete CLI bootstrap response and child-exit operation under one monotonic deadline. Added isolated Git and non-Git fixtures, bounded child ownership and output, deterministic readiness, actual concurrent clients, and a test-executable lifecycle host that shares production composition without exposing synthetic controls in `rt`.
- M06.02: Added a process-level coordination journey through the real `rt` executable, native IPC, and SQLite. Separate clients register definitions, create tasks, transition to ready, enforce task and instance claim exclusivity, append progress, reject an invalid handover without effects, prepare an atomic structured handover, read context afresh, continue with a second instance, and complete explicitly. Additional cases cover release, rejected repeated release, blocked-to-ready recovery, informational dependencies, cross-workspace rejection, bounded history pages, and a simultaneous two-client claim race with exactly one winner.
- M06.03: Added graceful and abrupt test-host loss followed by production daemon restart. Startup reconciliation marks nonfinal synthetic instances lost, closes active claims for instance end, blocks owned tasks atomically, preserves completed work and immutable history, and is a no-op on the next restart. A new instance must explicitly reopen and claim recovered work. The gate compares ordered durable event prefixes and suffixes, independent client reads, bounded pagination, narrative canaries, and complete project directory contents.
- M06.04: Published `docs/first-durable-slice.md`, added the named native CI step with two repetitions, and updated public status and platform evidence. The console-lifetime gate launches below a Unix pseudo-terminal or a new Windows console, closes that owning process, reconnects from an independent `rt` process, and stops the exact daemon generation. It establishes the inherited M05 lifetime condition on native CI without claiming real PTY child supervision, Terminal.app, Windows Terminal, SSH, full-screen rendering, or reattachment.

Local verification on native macOS aarch64:

- `cargo test -p relayterm-cli --test durable_slice --locked -- --test-threads=1 --nocapture`, `cargo test -p relayterm-daemon --test runtime_gate --locked`, `cargo test -p relayterm-daemon --test protocol_gate --locked`, `cargo test -p relayterm-cli --test cli --locked`, and `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked` passed.
- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo deny --locked check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner .`, and `git diff --check` passed. Cargo Deny retained its existing non-failing duplicate-package and unused-license-allowance warnings.

Candidate `ef196d789fe64c1ade206fa5e6273dbc1088abc6` passed Quality run `34050246932` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Every job passed locked fetch, formatting, all-target checks, Clippy with denied warnings, the detached daemon gate, two consecutive durable-slice runs, the ordinary parallel workspace suite, core-only tests, build, repository contracts, and whitespace checks. Security run `34050246913` passed dependency, advisory, license, ban, source, secret, and negative-control gates. Independent push-triggered runs `34050245498` and `34050245501` passed the same matrices and controls.

M06 is complete. Relayterm now proves durable coordination continuity across separate client and daemon processes, SQLite, native local IPC, graceful and abrupt test-host loss, production restart, and console-independent daemon lifetime. Real process supervision, PTY streams, terminal reconstruction, SSH behavior, reattachment, and the TUI remain M07 and later work.


## 2026-09-06: Prepare the M07 execution contract

Completed M07-DOC by creating `docs/M07_details.md` and linking it from the pending M07 queue. The contract defines 30 atomic child tasks covering the native PTY and terminal-state prototype, real interactive fixtures, validated launch and environment assembly, spawn/persistence failure handling, process lifetime, binary ingestion, bounded screen reconstruction, input leases, resize, termination, fair queues, protocol compatibility, administrative clients, and the three-platform acceptance gate. It separates inherited daemon console evidence from the new requirement to preserve real PTY children and records the native evidence and merge stop conditions.

Verification: cross-reviewed the plan against the pending ten M07 parent tasks, ADRs 0003 and 0005, the existing reserved protocol shapes, runtime composition, architecture boundaries, and native CI matrix. `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. A focused Python check verified ASCII content, local links, 30 unique atomic child IDs, preservation of all ten pending parents, and unchanged previous logbook bytes before this append. No Rust code changed, no new runtime evidence is claimed, and no M07 implementation task was closed.

## 2026-09-07: Implement and verify M07 real PTY supervision and reattachment

Completed M07.01-M07.10 and all 30 atomic child tasks.

- M07.01-M07.03: Prototyped native PTY handle ownership and provider-neutral terminal reconstruction, recorded the resulting decisions in ADR 0003, and added `relayterm-pty` and `relayterm-terminal` behind enforced architecture boundaries. Launch uses immutable definition or native-shell snapshots, native paths, argument arrays, validated working directories, an explicit environment allowlist, and secret-free errors. The test-only interactive fixture has no production simulation entry point.
- M07.04: Added an eight-session supervisor with reserved admission, idempotent launch receipts, starting and running persistence, compensating failure observations, real session and instance identifiers, bounded readers and writers, native wait ownership, and atomic final lifecycle observations through application services. Startup still marks persisted nonfinal instances lost and does not adopt stored process identifiers.
- M07.05-M07.08: Added bounded binary ingestion, `vt100` 0.16.2 parser state, 160,000-cell admission, bounded raw history and scrollback, truncation resynchronization, final-state retention, exact attachment snapshots, continuation offsets, per-connection attachment identity, exclusive input leases, ordered input sequences, coherent resize, per-session and global budgets, fair output rounds, independent control capacity, slow-consumer isolation, and bounded worker teardown. Parser tests cover split UTF-8 and CSI input, alternate-screen restoration, cursor state, attributes, wide and combining text, invalid input, unterminated controls, and exact boundaries.
- M07.06 and M07.09: Added verified native process-tree termination, confirmed exit observations, busy shutdown refusal, explicit `--terminate-sessions` cleanup, versioned and capability-negotiated session operations, typed client methods, real terminal frames, and administrative `rt session` create, list, status, attach metadata, bounded output, input, resize, detach, and terminate commands. Input accepts bounded file or stdin bytes and diagnostics never print raw terminal control data by default.
- M07.09d and M07.10: The real process gate coordinates a task through actual supervised instance IDs, two independent clients, native IPC, and SQLite. It rejects a competing claim, appends progress, prepares and closes an atomic handover, resumes through a second instance, and completes without deriving task state from terminal output. The native gate also launches one platform shell, two neutral interactive fixtures, and five capacity fixtures; rejects a ninth session; verifies environment filtering, output, full-screen redraw, Unicode, resize, reattachment, leases, high-volume truncation, descendant cleanup, failure isolation, and absence of persistent terminal captures. The console-lifetime gate launches three real default-shell PTYs, closes the originating console owner, reconnects independently, reads all three sessions, and performs explicit cleanup.
- The Windows prototype reproduced indefinite ConPTY cleanup with upstream `portable-pty` cursor inheritance. M07 pins the API-compatible `portable-pty-psmux` 0.9.7 adapter, which omits that flag, closes and joins writers before pseudo-console teardown, and serializes bulk cleanup. The Windows PowerShell fixture explicitly enables VT output. ConPTY exposes a rendered primary-screen representation rather than preserving the application's private alternate-screen indicator on the supported runner, so the native gate asserts that transformation and compares cells, cursor, dimensions, and modes across reattachment. Unix PTYs preserve the alternate-screen indicator. CI uses documented 30-second marker and 120-second scenario deadlines on hosted Windows because process security scanning produced measured startup delays.

Local verification on native macOS aarch64:

- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo test -p relayterm-daemon --test pty_gate --locked -- --nocapture --test-threads=1` passed twice consecutively with real native PTYs. The complete workspace suite independently passed the same gate and the three-session console-lifetime gate.
- `cargo deny --locked check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner .`, and `git diff --check` passed. Cargo Deny retained its existing non-failing duplicate-package and unused-license-allowance warnings.

Candidate `61a516abc5d7696b81482ebba605ea709eb0b6d1` passed Quality run `34065910975` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Every job passed formatting, all-target checks, Clippy with denied warnings, detached daemon lifecycle, the first durable slice twice, the real PTY gate twice, the complete workspace suite, core-only tests, build, repository contracts, and whitespace checks. Security run `34065910980` passed dependency, advisory, license, ban, source, full-history secret, candidate-source secret, and negative-control checks.

Localhost SSH refused the connection because no authorized SSH server was available, so optional SSH coverage is not claimed. M07 is complete for the required native matrix. Relayterm now supervises and reattaches to bounded real PTYs while the daemon remains alive. Host-restart adoption of live PTYs, persistent recordings, provider templates, worktrees, and the TUI remain outside M07. The next pending milestone is M08.

## 2026-09-07: Prepare the M08 execution contract

Completed M08-DOC by creating `docs/M08_details.md` and linking it from the pending M08 queue. The contract defines 36 atomic child tasks covering workspace entry, terminal lifecycle, bounded presentation and effects, coordination forms, authoritative terminal display and input, reconnect uncertainty, privacy, native TUI journeys and delivery. It identifies inherited shared-stream starvation, snapshot reconstruction and size limits, stale-edit protection, history exposure and independently failing CI repetitions as prerequisites to investigate and verify.

Verification: cross-reviewed the ten pending M08 parents, specification section 6.6, the M07 contract, relevant ADRs, current CLI/client/protocol/terminal boundaries and native workflow. `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. A focused Python check verified ASCII content, 36 unique child IDs, preservation of all ten pending parent tasks and unchanged previous logbook bytes before this append. No Rust code was changed, no runtime evidence is newly claimed, and no M08 implementation task was closed.

## 2026-09-07: Implement and verify M08 interactive TUI workflow

Completed M08.01, M08.02, M08.03, M08.04, M08.05, M08.06, M08.07, M08.08, M08.09, M08.10 and all 36 atomic child tasks.

- M08.01-M08.04: Added the bounded `relayterm-tui` client layer, screen and focus model, stable identity selections, fair independent input and event queues, coherent snapshot refresh, text-first status presentation, compact and subminimum layouts, and an idempotent raw-mode, alternate-screen and cursor lifecycle. The no-command `rt` path now validates terminal support before side effects, prompts for explicit workspace initialization, reuses the production daemon bootstrap, and preserves every administrative command and output contract.
- M08.05-M08.06: Added byte-aware task forms, transitions, claims, release, progress, structured handover, immutable history and contextual continuation through existing `LocalUser` operations. Revision checks reject stale edits, invalid handovers preserve state, competing claims show bounded context, final-task annotations do not reopen work, and uncertain writes are never retried automatically.
- M08.07: Added the capability-negotiated protocol-version-1 `session.read_display` operation. The daemon returns an authoritative bounded parsed viewport with neutral compact cells, cursor, modes, terminal revision, raw retention boundary and private parsed history offsets. The TUI renders these cells without replaying raw bytes or implementing a second terminal parser. It supports native key and bracketed-paste encoding, exclusive input leases, read-only attachments, focus escape, ordered resize, detach, session switching, lifecycle reporting and confirmed termination.
- M08.08-M08.09: Added bounded reconnect state, fresh generation and attachment discovery, draft preservation, explicit uncertainty, safe nonterminal text, redacted diagnostics, keyboard help and English workflow documentation. Client drafts, events, invalidations, input, cells, scrollback and diagnostics all have stated item or byte ceilings. Terminal frames, arguments, environment values and private paths do not enter diagnostics or SQLite.
- M08.10: Added a native outer-terminal gate that drives the actual `rt` executable through Unix PTY or Windows ConPTY against real SQLite and current-user IPC. It initializes a Unicode workspace, coordinates a task across two real fixture instances, uses a third native shell session, rejects a competing claim, appends progress, performs invalid and valid handovers, releases and resumes work, exercises input, resize, parsed history, high-volume truncation, client close and independent reattachment, verifies fresh child output and performs bounded owned cleanup. Each CI repetition is a separate step whose exit status propagates directly.
- The inherited Windows failure in run `34128844579` came from fixture readiness through an extra child-process path. The gate now launches the Rust test executable directly. Subsequent Windows candidates exposed startup assumptions, blocked input, unstable index selection and outer ConPTY screen-capture heuristics. The final implementation accepts Windows startup without a Unix device-status response, uses a dedicated input reader, preserves selection by UUID across snapshots, sends decoded native arrow keys, and distinguishes authoritative selection and IPC state from incomplete outer-capture row heuristics. Marker deadlines were not increased and failed repetitions were not retried into a pass.

Local verification on native macOS aarch64:

- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test -p relayterm-tui --locked`, `cargo test -p relayterm-client -p relayterm-terminal -p relayterm-protocol --locked`, `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- The exact native TUI gate passed repeatedly. The final local run reported startup median/p95/max of 161/181/193 ms over 20 samples after warm-up, navigation 30/33/36 ms over 100 samples, input-to-rendered-echo 139/151/155 ms over 100 samples, and 5,270,723 bytes/s during the bounded flood.
- `cargo deny --locked check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner .`, and `git diff --check` passed. Cargo Deny retained its existing non-failing duplicate-package and unused-license-allowance warnings.

Code candidate `acae16d37545b6acf7ae9484f0cd3bfc1e935db0` passed Quality run `34156535639` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. The stable Linux, macOS and Windows jobs each passed two separate TUI gate invocations, two real PTY gates, two durable-slice gates, the complete workspace suite, core-only tests, build and repository checks. Security run `34156535749` passed dependency, advisory, license, source, full-history and candidate secret scans and negative controls. Independent push-triggered Quality run `34156533278` and Security run `34156533242` passed the same final candidate.

The two stable-runner TUI passes reported startup p95 values of 364/365 ms on Linux, 475/509 ms on macOS and 395/394 ms on Windows. Navigation p95 was 21/21 ms, 164/167 ms and 22/21 ms respectively. Input p95 was 143/143 ms, 266/290 ms and 207/186 ms. Flood throughput remained above 2.65 MiB/s on every pass. Linux and Windows met the documented reference latency targets. The shared macOS runner reported `reference_target_met=false` while remaining within the documented hosted-runner guardrails; repeated local macOS runs met both reference targets. Exact metrics and methods are recorded in `docs/supported-platforms.md` and `docs/tui-workflow.md`.

Hosted CI provides native PTY and ConPTY automation but does not establish manual behavior in Terminal.app, the Windows Terminal graphical interface, xterm or SSH. No authorized SSH service or named-emulator session was available, so those optional observations remain assigned to M11. M08 is complete for its mandatory native matrix. Provider templates, worktrees, persistent recordings, live PTY adoption after daemon restart, M09 and M10 remain outside this milestone.

## 2026-09-08: Prepare the M09 execution contract

Completed M09-DOC by creating `docs/M09_details.md` and linking it from the pending M09 queue. The contract specifies 30 atomic child tasks for editable neutral templates, generic TUI definition management, credential-free executable checks, launch snapshot consistency, arbitrary CLI documentation and native continuity acceptance. It records source-inspected concurrency and selection risks that require reproduction, explicit template-copy versus TOML-import semantics, argument fidelity, bounded resolution and uncertainty handling. Official provider command references were consulted on the planning date and must be rechecked during implementation.

Verification: reviewed the five pending M09 parents against FR-2, FR-8, AC-14 and AC-15, the existing configuration, application, daemon, PTY, protocol and TUI contracts, relevant ADRs and M08 evidence. A focused Python check verified 30 unique child IDs, local links, ASCII text and unchanged previous logbook bytes. `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. This is documentation only: no Rust code changed, no M09 implementation task was closed, and no new runtime or native-platform evidence is claimed.

## 2026-09-08: Implement and verify M09 editable agent templates

Completed M09.01-M09.05 and all 30 atomic child tasks.

- M09.01: Added disabled, version-1 TOML templates for Claude Code, Codex, and OpenCode through the neutral `AgentDefinition` schema. The bounded embedded catalog parses the same files used for explicit import, exposes stable template keys through authenticated capability-negotiated IPC, and does not register definitions automatically. Official provider documentation was rechecked for the `claude`, `codex`, and `opencode` executable names. ADR 0006 now distinguishes copying a template into a new identity from stable-ID TOML import and records catalog upgrade behavior.
- M09.02: Added generic TUI definition listing, selection by stable ID, create, copy, edit, enable, disable, check, and launch actions. Forms preserve ordered, duplicate, empty, quoted, spaced, and Unicode arguments; capture their target and base revision when opened; reject stale writes without retargeting; and reconcile uncertain delivery without automatic resubmission. Updates require an existing ID, while explicit TOML import retains revision-checked stable-ID upsert semantics. Launch admission, returned instance and session IDs, persisted immutable snapshots, and executed arguments now come from one pinned definition revision.
- M09.03: Added one provider-neutral executable resolver shared by credential-free availability checks and supervised launch. It applies bounded PATH and working-directory rules, native Unix executable checks, explicit Windows suffix and launcher behavior, four resolver jobs plus sixteen bounded waiters, revision-aware results, and launch-time revalidation without shell interpolation. The authenticated protocol and `rt agent check` expose typed safe statuses and documented exit codes without returning commands, environment values, arguments, paths, or injected input in diagnostics.
- M09.04: Published `docs/agent-templates.md` with the account-free template and arbitrary-CLI workflow, provider-owned authentication, environment-name handling, resolver rules, Windows limitations, privacy boundaries, and the statement that agent output never mutates task state automatically. README, TUI, daemon, protocol, ADR, and platform references describe the same provider-neutral contract without adding provider APIs, installation, credential access, worktrees, or production simulation.
- M09.05: Added a real test-only unknown interactive executable and a native outer-terminal gate. The actual `rt` TUI creates and edits the definition, checks missing and available commands, launches exact argv, attaches keyboard input, claims a task, records progress, preserves the first launch snapshot across editing, launches a replacement, performs atomic handover and explicit completion, reconnects another client, and verifies durable definitions after daemon restart. The journey uses real SQLite, current-user IPC, PTY or ConPTY sessions, and bounded owned cleanup. CI runs two separate repetitions whose failures propagate independently.
- The inherited risk review reproduced stale-form, absent-ID update, empty-selection, launch/edit race, snapshot, returned-ID, and Windows restart defects. Stable selections and revision checks address the UI cases. Transactional revision pinning addresses launch consistency. The Windows restart investigation showed that a failed endpoint handshake did not prove the old daemon had released lifetime ownership. The runtime now holds a private per-workspace lock from before storage and recovery until supervised cleanup, explicit database-pool closure, and endpoint release; `daemon stop` waits for that release. The native harness reads one bounded newline-terminated JSON response instead of waiting for pipe EOF that a detached Windows child can retain. Deadlines and output limits remain unchanged.

Local verification on native macOS aarch64:

- `cargo test -p relayterm-cli --test agent_templates_gate --locked -- --nocapture --test-threads=1` passed twice consecutively after the final harness correction, in 4.29 seconds and 4.26 seconds. The reported availability checks were 20 to 30 ms for the unavailable command and 30 ms for the available command.
- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
- `cargo deny check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner --exit-code 1`, and `git diff --check` passed. Cargo Deny retained its existing non-failing duplicate-package and unused-license-allowance warnings.

Candidate `b0e8bf622c7c9c28ecfdcdba35d04e1fdbb262ca` passed Quality run `34247393166` on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. Every stable job passed two separate agent-template gate repetitions after all inherited durable-slice, real PTY, and TUI repetitions; the complete workspace suite independently passed the M09 gate again. Security run `34247393116` passed dependency, advisory, license, ban, source, full-history secret, candidate-source secret, and negative-control checks.

Optional installed and authenticated provider observations were not run and are not required for account-free acceptance. M09 is complete for the mandatory native matrix. Relayterm now gives built-in templates and an unfamiliar custom interactive CLI the same editable configuration, availability, launch, supervision, task, handover, and persistence paths. M10 worktree isolation remains pending and no M10 implementation was started.

## 2026-09-09: Prepare the M10 execution contract

Completed M10-DOC by creating `docs/M10_details.md` and linking it from the pending M10 queue. The contract specifies 36 atomic children under the six existing parents: native Git discovery and bounded commands, validated roots and branches, durable creation intents and receipts, conservative partial-outcome reconciliation, task ownership and selection, versioned persistence and IPC, immutable task-session launch context, real TUI/CLI controls, and native two-worktree acceptance. It requires investigation of installed migration resources, relative executable containment, stale forms, common-repository races, Git checkout hooks/filters and shutdown ownership. No automatic deletion or unrelated Git automation is authorized.

Verification: cross-reviewed the plan against FR-7, FR-8, AC-10, specification sections 6.7 and 9.5, ADRs 0002/0004/0005/0007, and the actual domain, application, SQL, protocol, launch, resolver and architecture-test boundaries. Consulted official Git worktree, rev-parse, check-ref-format and environment documentation on the planning date. A focused Python check verified ten contract sections, 36 unique child IDs, all six implementation parents still pending, ASCII text and byte-identical prior logbook content. `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. This delivery changes documentation only; it closes no M10 implementation task and claims no new native runtime evidence.

## 2026-09-09: Implement and verify M10 explicit Git worktree isolation

Completed M10.01-M10.05, M10.06a-M10.06f, and the corresponding 35 atomic child tasks. M10.06g remains pending until the PR is merged and its post-merge CI is verified.

- M10.01-M10.03: Added the bounded `relayterm-git` adapter after testing real local Git behavior in disposable repositories. It performs argument-array discovery, local commit resolution, NUL-delimited worktree parsing, new-branch creation, source-checkout preservation, explicit hooks and checkout-filter policy, output and time limits, portable branch and destination validation, canonical approved-parent containment, and a per-common-directory creation lock. The adapter does not invoke a shell, network helper, deletion, prune, repair, commit, merge, rebase, or move operation.
- M10.04: Added validated approved-root, creation-intent, receipt, worktree, task-selection, and immutable launch-snapshot records. Migration `0002_worktrees.sql` is embedded in the executable and preserves older workspace data. Stable operation IDs are committed before Git runs, duplicate payloads return the existing receipt, conflicting payloads fail, metadata-only events omit paths and refs, and an explicit reconciliation verifies a partial external result without another add or destructive compensation.
- M10.05: Activated additive `worktrees_v1` IPC operations and administrative `rt worktree inspect`, `create`, `list`, `operation`, `select`, `clear`, and `reconcile` commands. The real TUI captures task identity, workspace revision, operation identity, base, branch, parent, and leaf in a bounded draft; creates and selects through authenticated IPC; retains uncertain receipts; and launches task sessions only in the selected, freshly verified checkout. Worktree, working directory, task, definition, instance, and session identities are persisted from one launch admission. Generic non-Git workspaces continue to launch without Git.
- M10.06a-M10.06c: Published `docs/worktrees.md` and updated the specification, README, daemon, TUI, privacy, platform, architecture, protocol, persistence, and ADR contracts. Added a production-composition gate that drives the real TUI, `rt`, SQLite, native IPC, Git, and PTY or ConPTY to create two task-owned checkouts and launch isolated sessions. The gate preserves source HEAD and dirty files, rejects cross-task selection and live-session reassignment, verifies restart continuity and stable receipts, and completes the non-Git workflow. A focused recovery gate injects failure after confirmed `git worktree add`, reopens the real database, and finalizes through explicit reconciliation while proving Git still lists exactly two worktrees.
- M10.06d-M10.06f: The stable native CI matrix ran both M10 journeys twice in independent steps on Linux, macOS, and Windows. Candidate `f57ce9a948da5ee3e89c0143b55aadbb3430e031` passed Quality run `34350013365` and Security run `34350013342`. Linux used Git 2.55.0 and reported TUI runs of 1.331 and 1.311 seconds plus administrative runs of 1.592 and 1.590 seconds. macOS used Git 2.55.0 and reported 5.255/6.211 and 5.758/6.231 seconds. Windows used Git 2.55.0.windows.5 and reported 6.806/6.197 and 6.218/6.236 seconds. All inherited M06-M09 repetitions, workspace checks, build, repository controls, and Security controls passed in the same candidate.
- Native Windows investigation reproduced MSVC large-enum linting, inherited pipe lifetime, and Git for Windows path failures. The final code boxes the large command payload, reads administrative output through a bounded newline response, redirects Git reads to bounded temporary regular files, and translates Rust verbatim disk and UNC paths through exact UTF-16 only at the Git argument boundary. Failed candidates remained visible, deadlines were not extended, and no rejected creation was retried into a pass.

Local verification on native macOS aarch64 with Rust 1.98.1 and Git 2.53.0:

- `cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked`, `cargo test -p relayterm-git --locked`, the focused worktree recovery gate, and repeated real M10 gates passed during iteration. The final `cargo test --workspace --locked` passed with native IPC access and independently exercised the worktree, recovery, TUI, PTY, durable-slice, template, persistence, protocol, privacy, and architecture tests.
- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --locked`, and `cargo check -p relayterm-git --tests --target x86_64-pc-windows-msvc --locked` passed.
- `cargo deny check advisories licenses bans sources`, `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, `gitleaks git --redact --no-banner --exit-code 1`, and `git diff --check` passed. Cargo Deny retained its existing non-failing duplicate-package and unused-license-allowance warnings.

M10 implementation and mandatory native acceptance are complete. Automatic worktree cleanup, destructive Git automation, persistent terminal recordings, live PTY adoption after daemon restart, and the manual named-terminal and SSH matrix remain outside M10. No M11 implementation was started.

## 2026-09-09: Deliver M10 to main

Completed M10.06g. PR [#19](https://github.com/fpinero/relayterm/pull/19) merged the verified worktree implementation into `main` as commit `c4af056898449f32a4e2347f771922a62847e031`. Local `main` was fast-forwarded to that commit and matched `origin/main` byte for byte before this append-only delivery record was prepared.

The post-merge [Quality run 34352914098](https://github.com/fpinero/relayterm/actions/runs/34352914098) passed on native `ubuntu-24.04 / stable`, `ubuntu-24.04 / 1.98.1`, `macos-14 / stable`, and `windows-2022 / stable`. It retained two independent stable-runner repetitions of the M06 durable slice, M07 real PTY supervision, M08 TUI workflow, M09 agent-template journey, and M10 Git worktree isolation, followed by the complete workspace, core, build, repository, and whitespace checks. The post-merge [Security run 34352914181](https://github.com/fpinero/relayterm/actions/runs/34352914181) passed dependency, advisory, license, source, secret, and negative controls. M10 is closed, and the next pending milestone is M11.

## 2026-09-09: Prepare the M11 hardening execution contract

Completed M11-DOC. Added `docs/M11_details.md` and its localized link in TODO.md. The plan defines 50 atomic implementation tasks under M11.01-M11.10, an AC/FR/NFR evidence matrix, eleven inherited source-risk investigations, real integrated and fault journeys, predeclared performance/resource budgets, native offline evidence, mandatory named-terminal and SSH observations, private SQLite backup and non-destructive restore, threat review, and conditional delivery gates. Source observations are explicitly distinguished from reproduced defects. M11 implementation remains pending, and M12 distribution work is not included.

Verification: `python3 scripts/check_repository.py`, `python3 scripts/check_audit_controls.py`, `python3 scripts/check_secrets.py`, and `git diff --check` passed. A focused document check confirmed 50 unique child IDs across all ten parents, existing local Markdown targets, absence of U+2014, and preservation of every prior logbook byte. Reviewed the plan against the current TODO, specification, ADRs, workflow, public guides, and relevant Git/SQLite implementation. No production code or test behavior changed, and no M11 runtime acceptance is claimed by this documentary entry.
