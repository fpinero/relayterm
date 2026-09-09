# 0007: Worktree ownership and recovery

Status: Accepted and implemented by M10.
Task: M01.07 and M10.
Requirements: Sections 6.7, 9.5; FR-7; AC-10.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Create worktrees only on explicit user request through installed Git with argument arrays. Default roots live under private application data. Additional roots require explicit selection and validation. Non-Git workflows remain functional.

## Invariants and behavior

Use task IDs to propose a branch named rt/task-<task-id> and a stable destination; never interpolate a task title into shell commands. Let users select a base reference with HEAD as the default. Validate references with Git, canonicalize allowed roots, and reject traversal, symlink escapes, option-like unsafe inputs, or preexisting path/branch conflicts.

Record a recoverable creation intent before invoking Git. Git success followed by persistence failure is a partial outcome, not permission to delete a worktree. Reconcile the intent and actual Git state on retry/restart before creating anything else. Register only validated, associated worktree paths as session launch roots. Keep operation identifiers stable across retry. M10 fixes concrete state transitions and storage migration before implementation.

The durable creation phases are `prepared`, `applying`, `ready`, `failed`, and `needs_attention`. A receipt stores the operation, workspace, task, worktree and approved-root IDs, the original expected revision, canonical repository and common-directory identities, destination, branch, original base expression, pinned commit, request fingerprint, actor and timestamps. The intent commits in `prepared` before dispatch. A separate commit records `applying` before Git starts. A verified result commits the immutable worktree record, changes the receipt to `ready`, and selects it only if the task still permits association. Uncertain external outcomes remain visible and require bounded explicit reconciliation.

Each worktree belongs permanently to one task and workspace. A task can retain historical worktrees and select at most one ready record. Selection and clearing require a nonfinal, nonactive task without an open claim or a starting or running task-context session. Cancellation can win while Git is running. In that case a proven checkout remains recorded but unselected. No task state changes as a consequence of Git output.

Repository discovery uses Git's checkout top and common directory, including linked `.git` files. Creation requires the workspace root to equal the checkout top. The adapter resolves the base to a local commit before dispatch and invokes `git worktree add -b <branch> --no-track -- <destination> <commit>` as an argument array. Relayterm serializes creation against the common Git directory. It disables hooks, prompts, paging, lazy fetch and inherited Git repository redirection. Checkout filters are rejected.

No automatic branch/worktree deletion, commits, merges, or rebases. Cleanup guidance is manual and must preserve uncommitted work.

## Alternatives

A fictitious transaction spanning Git and SQLite cannot guarantee rollback. Automatic deletion after partial failure risks material data. Free-form names create injection and path hazards.

## Consequences

Some failures need visible reconciliation or manual intervention. Private paths must stay out of ordinary diagnostics. Worktrees are optional to normal use but required in AC-10 tests.

## Verification and ownership

The M10 native gate uses production `rt`, SQLite, authenticated IPC, installed Git and PTY or ConPTY. It creates two task-owned checkouts, verifies independent files and an unchanged source checkout, rejects cross-task selection and selection during a live task session, captures the worktree in the instance snapshot, restarts the daemon, recovers the same receipt without another add, and proves normal non-Git sessions remain available. Platform results are recorded in `docs/supported-platforms.md`.
