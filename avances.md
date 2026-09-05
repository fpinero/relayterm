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
