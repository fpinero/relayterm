# M11 Linux Codex Desktop handoff prompt

Communicate with me in Spanish. Write repository documentation and code comments in English. Do not use the Unicode em dash character.

Work in a local Linux clone of `https://github.com/fpinero/relayterm` through Codex Desktop. Git is the source of truth for the code and evidence.

The current M11 work is in draft PR #22:

`https://github.com/fpinero/relayterm/pull/22`

The remote branch is `feature/m11-hardening`. It already contains the completed macOS and Windows manual matrices and their corrections. Do not assume a fixed branch revision because the branch may advance after this prompt is written. Fetch first and report the actual remote head. Preserve the completed macOS and Windows evidence. Repeat an already completed observation only if a later source change affects the behavior it covered.

Start by reading the nearest `AGENTS.md`, `CLAUDE.md`, `TODO.md`, `avances.md`, `docs/M11_details.md`, `docs/reliability-workflow.md`, `docs/acceptance-matrix.md`, `docs/tui-workflow.md`, `docs/m11-macos-manual-observations.md`, and `docs/m11-windows-manual-observations.md`. Treat their M11 requirements as authoritative. Preserve unrelated local changes and never discard, hide, overwrite, or force-push them.

Inspect the repository before changing anything:

1. Run `git status --short --branch`.
2. Run `git fetch origin`.
3. If the checkout is clean, switch to `feature/m11-hardening` and update it with a fast-forward-only pull from `origin/feature/m11-hardening`.
4. If the checkout has local changes or has diverged, inspect and reconcile them without loss before continuing.
5. Record `git rev-parse HEAD`, `git log -5 --oneline`, `rustc --version --verbose`, `cargo --version`, `git --version`, `uname -a`, the relevant fields from `/etc/os-release`, the graphical terminal name and version, `$TERM`, `$COLORTERM` when present, the login shell and tested shell versions, `ssh -V`, and `rt --version` or the fixed candidate's `--version` output.

## Objective

Complete the three still-pending Linux manual observations for M11.07b using only synthetic data:

- MAN-LNX-1: Bash in an identified local xterm-compatible terminal.
- MAN-LNX-2: the configured generic shell in the same local terminal.
- MAN-LNX-3: a fresh OpenSSH connection to Linux, recording the client terminal, SSH client, server Linux version, and actual server shell.

Guide me interactively in Spanish, one short action at a time. Wait for my result after each manual action, interpret it, and record only what was actually observed. You may use reliable routine and reversible desktop control, but let me type passwords and handle security prompts. Never expose credentials, usernames, private paths, hostnames, unrelated projects, environment values, or personal data in screenshots or repository evidence.

Use a new isolated Relayterm home and a synthetic workspace for the Linux observations. Do not reuse personal projects. Keep the tested binary fixed for all three rows unless a defect requires a source correction. Record the full source commit and the Linux binary SHA-256. If source behavior changes, rerun every affected observation and mark earlier evidence as superseded rather than silently reusing it.

Before starting MAN-LNX-2, inspect how the installed candidate resolves a generic shell on this Linux environment. Record the actual executable and version. Do not silently substitute an unrelated shell or claim a distinct combination when it is identical to the Bash row. If the environment cannot provide the required configured generic-shell combination, keep MAN-LNX-2 pending and state the exact blocker.

## Required behavior for each local row

For Bash and the configured generic shell, demonstrate and record:

1. Open or initialize the isolated workspace in the identified xterm-compatible terminal.
2. Create and switch among three live synthetic shell sessions, preserving distinct output and stable session and instance identities.
3. Enter and exit a real full-screen alternate-screen console program available on the machine, preferably Vim when installed.
4. Render a wide Unicode character and a combining-character example correctly.
5. Resize rapidly through wide, narrow, tall, and below-minimum dimensions, then recover the normal screen and retained full-screen content.
6. Leave terminal input mode using the documented focus escape and open keyboard help.
7. Confirm that state remains understandable with no-color mode and does not depend on color alone.
8. Use two open Relayterm clients on the same session to verify one WRITER, one READ ONLY observer, rejected competing acquisition, shared output, explicit release, and ownership transfer in both directions.
9. Reproduce a stale task form conflict with synthetic titles. Confirm overwrite protection, retained local draft, explicit discard, and the saved authoritative value. The generic rejection message is already recorded as the non-blocking `UX-CONFLICT-MESSAGE` follow-up.
10. Detach and reattach while retaining the same child identities and context.
11. Exit normally with both `q` and the documented Ctrl-C navigation path, checking restoration of the outer shell prompt, cursor, input, and screen.
12. Terminate only the Relayterm client in a controlled recoverable-failure observation, then confirm outer-terminal restoration and that the daemon and children remain alive. Do not claim restoration by a process that was forcibly killed before it could clean up.
13. Close the actual graphical terminal window that hosts Relayterm. Open a new window, reconnect, and prove that the same daemon, child identities, and terminal context survived.
14. Use separate read-only administrative checks before and after disruptive observations to verify daemon generation, workspace revision, synthetic task identity, and session and instance identities.

Do not stop the daemon while proving terminal-window or client survival. Do not use `daemon stop --terminate-sessions` unless a separate authoritative step requires it and the isolated target has been checked immediately beforehand.

## Required behavior for the OpenSSH row

First inspect whether an OpenSSH server is already installed, configured, running, and authorized on the Linux target. Do not install or enable `sshd`, change its startup policy, modify authentication, open firewall ports, or broaden account access without my explicit authorization in the Linux session. A loopback connection is acceptable if it is a real fresh OpenSSH connection to the Linux server.

Through the authorized SSH connection:

1. Record the client terminal, SSH client version, server Linux version, and actual server shell.
2. Launch the same fixed Relayterm candidate against the isolated SSH workspace.
3. Confirm idle stability for more than 60 seconds with no transport-loss diagnostics, then verify input still works.
4. Cover full-screen entry and exit, Unicode, resize, minimum-size recovery, focus escape, help, color-independent state, three-session switching, exclusive input, form conflict protection, detach, and reattach.
5. End one connection normally and reconnect through a fresh SSH process, verifying the same daemon, children, identities, and context.
6. Close the actual client terminal window while SSH and Relayterm are active. Establish a new SSH connection from a new window and verify that the same children survived and that the abandoned connection's input lease was released.
7. Perform a controlled recoverable Relayterm-client failure inside SSH and confirm that the remote shell remains usable. Distinguish client restoration from operating-system or terminal recovery after a forced process kill.

Shell exit alone is not evidence of actual connection or window closure. Take screenshots only when useful, with synthetic content. Raw recordings are unnecessary.

## Defect handling and verification

If a behavioral defect appears, reproduce it with a focused native regression, fix it on `feature/m11-hardening`, run the affected tests and mandatory repository checks, and repeat every affected manual observation. Preserve failed evidence as superseded. Do not weaken a deadline, threshold, assertion, or test to obtain a pass.

For any behavior-changing candidate, run the narrow regressions during iteration and the complete required checks before handoff. At minimum, use the exact commands required by the current repository documents, including formatting, workspace check, Clippy with warnings denied, the workspace tests, workspace build, repository policy and audit controls, and the separately required M11 native gates. Do not claim a check that was not executed successfully.

If the Linux observations require only documentation changes, still run the repository documentation, policy, secret, and whitespace checks identified by the current branch. Final Quality and Security must run against the eventual final PR head.

## Evidence and repository updates

Create `docs/m11-linux-manual-observations.md` in English and record dated, sanitized evidence for every completed step. Include expected and actual results, observer confirmation, exact versions, UTC date, full source commit, Linux binary SHA-256, and the identity comparisons used around closures and failures. Do not include local screenshot paths, personal identifiers, private hostnames, credentials, or private filesystem paths.

Update `docs/acceptance-matrix.md` only for rows actually completed. Remove M11.07b from `TODO.md` only after all three Linux rows pass. Append a new entry to `avances.md`; do not edit or delete earlier entries. Keep M11.07e, final reconciliation, independent review, final CI, and merge tasks open.

After all three Linux rows pass, complete M11.07e only by reconciling the whole manual matrix. Confirm that no source correction invalidated macOS or Windows evidence. If a correction did affect prior behavior, keep the corresponding row open until it is repeated on that operating system.

Commit coherent changes to `feature/m11-hardening` and push that branch only after successful verification. Do not merge PR #22 yet. Keep it as a draft until the complete manual matrix, risks R1-R11, budgets, final Quality and Security results, and required independent review have all been reconciled. The independent reviewer must review the final evidence commit and cannot be replaced by self-approval.

At handoff, report the actual branch head, commits, files changed, commands executed, manual rows passed or still pending, any superseded observations, unresolved limitations, CI links, and the exact remaining blocker before PR #22 can be marked ready.
