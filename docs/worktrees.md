# Git worktrees

Relayterm can create an explicit Git branch and linked worktree for a task. Git remains optional. A non-Git workspace can use tasks, sessions, claims, progress, handovers, agent definitions, and the TUI without worktree support.

## Prerequisites and repository rules

Install Git and make it available as `git` to the daemon. M10 was developed with Git 2.53.0 on macOS. Native CI records the Git version used on Linux, macOS, and Windows for each candidate.

The workspace must be opened at the checkout top. A workspace opened at a repository subdirectory remains usable, but worktree creation reports `unsupported_root`. Relayterm supports ordinary repositories and linked worktree tops. The repository must have a local commit. Detached HEAD is accepted when `HEAD` resolves to a commit. Bare and unborn repositories are rejected without initialization or an invented commit.

Relayterm resolves the requested base locally and pins the resulting commit before creation. It does not fetch missing objects. Partial clones that need a lazy network fetch can therefore fail. Submodules are not initialized recursively. Dirty and untracked source-checkout files are left in place and are not copied into the new checkout.

Checkout filters declared by `.gitattributes` are rejected for creation because they can execute configured filter programs. Relayterm supplies a disabled hooks path, disables interactive prompts, paging, optional maintenance and lazy fetch, and removes Git environment variables that could redirect repository state. It never invokes a shell for Git arguments.

## Create and select a worktree

In the TUI, select a nonfinal task that is not active and press `w`. The form captures the task ID and workspace revision when it opens. Review the base, new branch, approved parent, destination leaf, and stable operation ID, then press `Ctrl-S`. Closing the form before submission creates nothing. An uncertain result retains the operation ID so it can be inspected before any deliberate new request.

The default branch is `rt/task-<task-id>`. The default destination is under Relayterm private application data. Titles never become branch or path input. Branches are new only. Existing branches, existing destinations, option-like values, traversal and invalid portable path leaves are rejected.

Use `o` on the Tasks screen to select the next ready worktree owned by that task. Use `u` to clear the selection. Clearing changes only the association. It never removes files, a Git registration, or a branch. Creation, selection and clearing are rejected while the task is active, claimed, final, or has a starting or running task session.

The administrative commands use authenticated local IPC:

```text
rt worktree inspect
rt worktree create <task-id> --expected-revision <revision> --operation-id <uuid> --branch <branch> --leaf <leaf>
rt worktree list [--task-id <task-id>]
rt worktree operation <operation-id>
rt worktree select <task-id> <worktree-id> --expected-revision <revision>
rt worktree clear <task-id> --expected-revision <revision>
rt worktree reconcile <operation-id> --expected-revision <revision>
```

The create request stores a durable intent before `git worktree add`. Reusing the same operation ID with the same request returns the stored receipt, including after restart. Reusing it with different input is a conflict. Clients may poll `operation`; they must not automatically resend `create` after unknown delivery.

## Launch behavior

A task session with no selection starts in the source checkout. A task with a ready selection starts in that worktree. Relayterm verifies the canonical checkout, common Git directory and expected branch immediately before launch. A missing or mismatched checkout stops the launch. Relayterm does not fall back to the source checkout.

Initial creation also verifies the resolved base commit. Later health checks permit ordinary commits made by the user on the recorded branch; they do not confuse a changed HEAD with a foreign checkout. Detached HEAD, branch replacement and another common Git directory remain failures.

Relayterm runs Git with a small explicit environment, disabled prompts, pager and lazy fetch, and no inherited repository override variables. It rejects configured checkout filters and filter attributes in the selected commit, including nested `.gitattributes`. Branch and destination admission is repeated while holding the per-common-directory Relayterm lock. Read output is drained through bounded memory rather than an unbounded temporary capture.

An explicit launch subdirectory must remain under the effective task root. Relative agent executables are resolved against that same root and working directory. The instance snapshot stores its task, selected worktree, actual working directory, definition snapshot, instance ID and session ID. Later association changes do not retarget an existing process.

## Recovery and manual cleanup

Git and SQLite cannot share one transaction. An operation can therefore remain `applying` or `needs_attention` after interruption. `rt worktree operation` shows its durable receipt. `rt worktree reconcile` requires a captured workspace revision, performs bounded inspection, and finalizes only when the recorded destination, common repository, branch, and pinned initial commit prove the intended result. It never runs `worktree add` again.

Relayterm preserves unprovable directories and branches. It does not delete, prune, repair, move, commit, merge, rebase, reset, stash, fetch, pull or push. Before manual cleanup, inspect every affected checkout for committed, modified and untracked files, then use Git's own documentation and commands deliberately. Relayterm provides no automatic cleanup command.

Ordinary diagnostics and events contain typed status, IDs and state changes. They exclude paths, branch text, base expressions and raw Git output. Authorized list and detail requests include local path and branch information. Pattern checks cannot recognize every possible secret, so do not place credentials in branch names or paths.

## Limits

- Up to 1,024 durable worktrees and intents per workspace.
- Up to 16 approved roots per workspace.
- Destination leaves are at most 128 UTF-8 bytes.
- Branch and base expressions are at most 256 UTF-8 bytes.
- Native path DTOs are at most 8,192 bytes.
- Lists default to 50 items and accept at most 200.
- Git output is bounded to 4 MiB for stdout and 64 KiB for stderr.
- Read-only Git commands have a five-second deadline. Creation has a 60-second deadline.
- A daemon admits at most two Git workers. Repository metadata also has an exclusive creation lock.

The adapter exposes safe status categories rather than raw Git stderr. Resource exhaustion fails explicitly and does not raise a bound or repeat a mutation.
