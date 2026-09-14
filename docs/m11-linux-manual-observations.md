# M11 Linux manual review

Status: Complete for MAN-LNX-1, MAN-LNX-2, and MAN-LNX-3. Final acceptance, risk, and budget reconciliation is recorded in the [M11 final technical review](m11-final-review.md). Independent PR review, merge, main synchronization, and post-merge CI remain open.

The observations were performed on 2026-09-13 UTC. The repository maintainer operated the native terminals while the assistant supplied one action at a time and performed separate read-only administrative checks. All workspaces, titles, markers, keys, and identifiers were synthetic. Local account names, hostnames, credentials, private project paths, raw terminal transcripts, and screenshot locations are omitted.

## Tested environment and candidate

- Platform: Ubuntu 24.04.4 LTS, x86_64, with Linux 7.0.0-31-generic.
- Local terminal: GNOME Terminal 3.52.0 using VTE 0.76.0.
- Terminal profile: `TERM=xterm-256color`, `COLORTERM=truecolor`, and `NO_COLOR=1` for every Relayterm client.
- Local shells: Bash 5.2.21 and Dash 0.5.12-6ubuntu5. The configured generic-shell row set `SHELL=/usr/bin/dash`, and process inspection confirmed `/usr/bin/dash` as the child executable.
- Full-screen program: Vim package 9.1.0016-1ubuntu7.20.
- SSH client and server: OpenSSH 9.6p1 Ubuntu-3ubuntu13.19 with OpenSSL 3.0.13.
- Toolchain: Rust 1.98.1, Cargo 1.98.1, and Git 2.43.0.
- Relayterm version: 0.1.0, debug build copied to an isolated candidate directory before observation.
- Candidate source: `05af62711e2da1d2a592d8d04d387be1cedc2be4`.
- Candidate binary SHA-256: `8417a0998bef00b1ab87b69d97b7ecfa05dbb3042c52534c06c90a9fa8d38984`.

The machine initially had no OpenSSH server or port 22 listener. With explicit maintainer authorization, the server package was installed while its global service and socket were runtime-masked. The observation used a manually started temporary server bound only to `127.0.0.1:22222`, public-key authentication with isolated synthetic Ed25519 keys, no password authentication, and no forwarding. `StrictModes` was disabled only in that temporary configuration because the synthetic authorized-key path was under the sticky, globally writable `/tmp` parent. No global SSH configuration was edited and no firewall rule was changed. After observation, both SSH clients exited normally, the temporary listener was stopped, the global service and socket were disabled and inactive, and ports 22 and 22222 had no listeners. The installed package remains available but disabled.

## MAN-LNX-1: GNOME Terminal and Bash

Administrative identity:

- Workspace ID: `5d74ad20-2f12-4ee1-940e-bd9a3e577fae`.
- Initial daemon generation: `8978a6e3-f4b1-472e-af2d-9a7f24d0717a`.
- Running session and instance pairs:
  - `1b39bd46-6fd4-478b-a3d8-b1071aee7431` / `bf2cbeac-d10a-4554-87cb-3e799f3f28ab`
  - `a1478ae9-e67f-4bb3-88a7-e3dff83123ee` / `3b2b8a0e-4193-4b7b-b989-68fef8031b7e`
  - `4f4cee30-6016-4d40-9ffe-1d9f3457416c` / `d97c4866-4332-48dc-b494-b335042235b6`
- Conflict task ID: `b2407ac6-35de-46c5-83d4-94515562efaf`.
- Observation revision after the authoritative external task edit: 10.

| Check | Expected result | Actual result and observer confirmation |
| --- | --- | --- |
| Initialize and launch | A fresh isolated workspace starts without personal project data. | The workspace initialized at revision 1. Three supervised children launched in the synthetic directory, and the first child reported `/bin/bash` and Bash 5.2.21. |
| Three live sessions | Three shells retain distinct output and stable identities while switching. | `MAN_LNX_1_SESSION_1`, `MAN_LNX_1_SESSION_2`, and `MAN_LNX_1_SESSION_3` remained visible after switching. Administrative reads matched all three identity pairs at revision 7. |
| Alternate screen | A real full-screen program enters and exits without corrupting the shell screen. | Vim displayed `MAN_LNX_1_FULLSCREEN_OK`, retained its alternate screen during resizing, and returned to the Bash prompt after `:q`. |
| Unicode | A wide character and combining sequence render coherently. | `Wide: 界 | Combining: é` rendered with the expected width and combining behavior, and the following prompt and cursor remained aligned. |
| Resize and minimum size | Wide, narrow, tall, and undersized layouts remain bounded and recover. | Repeated resizing stayed legible. At 57 by 23, the 80 by 24 minimum-size notice appeared. Enlargement restored the same Vim screen. |
| Focus, help, and no color | Focus escape returns to navigation, help opens, and state does not depend on color. | `Ctrl+]` returned to `NAVIGATION / READ ONLY`; keyboard help was legible; `CURRENT`, `RUNNING`, `INPUT`, `WRITER`, `NAVIGATION`, and `READ ONLY` remained explicit without color. |
| Exclusive input and transfer | Two clients share output, only one owns input, rejection is visible, and release transfers ownership both ways. | Output from each owner appeared in both clients. A competing `i` remained read-only and produced a rejected diagnostic in Events. Ownership transferred from B to A and back to B without closing either client. |
| Stale task form | A stale save is rejected without losing its draft or overwriting authoritative state. | At base revision 9, A retained `MAN-LNX-1 unsaved local draft` while B saved `MAN-LNX-1 saved externally`. A's `Ctrl+S` was rejected, the draft remained visible, explicit discard was required, and the authoritative external title remained at revision 10. |
| Detach, reattach, and exits | Detach preserves child context; `q` and navigation `Ctrl+C` restore the outer shell. | Reattachment returned to the same session transcript and identities. Both normal exit paths restored the outer Bash prompt, cursor, echo, and input. |
| Actual window closure | Closing the hosting terminal while Relayterm owns input does not stop the daemon or children. | The GNOME Terminal process-running warning was confirmed and the entire window was closed. Before and after comparison retained generation, revision, task, and all three running pairs. A new window reacquired the abandoned lease and produced `MAN_LNX_1_WINDOW_REOPEN_OK`, visible in the observer client. |
| Controlled client failure | SIGTERM of only the Relayterm client restores the outer terminal and preserves supervised state. | Process inspection distinguished the two clients and daemon. SIGTERM targeted only the new client. The client disappeared, the outer shell printed `MAN_LNX_1_SIGTERM_RESTORE_OK`, and administrative state retained the same generation, task, revision, and three running pairs. |
| Authoritative stop and reopen | Explicit session termination is durable and does not claim live PTY adoption. | `daemon stop --terminate-sessions` stopped the observed generation. Reopening created generation `68f635fb-cc3e-40ce-8b9e-df7fd714a700` at revision 13, retained the task, and showed the same three pairs as `terminated` with exit code 1. |

Result: Passed. Interactive confirmations and separate administrative comparisons agreed around every disruptive observation.

## MAN-LNX-2: GNOME Terminal and configured Dash

Administrative identity:

- Workspace ID: `7aaeb98a-3c9e-46b2-afef-156f4c0f5cbb`.
- Initial daemon generation: `0abc9e3c-0618-4c28-9b4f-2cfc19a1c7d0`.
- Running session and instance pairs:
  - `3bdc1f47-b2bc-46a1-b2f9-c845eb5d2c7a` / `21e0e90d-643c-4955-a6f4-91d0ab41cba6`
  - `68069333-bd09-4b9c-ab06-84e71fab68ff` / `165634c0-e7f8-4a65-8663-8eca885806cf`
  - `41875ed1-0ad6-4320-a22c-831e49baa8ac` / `8895361f-5b0c-4f48-9841-78f42edbf202`
- Conflict task ID: `e731591a-dc11-408d-8e0a-76616ea8ca8c`.
- Observation revision after the authoritative external task edit: 9.

| Check | Expected result | Actual result and observer confirmation |
| --- | --- | --- |
| Configured shell and directory | The generic-shell row launches the explicitly configured shell in the synthetic workspace. | The child reported `shell=/usr/bin/dash`, process inspection reported `exe=/usr/bin/dash`, and the current directory matched the synthetic workspace. This was distinct from MAN-LNX-1 Bash. |
| Three live sessions | Three Dash shells retain distinct output and stable identities while switching. | `MAN_LNX_2_SESSION_1`, `MAN_LNX_2_SESSION_2`, and `MAN_LNX_2_SESSION_3` remained visible after switching. Administrative reads matched all three pairs at revision 7. |
| Alternate screen and Unicode | Vim and Unicode behave through the configured generic shell. | Vim displayed `MAN_LNX_2_FULLSCREEN_OK` and returned to a Dash `$` prompt. The same wide and combining sample rendered correctly with an aligned cursor. |
| Resize and minimum size | Full-screen content redraws across dimensions and recovers from an undersized window. | Wide, narrow, and tall layouts remained coherent. At 62 by 25, the minimum-size notice appeared, and enlargement restored Vim. |
| Focus, help, and no color | Keyboard focus and state remain explicit without color. | `Ctrl+]`, help, and textual state labels behaved as in MAN-LNX-1. |
| Exclusive input and transfer | Two clients enforce one writer and share output bidirectionally. | B, A, and B again acquired input in sequence. `MAN_LNX_2_OWNER_B`, `MAN_LNX_2_OWNER_A`, and `MAN_LNX_2_OWNER_B_RETURN` appeared in both clients. Competing acquisition remained read-only and recorded a rejection in Events. |
| Stale task form | A stale Dash-row form cannot overwrite a concurrent edit. | A retained `MAN-LNX-2 unsaved local draft` while B saved `MAN-LNX-2 saved externally`. The stale save was rejected, the draft remained, explicit discard was required, and the external title remained authoritative at revision 9. |
| Detach, reattach, and exits | The same children and shell context survive client navigation and normal exits. | Detach and reattach retained the selected session and transcript. `q` and navigation `Ctrl+C` restored the outer shell; follow-up markers and ordinary shell input succeeded. |
| Actual window closure | Closing the writer window releases its lease without stopping durable state. | The real GNOME Terminal process-running warning was confirmed. After closure, generation, revision, task, and all three pairs remained unchanged. A new terminal reopened revision 9, acquired input, and produced `MAN_LNX_2_WINDOW_REOPEN_OK`, visible in the observer client. |
| Controlled client failure | SIGTERM targets only the TUI and restores the outer shell. | The daemon, A, and the new B client were distinguished by PID and terminal. Only B received SIGTERM. `MAN_LNX_2_SIGTERM_RESTORE_OK` executed in the restored shell, while administrative state retained the same generation, task, revision, and three running pairs. |
| Authoritative stop and reopen | Explicit termination persists task state and honest session loss. | Reopening after `daemon stop --terminate-sessions` created generation `185ee59e-9645-4a29-8fc4-6a8075575c1c` at revision 12, retained `MAN-LNX-2 saved externally`, and showed the same three identity pairs as `terminated` with exit code 1. |

Result: Passed. The generic-shell path was verified from the child process rather than inferred from the launcher environment.

## MAN-LNX-3: Fresh OpenSSH connection to Linux

Administrative identity:

- Workspace ID: `bbe5de9b-6984-45a3-8ce2-a305087d2b60`.
- Initial daemon generation: `1149315d-b507-436d-881f-3d7841691afc`.
- Running session and instance pairs:
  - `88815d19-4ea6-4c9a-a0c4-6ef46e22bb0a` / `c3d4c34c-db7d-40e7-bcf3-657541da115f`
  - `da8f81b4-ff74-4ca8-8047-5ba6fa427c39` / `f1e801f2-36d1-40fd-8cd2-98200ad2912f`
  - `aac7508f-e985-4492-b634-fba5392be41e` / `d940f9bc-c06a-41a4-a209-ef7eb9cb96af`
- Conflict task ID: `fc26a958-c0a9-4416-9704-44e00b5a1e2e`.
- Observation revision after the authoritative external task edit: 9.

| Check | Expected result | Actual result and observer confirmation |
| --- | --- | --- |
| Fresh SSH and PTY | A newly established SSH process provides a real remote PTY and starts Relayterm without a hosted dependency. | A fresh key-authenticated connection reported a non-empty `SSH_CONNECTION` to the isolated loopback listener, a `/dev/pts` SSH TTY, and `/bin/bash`. Relayterm initialized at revision 1. |
| Idle stability | An established connection remains usable after more than 60 seconds without input. | The TUI remained on keyboard help for more than 70 seconds with no transport-loss diagnostic or replacement connection. Input worked afterward. |
| Three live sessions | Three remote Bash children retain distinct output and stable identities while switching. | The three identity pairs were linked explicitly to `MAN_LNX_3_SESSION_1`, `MAN_LNX_3_SESSION_2`, and `MAN_LNX_3_SESSION_3`. Administrative reads matched them at revision 7. |
| Alternate screen and Unicode | Vim and Unicode remain coherent through SSH, Relayterm, and the supervised PTY. | Vim displayed `MAN_LNX_3_FULLSCREEN_OK`. The wide and combining sample rendered correctly, and the next prompt and cursor aligned normally. |
| Resize and minimum size | SSH propagates size changes and recovers the same full-screen content. | Wide, narrow, and tall Vim layouts redrew correctly. At 66 by 24, the 80 by 24 notice appeared. Enlargement restored the same Vim content and cursor. |
| Focus, help, and no color | Focus escape and keyboard help remain usable through SSH without color. | `Ctrl+]` returned to navigation; keyboard help remained legible; every relevant state was identified by text. Screen shortcuts returned from help without affecting the remote session. |
| Two SSH clients and input transfer | Independent SSH and Relayterm clients share one terminal with one writer. | Two fresh SSH processes opened the same session. B, A, and B again acquired input in sequence. Owner markers appeared in both clients. A competing acquisition stayed read-only and recorded an explicit rejection in Events. |
| Stale task form | A stale form over one SSH client cannot overwrite a save from another. | At base revision 8, A retained `MAN-LNX-3 unsaved local draft` while B saved `MAN-LNX-3 saved externally`. A's stale save was rejected with its draft intact. Explicit discard returned to the external title at revision 9. |
| Detach, reattach, and normal TUI exits | Client navigation preserves context and both normal exit paths restore the remote shell. | Repeated detach and reattach preserved the exact identity-linked transcripts. `q` and navigation `Ctrl+C` restored Bash inside SSH, with `SSH_CONNECTION` still pointing to the isolated listener. |
| Normal SSH disconnect and fresh reconnect | Closing one SSH connection normally preserves the daemon, task, and children. | After `logout` and `Connection to 127.0.0.1 closed.`, administrative reads retained generation, revision, task, and all three running pairs. A new SSH process reopened the workspace, reacquired input, and produced `MAN_LNX_3_SSH_RECONNECT_OK` in both clients. |
| Actual SSH hosting-window closure | Closing GNOME Terminal while SSH and Relayterm own input preserves state and releases the abandoned lease. | The real process-running warning was confirmed and the complete window was closed. Before and after reads were identical. A new window, new SSH process, and new Relayterm client acquired input and produced `MAN_LNX_3_WINDOW_REOPEN_OK`, visible in A. |
| Controlled Relayterm failure inside SSH | SIGTERM of only Relayterm restores the remote shell without ending SSH or supervised state. | Process inspection distinguished the temporary SSH listener, SSH server sessions, Relayterm daemon, and both TUI clients. SIGTERM targeted only B's TUI. The remote shell printed `MAN_LNX_3_SIGTERM_RESTORE_OK` and retained its SSH connection; administrative state remained unchanged. |
| Authoritative stop and reopen | Explicit termination records honest session loss while preserving durable task state. | Reopening after `daemon stop --terminate-sessions` created generation `ce727fff-4b0d-4f24-8003-6b0e77a766d6` at revision 12. The task remained backlog as `MAN-LNX-3 saved externally`, and the same three pairs were `terminated` with exit code 1. |

Result: Passed. The row used fresh operator-controlled SSH processes and did not infer SSH behavior from local PTY automation.

## Manual matrix reconciliation

No Relayterm source defect was reproduced on the fixed Linux candidate. The absent SSH server, missing runtime privilege-separation directory after deliberately suppressing service startup, and `StrictModes` rejection of a synthetic key below `/tmp` were test-environment setup conditions. They were resolved in the isolated SSH fixture without changing product source or relaxing the global SSH service.

The macOS final candidate maps to source `69ff8a84335811d238763691c78433de5a36d93a`. Later runtime changes through the Linux candidate either preserve the Unix path unchanged under `cfg(not(windows))` or are compiled only on Windows. The intervening protocol change modifies Windows test endpoint identities only. The macOS Terminal.app and SSH observations are therefore unaffected. The Windows final SSH candidate maps to source `136d46c2e5aeb3b26817a3574be3edb8d19a7ff9`; every later commit through the Linux candidate changes documentation and logbook state only, so the Windows observations are also unaffected.

All nine required Linux, macOS, and Windows rows now have candidate-specific passing evidence. The generic stale-form message remains the existing non-blocking `UX-CONFLICT-MESSAGE` proposal. Competing input acquisition produced an explicit Events diagnostic but no inline terminal-view explanation; `UX-INPUT-ACQUIRE-MESSAGE` records that non-blocking usability opportunity. Session ordering and naming observations remain non-blocking proposals. Final acceptance, risk, budget, PR review, merge, and post-merge CI gates remain open.
