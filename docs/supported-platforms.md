# Supported platforms and verification evidence

## Bootstrap matrix

Rust 1.98.1 is the pinned compiler and initial MSRV. CI also selects current stable. These are the required initial native targets:

| OS and runner | Target | Bootstrap evidence | Interactive evidence |
| --- | --- | --- | --- |
| Linux, ubuntu-24.04 | x86_64-unknown-linux-gnu | Native stable and pinned-compiler CI passed | Native M07 PTY gate passed; TUI M08/M11 pending |
| macOS, macos-14 | aarch64-apple-darwin | Local tests and native stable CI passed | Native M07 PTY gate passed; TUI M08/M11 pending |
| Windows, windows-2022 | x86_64-pc-windows-msvc | Native stable CI passed | Native M07 PTY gate passed; TUI M08/M11 pending |

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

## M05 daemon and CLI evidence

Candidate `7a4b0f9b590cf98f493eed0e338b2493c159c91b` passed [Quality run 34043677019](https://github.com/fpinero/relayterm/actions/runs/34043677019) and [Security run 34043677020](https://github.com/fpinero/relayterm/actions/runs/34043677020) on 2026-09-06.

| Native runner | Toolchain | Result |
| --- | --- | --- |
| ubuntu-24.04 | Stable | Passed |
| ubuntu-24.04 | 1.98.1 | Passed |
| macos-14 | Stable | Passed |
| windows-2022 | Stable | Passed |

Each quality job passed the dedicated detached daemon lifecycle gate before the complete workspace suite. The process gate initializes disposable Git and non-Git workspaces whose paths contain spaces and Unicode, starts Relayterm from a disposable native shell (`sh` on Unix and PowerShell on Windows), confirms the launcher exits within a monotonic deadline, reconnects from a separate CLI process, preserves a durable task across shutdown and restart, serializes simultaneous starts, and stops the exact reported generation. The Windows runner exposed two bounded-wait defects and one concurrent-start defect in earlier candidates. The final implementation reads one bounded bootstrap response without waiting for pipe EOF, bounds the complete connection handshake, and serializes endpoint probing, process creation, and readiness under a private per-workspace startup lock.

The daemon starts through the absolute `rt` executable with argument arrays and null standard streams. Native CI establishes independent lifetime after the disposable shell process exits. Hosted runners do not provide an interactive terminal window to close, so this evidence does not claim manual Windows Terminal, Terminal.app, or SSH behavior. Those terminal and connection scenarios remain assigned to the later interactive milestones.

Security CI passed dependency, advisory, license, ban, and source checks, full-history and candidate-source secret scans, and negative controls. M05 adds no provider execution, PTY, terminal stream, or production fake-session capability.

## M06 first durable slice evidence

Candidate `ef196d789fe64c1ade206fa5e6273dbc1088abc6` passed [Quality run 34050246932](https://github.com/fpinero/relayterm/actions/runs/34050246932) and [Security run 34050246913](https://github.com/fpinero/relayterm/actions/runs/34050246913) on 2026-09-06. An independent push-triggered repetition passed [Quality run 34050245498](https://github.com/fpinero/relayterm/actions/runs/34050245498) and [Security run 34050245501](https://github.com/fpinero/relayterm/actions/runs/34050245501).

| Native runner | Toolchain | Result |
| --- | --- | --- |
| ubuntu-24.04 | Stable | Passed |
| ubuntu-24.04 | 1.98.1 | Passed |
| macos-14 | Stable | Passed |
| windows-2022 | Stable | Passed |

Every quality job ran the first durable slice twice before the ordinary parallel workspace suite. Separate `rt` processes used real local IPC and SQLite to exercise Git and non-Git project roots, exclusive and competing claims, progress, structured handover, release, continuation, graceful and abrupt test-host loss, production restart reconciliation, ordered events, pagination, privacy, and source-tree cleanliness. The runtime ownership regression proves that a contender cannot reconcile persisted state before acquiring the endpoint.

The same native gate launches the daemon from a console-lifetime fixture, closes the owning console or pseudo-terminal process, reconnects independently, and stops the reported generation. Unix uses a pseudo-terminal allocated by `script`; Windows uses a process created with a new console. This is automated native process and console lifetime evidence. It does not establish Terminal.app, Windows Terminal, SSH, PTY child supervision, full-screen rendering, or reattachment behavior. M07 supplies the native PTY evidence below.

## M07 real PTY supervision evidence

Candidate `61a516abc5d7696b81482ebba605ea709eb0b6d1` passed [Quality run 34065910975](https://github.com/fpinero/relayterm/actions/runs/34065910975) and [Security run 34065910980](https://github.com/fpinero/relayterm/actions/runs/34065910980) on 2026-09-07.

| Native runner | Toolchain | Result |
| --- | --- | --- |
| ubuntu-24.04 | Stable | Passed |
| ubuntu-24.04 | 1.98.1 | Passed |
| macos-14 | Stable | Passed |
| windows-2022 | Stable | Passed |

Every Quality job ran the real PTY gate twice before the complete workspace suite, which ran the gate again. The gate uses Unix PTYs or Windows ConPTY, one native default shell, two neutral interactive fixture sessions, and five additional fixture sessions. It verifies the eight-session capacity and ninth-session rejection, idempotent launch receipts, immutable command and environment assembly, real instance IDs, exclusive claims, progress, atomic handover, ordered continuation, exclusive input leases, resize, high-volume truncation and resnapshot, descendant termination, durable lifecycle observations, and absence of persistent terminal captures.

The terminal-state fixture verifies full-screen redraw, Unicode, dimensions, cursor and input-mode continuity, authoritative cell equality after an independent client reconnect, and correct continuation after raw history truncation. Unix preserves the alternate-screen indicator. ConPTY interprets application VT sequences and exposes the rendered primary-screen representation on the supported Windows runner, so the Windows gate asserts that transformed mode and compares the complete visible state across reattachment. Provider-neutral parser tests independently verify alternate-screen entry and exit, hidden-buffer restoration, split UTF-8 and escape sequences, colors, attributes, cursor operations, invalid input, and exact allocation limits.

The console-lifetime gate now launches three real default-shell PTYs through the detached production daemon. It closes the originating Unix pseudo-terminal owner or Windows console owner, reconnects from a separate `rt` process, reads all three live sessions, and performs explicit bounded cleanup. This proves child survival while the daemon host remains alive. A daemon restart marks unrecoverable sessions lost and does not claim live PTY adoption or terminal-state persistence.

Localhost SSH was probed during M07 validation and refused the connection because no authorized SSH service was available. SSH coverage is therefore not claimed. Terminal.app, Windows Terminal UI, keyboard focus, monochrome rendering, and small-window behavior remain part of the M08 and M11 manual client matrix.

## Planned shell and terminal matrix

| Platform | Shells | Terminals and connections to verify |
| --- | --- | --- |
| Linux | Bash, configured generic shell | xterm-compatible local terminal and OpenSSH |
| macOS | Zsh, Bash | Terminal.app and OpenSSH |
| Windows | PowerShell, cmd.exe | Windows Terminal with ConPTY and supported OpenSSH configurations |

Native PTY full-screen redraw, Unicode, resize, process survival, and reattachment are covered by the M07 gate. TUI focus switching, presentation in named terminal applications, monochrome use, small-window behavior, and SSH detach/reattach require later empirical tests. No terminal or architecture outside the tested matrix is claimed supported.

## Prerequisites and limitations

Development requires rustup and native linker/build tools. Git is needed for repository validation and later optional worktree isolation. Provider CLIs, credentials, graphical desktops, and hosted accounts are not needed for automated runtime acceptance.

The administrative daemon, CLI, real supervised sessions, PTY interaction, and terminal reattachment workflow are available. Invoking `rt` without an administrative command still reports that the TUI is pending. Host restart recovery of live processes is outside the MVP.
