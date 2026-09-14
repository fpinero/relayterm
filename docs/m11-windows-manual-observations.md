# M11 Windows manual review

Status: Complete for MAN-WIN-1, MAN-WIN-2, and MAN-WIN-3. Final acceptance, risk, and budget reconciliation is recorded in the [M11 final technical review](m11-final-review.md). Independent PR review, merge, main synchronization, and post-merge CI remain open.

The observations were performed from 2026-09-12 through 2026-09-13 UTC. The repository maintainer operated the native terminals while the assistant supplied one action at a time and performed separate read-only administrative checks. All workspaces, titles, markers, and identifiers were synthetic. Private account names, hostnames, absolute profile paths, credentials, and screenshot locations are omitted.

## Tested environment and candidates

- Platform: Windows 10 Pro, version 10.0.19045, build 19045.7663, x86_64.
- Local terminal: Windows Terminal 1.24.11911.0 using ConPTY.
- Local shells: PowerShell 7.6.5 and `cmd.exe` 10.0.19045.7663.
- Toolchain: Rust 1.98.1, Cargo 1.98.1, and Git for Windows 2.47.1.windows.2.
- Relayterm version: 0.1.0, debug builds copied to isolated candidate directories before observation.
- Initial source: `e031af86a164606b02a1e0b79115f881d30a8831`. Initial Windows binary SHA-256: `94e37a31fad15812bdbb36d91504ff27b85efdfd8661bb7d3b63d4cba65c01d7`.
- Native `cmd.exe` correction source: `f3d0ca6e60d5fa1a6204e6e997d07d249a45086a`. Corrected binary SHA-256: `01effc97672fe21e82587e880d5942903f8b2c91b37a923b78df47a2ce3d6d6d`.
- Windows OpenSSH lifetime correction source: `136d46c2e5aeb3b26817a3574be3edb8d19a7ff9`. Final binary SHA-256: `41c49871965d91b628928d505a3f717c4969ad37fc0589eb2220e16368c4fd14`.
- Every candidate used `NO_COLOR=1`. State remained identifiable through textual labels such as `CURRENT`, `RUNNING`, `INPUT`, `WRITER`, `NAVIGATION`, `READ ONLY`, and explicit diagnostics.

The PowerShell observation was complete before either correction. The first correction only changes the `cmd.exe` working-directory boundary and test support. The second changes detached daemon creation when the launcher is inside a Windows job object, with a compatibility fallback when breakaway is denied. PowerShell and local `cmd.exe` observations that do not involve an OpenSSH job were therefore unaffected. The failed pre-correction SSH persistence observations are explicitly superseded below.

## MAN-WIN-1: Windows Terminal and PowerShell

Candidate: initial source `e031af86a164606b02a1e0b79115f881d30a8831`, binary SHA-256 `94e37a31fad15812bdbb36d91504ff27b85efdfd8661bb7d3b63d4cba65c01d7`.

Administrative baseline:

- Workspace ID: `32ca2e90-e38d-4ef3-ac50-9e9a047445f9`.
- Daemon generation: `845d9f4a-720a-429a-9a25-79332b5fc79f`.
- Running session and instance pairs:
  - `3fe03234-500c-4946-b714-fce35414a916` / `e25f9877-9365-4d09-b6ea-b696a356aed3`
  - `6212fc09-54eb-4d64-ac1e-f533804d25ad` / `ff4216ae-d48b-4d66-a65f-72268fb5ee58`
  - `a63d0dd5-784a-4e70-a37f-969b0be6fae5` / `58f71400-9d7b-435b-859c-cce7c4773959`
- Conflict task ID: `b99af826-beba-4978-a849-fe69744d7260`.

| Check | Expected result | Actual result and observer confirmation |
| --- | --- | --- |
| Initialize and launch | A fresh isolated workspace starts without personal project data. | The initialization prompt was accepted, the TUI opened, and the enabled synthetic PowerShell definition was visible. |
| Three live sessions | Three shells retain distinct output and stable identities while switching. | The maintainer created three running sessions, printed `PS_SESSION_1`, `PS_SESSION_2`, and `PS_SESSION_3`, and switched among their retained screens. Administrative reads matched all three identity pairs. |
| Alternate screen | A real full-screen program enters and exits without corrupting the shell screen. | Vim 9.1.785 displayed `PS_FULLSCREEN_OK`, remained usable during resizing, and returned to the retained PowerShell screen. |
| Unicode | A wide character and combining sequence render coherently. | `Wide: 界 | Combining: é` rendered correctly in the observed terminal and font. |
| Resize and minimum size | Wide, narrow, tall, and undersized layouts remain bounded and recover. | Rapid resizing remained legible. At 61 by 32 the 80 by 24 minimum-size notice appeared, and enlarging the window restored Vim content. |
| Focus, help, and no color | The documented focus escape returns to navigation, help opens, and state does not depend on color. | `Ctrl+]` returned to `NAVIGATION / READ ONLY`; `?` opened keyboard help; all relevant states remained explicit with no color. |
| Exclusive input and transfer | Two clients share output, only one owns input, acquisition conflicts are visible, and explicit release transfers ownership both ways. | The observer client remained read-only. Five competing `i` attempts produced five rejected diagnostics. Output written by each owner appeared in both clients. Ownership transferred from A to B and back to A without closing either client. |
| Stale task form | A stale save is rejected without losing the draft or overwriting authoritative state. | `PS conflict baseline` was edited locally to `PS local draft` while an administrative client saved `PS saved externally`. `Ctrl+S` was rejected, the local draft remained visible, explicit discard was required, and the detail returned to `PS saved externally`. |
| Detach, reattach, and exits | Detach preserves child context; `q` and navigation `Ctrl+C` restore the outer shell. | Reattachment returned to the same session transcript. Both exit paths restored the PowerShell prompt, cursor, and input, and follow-up commands succeeded. |
| Actual window closure | Closing the Windows Terminal window does not stop the daemon or children. | A new PowerShell window reopened the same workspace with all three sessions running, the same identities, and retained output. The abandoned input lease was released and the new client acquired input. |
| Forced client failure | Killing only the Relayterm client leaves the outer shell and supervised children usable. | The exact client PID was distinguished from the daemon before termination. Windows restored control to the outer PowerShell process, which printed `PS_FORCED_CLIENT_OS_RECOVERY`. Administrative reads retained the daemon generation, task, and all three running identity pairs. This is operating-system recovery after a forced kill, not client cleanup. |

Result: Passed. The maintainer confirmed each interactive result, and separate administrative comparisons agreed before and after disruptive operations.

## MAN-WIN-2: Windows Terminal and cmd.exe

The initial candidate launched `cmd.exe` with a verbatim extended-length current directory. Native `cmd.exe` treated it as a UNC-style path, warned that UNC paths are unsupported, and defaulted to the Windows directory. This failed the required workspace-context check. Source `f3d0ca6e60d5fa1a6204e6e997d07d249a45086a` converts only the `cmd.exe` working directory to the equivalent native drive path at the process boundary. The failed initial observation is superseded.

Candidate: corrected source `f3d0ca6e60d5fa1a6204e6e997d07d249a45086a`, binary SHA-256 `01effc97672fe21e82587e880d5942903f8b2c91b37a923b78df47a2ce3d6d6d`.

Administrative baseline:

- Workspace ID: `b7e70915-6d09-41b9-aff9-7eb5e34f0cb8`.
- Daemon generation: `c6d5220b-9eb9-4358-a1c8-6e13c3a7efd9`.
- Final observed workspace revision: `9`.
- Running session and instance pairs:
  - `bc310e69-6eb5-4a09-918e-d0be271c7d8f` / `331ccb65-0cec-47f1-85ad-be21bc8fce91`
  - `79a90de5-12cb-490d-bf31-aa3cbece52e4` / `fa5da7e2-6c88-409a-bd3f-66ec0dacbc21`
  - `c57ebc51-6b24-4313-9403-a1d795665ad1` / `4cda3005-d99a-45cb-a638-13b33d1c8c44`
- Conflict task ID: `dba71bfb-bdde-44e7-ad6e-e6897034054a`.

| Check | Expected result | Actual result and observer confirmation |
| --- | --- | --- |
| Initialize and native working directory | A fresh workspace opens and every `cmd.exe` child starts in the synthetic workspace directory. | The corrected candidate initialized cleanly. `cd` returned the exact synthetic workspace path with no UNC warning or fallback to the Windows directory. |
| Three live sessions | Three shells retain distinct output and stable identities while switching. | The three sessions printed `CMD_FIXED_1`, `CMD_FIXED_2`, and `CMD_FIXED_3`. `j`, arrow navigation, and reattachment returned to the intended retained screens. Administrative reads matched the identity pairs. |
| Alternate screen | A real full-screen child enters and exits cleanly. | Vim displayed `CMD_FULLSCREEN_OK` in its alternate screen at multiple sizes and returned to the original `cmd.exe` prompt and transcript. |
| Unicode | UTF-8 wide and combining sequences remain coherent. | After selecting UTF-8 and emitting text through PowerShell, the observer confirmed `Wide: 界 | Combining: é`. A transient replacement glyph in an earlier command echo was distinguished from the final child output. |
| Resize and minimum size | Wide, narrow, tall, full-screen, and undersized layouts recover. | Vim remained legible across the tested sizes. At 76 by 20 the 80 by 24 notice appeared, and enlarging restored the prior content. |
| Focus, help, and no color | Focus escape, help, and textual state remain usable. | `Ctrl+]`, keyboard help, `CURRENT`, ownership labels, and diagnostics remained legible without color. |
| Exclusive input and transfer | Two clients enforce one writer and support bidirectional explicit transfer. | Competing acquisition stayed read-only and recorded rejection diagnostics. `CMD_OWNER_A`, `CMD_OWNER_B`, and `CMD_OWNER_A_RETURN` appeared in both clients as ownership moved A to B to A. |
| Stale task form | A stale draft is protected and explicitly discarded. | The sequence `CMD conflict baseline`, `CMD local draft`, and `CMD saved externally` reproduced the generic rejection, retained the local draft, required explicit discard, and preserved the authoritative title. |
| Detach, reattach, and exits | Child context survives detach; `q` and `Ctrl+C` restore the outer `cmd.exe`. | Reattachment retained output. `q` and navigation `Ctrl+C` restored the outer prompt, cursor, screen, and input; `hello` and `CTRL_C_OK` were accepted. |
| Actual window closure | Closing the hosting window preserves the daemon, task, children, and context. | A new Windows Terminal window reopened the same generation, revision, task, and three identity pairs. Prior output remained, the abandoned lease was released, and `CMD_FORCED_CLIENT_OS_RECOVERY` later confirmed continued shell input. |
| Forced client failure | The exact Relayterm client can be killed without killing daemon or children. | The client PID was isolated from the daemon PID and forcibly terminated. The outer `cmd.exe` recovered under Windows, accepted `CMD_FORCED_CLIENT_OS_RECOVERY`, and administrative readback retained all durable and live identities. Residual screen content was not misclassified as Relayterm cleanup. |

Result: Passed on the corrected candidate. The later OpenSSH-only process-job correction does not affect this local non-job parent observation.

## MAN-WIN-3: Fresh Windows OpenSSH connection

The maintainer explicitly authorized installation and configuration of Windows OpenSSH Server for this test machine. OpenSSH Server capability installation completed without a required restart. `sshd` ran with manual startup. The default broad inbound rule was disabled, and a dedicated inbound rule allowed only IPv4 loopback source and destination addresses. The connection was a real fresh `ssh.exe` process to `127.0.0.1`, not a local shell substituted for SSH.

Environment recorded through the connection:

- Client terminal: Windows Terminal 1.24.11911.0.
- SSH client: OpenSSH_for_Windows_9.5p1 with LibreSSL 3.8.2.
- Server platform: Windows 10 Pro 10.0.19045.7663.
- Actual server shell: `C:\WINDOWS\system32\cmd.exe`.
- The isolated known-hosts file accepted the server key only after its fingerprint matched a separate administrative readback. The fingerprint, account, and hostname are intentionally not published.

### Interaction evidence retained from the first SSH candidate

Candidate `f3d0ca6e60d5fa1a6204e6e997d07d249a45086a`, SHA-256 `01effc97672fe21e82587e880d5942903f8b2c91b37a923b78df47a2ce3d6d6d`, completed the behavior below before connection-lifetime testing found the daemon defect. These behaviors are independent of daemon job breakaway and remain applicable:

- More than 60 seconds of idle time produced no transport-loss diagnostic, and input worked afterward.
- Three running `cmd.exe` sessions retained independent markers and stable identities while switching.
- Vim displayed `SSH_FULLSCREEN_OK`, survived wide, narrow, tall, full-screen, and undersized layouts, and restored the shell on exit.
- The 80 by 24 minimum-size notice appeared at 58 by 31 and enlargement restored the prior Vim screen.
- `Wide: 界 | Combining: é` rendered correctly.
- `Ctrl+]` returned to navigation, keyboard help opened, and all state remained understandable with `NO_COLOR=1`.
- Two SSH Relayterm clients showed one writer and one observer, rejected competing acquisition, replicated output, and transferred ownership explicitly in both directions.
- The stale task sequence `SSH conflict baseline`, `SSH local draft`, and `SSH saved externally` confirmed rejection, retained draft, explicit discard, and authoritative saved content. The conflict task ID was `4628f703-3653-4734-ba11-24fd21bf4005`.
- Detach and reattach returned to the same child context.

### Superseded connection-lifetime failure

After the first candidate returned from terminal input to the session list, the SSH connection closed. Administrative inspection found `sshd` still running but the Relayterm daemon and its supervised sessions had stopped. The daemon inherited the OpenSSH Windows job object, so closing the SSH job terminated the process tree despite detached process flags. This was a required product defect, not a test-environment limitation.

Source `136d46c2e5aeb3b26817a3574be3edb8d19a7ff9` requests `CREATE_BREAKAWAY_FROM_JOB` together with the existing detached-process flags. If Windows denies breakaway, the launcher retries the prior detached mode so ordinary restricted host jobs remain compatible. Focused tests, the complete workspace suite, Clippy with warnings denied, and two explicit hardening repetitions passed before the corrected manual retest.

### Corrected connection-lifetime evidence

Final candidate: source `136d46c2e5aeb3b26817a3574be3edb8d19a7ff9`, binary SHA-256 `41c49871965d91b628928d505a3f717c4969ad37fc0589eb2220e16368c4fd14`.

Administrative baseline and final comparison:

- Workspace ID: `07f8d106-837e-442f-a116-9ab59d92bc0c`.
- Daemon generation before and after all corrected closures: `ec5022ed-7136-4fa5-ae33-781979139fcd`.
- Final workspace revision: `17`.
- Durable sentinel task: `99e235a7-ed70-4e21-beda-81c84e45bc15` with title `SSH persistence sentinel` and backlog status.
- Running session and instance pairs before and after all corrected closures:
  - `46e598c4-4d8f-46bb-8901-cddb690b18ec` / `1204ebb6-6d8a-49c1-9a8d-c66d7a9a659a`
  - `c40648d3-0e0e-4b00-bb93-51bec4bbb188` / `37014001-2b41-453f-86a2-5ae64e8af4a7`
  - `cc40af84-6397-4396-b8f7-2a6e26df23cb` / `17e56f72-1bc5-4d86-925f-aac9d04365ae`

Three accidental extra sessions were terminated explicitly with `t`; their durable records remained honestly visible as terminated and were excluded from the three-running-session baseline.

| Check | Expected result | Actual result and observer confirmation |
| --- | --- | --- |
| Corrected launch and sessions | A fresh SSH workspace starts three shells in the synthetic directory. | The corrected candidate initialized directly through the real SSH connection. Three running shells printed `SSH_FIXED_1`, `SSH_FIXED_2`, and `SSH_FIXED_3`; `cd` returned the synthetic workspace. |
| Normal disconnect and fresh reconnect | Normal Relayterm exit followed by `exit` closes SSH without killing the daemon or children. | After `Connection to 127.0.0.1 closed.`, independent readback retained generation `ec5022ed...`, revision `16`, and all three identity pairs. A fresh SSH process reopened the workspace, retained `SSH_FIXED_1`, acquired input, and printed `SSH_NORMAL_RECONNECT_OK`. |
| Actual hosting-window closure | Closing Windows Terminal while SSH and Relayterm are active must preserve state and release the abandoned lease. | The entire window was closed while the client held `WRITER`. Readback before any reconnect retained the same daemon and sessions. A new Windows Terminal window, new SSH process, and new Relayterm client recovered the transcript, acquired `WRITER`, and printed `SSH_WINDOW_REOPEN_OK`. |
| Forced Relayterm-client failure inside SSH | Only the Relayterm client dies; the remote shell, daemon, and children survive. | Administrative process-tree inspection distinguished client PID `15708` from daemon and child processes. An elevated operator terminated that exact PID. The forced process could not restore the screen, but Windows returned control to the remote `cmd.exe`, which printed `SSH_FORCED_CLIENT_OS_RECOVERY`. The PID was absent afterward while generation, revision, and all three session pairs remained unchanged. |
| Durable identity around closure | Daemon generation, workspace revision, task ID, and child identities remain stable. | A synthetic sentinel task was created at revision `17`. Independent reads before and after a further normal SSH closure returned the same generation, revision, task ID/content/status, and three running identity pairs. |

Result: Passed on the final corrected candidate. The pre-correction daemon-loss result remains recorded as superseded evidence and is not counted as a passing persistence observation.

## Verification and remaining limitations

The native Windows correction work passed formatting, workspace compilation, Clippy with warnings denied, the complete workspace test suite, workspace build, two independent hardening gate repetitions, dependency policy, audit negative controls, repository policy, secret scanning, Gitleaks, and whitespace checks. The Windows process-breakaway unit test and the real TUI template regression that initially exposed the restricted-job compatibility fallback both passed.

The separate 180-second resource gate failed its reference throughput threshold twice on this test machine. It processed the same 388,786,246-byte workload at 1,960,566 and 1,947,797 bytes per second, below the 2,097,152-byte-per-second reference threshold. Both runs remained within the declared memory, plateau, handle, restart, reconnect, and abrupt-disconnect budgets. The result is recorded as a repeatable local performance miss and was not relabeled as a pass. Required native CI repetitions remain authoritative for the shared-runner budget.

The generic conflict message remains the non-blocking `UX-CONFLICT-MESSAGE` follow-up. Session list ordering remains the non-blocking `UX-SESSION-ORDER` follow-up. OpenSSH Server remains a test-machine dependency configured by explicit maintainer authorization; no external-network SSH claim is made.
