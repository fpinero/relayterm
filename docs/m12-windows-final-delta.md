# M12 Windows final delta

## Scope and decision

This report records the bounded Windows continuation on 2026-09-24. It preserves
the original failed attempts and reuses the completed native physical correction
round. No SSH or physical keyboard journey is repeated. M12 remains open.

| Windows gate | Decision | Reason |
| --- | --- | --- |
| Local debug input latency | Failed | Standalone and included hardening repetitions reproduce the unchanged 250 ms reference miss. Passing release measurements do not waive debug failures. Root cause is not yet established. |
| Independent native runtime | Blocked | The available Windows machine is the developer host. The operator requested using it. Constrained PATH and a disposable home do not prevent access to its checkout, compiler or installed runtimes. No independent environment was supplied. |

Completing this bounded investigation is not equivalent to passing either gate.
There is no maintainer waiver of Windows performance or runtime independence.
The supplied maintainer handoff reports completed Linux runtime, macOS restricted
runtime and final macOS SSH presentation including disconnected exit. Those are
handoff evidence, not Windows-local observations. The accepted forced SSH-client
reset and fixed-grid historical-output limits do not waive these Windows gates.

## Evidence identity

- Work branch: `fix/m12-windows-final-candidate`, preserved from the clean checkout.
- Initial Windows delivery and baseline harness: `25c1092849600dc833afefa4cfd58f3a6d1861fb`.
- Fetched and fast-forwarded base: `684582bcc40e3798abb63a39870374935041ca41`.
- Product: `691a8fbb45980658b98d647a85ea8305b2325938`.
- Version: `0.1.0`; empty CLI production feature set; `x86_64-pc-windows-msvc`.
- Developer host: Windows 10 Pro 22H2, build 19045.7725, x64, Intel Core
  i7-8665U, eight logical processors and 16 GiB physical memory.
- Automation shell: PowerShell 7.6.6; native child shells: system cmd.exe.
  Automated TUI measurements use ConPTY and vt100 parsing, not an observed
  Windows Terminal window. Historical physical observations used Windows
  Terminal 1.24.11911.0 and retain their original mapping.
- Debug: Cargo test profile, unoptimized with debuginfo, Rust/Cargo 1.98.1.
  Debug executable SHA-256:
  `2166952baac7b81cf6c571794b8ea3edf0a8488275aa3cac033dab3eff9c5025`.
- Retained release: original MSVC 19.29.30158/tools 14.29, SDK 10.0.19041.0,
  native release profile with the recorded path remapping and deterministic
  linker flags. No replacement product build was made.
- Host VCRUNTIME140.dll: 14.44.35211.0; UCRT ucrtbase.dll: 10.0.19041.7725.
  The installed x64 redistributable registry separately reports v14.42.34433.00.
  Registry package and actual DLL versions are not conflated.

The product and baseline harness are unchanged between 25c1092 and 684582b.
The optional newer Mac audit/runtime reports were absent; local-only Mac commits
f623597 and d8d789e were not treated as fetch prerequisites or recreated.

## Retained package

All files remain local to Windows, outside Git. The original two clean product
builds and package directories were preserved. Checksums were rechecked before
use, including the external manifest against SHA256SUMS and the archive member
against built and installed copies. Signature state is `NotSigned`.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| rt.exe | 11,804,160 | `2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708` |
| relayterm-0.1.0-x86_64-pc-windows-msvc.zip | 4,497,901 | `875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94` |
| relayterm-0.1.0-x86_64-pc-windows-msvc.manifest.json | 541 | `c480ec0df02791d06877dbd4be5f4b680051ee709e22c74aa3d99bcc83e589b1` |
| SHA256SUMS | 230 | `9d6561ea52eb02ce6fac36d9d8c048a845f44a0073870750d2595d1788f28cb5` |

The ZIP root is `relayterm-0.1.0-x86_64-pc-windows-msvc`. Its nine regular
members, with uncompressed byte sizes, are:

| Member | Bytes |
| --- | ---: |
| INSTALL.md | 6,069 |
| LICENSE | 11,558 |
| RECOVERY.md | 6,583 |
| THIRD_PARTY_LICENSES.txt | 472,415 |
| THIRD_PARTY_NOTICES.txt | 36,410 |
| install_release.ps1 | 2,219 |
| install_release.sh | 1,562 |
| manifest.json | 81,649 |
| rt.exe | 11,804,160 |

PE machine is 0x8664. The retained import inspection identifies VCRUNTIME140.dll
and UCRT API sets for math, string, heap, utility, time, runtime, stdio and locale;
there is no separate SQLite DLL. Host runtime availability is verified only for
this machine, not for a clean Windows image without its preinstalled components.

The hosted Windows release binary with SHA-256
`07680592eda496da1407c339e481f410b499d7c5d50cbcc71a8eb947bd3bd056`
is a different artifact. Its passing 124 ms input p95 is not the retained local
package or a replacement for either failed local debug run.

## Latency procedure and diagnosis

The original private logs were read before reruns. Ordinary workspace input was
208/290/337 ms (median/p95/max); serial included-hardening input was 207/294/445
ms. Neither log reaches throughput reporting, and their successful startup-test
output was captured rather than printed. Missing measurements remain unavailable.

Before measurement, both `CI` and `RELAYTERM_TEST_RT` were absent. No inherited
setting was erased. `CI` stayed absent throughout, so the local 100 ms navigation
and 250 ms input references applied, not the hosted 500/1,000 ms guardrails.
Only release subprocesses received the explicit retained installed-binary override.
The baseline command compiled the two test targets with `--no-run` first. There
was no concurrent build, other native test/runtime journey or parallel test execution.
Lightweight read-only source/evidence inspection continued during the runs.
Ordinary desktop background activity remained present and was not terminated.
Pre-run CPU samples describe load, not continuous per-core or thermal isolation.

The operator was told the repetition count before execution: three standalone
TUI, three included hardening, then three of each with the retained release.
All twelve attempts are retained. Two-second total-CPU samples ranged from
4.7% to 44.3%; rows with high background load are not relabeled as idle runs.

Exact baseline commands, each executed three times in the declared profile:

```text
cargo test -p relayterm-cli --test tui_gate --test hardening_gate --locked --no-run
cargo test -p relayterm-cli --test tui_gate tui_initializes_launches_detaches_and_reopens_without_stopping_children --locked -- --exact --nocapture --test-threads=1
cargo test -p relayterm-cli --test hardening_gate tui::tui_initializes_launches_detaches_and_reopens_without_stopping_children --locked -- --exact --nocapture --test-threads=1
```

For release subprocesses only, set `RELAYTERM_TEST_RT=<retained-install>/rt.exe`.
Harness and neutral Rust fixture remain debug executables in both comparisons;
only the product executable changes profile. Tests use disposable synthetic
projects/homes and their existing native serialization lock.

### Disclosed diagnostic instrumentation

The baseline harness file blob is `d2c9bf0c3cfc75878473e3aac4376d15070220ca`.
This delivery adds diagnostic-only harness blob
`1347e0aa1f545442f68f6eb9148938fc6dc30f57`. It prints microsecond statistics and
the exact-duration reference comparison, and reads the completed synthetic flood
marker through an independent administrative attachment after collecting all
input samples but before asserting latency. It prints only numeric metrics,
never captured terminal contents. Unavailable readbacks are labeled unavailable.
The original later throughput assertion remains unchanged. Production code,
fixture workload, sample count, ordering of timed samples and budgets are unchanged.

One additional standalone and one included repetition per profile were declared
before running, four total. They are separate diagnostic attempts, not replacements
for the baseline. Admission samples total CPU every two seconds, seeks two
consecutive readings below 15%, and records all readings with a 30-second bound.
This does not guarantee that background load stays low during the test.

### What the investigation establishes

Five of six baseline debug runs fail; all six baseline installed-release runs
pass. The issue reproduces with each scenario alone, without an overlapping
build, including attempts admitted with low aggregate CPU. Neither full-workspace
parallelism nor a silently selected release executable explains all failures.
Debug medians around 188-230 ms are consistently above release medians around
125-126 ms. This supports a profile-dependent timing difference, not attribution
to a particular product function.

The 250 ms printed baseline failure is not a changed budget: the existing log
uses `Duration::as_millis()`, which truncates, while the assertion compares full
durations. Its sub-millisecond value was not retained and cannot be reconstructed.
New microsecond output and `reference_target_met` remove this reporting ambiguity.

Source inspection identifies a serialized input/refresh/render loop, a 33 ms
terminal refresh interval and a 20 ms test observation poll. Full terminal cell
snapshots and serialization are possible debug-cost contributors. No timing
trace isolates one of these as the cause. No product fix, budget increase or
maintainer waiver is claimed. Next work must profile the measured Windows debug
input path before changing product behavior, then rerun both exact scenarios and
the same installed comparison. Any product change needs explicit artifact impact
mapping and affected native/CI rechecks before replacing the retained package.

There is also an existing workload-coverage limitation: the fixture writes only
80 chunks (2,621,600 bytes total), before navigation and input sampling. A finite
burst and its measured completion rate do not prove sustained concurrent output
through all 100 input samples. The executable harness asserts at least 1 MiB/s;
the acceptance matrix describes at least 2 MiB/s concurrent output. Several
passing rows fall below 2 MiB/s. Passing the unchanged harness is therefore not
new proof of that stronger load requirement. This delivery does not silently
alter the workload or reinterpret the specification; final reconciliation must
address the coverage gap while preserving these results.


### Complete measurement ledger

Triples below are median / nearest-rank p95 / maximum in milliseconds. Each
navigation and input row has 100 samples; each startup result has 20 after one
warm-up. Each numbered row is one invocation, not an average across repetitions.
N/A means not emitted or not part of that exact scenario. Baseline integers are
truncated by the original logger. Diagnostic decimals derive from microseconds.
Throughput is the finite flood's bytes/s, not a sustained-overlap claim.

| Historical local attempt | Startup | Navigation | Input | Bytes/s | Local reference result |
| --- | --- | --- | --- | ---: | --- |
| Ordinary workspace debug, standalone, 2026-09-21 | N/A | 21 / 22 / 25 | 208 / 290 / 337 | N/A | Failed |
| Serial workspace debug, included hardening, 2026-09-21 | N/A | 21 / 43 / 44 | 207 / 294 / 445 | N/A | Failed |
| Installed release suite, 2026-09-21 | 156 / 159 / 162 | 21 / 21 / 22 | 125 / 127 / 243 | 2,416,397 | Passed |

Historical local CPU samples were not retained. These failed workspace runs
used the original harness; their missing throughput cannot be recovered from a
later invocation. Earlier build-overlap shutdown failures remain documented in
[the chronological Windows record](m12-windows-manual-observations.md); they did
not produce a complete latency measurement and are not omitted passing attempts.

| Current attempt | Product profile | Scenario | CPU before, % | Navigation | Input | Bytes/s | Local reference result |
| --- | --- | --- | ---: | --- | --- | ---: | --- |
| B1 | debug | standalone | 11.4 | 21 / 42 / 42 | 230 / 381 / 657 | N/A | Failed (exit 101) |
| B2 | debug | standalone | 44.3 | 22 / 43 / 47 | 188 / 250 / 424 | N/A | Failed (exit 101) |
| B3 | debug | standalone | 4.7 | 21 / 42 / 43 | 187 / 189 / 208 | 1,915,128 | Passed |
| B4 | debug | included hardening | 6.1 | 21 / 22 / 45 | 189 / 314 / 379 | N/A | Failed (exit 101) |
| B5 | debug | included hardening | 29.0 | 21 / 43 / 43 | 188 / 253 / 314 | N/A | Failed (exit 101) |
| B6 | debug | included hardening | 7.6 | 21 / 22 / 42 | 190 / 307 / 399 | N/A | Failed (exit 101) |
| B7 | retained release | standalone | 9.5 | 21 / 22 / 22 | 125 / 147 / 170 | 2,016,688 | Passed |
| B8 | retained release | standalone | 7.4 | 21 / 21 / 22 | 125 / 127 / 146 | 2,142,628 | Passed |
| B9 | retained release | standalone | 6.7 | 21 / 21 / 22 | 125 / 148 / 239 | 2,636,798 | Passed |
| B10 | retained release | included hardening | 7.7 | 22 / 23 / 24 | 126 / 149 / 162 | 1,350,783 | Passed |
| B11 | retained release | included hardening | 11.5 | 21 / 21 / 22 | 126 / 148 / 150 | 2,910,221 | Passed |
| B12 | retained release | included hardening | 44.0 | 21 / 21 / 22 | 126 / 147 / 174 | 2,506,055 | Passed |
| D1 | debug | standalone | 3.4 | 21.126 / 21.686 / 42.458 | 189.141 / 300.060 / 486.511 | 2,399,820 | Failed (exit 101) |
| D2 | debug | included hardening | 11.6 | 21.397 / 22.004 / 42.366 | 188.148 / 252.350 / 401.006 | 2,057,945 | Failed (exit 101) |
| D3 | retained release | standalone | 4.2 | 21.190 / 21.910 / 22.110 | 125.461 / 126.469 / 144.803 | 2,782,093 | Passed |
| D4 | retained release | included hardening | 7.5 | 21.298 / 21.799 / 22.060 | 125.817 / 147.561 / 207.393 | 2,667,993 | Passed |

B1-B12 use the unchanged baseline harness. D1-D4 use the disclosed diagnostic
harness. All four diagnostic admissions reached the two-sample CPU condition.
Seven of eight debug attempts failed; all eight retained-release attempts passed.
The passing debug B3 is preserved and does not resolve the other misses. B2's
printed 250 ms is a failure under the full-duration comparison, not a pass.
Startup was not timed in these journey invocations.

Two separately declared startup invocations used the diagnostic harness and
exact command below, once per product profile. They are not repeated full TUI
journeys. Both passed the unchanged two-second startup reference.

```text
cargo test -p relayterm-cli --test tui_gate existing_daemon_reaches_usable_screen_within_budget --locked -- --exact --nocapture --test-threads=1
```

| Current startup invocation | CPU before, % | Startup median / p95 / maximum, ms | Result |
| --- | ---: | --- | --- |
| debug | 3.6 | 279 / 312 / 332 | Passed |
| retained-release | 12.0 | 204 / 209 / 212 | Passed |

The separate historical hosted observations below were read back from retained
logs, not rerun here. They all met the local reference on their runner, with CI
classification still enabled. They do not establish local Windows performance.

| Hosted invocation at e4431f7 | Startup | Navigation | Input | Bytes/s |
| --- | --- | --- | --- | ---: |
| Native standalone 1 | 169 / 170 / 189 | 21 / 22 / 22 | 190 / 212 / 274 | 2,496,314 |
| Native standalone 2 | 169 / 190 / 190 | 21 / 22 / 43 | 208 / 229 / 291 | 2,468,680 |
| Native included hardening 1 | 169 / 189 / 189 | 21 / 22 / 42 | 190 / 211 / 232 | 2,567,583 |
| Native included hardening 2 | 169 / 170 / 189 | 21 / 22 / 42 | 208 / 210 / 292 | 2,569,291 |
| Hosted installed release | 85 / 85 / 85 | 21 / 21 / 21 | 124 / 124 / 125 | 3,039,959 |

## Native runtime procedure and results

The user selected the current Windows machine. A fresh project, private home,
installation, synthetic collision and backup destination were created outside
the checkout. The packaged installer and product ran from that disposable root.
Python/PowerShell orchestration stayed on the developer host; no isolation is
claimed for it. No OS feature, account, execution policy, service or security
setting was changed. No provider account was used.

Capability discovery found no WindowsSandbox.exe and no guest in the Hyper-V
virtualization inventory, although the host virtualization services are running.
There was no identified independent Windows guest to use. A fresh account alone
would not prove exclusion of developer tooling. This blocks only independent
runtime acceptance; it does not invalidate the explicitly classified host checks.

Safe boundary controls deliberately exposed the limitation: a child cmd.exe
could not resolve `cargo` through the constrained PATH (expected nonzero status),
but the same user could still access the checkout and the compiler's absolute
path. This is a failed isolation boundary, not a passing sandbox test. The actual
daemon loaded VCRUNTIME140.dll 14.44.35211.0 and ucrtbase.dll 10.0.19041.7725 from
System32. A clean image's runtime availability or missing-runtime behavior was
not tested. Network isolation was not enforced.

The following aliases stand for distinct absolute disposable paths, not real
personal paths: `PACKAGE`, `EXTRACTED`, `INSTALL`, `COLLISION`, `PROJECT`, `HOME`,
`BACKUP` and `RESTORED`. BACKUP and RESTORED were new destinations under the
product-created private data directory. All CLI calls used the exact installed
binary, `--workspace PROJECT --home HOME --format json`, and file-backed output
with a 30-second bound. RESTORED replaced HOME only after the original stopped.

| Check | Procedure or exact command shape | Result and evidence class |
| --- | --- | --- |
| Package/inventory/hash | `python scripts/package_release.py inspect PACKAGE.zip`; verify archive and external manifest against SHA256SUMS before extraction | Passed, current host automation; nine entries and original product hash |
| Packaged install | `powershell.exe -NoProfile -File EXTRACTED/install_release.ps1 -ReleaseRoot EXTRACTED -Destination INSTALL` | Passed, current host automation; no global PATH change |
| Collision and repeat refusal | Run the same helper against COLLISION containing an unrelated synthetic rt.exe, then again against INSTALL | Both rejected with exit 1; unrelated sentinel and candidate bytes unchanged |
| Discovery | Process-local `PATH=INSTALL;System32;Windows`; PowerShell `(Get-Command rt -CommandType Application).Source; rt --version`; cmd.exe `/d /c "where rt && rt --version"` | Both resolved the exact installed file and version 0.1.0 |
| Help/version | `rt --help`; `rt --version` | Passed from the disposable project |
| Initialization/daemon | `workspace init --name "Windows runtime validation"`; `workspace status` | Started detached daemon; later independent CLI clients connected |
| Three native shells | Register three neutral enabled definitions for absolute system cmd.exe with `/Q`, then `session create --definition-id ID` | Three concurrent real ConPTY shells running; independent marker readbacks passed |
| Names/order | `session rename ID --name "Shell N" --expected-revision REV`; `session list-ordered` | Shell 1, Shell 2, Shell 3 retained creation order and stable identities |
| Claim conflict | Create synthetic task, `task transition ID ready --expected-revision REV`, claim with first instance, claim with second | Second claim rejected, exit 5 and request_rejected; revision unchanged |
| Progress/handover/successor | `progress append ID --stdin`; `handover create ID --expected-revision REV --stdin`; `handover get ID`; successor claim; transition to done | Structured content read back, predecessor released, successor completed |
| Automated TUI/continuity | The eight installed-release standalone/included TUI journeys in the ledger | Passed on host ConPTY with debug fixture harness; initialization, launch, detach/reopen and child continuity covered. This is separate from the three cmd.exe CLI journey and is not independent-runtime proof |
| Normal exit/cursor/usable shell | Reuse the completed physical Windows correction round and both restored PowerShell prompts | Historical operator observations; no new physical claim or repeat requested |
| Private live backup | `backup create --destination BACKUP` while three shells run | Passed with private parent; pre-backup complete ordered sessions, task, history and claims retained for comparison |
| Fresh-home restore | Stop original with `daemon stop --terminate-sessions`; `backup restore --source BACKUP --destination RESTORED`; `workspace open` using RESTORED | Same workspace/session/instance identities, names and ordinals; task/history/claims exactly equal; all former live sessions honestly lost |
| Owned-file removal | Stop the identified restored daemon, verify owned path and executable hash, `Remove-Item -LiteralPath INSTALL/rt.exe` | Only this new installed executable removed; every recorded state/backup file and unrelated marker hash unchanged. Original retained installation remains |
| Independent environment | Repeat the applicable installed procedure in an identified independent Windows boundary with positive/negative access controls | Blocked; host checks above do not close this row |

The private command ledger preserves exact resolved invocations and statuses.
Synthetic definitions used system cmd.exe, one `/Q` argument, terminal capability,
enabled state and PATH allowlisting. Task/progress/handover inputs contained only
synthetic text. No test helper was installed as a product runtime dependency.
Optional Git/worktree evidence remains the previously passing installed gate.

### False start and disposition

The first private runner correctly received exit 1 from `where cargo` but then
incorrectly asserted that stdout must be nonempty. Missing-command diagnostics
were on stderr, so the runner stopped before workspace initialization. This is
a runner assertion failure, not evidence of product failure or isolation success.
The original runner and command records were retained. After correcting the
stdout assumption, execution resumed from that checkpoint using the same install
and collision files. It did not overwrite state or claim a second fresh full run.
The resumed native CLI journey and module inspection completed successfully.

A later positive access control initially asserted the repository-pinned compiler
version outside the checkout. The absolute compiler ran successfully but selected
the user's default Rust 1.98.0 there, so that overly specific assertion failed.
The original probe is retained. Repeating the control with its actual criterion
(successful absolute compiler execution) passed and confirmed the missing isolation
boundary. Performance builds ran inside the pinned checkout with Rust 1.98.1;
the external probe did not change toolchains, settings or measured binaries.

### Independent environment preparation still required

Minimum future setup: one identified disposable Windows x64 VM or Windows
Sandbox instance, with no shared developer checkout/toolchain and with the three
retained package files copied into it. Keep originals outside it. Inventory OS,
architecture and actual UCRT/VCRUNTIME versions before execution. If the x64
runtime is missing, obtain separate authorization for its official installation;
record before/after state rather than silently adding it. Creating a VM, enabling
Sandbox or changing security needs the maintainer's authorization under this
handoff. None was performed here.

Use positive controls that can read the copied package and create disposable
private state, and negative controls that cannot read the developer checkout or
execute its compiler by absolute path. PATH absence alone is insufficient. Run
the installed procedure and automated TUI/continuity inside that boundary, then
compare restored identities, claim/history and lost-session state. Reuse the
already completed physical observations. Preserve originals and stop only owned
validation processes. Until these controls and checks run, the gate stays blocked.

## Cleanup and verification

The test harness cleaned its owned test daemons. Host-runtime daemons were
stopped through their exact installed binary and explicit workspace/home. Final
inspection found no process using either retained or new installed release.
One debug daemon predating this continuation (2026-09-21) was identified and
preserved; it was not killed by name or included as an owned cleanup target.
Raw logs, attempt JSON, captures, original runner, state, backups and package
copies remain outside Git. No private artifacts were uploaded.

The retained release hash is unchanged. The debug executable hash is also
unchanged after diagnostic-harness compilation. Only the test harness and public
evidence changed. No native release rebuild or complete CI run was performed.
`cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
The exact native tests and their failures are listed above; full workspace test
success is not claimed. The original passing CI does not certify the new
diagnostic lines, which have the separate local executions above.

Public-evidence validation passed `python scripts/check_repository.py` (70
Markdown files and eight ADRs), `python scripts/check_secrets.py` (including
rejection of its synthetic secret negative control),
`python scripts/check_audit_controls.py` (license and dependency-ban negative
controls), and `git diff --check`. The final logbook check compares every
pre-existing byte with the saved base and requires an unchanged prefix.

Read-only public API queries reconfirmed successful
[Quality 35637866692](https://github.com/fpinero/relayterm/actions/runs/35637866692)
and [Security 35637869615](https://github.com/fpinero/relayterm/actions/runs/35637869615)
at e4431f7b357fe1388963fa94675a567875492fc3. The initial `gh run view` attempt
could not authenticate; the public API supplied the readback without requesting
credentials. No workflow dispatch occurred. Branch-push CI is not expected:
both workflows restrict push triggers to main. Publication must still inspect
the actual delivery SHA for any triggered runs and verify remote equality.

## Remaining global prerequisites and transfer

1. Diagnose and resolve or explicitly reconcile the failed Windows debug gate,
   including the finite-flood coverage mismatch. No implicit waiver exists.
2. Complete the independent Windows runtime procedure above. Host success is
   insufficient, even though the same package passes its functional checks.
3. On Mac, reconcile its local f623597/d8d789e reports and supplied completed
   runtime/SSH evidence with the older shared queue. Do not repeat SSH or physical
   correction tests to compensate for documents absent on Windows. Keep the
   maintainer's two accepted limitations recorded with the original failures.
4. Collect the retained Linux, Mac and Windows package/manifest/SHA256SUMS sets
   at an authorized final inventory location. Linux executable/archive identities
   remain `26f7f90e27da12239c3130e1163951a4cfe2f08e448fc8b28f6a46cec642d8d7` and
   `a9e5bb84b474ab8f37e9778046ec530b3833df67261b294549d1352df63a3285`.
   Read Mac's exact retained identities from its local report; do not guess them.
5. Reconcile all 16 acceptance criteria, phase exit conditions, runtime floors,
   performance coverage and the final three-platform artifact inventory. Only
   then consider final M12 integration into main and a separately authorized
   release decision. No main merge, tag, release, branch deletion or artifact
   publication is authorized by this Windows evidence delivery.

Windows retains the ZIP, external manifest and SHA256SUMS together in its
original package directory. Mac does not yet have them. A concrete next transfer
is to use the maintainer's chosen authenticated file channel to copy exactly
those three files unchanged into a new `windows-x64-691a8fb` directory on Mac,
then run `shasum -a 256 -c SHA256SUMS` there and inspect the manifest. An scp
equivalent, after the destination is supplied and authorized, is:

```powershell
$Files = @('relayterm-0.1.0-x86_64-pc-windows-msvc.zip', 'relayterm-0.1.0-x86_64-pc-windows-msvc.manifest.json', 'SHA256SUMS')
foreach ($File in $Files) {
    scp (Join-Path $PackageDirectory $File) $AuthorizedMacDestination
    if ($LASTEXITCODE -ne 0) { throw 'Transfer failed.' }
}
```

No destination was supplied, so no transfer was attempted. Do not rebuild for
transfer or include packages in source commits. See the self-contained
[Mac continuation](m12-windows-final-delta-continuation.md).
