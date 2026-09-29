# ConPTY evaluation integration and evidence boundaries

## Candidate reconciliation

The Windows delivery on 2026-09-29 contains three patches against 43ba692.
The original sustained harness was already integrated in f8837ee on Mac. Its
reverse-application check passed; it was not applied twice. Patches 01 and 02
were reviewed and applied in order, retaining the Mac evidence and append-only
logbook. Their SHA-256 values are:

- Diagnostic delta: c9f02aee39bad96b9de2d52bc7a554e49a08bd23ced9bcc502d8654a3531ef6a.
- Evaluation delta: bcb17d353a23bdfe818c246df309004e5a817412675fb61697f32f2048bd35db.

The complete chain also applied in an isolated checkout of the original base.
All attachment checksums, separate report copies and nine delivered LF source
hashes matched. The original sustained workload/helper remains byte-identical.
Raw reports, numerical data, intermediate attempts and private artifacts remain
outside version control. This document is the sanitized reconciliation.

The delivered product changes refresh a terminal snapshot before drawing and
limit each daemon reader parse batch to 16 KiB rather than 64 KiB. The subsequent
Mac compatibility comparison is recorded below, separately from the exact import. No output is
dropped, no retention bound is changed, and no protocol or persistence migration
is introduced. Opt-in aggregate diagnostics compile under test-hooks; ignored
fixtures are distinct from acceptance. The Windows native tests preserve their
behavioral assertions while distinguishing runtime screen representation and
using the documented native release-control byte.

## Explicit experimental runtime

The [isolated evaluation scripts](../scripts/conpty_evaluation/README.md) pin
Microsoft.Windows.Console.ConPTY 1.24.260710001 and a delta to
portable-pty-psmux 0.9.7. The dependency delta is stored as a patch, not silently
installed in the repository. Cargo.toml and Cargo.lock remain unchanged. Ordinary
builds still use the system backend, even after these source changes are applied.

The experiment requires a separately prepared dependency, explicit Cargo path
override and explicit absolute runtime selection. The adapter retains the input
endpoint and provides a bounded, split-safe DA1 startup response without removing
output. Its supplied invalid runtime selection fails rather than falling back.
Windows reports pinned package hash and Microsoft signature checks; Mac has not
loaded, downloaded or executed the Windows runtime. The pure bootstrap matcher
can be tested independently without claiming Windows FFI coverage.

The evaluated source is a new product candidate. Earlier retained binaries and
hosted checks for 691a8fb do not certify its changed TUI/reader. Preserve unaffected
historical functional evidence, but verify affected behavior and map new artifacts
before final acceptance. This integration does not adopt a distribution runtime.

## Imported Windows results

The non-corporate host was Windows 10 Pro 10.0.19045, x64, Intel i7-8665U with
16 GB RAM. It is distinct from the earlier Windows 11 VM runtime journey.
Hypervisor presence does not establish a guest or a cause. The final serial
sequence had CI and RELAYTERM_TEST_RT absent, no reader instrumentation or
concurrent compilation, and unchanged workload/latency thresholds.

| Run | Observations | Minimum interval bytes/s | Navigation p95 ms | Echo p95 ms |
| --- | ---: | ---: | ---: | ---: |
| Debug 1 | 223 | 2,209,760 | 21.497 | 166.717 |
| Release 1 | 225 | 3,095,082 | 21.057 | 83.264 |
| Debug 2 | 222 | 2,408,758 | 21.519 | 166.748 |
| Release 2 | 224 | 3,095,476 | 21.073 | 102.272 |
| Hardening debug | 222 | 2,307,866 | 21.525 | 166.771 |

All five passed the 120-second span, full-screen progress, 100 samples per latency
category, all three load windows and unchanged local budgets. Mac independently
ran the reviewed evidence verifier across all 1,116 observations and 1,000 latency
samples, using conservative microsecond-rounding denominators. This checks the
exported arithmetic and command records; it is not a Windows re-execution.

The earlier system-backend five failures and historical debug misses remain in
[the preceding reconciliation](m12-windows-sustained-reconciliation.md). The packet
also preserves the experimental runtime's intermediate failures before refresh,
reader and adapter corrections. A valid-load debug echo miss of 253.530 ms, later
invalid-load attempts, writer-lifetime failures, startup handshake and native
input/screen assumptions were not relabeled as final passes. The combined final
candidate passed; the comparisons do not identify one exclusive cause for every
historical failure or an OS defect.

Reported Windows regression passes include workspace libraries, all-target Clippy
with and without all features, daemon protocol/PTY/runtime gates, eight selected
TUI tests, original PTY tests and TUI controls with the runtime absent. The legacy
finite-burst journey was not in that TUI selection. An explicitly enabled resource
run passed memory/reconnect limits: daemon maximum 363,503,616 bytes, TUI maximum
32,796,672 bytes, fixture child maximum 47,640,576 bytes, and daemon handles 324
initially versus 322 after 100 reconnects (20 abrupt). OpenConsole helper memory
was not separately measured; no owned helper remained after the final sequence.

## Remaining distribution work

Before adopting the experimental runtime into ordinary Windows builds, define
runtime bundling and discovery, trust/integrity validation, licensing/notices,
update ownership and supported architectures/OS scope. Choose the default and
failure/fallback behavior explicitly, then validate the actual packaged candidate
with standard accounts. Include helper-process resources and lifecycle in the
inventory. Preserve explicit package/executable/source mappings; no retained
package is replaced by the experiment. Avoid installing dependencies or changing
host security settings implicitly.

Linux/hosted checks, affected final artifact coverage, corporate ACL/reputation
disposition and global acceptance remain open. Corporate probes stay deferred.
Completed physical, SSH and Windows VM functional journeys were not repeated.

## Mac verification

Verification uses the ordinary native Mac backend, without the experimental
runtime, CI, product override or reader metrics variables. Formatting and
all-feature/all-target Clippy passed. The delivered pure bootstrap matcher test
passed when compiled independently with rustc; this does not exercise Windows
FFI. Python syntax and the preparation script's Windows-only guard passed without
downloading anything or creating the requested artifact directory. PowerShell is
not installed here, so the Windows runner was reviewed rather than executed.

The initial full workspace run exited 101. Its hardening sustained inclusion
passed at navigation/echo p95 25/101 ms. The later tui_gate sustained inclusion
failed two conservative intervals at approximately 41 and 42 seconds, at
1,820,336 and 1,405,222 bytes/s. Its screen progressed, gaps remained bounded and
its producer completed the target total. The echo window passed, but the entire
load and navigation windows failed, so that attempt's latency is not acceptance
evidence. Observer duration reached 71.220 ms. Neither producer totals nor that
observer timing establish the cause of the two failed intervals.

The failure is retained with raw private observations. Source and thresholds were
not changed in response. The follow-up verification runs the remaining workspace
checks with only this sustained selector excluded, then a focused debug repeat,
a release sustained check and the explicitly enabled resource scenario. The remaining workspace checks passed. The focused debug repeat failed one
interval at approximately 16 seconds (2,085,676 bytes/s), with observer duration
95.105 ms; its echo window passed. Release passed at navigation/echo p95 26/75 ms.
The explicitly enabled resource scenario passed with 60 observations, daemon
maximum 367,673,344 bytes, TUI maximum 21,807,104 bytes, fixture child maximum
18,448,384 bytes, and 48 daemon handles both before and after 100 reconnects.
These results refer to the delivered all-platform 16 KiB candidate.

To isolate the shared reader-batch change, a local compatibility variant keeps
16 KiB on Windows and restores the pre-existing 64 KiB on other platforms. It
keeps TUI refresh ordering, diagnostics and the acceptance helper unchanged.
Native Mac PTY and runtime tests passed on this retained compatibility variant.
Two focused debug repetitions and one release check passed serially, with all
three load windows valid and unchanged thresholds. This is a local source delta;
the final supervisor file intentionally differs from the delivered Windows hash.
Its Windows branch retains the evaluated 16 KiB batch; no Windows execution of
this integrated source tree is claimed. Linux retains its previous 64 KiB value,
but the shared TUI change still needs native Linux verification.

| Final Mac variant | Navigation p95 ms | Echo p95 ms | Whole-load consumed bytes/s |
| --- | ---: | ---: | ---: |
| Debug 1 | 25 | 119 | 3,145,661 |
| Debug 2 | 25 | 120 | 3,145,722 |
| Release | 27 | 75 | 3,145,698 |

Each final run collected 239 observations, more than 120 seconds of qualified
load and 100 samples of each latency category. Builds preceded measurement;
no verification/compiler job ran concurrently with these timed runs. Exact raw
samples, source and executable identities are retained privately. The comparison
supports preserving the existing Unix batch while limiting the tested smaller
batch to Windows. It does not prove a unique cause for every earlier pause.

The full remaining workspace suite and resource check described above exercised
the imported 16 KiB candidate. Final-variant checks include both debug/release
builds, default and all-feature/all-target Clippy, native PTY/runtime regressions,
workspace library tests and the three focused sustained passes. The complete
workspace integration suite and resource scenario were not repeated after the
platform guard; their source boundary remains explicit. Default full-suite green,
Linux/hosted checks and packaged-runtime acceptance are not implied.

Final integrated supervisor SHA-256: d5faf6bcba3044ee2af86db0e1bd84d5b3cb7bb7f05804acfe9859bd62d40219.
