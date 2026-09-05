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
