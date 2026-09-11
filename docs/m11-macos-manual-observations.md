# M11 macOS manual review

Status: Complete for MAN-MAC-1, MAN-MAC-2, and MAN-MAC-3. M11 remains open for the Linux and Windows manual rows, final reconciliation, CI, and independent review.

This report retains the observations in chronological order. Statements about pending work in the initial sections describe the state at that point in the review; the final disposition governs the current status.

## Tested candidate and environment

- Initial code candidate: `a4dfd0ad807e013ac8845a2c44c352ae336873d1`.
- Final macOS candidate source: `69ff8a84335811d238763691c78433de5a36d93a`. Its rebuilt `target/debug/rt` SHA-256 is `ebf27bbd945afcce30fa99aa52738763ae5836426503d07abf15d8b1dd1aa2b8`, exactly matching the manually observed v4 binary.
- Platform: macOS 26.5.2, build 25F84, arm64.
- Terminal.app: 2.15, build 470.2.
- Shell: Zsh 5.9. Rust: 1.98.1. Git: 2.53.0. Relayterm: 0.1.0, debug build.
- Terminal profile: UTF-8, xterm-256color, Menlo Regular 13 after the font comparison. The launcher sets NO_COLOR=1.
- Observer: repository maintainer, with assistant-guided steps and separate read-only administrative checks. This is not an independent human review.
- Runtime data and projects are synthetic and private. Screenshots and personal paths are not included in this report.

## Initial observed results

| Check | Evidence and result |
| --- | --- |
| Initialization and shell input | Passed after correcting the launcher's overlong runtime socket path. Shell echoed the synthetic marker. |
| Unicode example | U+754C and e plus U+0301 rendered correctly inside and outside Relayterm with Menlo. Andale Mono displayed replacement glyphs even outside Relayterm; UTF-8 byte inspection was correct. |
| Help | The ? key displayed keyboard help. |
| Small window and redraw | At 67x23, bounded 80x24 guidance appeared. Repeated enlargement restored the help screen. |
| Full-screen child | System Vim displayed the synthetic marker, handled resizing, and restored the shell screen on exit. Backspace worked with explicit nocompatible/backspace settings. |
| Multiple sessions | Both synthetic echo definitions returned distinct A and B responses while shells remained running. |
| Exclusive input | A second client remained READ ONLY while the first held input. |
| Explicit ownership transfer | Unresolved: the observer reported failure to acquire after release while both clients stayed open. Acquisition succeeded after the owner client exited. |
| Shared display | Both clients showed output from the same attached session. |
| q and Ctrl-C navigation exit | Both restored outer-shell input, prompt, and cursor; synthetic echo checks succeeded. |
| Actual terminal-window closure | Observer closed the owning window and reopened in a new terminal. Saved before/after checks confirmed the same daemon generation and all four original running instance/session identities. B retained its display and answered AFTER_REOPEN. |
| Stale form | A concurrent administrative edit caused the old TUI edit to be rejected. Its draft stayed visible; saved content and revision were unchanged. The message did not explain the concurrent-edit cause. |
| Graceful daemon restart | Task and definitions were unchanged, session identities persisted, and all four sessions appeared terminated in both administrative readback and the TUI. |
| No-color observations | Observed screens retained textual state labels with NO_COLOR=1. This is evidence for the displayed states, not an exhaustive accessibility audit. |

## Initial findings and remaining work recorded on 2026-09-10

1. Automatic workspace refresh stopped while a child terminal remained attached, even on the Agents screen. Externally registered definitions appeared only after R. A new native TUI regression reproduced this issue. A local source correction separates workspace refresh scheduling from terminal display polling; the focused regression then passed.
2. Explicit input-release behavior still requires manual reproduction and reconciliation. Source inspection found that release errors are discarded while local input state is cleared. A native two-client regression successfully transferred input in both directions without closing either client; this does not override the manual observation.
3. Repeated transport-loss diagnostics remain unexplained. Their presence did not imply that the daemon had exited.
4. Conflict rejection needs clearer user-facing guidance. Data protection and draft retention were observed successfully.
5. Editable session names were requested and recorded as UX-SESSION-NAMES in TODO. Implementation and scheduling remain pending.
6. Recoverable client-failure restoration and abrupt-daemon-loss reconciliation are not established by the graceful exit/restart observations above.
7. Bash, macOS SSH, Linux, and Windows manual cells remain unexecuted in this review. Final candidate evidence and post-fix manual repetitions are still required.

At this point in the review, the manual launcher still used the original copied candidate. Local source changes had not been pushed, merged, or substituted into the observer's running candidate, and no complete manual cell was declared passed.

## Local correction validation

The focused refresh regression failed before the correction and passed afterward. Formatting, workspace Clippy with warnings denied, the complete workspace test suite, and the workspace build passed on native macOS. Audit negative controls, candidate secret checks, and diff whitespace checks also passed. Native Linux/Windows CI has not been run for the local correction.

The standard repository documentation checker remains blocked by trailing whitespace in the pre-existing untracked operator review document, which was preserved. A supplementary tracked-file check passed; it does not replace or claim success for the full candidate check.

## 2026-09-10: Native Terminal.app transfer retest, first direction

The maintainer confirmed that a second open client stayed READ ONLY while the first held WRITER. After a single focus-release chord in the first client, the second acquired WRITER without either client closing. Its shell executed `echo TRANSFER_OK`; both clients displayed the command and output, with the first remaining READ ONLY. Shared display is expected for clients attached to the same session. Reverse transfer remains pending. This observation does not establish release-error handling or explain the earlier failure.

Candidate: local working-tree build, binary SHA-256 `aca1e5ece84092665b55dbda63597dff2e99f78e87b9e2b1ad256455613bdcf1`. The isolated v2 launcher uses the workspace-refresh correction.

## 2026-09-10: Native Terminal.app reverse transfer confirmed

On the isolated v2 candidate, the maintainer released input in the second client and acquired it in the first without closing either window. `echo TRANSFER_BACK_OK` executed in the first client and appeared in both. The first displayed INPUT/WRITER and the second NAVIGATION/READ ONLY. Normal explicit ownership transfer is now manually observed in both directions. Release-error handling and the cause of the earlier failed transfer remain unresolved; this does not close M11.07e-input in full.

## 2026-09-10: Automatic agent refresh confirmed in Terminal.app

With a child terminal still attached, the second client remained on Agents while an administrative client registered `Manual refresh check`. The maintainer confirmed that the enabled definition appeared automatically, without pressing R or changing screens. Administrative readback independently confirmed registration. This validates the workspace-refresh correction manually on the isolated v2 macOS/Zsh candidate.

## 2026-09-10: Bash input and Unicode confirmed

In the isolated v2 Bash workspace, the maintainer observed INPUT/WRITER and successfully executed the version and UTF-8 byte printf checks. The displayed shell version was 3.2.57(1)-release. U+754C and e plus U+0301 rendered correctly with the retained Menlo profile. Full-screen, resize, detach and remaining Bash matrix observations are still pending.

## 2026-09-10: Bash full-screen and resize confirmed

The maintainer confirmed successful execution of the Bash full-screen procedure on the isolated v2 candidate. System Vim displayed `BASH_FULLSCREEN_OK` in insert mode across repeated window reductions and enlargements. The submitted observations show the marker and cursor retained at different dimensions. After exiting without saving, Bash executed `echo BASH_SCREEN_RESTORED_OK` and displayed its output and prompt. This verifies the full-screen child and return to the inner Bash shell; outer-terminal restoration and the other remaining matrix checks are separate.

## 2026-09-10: Bash help and minimum-size recovery confirmed

The maintainer opened keyboard help after releasing input. At 66 columns by 26 rows, Terminal.app displayed the bounded minimum-size guidance requiring 80 by 24. Enlarging the window restored keyboard help. The screenshots and maintainer confirmation establish help navigation and recovery from insufficient width on the v2 Bash candidate.

## 2026-09-10: Bash q exit and reattachment confirmed

The maintainer confirmed successful q exit, outer-terminal input with `OUTER_TERMINAL_OK`, and reopening the v2 Bash launcher. The reattached terminal retained prior output and executed `BASH_AFTER_REOPEN_OK`. Administrative before/after comparison confirmed the same daemon generation and unchanged running instance and session identities. Actual window closure and Ctrl-C navigation exit remain separate checks.

## 2026-09-10: Bash client Ctrl-C exit confirmed

After returning to navigation, the maintainer pressed Ctrl-C and confirmed normal outer-shell input with `BASH_CTRL_C_RESTORED_OK`. Administrative readback confirmed the same daemon generation and unchanged running Bash instance/session after client exit. This establishes client exit and terminal restoration, not daemon termination or a crash. The saved readback also provides a baseline for the pending actual window-closure observation.

## 2026-09-10: Bash actual terminal-window closure confirmed

The maintainer closed the Terminal.app window hosting the v2 Bash client using the native termination confirmation, then opened a new window and reattached. Prior output remained visible and `BASH_WINDOW_REOPEN_OK` executed successfully with INPUT/WRITER. Administrative comparison against the pre-closure baseline confirmed the same daemon generation and unchanged running Bash instance/session identities. Actual window closure therefore preserved the supervised Bash session in this observation. The full Bash matrix remains partial.

## 2026-09-10: Second Bash session confirmed

The maintainer detached from the first Bash terminal and created a second shell through the TUI. The new shell displayed `BASH_SESSION_2` and did not contain the first session's command output. Administrative readback confirmed two running instances. The three-session and switching observation remains pending.

## 2026-09-10: Three Bash sessions and switching confirmed

The maintainer confirmed three running sessions and successful switching among them. Each retained its own output: the original test transcript, `BASH_SESSION_2`, and `BASH_SESSION_3`. Administrative readback independently confirmed three running instances. The maintainer also noted that the newest session appeared first and preferred new sessions appended at the end. This ordering preference is a usability follow-up, not a failed session-isolation check.

## 2026-09-10: Bash stale task edit rejected with draft retained

After an external client saved `Bash saved externally`, the maintainer submitted the open `Bash local draft` form. The TUI displayed `Error: The server rejected the request.` and retained the local draft. Administrative readback confirmed the saved task content and revision were unchanged by the rejected submission. Conflict protection is observed; the error still does not explain the concurrent-edit cause. Explicit draft discard and return to the saved content remain pending.

## 2026-09-10: Bash conflicting draft discard confirmed

After discarding the rejected local draft, the maintainer observed `Bash saved externally` in both the task list and detail, with `Initial version` retained. Administrative readback agreed. The Bash stale-edit sequence now covers rejection, draft retention, saved-content protection, and explicit discard. A separate baseline records all three running Bash session identities before the pending combined window-closure check.

## 2026-09-10: Three Bash sessions survive actual window closure

The maintainer closed the hosting terminal window, reopened the v2 Bash client in a new window, and observed all three sessions running with their separate previous content. Administrative comparison confirmed the same daemon generation, the same three instance/session identity pairs, and unchanged saved task content. This completes the three-session survival observation for the v2 Bash local test. SSH and outstanding fault-handling checks remain separate and M11 remains open.

## 2026-09-10: SSH loopback starts, idle transport failure reproduced

The maintainer enabled remote-user Full Disk Access and reconnected after access to the test launcher under Documents was denied. Directory listing and the v2 TUI then worked over SSH loopback. The TUI alternated between CURRENT and DISCONNECTED without manual reconnection, and Events showed repeated transport-loss diagnostics. Administrative readback found the daemon ready with the same generation and three running sessions. This observation does not pass the SSH stability check.

An isolated protocol regression reproduced an established connection failing after 11 seconds without traffic. The daemon incorrectly applied its 10-second partial-frame timeout before any frame byte arrived; the event client also applied a request deadline while waiting for an event. Both established idle readers now wait for the first byte and retain bounded partial-frame completion. The regression then delivered the next event without accepting a replacement connection. Native SSH validation of the corrected candidate remains pending.

## 2026-09-10: Idle correction candidate ready for SSH observation

The corrected v3 candidate passed the native workspace suite (170 passed, 15 ignored), formatting, Clippy with warnings denied, build, secret checks, and audit negative controls. A separate Bash fixture and launcher were prepared without restarting the existing v2 daemon. An additional ad hoc PTY observation could not be completed because its terminal harness failed initialization and later hung during process cleanup; it is not counted as a passed TUI stability check. The isolated protocol regression and IPC deadline checks passed. The maintainer must still observe the v3 TUI over SSH in Events for at least 60 seconds and confirm that idle transport-loss diagnostics no longer recur.

## 2026-09-10: V3 SSH idle stability confirmed manually

The maintainer left the v3 Bash TUI on Events over SSH loopback for substantially more than 60 seconds and reported no disconnections. The supplied screenshot shows CURRENT and an empty diagnostics panel. The exact elapsed duration was not measured. This passes the manual idle-stability retest for the corrected candidate in this environment. Interactive input after idle and the remaining SSH matrix checks remain separate; this does not close M11 or establish external-network SSH coverage.

## 2026-09-10: V3 SSH input after idle confirmed

After the prolonged idle observation, the maintainer selected the only running session, attached in navigation mode, acquired input, and executed `echo SSH_AFTER_IDLE_OK`. Screenshot evidence shows the marker, the returned Bash prompt, INPUT/WRITER, and CURRENT. This confirms interactive input and output after idle over SSH loopback. The session list displays session and instance identifiers rather than a Bash label; instructions should refer to the running session row. SSH detach/reconnect and the remaining manual matrix checks remain pending.

## 2026-09-10: V3 SSH graceful reconnect confirmed

The maintainer exited the TUI, closed the SSH connection, reconnected over SSH loopback, relaunched v3, and attached to the existing session. The supplied screenshot shows both `SSH_AFTER_IDLE_OK` and `SSH_RECONNECTED_OK`, the returned Bash prompt, INPUT/WRITER, and CURRENT. Administrative readback confirms the unchanged daemon generation and the same running session and instance identifiers shown before reconnection. Graceful SSH reconnect with retained output and resumed input passes. Abrupt hosting-window closure over SSH and the remaining manual matrix checks are not covered by this observation.

## 2026-09-10: V3 session survives SSH window closure

The maintainer closed the hosting Terminal.app window while the v3 TUI was attached in input mode. The confirmation identified the SSH process as the process to terminate. After opening a new window, reconnecting over SSH loopback, and relaunching v3, the existing Bash session retained both earlier markers and successfully printed `SSH_WINDOW_REOPEN_OK`. The final screenshot shows the returned prompt, INPUT/WRITER, and CURRENT. Administrative comparison confirms the same daemon generation and the same running session/instance pair. This passes the SSH hosting-window closure and subsequent reattachment check. Remaining SSH rendering and other manual matrix coverage are not implied.

## 2026-09-10: V3 SSH full-screen resize and restoration confirmed

The maintainer ran the system vi in the v3 Bash session over SSH loopback, entered `SSH_FULLSCREEN_OK`, and resized the hosting terminal through multiple wide, narrow, and tall layouts. The supplied screenshots preserve the marker and insertion cursor, with CURRENT and INPUT/WRITER visible. After exiting vi without saving, `SSH_SCREEN_RESTORED_OK` printed and the Bash prompt returned. The maintainer confirmed correct behavior at all tested sizes. This passes the SSH full-screen resize and alternate-screen restoration observation. Unicode rendering, below-minimum window behavior, and other remaining matrix checks are separate.

## 2026-09-10: V3 SSH Unicode and minimum-size recovery confirmed

The maintainer confirmed correct wide-character and combining-character rendering with the selected terminal font. Screenshot evidence shows `Wide: 界 | Combining: é` and the returned Bash prompt. After leaving input mode and opening help, shrinking the window to 72 columns by 29 rows displayed the minimum 80-by-24 notice. Enlarging the window restored keyboard help with CURRENT visible. These observations pass the Unicode, focus escape, below-minimum width, and help-restoration checks over SSH loopback for v3. They do not establish every font, a below-minimum-height case, or completion of the remaining manual matrix.

## 2026-09-10: V3 three-session switching over SSH confirmed

The maintainer created two additional Bash sessions over SSH loopback, printed `SSH_SESSION_2` and `SSH_SESSION_3` in their respective sessions, and confirmed switching among all three with independent retained content. Screenshots show the two new markers and three running session rows with CURRENT visible. Administrative readback independently confirmed three running instances and saved their identity baseline. This passes three-session creation, isolation, and switching for v3 over SSH. Combined three-session survival after SSH window closure remains a separate pending observation.

## 2026-09-10: V3 three-session graceful SSH reconnect confirmed

The maintainer exited the TUI and ended the SSH connection with `exit`, then reconnected and relaunched v3. Screenshots show all three running sessions, their separate retained output, and CURRENT. The original session retains its earlier reconnect, resize, and Unicode transcript; the other sessions retain `SSH_SESSION_2` and `SSH_SESSION_3`. Administrative comparison against the three-session baseline confirms the same daemon generation and all three unchanged running session/instance pairs. This passes graceful SSH reconnection with three persistent sessions. The supplied sequence shows normal SSH exit, not abrupt hosting-window closure with three sessions. The earlier single-session SSH window-closure observation remains valid; M11 and its remaining matrix checks remain open.

## 2026-09-11: V4 input transfer and recoverable client failure confirmed

The maintainer attached two Terminal.app clients to the same v4 Bash session. While the first client held INPUT/WRITER, the second client's input acquisition was rejected, remained READ ONLY, and produced redacted rejected diagnostics in Events. After explicit release, ownership moved from the first client to the second and then back to the first without closing either client. The markers `V4_OWNER_A`, `V4_OWNER_B`, and `V4_OWNER_A_RETURN` appeared in both clients. This manually confirms exclusive input, normal release, bidirectional ownership transfer, and shared terminal output.

The upper TUI client was then terminated with SIGTERM while the daemon and lower client remained active. Terminal.app restored its outer Zsh prompt, cursor, input, and output; `Hello_World` and `V4_SIGTERM_RESTORED` executed successfully. Administrative readback confirmed that the daemon remained active and all three original session and instance identity pairs remained running. Relaunching v4 displayed the same three sessions. Reattaching to the first session restored its prior history, allowed immediate input acquisition, and displayed `V4_AFTER_SIGTERM` in both clients. This confirms terminal restoration after recoverable client termination, connection-scoped lease cleanup, daemon and session persistence, and successful reattachment. It does not induce an ambiguous transport result, exercise explicit `R` reconciliation, or establish abrupt daemon-loss recovery.

Candidate: local working-tree v4 build, binary SHA-256 `ebf27bbd945afcce30fa99aa52738763ae5836426503d07abf15d8b1dd1aa2b8`. Workspace `a9d51aba-dfcb-4f6a-9531-fa44c4128e95` retained sessions `38558a2f-9edf-40e2-b49f-0c68f3fbf19b`, `6d028a1d-f2fe-4b27-bf4c-718722066a52`, and `9614e1ad-7caf-4f0d-acdf-300d8a092b01` throughout the observation.

## 2026-09-11: V4 SSH task conflict confirmed

The maintainer opened an edit form for `V4 SSH conflict baseline` through a fresh loopback OpenSSH connection and retained the unsaved title `V4 SSH stale draft`. A separate local Terminal.app client saved `V4 local saved first` first. Submitting the stale SSH form was rejected, the form remained open, and its draft remained intact. After explicit discard, the SSH client displayed the authoritative saved title `V4 local saved first`. Administrative readback confirmed the same saved value and unchanged task identity. This passes the SSH form-conflict, draft-retention, overwrite-protection, and explicit-discard observation. The displayed `Error: The server rejected the request.` is safe but too generic; clearer concurrent-edit guidance is retained as a non-blocking usability follow-up.

## 2026-09-11: V4 three-session abrupt SSH closure confirmed

Through loopback OpenSSH, the maintainer observed all three running sessions, attached to the first, acquired input, and executed `V4_SSH_THREE_BEFORE_CLOSE`. The hosting Terminal.app window was then closed without releasing input or exiting Relayterm. The native confirmation identified `ssh` as the process to terminate. Administrative readback after closure confirmed the same daemon generation, the same three running session and instance identity pairs, and the saved task. A fresh Terminal.app window established a new SSH connection and relaunched v4. All three sessions remained present, the first retained `V4_SSH_THREE_BEFORE_CLOSE`, and it accepted `V4_SSH_THREE_AFTER_REOPEN`. This passes actual SSH connection closure, connection-scoped lease release, three-child survival, fresh SSH reattachment, retained context, and resumed input.

## 2026-09-11: V4 SSH client restoration and explicit daemon restart confirmed

The Relayterm client inside the reopened SSH connection was terminated with SIGTERM while the SSH process, daemon, and local TUI client remained active. The remote shell restored its prompt, cursor, input, and output, and `V4_SSH_CLIENT_RESTORED` executed successfully. This passes recoverable client-failure restoration inside the SSH terminal.

With explicit maintainer authorization, `rt daemon stop --terminate-sessions` then stopped generation `d773bfac-b0e9-4729-9af7-7cd5a656b478` and terminated the three synthetic sessions. Relaunching v4 through SSH started ready generation `f52e397b-245d-4810-9567-3a22f8f2ed55`. The TUI and administrative readback showed the same three session and instance identities as `terminated`, while task `120b268f-2adc-4120-9562-1fa1894f097f` remained backlog with title `V4 local saved first`. The final durable revision was 12. This passes explicit daemon stop, durable task recovery, honest session-loss presentation, and daemon reopening for the observed workspace.

## 2026-09-11: macOS manual matrix disposition

The combined dated observations now cover the required Terminal.app Zsh, Terminal.app Bash, and fresh OpenSSH-to-macOS combinations. Shell-specific rendering and terminal behavior were observed on the original candidate, and every affected behavior was repeated on isolated post-fix candidates through v4. The observed v4 SHA-256 maps exactly to source commit `69ff8a84335811d238763691c78433de5a36d93a`. No further macOS manual behavior observation is pending unless source behavior changes. Linux and Windows manual rows, final review, CI, and merge gates remain open, so M11 is not complete.
