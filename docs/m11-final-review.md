# M11 final technical review

Status: M11 is complete. The owner approved delivery after assistant-assisted technical review. PR #22 is merged, post-merge Quality and Security passed, and the M12 handoff is prepared.

## Reviewed candidate

The final source candidate is `d6efd7f1370c6e98aa558b1014494aa1ed61c6d3`. It is 67 changed files and 67 commits ahead of `origin/main` commit `5e57c04e872b20f79c60a70943aa2572b4fa6d15`. [PR #22](https://github.com/fpinero/relayterm/pull/22) was mergeable with a clean merge state when this review was recorded.

The final source correction makes the silent Git status and worktree-add helpers terminate their isolated residual process groups after either success or bounded failure. A focused Unix regression exercises both paths, and the existing platform regression continues to exercise captured-output Git commands. CI runs the applicable regressions twice on each stable native target.

This evidence record is a documentation-only descendant of the source candidate. The post-manual source correction changes only the Git adapter and its CI containment command. It does not change terminal rendering, session supervision, input leases, daemon detachment, local IPC, shell launch, or SSH behavior. The nine earlier manual rows therefore remain applicable. The automated worktree and Git gates were repeated on the final source candidate.

## Acceptance reconciliation

Every AC-1 through AC-16 row in [the acceptance matrix](acceptance-matrix.md) is `Passed` or `Passed for M11`. The `Passed for M11` qualification retains only clean-machine installation, PATH conflict, packaging, and release work assigned to M12. FR-10 export remains deliberately deferred and was not redefined as database backup.

All nine mandatory manual rows passed with dated, candidate-mapped observations:

| Platform | Local observations | Remote observation | Result |
| --- | --- | --- | --- |
| Linux | GNOME Terminal with Bash and configured Dash | Fresh loopback OpenSSH connection | Passed |
| macOS | Terminal.app with Zsh and Bash | Fresh loopback OpenSSH connection | Passed |
| Windows | Windows Terminal with PowerShell and cmd.exe | Fresh loopback Windows OpenSSH connection | Passed |

The complete 50-task M11 plan is accounted for. M11.01 through M11.09 and M11.10a through M11.10d have implementation or evidence. The owner approved M11.10e and PR #22 was merged under M11.10f. M11.10g is complete with passing post-merge CI and the documented M12 handoff.

## Risk reconciliation

| Risk | Reviewed control and evidence | Disposition |
| --- | --- | --- |
| R1 | All Git command variants use bounded waits and isolated native containment. Captured and silent helper regressions terminate delayed descendants, including the final `run_status` and `run_quiet` correction. | Passed |
| R2 | Git clears its environment, disables hooks, file monitors, credentials, filters, lazy fetch, and network transports, and rejects configured or committed checkout filters. | Passed |
| R3 | A common-repository native lock serializes branch and destination admission across processes. | Passed |
| R4 | Creation pins the initial commit while later launch permits ordinary user commits and still verifies repository, branch, and checkout identity. | Passed |
| R5 | NUL-delimited Git parsing and native path handling reject malformed, lossy, reserved, redirected, and unsupported identities before effects. | Passed |
| R6 | Bounded Git admission, durable operation IDs, cancellation barriers, conservative uncertain outcomes, and explicit read-only reconciliation prevent repeated or destructive effects. | Passed |
| R7 | Launch resolves task, definition, selected worktree, executable, arguments, and revision in one admission, then persists the matching immutable snapshot. | Passed |
| R8 | Native commands and both required repetitions fail independently, bounded harnesses cap waits and output, and no required step uses retries or `continue-on-error`. | Passed |
| R9 | Backup and restore use private staging, no-replace publication, integrity and schema checks, embedded migration, fresh destinations, and preservation of source and backup material. | Passed |
| R10 | Automated PTY and ConPTY results remain distinct from the complete nine-row named-terminal and OpenSSH matrix. | Passed |
| R11 | Public collections use revision-checked 200-item SQLite pages and bounded mutation projections. The scale gate retains 10,000 tasks and 100,001 progress rows without full-history UI materialization. | Passed |

Architecture tests preserve the domain, application, protocol, persistence, platform, Git, PTY, daemon, client, and TUI dependency boundaries. IPC remains local and current-user restricted. Input leases are connection scoped and are released on disconnect. Terminal state remains bounded in memory and absent from SQLite and diagnostics. Runtime paths, databases, sockets, backups, and staging use private native permissions. No provider integration, telemetry, hosted dependency, automatic Git cleanup, or automatic mutation from agent output was introduced.

## Budget reconciliation

Final [Quality run 34825227119](https://github.com/fpinero/relayterm/actions/runs/34825227119) and [Security run 34825227292](https://github.com/fpinero/relayterm/actions/runs/34825227292) passed on the source candidate. Quality passed Ubuntu 24.04 stable, Ubuntu 24.04 with Rust 1.98.1, macOS 14 arm64, and Windows Server 2022. The stable targets each passed two independent M11 hardening, protocol fault, abrupt SQLite, Git cancellation, Git descendant, durable-scale, offline, and sustained-resource runs, followed by backup and restore, the complete workspace, core, build, repository, and whitespace controls.

| Native stable runner | Startup p95, ms | Navigation p95, ms | Input p95, ms | Sustained resource output, bytes/s | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Ubuntu 24.04 | 142 / 162 | 21 / 21 | 164 / 164 | 4,253,264 / 4,235,753 | Passed |
| macOS 14 | 359 / 380 | 167 / 168 | 308 / 273 | 4,827,924 / 4,865,431 | Passed |
| Windows Server 2022 | 127 / 127 | 21 / 21 | 185 / 186 | 3,434,472 / 3,457,845 | Passed |

All startup p95 values remained below two seconds. Ubuntu and Windows met the reference navigation and input targets. Hosted macOS missed those reference targets while remaining inside the predeclared 500 ms and one-second hosted guardrails. The identified local macOS reference machine passed those targets. Every sustained resource run exceeded 2,097,152 bytes/s, kept daemon and TUI memory below 512 MiB, kept the final plateau within 32 MiB, returned handles within 16 of baseline, completed 100 reconnects including 20 abrupt disconnects, and rejected the ninth session before spawn.

The supplemental local Windows resource runs remain recorded at 1,960,566 and 1,947,797 bytes/s, below 2,097,152 bytes/s. They passed every other memory, plateau, handle, restart, reconnect, and abrupt-disconnect condition. This is a non-blocking machine-specific observation because the predeclared contract requires one identified developer-class reference environment, which the local macOS run satisfies, plus passing hosted CI budgets on all three operating systems, which the final candidate satisfies. The Windows values remain visible, the threshold was not changed, and the failed local runs were not relabeled. This disposition is an acceptance interpretation, not a security exception.

Local macOS verification on the final source passed formatting, all-target checks, Clippy with warnings denied, the complete workspace suite, build, two hardening-gate repetitions, two focused Git descendant repetitions, Cargo Deny, audit negative controls, candidate secret scanning, full-history Gitleaks, repository checks in a candidate-only detached worktree, and whitespace checks. Cargo Deny retained only its documented non-failing duplicate-package and unused-license-allowance warnings.

## Review decision

No unresolved acceptance blocker, critical or high security finding, or unreviewed security exception remains in M11. The generic stale-conflict message, competing-input explanation, session names, session ordering, and form-cursor visibility remain explicit non-blocking usability proposals.

The sole maintainer explicitly approved delivery after assistant-assisted technical review and completed automated and manual evidence. This supersedes the earlier external-approval requirement. No independent human review is claimed, and no alternate account is needed. No acceptance threshold or security control changed.

PR #22 was merged as `dc5b2e6067d50345828ac175858029434ce3c081`. Local and remote main were synchronized. [Post-merge Quality](https://github.com/fpinero/relayterm/actions/runs/34833773372) and [Security](https://github.com/fpinero/relayterm/actions/runs/34833773375) passed on that merge commit. The [M12 handoff](m12-handoff.md) is prepared; no M12 implementation or release publication is included.
