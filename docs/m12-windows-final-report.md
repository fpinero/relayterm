# M12 Windows final validation report

## Decision and evidence identity

The focused Windows physical correction round is complete. No further operator
visual check is required for its unchanged candidate. This is not global M12
acceptance: local debug latency, remaining Linux/SSH presentation,
independent clean-runtime proof and final acceptance have distinct gates.

Delivery branch: `fix/m12-windows-final-candidate`. Original delivery:
`b9871e1936157dcd2bcd64ceb8f33da3f6220a38`. Product source:
`691a8fbb45980658b98d647a85ea8305b2325938`. Verified ancestry connects the
Linux correction, product candidate and delivery. e23be95 changes only the test
harness and documentation; e4431f7 records completed physical evidence. No
production change or untested replacement binary was introduced in Windows.

The [chronological Windows evidence](m12-windows-manual-observations.md) retains
individual observations, commands, failures and readbacks. This report summarizes
the final state without replacing that history.

## Native builds, packages and installation

Environment: Windows 10 Pro 22H2, build 19045.7725, x64; PowerShell 7.6.6;
Windows Terminal 1.24.11911.0; Rust/Cargo 1.98.1; MSVC 19.29.30158,
tools 14.29; Windows SDK 10.0.19041.0. Version 0.1.0, empty production
feature set, unsigned. Native target: `x86_64-pc-windows-msvc`.

| Retained artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Installed rt.exe | 11,804,160 | `2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708` |
| Normalized ZIP | 4,497,901 | `875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94` |

Two isolated clean native source checkouts produced byte-identical executables
and packages through the checked-in release scripts. External manifest and
archive checksums passed before extraction; the archive had exactly nine
expected entries. PE inspection confirmed x64, linker 14.29, VCRUNTIME140 and
UCRT dependencies. Authenticode reported NotSigned.

Extracted smoke passed help/version, initialization, detached daemon, real ConPTY
child and orderly shutdown. The packaged installer created a dedicated install;
installed hash/help/version and process-local command discovery passed in both
PowerShell and cmd.exe. Synthetic collision refusal preserved the unrelated
sentinel, repeat installation was refused, and an additional Unicode/space-path
install passed hash/version checks. These developer-host checks do not certify
an independent clean Windows runtime. No global PATH, SSH or security setting
was changed. Earlier installations, runtime homes and backups were preserved.

## Automated evidence and failures

| Verification | Observed result and limit |
| --- | --- |
| Exact-installed presentation, TUI, worktree and backup/restore gates | 12 passed, one helper ignored, corrected delivery harness; same installed product hash |
| Release performance | Startup p95 159 ms; navigation p95 21 ms; input echo p95 127 ms; flood 2,416,397 bytes/s |
| Source/tooling | Formatting, all-target check, Clippy with denied warnings, core-only tests and workspace build passed |
| Security/privacy | cargo-deny advisories/licenses/bans/sources, history gitleaks, audit negative controls and candidate secret checks passed |
| Release-tool tests | 22 tests, four POSIX-only skips, no failures |
| Non-CLI workspace tests | 166 executions passed, eight ignored, serial execution, exit zero |
| Ordinary full debug workspace | Failed input p95 290 ms against unchanged 250 ms local reference |
| Serial full debug workspace | Failed input p95 294 ms in included hardening TUI journey; not converted into a pass |
| Earlier loaded runs | Agent-template/disconnected shutdown failures preserved; those paths passed later, without erasing original logs |

The passing release measurement does not waive either debug failure. No timeout,
reference target or hosted guardrail was increased. Full workspace success is
not claimed. M12.WINDOWS-DEBUG-LATENCY remains a bounded investigation, not a
request for another user visual session.

Original hosted [Quality 35629938330](https://github.com/fpinero/relayterm/actions/runs/35629938330)
failed both Windows jobs at documentation descendant 167563e; five other jobs
passed. The Windows failures concerned wrapped logical versus physical footer
rows and a literal Unix alternate-screen escape assertion under ConPTY. Later
Windows steps were skipped. [Security 35629941870](https://github.com/fpinero/relayterm/actions/runs/35629941870)
passed at that same source identity.

The local harness correction reads physical rows for the footer, adds a wrapping
regression, requires successful exit and visible cursor on all platforms, and
limits the literal Unix escape assertion to Unix. Windows terminal restoration
was independently observed physically. It does not weaken product performance
budgets or replace the tested executable. New hosted results are recorded below
when available; a dispatched or running job is not a passing result.

## Physical observations and independent state checks

| Operator observation | Independent corroboration and result |
| --- | --- |
| CURRENT, INPUT, WRITER followed by Ctrl-] | NAVIGATION/READ ONLY appeared on Sessions; repeated release retained the screen |
| A kept Draft A; B saved Saved B | Same session identity; persisted revision moved from 4 to 5 only for B's save |
| A's stale submission rejected | Error with draft/cursor retained; full winning snapshot unchanged |
| First Ctrl-R review, second Ctrl-R adoption | Separate review; updated Saved B context; retained draft/cursor; wrapped Info guidance required explicit Ctrl-S |
| Adoption and safe discard | Entire persisted winner remained revision 5; no automatic write or overwrite |
| Empty task submission | Error: Title is required.; form/cursor retained; independent empty task snapshot unchanged |
| Fresh writer immediately before authorized daemon stop | Exact candidate daemon and disposable state identified; stop returned lifecycle stopped and exit zero |
| Disconnect | DISCONNECTED/NAVIGATION/READ ONLY/RECONCILE WITH R; no confirmed WRITER or misleading Attaching |
| Repeated release while disconnected | Sessions retained; Events selected only deliberately |
| More than 20 seconds in Events | Same four diagnostics, exactly one transport-loss notice, no continuing flood |
| Both client exits | Successful exit messages, restored PowerShell prompts/cursors and synthetic echo output in both terminals |

Final process inspection found zero processes using the exact installed
candidate, with its hash unchanged. The daemon was not restarted. Other
processes were preserved. Private images, logs, runtime state, backups and
artifacts remain outside Git. Header and writer-size observations continue to
refer to their original 0bc4b63/9cf91f7 binaries, with unchanged-code mapping;
they are not falsely reported as new observations of 691a8fb.

## Linux handoff and remaining ownership

Use [the Linux delta continuation](m12-linux-final-delta-continuation.md).
Linux's completed 9cf91f7 journey and 15794ad correction round must not be
repeated. Only changed local Info/Error presentation and any still-uncovered
SSH presentation delta need operator observation. Native final packaging,
installed gates and available clean-runtime preparation are autonomous work.

Keep the original failed SSH-client interruption recovery and fixed-grid history
limitations visible for acceptance disposition. No new full M11/M12 journey,
another Windows visual round, M13, merge, tag or release is implied by publishing
this evidence branch.

## Authorized branch delivery and hosted recheck

The operator authorized branch publication after completing the physical round.
The branch was pushed without force, and remote HEAD was verified as e4431f7.
No PR, merge, tag or release was created. Quality and Security were explicitly
dispatched for that commit because branch pushes do not trigger these workflows.

- [Security 35637869615](https://github.com/fpinero/relayterm/actions/runs/35637869615)
  completed successfully on e4431f7.
- [Quality 35637866692](https://github.com/fpinero/relayterm/actions/runs/35637866692)
  completed successfully on e4431f7: all seven jobs passed. These include all
  three native release jobs, stable Linux/macOS/Windows and pinned Linux.

The Windows stable job completed all 26 independent repetition steps, including
both TUI, hardening, fault, SQLite, cancellation, containment, scale, offline and
sustained-resource passes, plus full workspace/core/build/documentation checks.
The original failed Quality run remains historical evidence. This closes
M12.WINDOWS-HOSTED-RECHECK. The native Windows log reports input p95 values of
212/229 ms in the two direct TUI passes and 211/210 ms in hardening, all meeting
250 ms on that runner. These do not erase the local debug 290/294 ms failures.
Final run/job metadata and Windows job logs are retained privately.

Later commits containing only this report, continuation guidance and evidence
can reuse those hosted results with the explicit ancestor mapping. They do not
justify repeating the user's completed physical tests or replacing artifacts.

### Hosted Windows release identity

Completed job 106459661237 independently built and packaged e4431f7 twice on
Windows Server 2022. Its retained log reports identical copies within that
runner, not equality with the local Windows 10 artifacts above:

| Hosted artifact | Bytes | SHA-256 |
| --- | --- | --- |
| rt.exe | 11,761,664 | `07680592eda496da1407c339e481f410b499d7c5d50cbcc71a8eb947bd3bd056` |
| ZIP | 4,475,867 | `876cb82c106d51185f7552b1f77f90b332b2ac780e876815ec4e4904a76fd278` |

The extracted TUI gate passed eight tests with one ignored helper; presentation,
worktree and backup/restore added one, two and one passing tests respectively.
Hosted startup p95 was 85 ms, navigation 21 ms and input echo 124 ms. The latter
met the 250 ms local reference as well as the hosted guardrail. These are hosted
measurements, not the local 159/21/127 ms observations. Archive metadata was
read from the job log; no downloadable retained CI archive is claimed. The
operator-tested installed binary remains unchanged with its original hash.
