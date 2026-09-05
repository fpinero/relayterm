# Supported platforms and verification evidence

## Bootstrap matrix

Rust 1.98.1 is the pinned compiler and initial MSRV. CI also selects current stable. These are the required initial native targets:

| OS and runner | Target | Bootstrap evidence | Interactive evidence |
| --- | --- | --- | --- |
| Linux, ubuntu-24.04 | x86_64-unknown-linux-gnu | Native stable and pinned-compiler CI passed | Pending M07/M08/M11 |
| macOS, macos-14 | aarch64-apple-darwin | Local tests and native stable CI passed | Pending M07/M08/M11 |
| Windows, windows-2022 | x86_64-pc-windows-msvc | Native stable CI passed | Pending M07/M08/M11 |

The bootstrap code at `0d14f05` passed Quality run `33974825769` and Security run `33974825731` on 2026-09-05. The pinned-toolchain CI job also runs on Linux. Record actual compiler host and runner OS for each candidate. Local macOS evidence does not establish Linux or Windows success, and cross-compilation is not native behavior verification. Refer to `avances.md` for executed checks; do not infer results from the presence of workflow files.

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
