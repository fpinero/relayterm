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
