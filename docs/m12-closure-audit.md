# M12 closure audit and next handoff

## Current closure gates on 2026-10-01

This table is the current execution queue. Dated checkpoints below retain their
historical findings and artifact identities; their older next-action statements
do not restart work completed by later checkpoints. A documentation-only change
does not require a rebuilt package or repeated physical journey.

The [ordinary Windows report](m12-windows-distribution-validation.md) closes
runtime integration, ordinary debug/release sustained measurements and helper
resource/lifecycle automation for its exact f2b3f6f package. The
[Mac distribution regression](m12-macos-distribution-regression.md) closes
native Mac dependency/tooling, workspace, resource/fault, reproducible package,
exact-extracted tests and POSIX installer checks at e712603. These are distinct
artifacts; earlier failures are retained and global acceptance is still open.

| Remaining gate | Next bounded action | Evidence that must be reused |
| --- | --- | --- |
| Native Linux delta | Follow the [bounded Linux handoff](m12-linux-distribution-handoff.md) for affected dependency/tooling, locked workspace/resource/fault and exact-package checks; Mac execution cannot certify Linux. | Existing unaffected Linux physical and isolated-runtime observations. |
| Windows operator delta | Run observe_windows_package.ps1 once in an existing actual standard account and normal console using the final Windows package hash. Record input, narrower/wider redraw, normal shell exit and restored console. A filtered administrator token is insufficient. | Passed ordinary package, sustained/helper, installation and automated runtime-negative checks. No new Windows build is required for this observation. |
| Runtime floor and independence | Reconcile existing Mac/Linux/Windows VM isolation and declared oldest OS with changed dependencies/artifacts. Execute only genuinely missing proof, recording its environment; do not relabel developer-host smoke as an independent machine. | Existing Mac isolation, Linux independent runtime and Windows 11 VM journey, each at its original source. |
| Hosted and final acceptance | Obtain current native hosted quality/security coverage when explicitly authorized, then reconcile AC-1 through AC-16, phases and the three-target artifact inventory. | Old hosted jobs retain ancestor scope; local cached cargo deny is not fresh hosted security coverage. |
| Practical trial | Complete the prepared private real-agent implementation/test/review handovers with the operator and an explicitly hashed executable. | Existing trial preparation and private state; version probes alone are not authentication or interactive success. |
| Publication | Prepare the reviewed inventory and request the maintainer's concrete publication decision after technical gates pass. | No release, tag, merge, PR or binary upload is authorized by this continuation. |

Corporate-host diagnosis remains deferred. Do not access that host, change
security settings or infer FortiClient/ACL/reputation causality. Preserve the
recorded failure; corporate deferral is not a new test request or an acceptance
waiver. Existing accepted fixed-grid and interrupted-SSH limits remain unchanged.

For the Windows operator delta, use the existing source script against the
retained package, not a newly rebuilt executable:

```powershell
.\scripts\observe_windows_package.ps1 -CandidateRoot 'C:\validation\installed' -ExpectedHash 'ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa'
```

Replace only the synthetic candidate directory with the actual retained package
location. Confirm standard_account=true. Follow the script's session/input/resize/
exit instructions. Retain its operator-observation.json privately and provide
only cropped synthetic session and restored-console screenshots plus confirmation
of input and redraw. Do not include account names or private paths in public reports.
This is a Windows operator step, not a request to repeat Mac screenshots.


## Integrated Mac checkpoint on 2026-09-30

The [integrated Mac validation](m12-macos-integrated-validation.md) closes the
local full-workspace/resource repetition gap after the Unix reader guard.
Commit 6e57332 passed the complete default suite, explicit resource/fault checks,
two reproducible production builds/packages and affected tests through the exact
extracted release. Debug sustained p95 navigation/echo was 30/110 ms in both
inclusions; packaged release was 31/78 ms. All load windows qualified.
No product change or budget adjustment was needed. Historical failures remain.

The Mac artifact is newly mapped, not a replacement for retained 691a8fb evidence.
Windows runtime distribution and actual packaged acceptance, Linux/hosted checks,
OS-floor/independence reconciliation and global sign-off remain open. Keep the
operator-guided trial and publication choices for later operator availability.
No corporate probe or repeated physical/SSH/VM journey was performed.

## Experimental runtime continuation on 2026-09-29

The [ConPTY evaluation reconciliation](m12-conpty-evaluation.md) supersedes the
next-action portion of the earlier failed-load checkpoint. Five final Windows
runs pass with the explicit experimental runtime plus TUI/reader corrections.
Only the missing diagnostic/evaluation deltas are integrated on Mac. Ordinary
Windows packaging remains unchanged and is not covered by those passes. The
changed product requires new candidate mapping and affected native/hosted checks.
No global acceptance or corporate-host retry follows.

## Windows continuation on 2026-09-29

The [native Windows sustained reconciliation](m12-windows-sustained-reconciliation.md)
records five failed load gates on Windows 10, separate from the Windows 11 VM
runtime journey. Their latency observations cannot qualify acceptance. Continue
local diagnosis and evidence-supported correction on the Windows development
host, then review/integrate its patch and run affected Mac/Linux checks. Avoid
parallel edits to the same implementation on Mac. Corporate probes remain deferred;
historical failures, retained packages and global M12 acceptance stay separate.

## Work sequence on 2026-09-28

Continue with the [sustained performance harness](m12-sustained-performance.md),
then a mapped non-corporate Windows comparison and the local operator-guided
practical trial. The corporate host is excluded from further probes or retries
at the operator's request to avoid security alerts. This is a deferral, not a
confirmed application-reputation root cause or a waiver of the recorded failure.
Local harness verification now passes two Mac repetitions per profile and the
hardening inclusion. The Windows patch/prompt and private operator trial are
prepared. Native Windows/Linux measurements and the actual real-agent trial
remain pending. No global acceptance, branch delivery or release is implied.

## Latest evidence on 2026-09-25

The [prepared Windows VM reconciliation](m12-windows11-vm-reconciliation.md)
records passing standard-account runtime, coordination, real-console TUI and
populated recovery gates, with original init response capture unavailable. Earlier
Windows-runtime pending statements below are historical checkpoints. Corporate
ACL failure, debug latency, sustained-load measurement and final acceptance remain
open. Reuse completed Mac/SSH and VM journeys; do not infer FortiClient causality.

## Update on 2026-09-24

The independent Windows 11 runtime gate failed during initialization. See the
[ACL review and diagnostic contract](m12-windows11-acl-review.md) for imported
evidence, Mac reconciliation and the next bounded engineering step. Downstream
Windows checks are blocked. Mac runtime and SSH remain passed. Debug latency,
harness-load reconciliation and global acceptance remain open. This continuation
does not authorize remote delivery or host configuration changes.

## Decision on 2026-09-23

M12 is not ready for final acceptance or a completion merge. The three native
physical rounds, reproducible packages and hosted checks are complete for the
mapped candidate. Independent Windows runtime proof and Windows local debug performance
disposition remain open. The bounded [SSH presentation delta](m12-macos-ssh-final-delta.md)
now passes, including normal exit from the disconnected client. The macOS isolated native runtime
now passes as recorded in the [runtime report](m12-macos-final-runtime.md). The
maintainer has explicitly accepted the two documented terminal limits below. Do not repeat completed physical journeys to resolve these gaps.

This audit completes M12.CLOSURE-AUDIT, not M12.06. It does not change product
code, budgets, historical failures or the MVP exclusions. Publication policy,
signing, tags and releases remain separate from technical candidate acceptance.

## Repository reconciliation

Fetched origin and checked ancestry without modifying existing user material.
The starting macOS checkout contained an unrelated untracked reviewer document;
it is preserved and excluded from this delivery. The initial audit used
`feature/m12-closure-audit`. Final validation continues on
`fix/m12-final-validation`, based directly on the latest Linux delivery and
containing only the selected public audit material. The requested real-agent
trial remains in an ignored local directory, outside this branch history.

| Evidence boundary | Commit |
| --- | --- |
| Current local and remote main | `96c3139` |
| Product used for final native packages | `691a8fbb45980658b98d647a85ea8305b2325938` |
| macOS published delivery | `b9871e1936157dcd2bcd64ceb8f33da3f6220a38` |
| Passing hosted checks | `e4431f7b357fe1388963fa94675a567875492fc3` |
| Windows published delivery and Linux harness | `25c1092849600dc833afefa4cfd58f3a6d1861fb` |
| Linux published delivery | `684582bcc40e3798abb63a39870374935041ca41` |

These commits form one ancestor chain in the listed order after main. Linux
already contains the Windows and macOS deliveries. No cherry-pick or separate
merge of those branches is needed. Between product 691a8fb and Linux 684582b,
the only non-documentation change is `crates/relayterm-cli/tests/tui_gate.rs`:
physical footer rows, a wrapping regression, platform-aware restoration checks
and administrative test diagnostics. Product sources, dependencies, migrations
and release scripts are unchanged.

Read-only GitHub queries reconfirmed [Quality 35637866692](https://github.com/fpinero/relayterm/actions/runs/35637866692)
and [Security 35637869615](https://github.com/fpinero/relayterm/actions/runs/35637869615)
on e4431f7. Quality's three native release jobs, three stable jobs and pinned
Linux job all succeeded. Security succeeded. Private vulnerability reporting is
enabled. No workflow was dispatched for this documentation audit.

## Artifact reconciliation

All three retained native candidates declare version 0.1.0, product 691a8fb,
production features only and unsigned distribution. Per-platform equality means
two builds/packages with the same declared native inputs, not equality between
operating systems or with separate hosted artifacts.

| Target | Executable bytes and SHA-256 | Archive bytes and SHA-256 | Audit basis |
| --- | --- | --- | --- |
| Linux x86_64 GNU | 13,034,640; see note below | 4,838,446; `a9e5bb84b474ab8f37e9778046ec530b3833df67261b294549d1352df63a3285` | Linux delivery report, not a local Mac copy |
| macOS arm64 | 11,050,320; `4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060` | 4,386,181; `4d8c1dda4d7fa9ea412928365e8dbc9b550f02d0f3766b3462c1cd7832955007` | Retained local files checked again in this audit |
| Windows x86_64 MSVC | 11,804,160; `2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708` | 4,497,901; `875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94` | Windows delivery report, not a local Mac copy |

Linux executable SHA-256:
`26f7f90e27da12239c3130e1163951a4cfe2f08e448fc8b28f6a46cec642d8d7`.
The [Linux report](m12-linux-final-report.md),
[macOS report](m12-macos-correction-candidate.md) and
[Windows report](m12-windows-final-report.md) retain build/runtime inventories,
commands, installed gates and physical evidence separately.

The Mac audit independently hashed both retained archives, both manifests,
the archived executable and the extracted executable. External SHA256SUMS,
`package_release.py inspect`, and `package_release.py compare` with the two
manifest paths passed. The inventory contains exactly nine expected entries.
`otool -L` reports only libiconv.2.dylib and libSystem.B.dylib; the extracted
executable reports rt 0.1.0. No artifact was rebuilt or replaced.
Initial comparison invocations used directory/archive operands and one wrong
working directory; they failed before mutation. The corrected manifest-based
command passed. These invocation errors are not product failures or new builds.

Linux independent Ubuntu runtime evidence passes. The [macOS isolated runtime](m12-macos-final-runtime.md) now passes with enforced
dependency isolation. Windows developer-host installation alone still does not
establish its independent clean-runtime acceptance.
The final artifact freeze remains conditional on the open gates below.

## Acceptance criteria reconciliation

This table concerns M12 closure. It preserves the passed M11 baseline in the
[acceptance matrix](acceptance-matrix.md) and the source mapping above. Supported
rows do not imply that phase 5 or global acceptance has passed.

| Criterion | Reusable evidence and current disposition |
| --- | --- |
| AC-1 | Installed initialization and private state pass; final clean quick start remains open on Windows. |
| AC-2 | M11 daemon/IPC and final extracted/installed gates support this row; no affected product delta after 691a8fb. |
| AC-3 | Three-session native journeys, stable identities/names/order and exact-installed gates support this row. |
| AC-4 | Three native physical correction rounds pass. Changed SSH presentation passes; local Windows input latency remains open; the fixed-grid limit is accepted. |
| AC-5 | Exclusive claim regressions and installed journeys support this row; task claims remain separate from terminal input leases. |
| AC-6 | Progress/handover journeys and hosted regression coverage support this row. The remaining Windows clean-runtime journey must exercise their packaged entry points. |
| AC-7 | Successor/resume and durable context evidence support this row; reuse existing physical observations. |
| AC-8 | Child survival, stable identity and reconnect evidence remain valid. The bounded changed SSH presentation passes; the interrupted outer-terminal recovery limit is accepted. |
| AC-9 | Restart/lost-session and backup/restore gates support this row; no claim that live processes survive daemon restart. |
| AC-10 | Exact-installed worktree gates and native journeys support this row. The empty Linux manual backup fixture does not prove populated-worktree restore. |
| AC-11 | Passing core-only/native checks and unchanged product dependency direction support this row. |
| AC-12 | Seven Quality jobs and Security pass on the mapped ancestor. Independent runtime packaging acceptance remains a separate M12 prerequisite. |
| AC-13 | Source/history/privacy controls and native archive inventories support this row; raw evidence and private runtime data remain outside Git. |
| AC-14 | M11 offline/SSH and Linux independent no-network runtime evidence pass; complete the applicable Windows clean-runtime workflow without hosted prerequisites. |
| AC-15 | Neutral custom commands and agent-template regressions remain supported by unchanged source and final hosted checks. |
| AC-16 | Single binary, installer discovery and collision preservation pass on developer hosts; independent Windows installation remains open. |

Phases 0 through 4 retain their completed milestone and final regression
coverage: architecture/CI, durable state/IPC, PTY supervision, TUI coordination,
and neutral templates/worktrees. Phase 3's changed SSH presentation now has passing
final-candidate observations. Phase 5 cannot close until the outstanding acceptance and
clean-machine criteria pass. No new critical/high security exception was added.

The resource inventory and fixed workloads in the acceptance matrix remain
unchanged. Retained installed input p95 measurements are 154 ms on Linux and
127 ms on Windows. The Windows local debug runs measured 290 and 294 ms, misses
of 40 and 44 ms against 250 ms. Neither serial execution nor passing release/CI
measurements establishes a cause or resolves those failures.

## Finite remaining work

| Pending task | Owner or prerequisite | Required completion proof |
| --- | --- | --- |
| M12.NATIVE, Windows | Same prerequisite on supported Windows x86_64 | Use the retained Windows package, inspect UCRT/VCRUNTIME prerequisites and run the bounded installed journey. Mac/Linux cannot substitute for ConPTY runtime evidence. |
| M12.WINDOWS-DEBUG-LATENCY | Original Windows debug environment and private logs | Diagnose the 290/294 ms misses using controlled same-source/harness comparisons; retain failures and unchanged budget, document cause and fix or explicit evidence-backed acceptance disposition. |
| M12.06 and M12.07 | All technical prerequisites above | Final AC/phase/budget reconciliation, frozen three-target artifact inventory and concrete handoff; publication decision remains separate. |

At the initial audit checkpoint, no authorized SSH destination or usable
independent native runtime had been established. The operator subsequently
enabled system Remote Login temporarily. The bounded loopback SSH observations
and enforced-isolation macOS native runtime now pass in their linked reports.
The agent did not change SSH/security settings or create an alternate listener.
Windows still requires its own native runtime evidence; a VM is not mandatory.
Docker or Mac execution cannot substitute for Windows ConPTY coverage.

## Accepted terminal limits

On 2026-09-23 the maintainer explicitly accepted both documented limits for this
candidate: forced local SSH-client termination can prevent terminal restoration
bytes from arriving and require a local `reset`; fixed-grid history does not
reflow and can lose clipped historical cells after narrowing. Fresh output must
still use the current writer dimensions. Normal exit must restore the terminal.

This closes M12.CORRECTION-PLATFORMS as an explicit limitation disposition, not
as a passing automatic-recovery observation. Original failed observations remain
in the Linux report. AC-4/AC-8 retain child-survival, identity and normal-exit
requirements. The outstanding Windows native runtime and
performance gates are not waived by this decision.

The maintainer also requested main delivery and releases for the three targets.
That is explicit delivery/publication authorization. Full MVP acceptance cannot
be inferred from it. Stable publication remains unready while mandatory evidence
is absent; a possible explicitly limited prerelease requires its own clear
status and verified obtainable artifacts. No remote delivery occurred in this
audit checkpoint.

## Bounded Windows diagnostic procedure

Use the actual retained logs before rerunning anything. Confirm the product SHA,
harness SHA, profile, machine/runtime, output rate and environment classification.
`assert_latency` in `tui_gate.rs` applies 250 ms locally and a separate hosted
guardrail only when CI is set. `hardening_gate.rs` includes the same TUI scenario;
the two failures are related workload observations, not two independent product
bugs. Root cause remains unestablished.

Run the affected scenario alone under the ordinary local classification:

```powershell
cargo test -p relayterm-cli --test tui_gate tui_initializes_launches_detaches_and_reopens_without_stopping_children --locked -- --exact --nocapture --test-threads=1
cargo test -p relayterm-cli --test hardening_gate tui::tui_initializes_launches_detaches_and_reopens_without_stopping_children --locked -- --exact --nocapture --test-threads=1
```

For debug reproduction ensure RELAYTERM_TEST_RT is not redirecting the command
to the retained release. Record and preserve existing environment settings; do
not set CI to bypass the local target. Compare controlled debug and release
results with the same workload, declared fixture/harness and load, recording
median/p95/max and throughput. Profile a reproduced miss before changing code.
Do not attribute it to debug overhead or machine load without measurements.
A product correction requires a new candidate and affected evidence; a test-only
correction requires honest harness mapping without silently changing budgets.

## M13 boundary and CLI continuation prompt

No M13 definition exists in TODO.md at any of the 64 inspected local/remote
references. No commit subject matches M13. The current specification's MVP plan
ends at phase 5 and the roadmap ends at M12. Therefore an M13 coding contract
cannot be derived from the checked-in roadmap. Do not invent a new feature or
rename unfinished M12 acceptance work as M13. The maintainer redirected the next work to MVP delivery and a real-agent trial,
without supplying a new M13 scope.
Do not create an M13 plan. The requested next exercise is the
real-agent MVP trial, retained locally outside version control and proposed
separately from acceptance.

Use this bounded prompt to finish M12 before authoring an M13 implementation plan:

```text
Continue Relayterm from fix/m12-final-validation and its closure audit.
Read AGENTS.md, PROJECT_VISION.md, MVP_TECHNICAL_SPEC.md, README.md,
TODO.md, docs/M12_details.md, docs/m12-closure-audit.md, the final three
platform reports, docs/acceptance-matrix.md and the latest avances.md.
Inspect status, preserve unrelated changes, fetch origin and verify ancestry.
Linux delivery 684582b already contains Windows 25c1092 and macOS b9871e1.
Product source is 691a8fb; later changes are documentation and test harness.

Work on the earliest available remaining M12 prerequisite in the audit.
Reuse completed native physical rounds, retained packages and passing Quality
35637866692/Security 35637869615. Do not rebuild, rerun complete CI or request
another full visual journey because documentation changed. Keep product,
package, harness, automation and physical evidence identities separate.

Reuse the completed macOS isolated runtime and final SSH presentation report.
Complete independent native Windows runtime checks when an appropriate
environment is available. Diagnose Windows debug input-p95 misses 290/294 ms
without increasing 250 ms or substituting passing release/hosted results.
The listed SSH presentation delta has passed; do not repeat it without a
concrete product change invalidating its evidence.
Do not modify SSH/security configuration or kill the SSH client again.
Both documented terminal limits were explicitly accepted on 2026-09-23;
preserve the failed observations and the scope of that acceptance. Continue independent work when blocked.

Keep TODO pending-only and avances append-only. Close M12 only after every
mandatory gate passes or receives an explicit, documented permitted disposition.
Report exact commands, hashes and unavailable prerequisites. Preserve failures.
No M13 implementation until M12 is closed and its new scope is defined.
No push, merge, tag or release is authorized by this reusable prompt alone.
Communicate in Spanish; write documentation and code comments in English.
```
