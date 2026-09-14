# Installed quick start

This journey uses one verified, extracted Relayterm candidate, a disposable project, a dedicated private home, and neutral local commands. It requires no Relayterm account, provider account, telemetry, or network access. Git is needed only for the optional worktree step.

## Prepare the disposable workspace

Follow [installation](install.md), then set an absolute executable path. Do not rely on another `rt` found on `PATH`.

On POSIX systems:

```sh
RT="$HOME/.local/opt/relayterm/0.1.0/rt"
project_root="$(mktemp -d)/relayterm-project"
private_root="$(mktemp -d)/relayterm-private"
mkdir -p "$project_root" "$private_root"
chmod 700 "$private_root"
"$RT" --version
"$RT" --workspace "$project_root" --home "$private_root" workspace init --name "Quick start"
```

In PowerShell:

```powershell
$RT = Join-Path $env:LOCALAPPDATA "Relayterm\bin\0.1.0\rt.exe"
$Base = Join-Path ([System.IO.Path]::GetTempPath()) ("relayterm-" + [Guid]::NewGuid().ToString("N"))
$ProjectRoot = Join-Path $Base "project"
$PrivateRoot = Join-Path $Base "private"
New-Item -ItemType Directory -Path $ProjectRoot, $PrivateRoot | Out-Null
& $RT --version
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& $RT --workspace $ProjectRoot --home $PrivateRoot workspace init --name "Quick start"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

From `cmd.exe`, use the same prepared directories and quote every path:

```batch
set "RT=%LOCALAPPDATA%\Relayterm\bin\0.1.0\rt.exe"
"%RT%" --version
if errorlevel 1 exit /b %errorlevel%
"%RT%" --workspace "%TEMP%\relayterm-project" --home "%TEMP%\relayterm-private" workspace init --name "Quick start"
if errorlevel 1 exit /b %errorlevel%
```

Running `rt` without a subcommand in an uninitialized interactive terminal asks before creating private state. The explicit command above is suitable for a reproducible journey.

## Create neutral definitions and sessions

Open the TUI with the same workspace and home:

```sh
"$RT" --workspace "$project_root" --home "$private_root"
```

On the Agents screen, create three definitions for commands available on the machine. Use `/bin/sh` on Unix or `%SystemRoot%\System32\cmd.exe` on Windows for the first. Two additional synthetic interactive executables may be copied test fixtures in a disposable directory. Keep definitions neutral, check availability with `v`, enable with Space, then launch with `a`. No production fake-session switch exists.

On Sessions, confirm oldest-first creation order. Press `n` to name each session. Duplicate names are allowed, clearing restores `Session N`, and the adjacent abbreviated ID distinguishes rows. Confirm the full session and instance IDs in details. PageUp and PageDown move through bounded pages without replacing identity.

Press Enter to attach, `i` to acquire input, and Ctrl-Space to detach. Resize the terminal and reattach. With two TUI clients, let the first retain input and press `i` in the second. The second must remain read-only and show: `Another client controls input. This view remains read-only. Try i after that client releases input.` Release or detach in the first client, then acquire explicitly in the second.

## Coordinate a task and handover

Create a task from Tasks with `n`. The physical cursor must follow the active field through wide and combining text, wrapping, Tab, Shift-Tab, Home, End, Backspace, Delete, multiline input, Ctrl-U, resize, validation failure, and cancellation.

Make the task ready, claim it for the first running instance, and verify a competing claim for the second instance is rejected. Append progress with a nonempty summary and explicit verification. Prepare a structured handover, which atomically closes the first claim and leaves the task `handover_ready`. Claim from the second instance, read the prior progress and handover, then complete the task.

For a stale-form check, open the same editable form in two clients. Save from one client, then submit the other. The losing draft remains intact and no overwrite occurs. Ctrl-R first shows authoritative state for review. A second Ctrl-R adopts its revision while retaining the draft. A later Ctrl-S is the only deliberate resubmission. A result whose delivery is unknown remains separate and is never replayed automatically.

Close and reopen one TUI while the daemon stays alive. Sessions, names, tasks, and terminal state remain available. Stopping the daemon can terminate live sessions, and a daemon or host restart cannot adopt old PTYs. Persisted nonfinal sessions become honestly lost during recovery.

## Exercise an optional worktree

Create a separate disposable Git repository, configure identity only inside it, add one synthetic commit, and initialize that repository as another Relayterm workspace. Create a task, make it ready, use the Worktrees screen to inspect the repository and approve a private parent, then create and select one task worktree. Launch the task in that checkout and verify its working directory and file edits remain separate from the source checkout. Relayterm performs no network Git operation and never deletes the branch or directory automatically.

## Back up and restore

Create a new private backup destination with the daemon active or stopped:

```sh
"$RT" --workspace "$project_root" --home "$private_root" backup create --destination /private/path/relayterm-backup
```

Stop the original workspace before restore. Restore into a fresh private home, start it explicitly, and verify names, tasks, claims, progress, handovers, events, launch snapshots, and worktree metadata. Project files, Git objects, credentials, live processes, terminal state, and scrollback are excluded.

```sh
"$RT" --workspace "$project_root" backup restore --source /private/path/relayterm-backup --destination /private/path/restored-home
"$RT" --workspace "$project_root" --home /private/path/restored-home workspace open
```

Stop only the disposable workspace daemon. Delete temporary projects or private homes only after verifying that their paths belong to this journey. The automated release smoke performs help, version, initialization, detached startup, one real synthetic PTY, and orderly shutdown from the extracted archive. The full interactive steps remain native operator observations.
