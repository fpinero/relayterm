# Supported platforms and verification evidence

## Bootstrap matrix

Rust 1.98.1 is the pinned compiler and initial MSRV. CI also selects current stable. These are the required initial native targets:

| OS and runner | Target | Bootstrap evidence | Interactive evidence |
| --- | --- | --- | --- |
| Linux, ubuntu-24.04 | x86_64-unknown-linux-gnu | Native stable and pinned-compiler CI passed | Pending M07/M08/M11 |
| macOS, macos-14 | aarch64-apple-darwin | Local tests and native stable CI passed | Pending M07/M08/M11 |
| Windows, windows-2022 | x86_64-pc-windows-msvc | Native stable CI passed | Pending M07/M08/M11 |

The bootstrap code at `0d14f05` passed Quality run `33974825769` and Security run `33974825731` on 2026-09-05. The pinned-toolchain CI job also runs on Linux. Record actual compiler host and runner OS for each candidate. Local macOS evidence does not establish Linux or Windows success, and cross-compilation is not native behavior verification. Refer to `avances.md` for executed checks; do not infer results from the presence of workflow files.

## M02 domain and application evidence

Candidate `9a92d2714664e5b6b46b0a438e527747b2cfd643` passed [Quality run 34019822893](https://github.com/fpinero/relayterm/actions/runs/34019822893) and [Security run 34019822888](https://github.com/fpinero/relayterm/actions/runs/34019822888) on 2026-09-06.

| Native runner | Toolchain | Result |
| --- | --- | --- |
| ubuntu-24.04 | Stable | Passed |
| ubuntu-24.04 | 1.98.1 | Passed |
| macos-14 | Stable | Passed |
| windows-2022 | Stable | Passed |

Each quality job passed locked dependency fetch, formatting, all-target checks, Clippy, workspace and core-only tests, build, repository contracts, and whitespace validation. The tests cover task and instance state matrices, claims, atomic handover/loss handling, deterministic transaction conflicts, privacy, and architecture boundaries. Security checks passed dependency/advisory/license/source auditing, Git-history and candidate-source scanning, and negative controls. This closes the M02 domain gate, without establishing durable persistence, IPC transport, or interactive terminal behavior.

## M03 private persistence evidence

Candidate `9b539b9768d658e55551ff9edd10177e5528d514` passed [Quality run 34025433599](https://github.com/fpinero/relayterm/actions/runs/34025433599) and [Security run 34025433476](https://github.com/fpinero/relayterm/actions/runs/34025433476) on 2026-09-06.

| Native runner | Toolchain | Result |
| --- | --- | --- |
| ubuntu-24.04 | Stable | Passed |
| ubuntu-24.04 | 1.98.1 | Passed |
| macos-14 | Stable | Passed |
| windows-2022 | Stable | Passed |

Each quality job passed locked dependency fetch, formatting, all-target checks, Clippy, workspace and core-only tests, build, repository contracts, and whitespace validation. Native tests cover private filesystem permissions, Windows protected ACL ownership and broad-access rejection, bundled SQLite requirements, transactional migrations, durable entity reconstruction, conflicting claims, append-only history, consistent event watermarks, backup reopening, and separate-process continuity. Security checks passed dependency, advisory, license, and source auditing, full-history and candidate-source secret scans, and negative controls. This closes the M03 persistence gate. IPC transport, daemon lifecycle, real process supervision, and interactive terminal behavior remain pending.

## M04 local protocol evidence

Candidate `ec1953e` passed [Quality run 34030632309](https://github.com/fpinero/relayterm/actions/runs/34030632309) and [Security run 34030632344](https://github.com/fpinero/relayterm/actions/runs/34030632344) on 2026-09-06.

| Native runner | Toolchain | Result |
| --- | --- | --- |
| ubuntu-24.04 | Stable | Passed |
| ubuntu-24.04 | 1.98.1 | Passed |
| macos-14 | Stable | Passed |
| windows-2022 | Stable | Passed |

Each quality job passed locked dependency fetch, formatting, all-target checks, Clippy with denied warnings, workspace and core-only tests, build, repository contracts, and whitespace validation. Native tests cover private Unix sockets, symmetric UID policy, protected Windows named-pipe DACL inspection through actual handles, denied access, endpoint collision and cleanup, strict framing, two-client coordination, ordered durable events, coherent snapshot refresh, read reconnection, mutation uncertainty, and continuity across separate server and client processes. Security checks passed dependency, advisory, license, ban, and source auditing, full-history and candidate-source secret scans, and negative controls.

Acceptance follow-up candidate `14ed5ac` passed [Quality run 34032866039](https://github.com/fpinero/relayterm/actions/runs/34032866039) and [Security run 34032866081](https://github.com/fpinero/relayterm/actions/runs/34032866081) on the same native matrix. It adds coalesced event wakeups, persistent bounded subscription queues, correlated event and synchronization controls, deterministic cancellation recovery, cursor-expiry handling, and resource-release tests. An earlier candidate exposed a Windows named-pipe late-response correlation failure. The client now replaces an unusable connection before sending a new request, and the repeated native Windows gate passed.

This closes the M04 local protocol gate. Detached daemon startup, CLI administration, real process supervision, PTY behavior, and the TUI remain pending in M05 and later milestones.

## Planned shell and terminal matrix

| Platform | Shells | Terminals and connections to verify |
| --- | --- | --- |
| Linux | Bash, configured generic shell | xterm-compatible local terminal and OpenSSH |
| macOS | Zsh, Bash | Terminal.app and OpenSSH |
| Windows | PowerShell, cmd.exe | Windows Terminal with ConPTY and supported OpenSSH configurations |

Full-screen redraw, Unicode, resize, focus switching, monochrome use, small-window behavior, and SSH detach/reattach require later empirical tests. No terminal or architecture outside the tested matrix is claimed supported by this bootstrap.

## Prerequisites and limitations

Development requires rustup and native linker/build tools. Git is needed for repository validation and later optional worktree isolation. Provider CLIs, credentials, graphical desktops, and hosted accounts are not needed for automated runtime acceptance.

The current TUI and daemon entry points deliberately return unavailable errors. The future daemon must stay alive for supervised sessions to survive disconnects. Host restart recovery of live processes is outside the MVP.
