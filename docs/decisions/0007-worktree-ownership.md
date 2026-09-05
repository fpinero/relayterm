# 0007: Worktree ownership and recovery

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.07.
Requirements: Sections 6.7, 9.5; FR-7; AC-10.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Create worktrees only on explicit user request through installed Git with argument arrays. Default roots live under private application data. Additional roots require explicit selection and validation. Non-Git workflows remain functional.

## Invariants and behavior

Use task IDs to propose a branch named rt/task-<task-id> and a stable destination; never interpolate a task title into shell commands. Let users select a base reference with HEAD as the default. Validate references with Git, canonicalize allowed roots, and reject traversal, symlink escapes, option-like unsafe inputs, or preexisting path/branch conflicts.

Record a recoverable creation intent before invoking Git. Git success followed by persistence failure is a partial outcome, not permission to delete a worktree. Reconcile the intent and actual Git state on retry/restart before creating anything else. Register only validated, associated worktree paths as session launch roots. Keep operation identifiers stable across retry. M10 fixes concrete state transitions and storage migration before implementation.

No automatic branch/worktree deletion, commits, merges, or rebases. Cleanup guidance is manual and must preserve uncommitted work.

## Alternatives

A fictitious transaction spanning Git and SQLite cannot guarantee rollback. Automatic deletion after partial failure risks material data. Free-form names create injection and path hazards.

## Consequences

Some failures need visible reconciliation or manual intervention. Private paths must stay out of ordinary diagnostics. Worktrees are optional to normal use but required in AC-10 tests.

## Verification and ownership

M10 uses disposable repositories to test two isolated tasks, missing Git, non-Git roots, invalid refs, spaces/Unicode, collisions, symlink escapes, concurrent requests, and interruption after Git succeeds. Verify unrelated session working directories never change.
