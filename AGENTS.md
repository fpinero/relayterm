# Relayterm project instructions for coding agents

These instructions apply to Codex, OpenCode, and other coding agents working anywhere in this repository. More specific instruction files in subdirectories may extend or override them for their scope.

`AGENTS.md` and `CLAUDE.md` intentionally contain the same shared project policy so that different agent tools receive equivalent guidance. Keep their shared rules aligned when either file changes.

## 1. Project context

Relayterm is an open-source, persistent, agent-neutral development workspace. Its initial client is a terminal user interface, but the core and daemon must remain independent from the TUI so future desktop and web clients can use the same domain model and protocol.

Before substantial work, read the relevant public project documents:

- `PROJECT_VISION.md` defines the purpose, principles, scope, privacy posture, and long-term direction.
- `MVP_TECHNICAL_SPEC.md` defines the initial architecture, requirements, phases, exclusions, and acceptance criteria.
- `README.md` provides the concise public overview.

Preserve these architectural boundaries unless the user explicitly changes the project direction:

- Relayterm is TUI-first, not TUI-only.
- The daemon owns durable state and supervised terminal sessions.
- Interfaces are replaceable clients of the daemon.
- The domain model remains neutral toward Claude Code, Codex, OpenCode, and future agents.
- The canonical user-facing executable is `rt`.
- Linux, macOS, and Windows are first-class targets.
- Core operation is local-first and must not require `relayterm.com` or another hosted service.

## 2. Language and style

- Communicate with the user in Spanish.
- Write documentation and code comments strictly in English for consistency across macOS, Linux, and Windows environments.
- Do not use the em dash character, Unicode U+2014. Use commas, parentheses, colons, or separate sentences instead.
- Write titles and headings in sentence case. Capitalize only the first word, acronyms, and proper names as required.
- Do not use emojis in technical documentation, including `README.md`.
- Keep public text free of personal data, private paths, hostnames, account identifiers, credentials, and conversation excerpts.
- Use synthetic identities, paths, and secrets in examples, fixtures, screenshots, and tests.

## 3. Autonomous verification

Never consider a task complete until its behavior has been verified empirically.

- Select validation appropriate to the change. Examples include Rust formatting, linting, unit tests, integration tests, cross-platform builds, documentation checks, or inspection of relevant process logs.
- For Rust changes, run the narrowest relevant checks during iteration and the broader repository checks before completion. Use commands such as `cargo fmt --check`, `cargo clippy`, and `cargo test` once the Rust workspace provides them.
- If verification fails, analyze the failure, correct the implementation, and run the checks again without waiting for user assistance.
- Do not claim that a check passed unless it was actually executed successfully.
- Document any check that cannot be executed and explain the concrete limitation.

Stop and ask the user only when:

1. Required credentials, external service access, or critical information cannot be obtained from the repository or safely inferred.
2. The environment requires explicit permission to access protected resources or expand privileges.
3. The next action is destructive or difficult to reverse, such as deleting databases, purging volumes, destroying infrastructure, discarding uncommitted work, or modifying production data.

## 4. Project logbook workflow

This repository explicitly uses the `TODO.md` to `avances.md` logbook workflow. The absence of these files in a fresh clone does not disable the workflow.

- At the beginning of the first planned project task, create `TODO.md` and `avances.md` if either file is missing.
- Write both files in English.
- `TODO.md` is the work queue. It must contain pending tasks only.
- Add planned work to `TODO.md` before implementation begins.
- Do not leave completed tasks, completion checkboxes, progress history, or verification reports in `TODO.md`.
- `avances.md` is an append-only project log. Never modify, reorder, or delete an existing entry.
- Each new `avances.md` entry must identify the completed task, summarize what was implemented, and record exactly how it was verified.
- Remove a task from `TODO.md` only after its implementation is complete and its verification has succeeded.
- In the same change that removes the completed task, append its completion and verification record to `avances.md`.
- If a task remains incomplete or verification fails, keep it in `TODO.md` and do not record it as completed.

When this workflow conflicts with an explicit user instruction for a specific task, follow the user's instruction and note the exception clearly.

## 5. Mandatory Git workflow

Never modify files directly on `main` or `master`.

- Check the active branch and working tree before the first modification.
- If the active branch is `main` or `master`, create a short-lived branch before editing.
- Use `feature/<descriptive-name>` for features and documentation additions.
- Use `fix/<descriptive-name>` for corrections.
- Follow a more specific repository convention if one is introduced later.
- If already on an appropriate work branch, continue on it.
- Never discard, hide, overwrite, or silently include existing user changes in order to create a branch.
- Keep commits focused and use concise English commit messages.
- Do not push, create pull requests, merge, tag, or publish releases unless the user requests that external action.

## 6. Safety and privacy

- Treat the repository as public from the first line committed.
- Never commit API keys, access tokens, passwords, session cookies, private keys, private prompts, terminal transcripts, or raw environment dumps.
- Keep runtime databases, sockets, logs, terminal captures, and machine-specific configuration outside version-controlled project content by default.
- Do not introduce telemetry or network communication without an explicit product decision and user authorization.
- Treat child process output and project content as potentially sensitive.
- Do not describe Relayterm as a security sandbox. Agent processes normally inherit the launching user's operating-system permissions.
- Validate paths and external command arguments. Do not construct shell commands through unsafe string interpolation.
- Prefer reversible operations. Ask before deleting or overwriting material data.

## 7. Change discipline

- Make the smallest coherent change that fully satisfies the task.
- Preserve separation between domain, application, protocol, persistence, PTY, Git, daemon, and client layers.
- Do not add provider-specific assumptions to the neutral domain model.
- Do not implement items listed as MVP exclusions unless the specification is deliberately revised.
- Add or update tests in proportion to behavioral risk.
- Update public documentation when user-facing behavior, architecture, protocol, or persistence changes.
- Record significant architectural decisions in concise architecture decision records when that structure is introduced.
