# M12 Windows ordinary distribution validation

## Scope and source mapping

M12 remains open. This checkpoint integrates and verifies an ordinary Windows x64
package on a non-corporate Windows 10 Pro development host, build 10.0.19045.
The host has four CPU cores, eight logical processors and 16,915,288,064 bytes
of physical memory. Hypervisor presence alone does not establish a VM journey.
Automation used a non-elevated filtered administrator token. It does not prove
an interactive standard-account journey or a clean-machine installation.

Delivered baseline: 5ca6ab9fbacbaf6035ffae2f387610d325791659. Product build source:
f2b3f6f446466f5b3657c3cd91746735cab59d78. Subsequent report/logbook commits do not change production code.
The branch is fix/m12-final-validation in a separate clean local clone. The original
dirty checkout, unique patches and historical artifacts remain preserved. Thirteen
original source-file hashes were compared again and remained identical.

The complete locked workspace passed at d346650b8e3f12c20eeca39c52a05ca38a197585:
207 passed, 27 ignored, exit 0 in 842.657 seconds. Later production differences are
canonical runtime metadata and rustfmt-only Windows adapter changes. Final adapter
unit tests and all-feature/all-target lint passed, followed by exact-package gates
on the newly built executable. Native measurements are scoped to their recorded
binaries; no untested rebuild is substituted. The sustained harness is unchanged
from the delivered baseline. The Windows reader remains 16 KiB and Unix 64 KiB.

See [runtime contract](windows-runtime-distribution.md) and
[architecture decision](decisions/0009-windows-packaged-conpty.md).

## Artifact and runtime identities

Two clean native builds in separate directories produced identical executable
bytes. Two archives also matched byte for byte. Build features are empty, profile
release, target x86_64-pc-windows-msvc, Rust/Cargo 1.98.1. The archive is unsigned.

- rt.exe SHA-256: ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa (11835904 bytes).
- Archive SHA-256: 039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50 (5113812 bytes).
- Microsoft.Windows.Console.ConPTY 1.24.260710001 package SHA-256: 175640566a3b59c4b132070ee96c2c77e5ab7edd2e92732a5eb3610bbf63d90e.
- conpty.dll SHA-256: 39fba2713e2495117b1591ae8c32a3b904bea7aa66069cf7815e2844c76d75d8.
- OpenConsole.exe SHA-256: b7fd936c2668b87b9ecf7b3366dc6568afc1c6f981874cba3e955a1c35cf8160.

Microsoft signatures and package/file hashes passed. PE inspection identified all
three files as machine 0x8664 and verified the three Conpty-prefixed ABI exports.
The imported VCRUNTIME140.dll exists in System32, version 14.44.35211.0. No system
installation or private tool DLL was used to establish that prerequisite.
The exact 13-file inventory, license, notices, provenance and hashes passed archive
inspection, constrained-PATH smoke and extraction. Installed bytes matched extraction.
Reinstallation/helper collisions preserved existing bytes; owned removal preserved
foreign markers and external private state. Rollback and corrupted-helper cases
passed the native installer tooling tests. PATH resolution was verified in a child
process without changing persistent PATH.

## Automated checks

Workspace formatting, separate vendor formatting, default lint and all-feature/
all-target lint with warnings denied passed. The six adapter tests passed. The
release/tooling set ran 31 tests with four POSIX skips and no failures. Native
PTY, IPC, permissions, lifecycle, refresh/input, coordination, worktree, backup/
restore and hardening coverage passed in the locked workspace. Explicit SQLite
kill, Git cancellation and ignored Git-descendant gates also passed.

Offline cargo deny advisories/licenses/bans/sources, license/ban negative controls,
repository contracts, candidate secret negative control and Git-history gitleaks
passed. Duplicate-version warnings remain allowed. Cached advisories do not prove
fresh hosted security evidence. Hosted CI was not triggered.

The final installed release passed TUI (10 active tests, six explicit ignores),
session presentation, worktree and populated backup/restore gates. It also passed
runtime negatives for missing DLL/helper, wrong architecture and altered DLL/helper,
with numeric diagnostics and no private-state mutation. Valid ambient CWD/PATH/
experimental inputs did not rescue an invalid executable directory.
RELAYTERM_TEST_RT was set only in exact-package test children; their record includes
the release product hash before and after execution. Ordinary native gates removed
CI, experimental runtime selection and reader diagnostic/delay variables.

## Sustained evidence

The required native sequence was debug/release/debug/release, then debug hardening.
Focused runs had 360-second external limits. Independent arithmetic recomputed every
consumed interval/window, producer count and latency sample with unchanged limits:
2 MiB/s, at least 120 seconds, 100 navigation and echo samples, maximum three-second
sampling gaps, navigation p95 100 ms and echo p95 250 ms. All six rows qualified.
Printed microsecond arithmetic is retained separately from native nanosecond assertions.

| Scenario | Observations | Navigation p95 ms | Echo p95 ms | Maximum echo ms | Whole-span consumed bytes/s |
| --- | --- | --- | --- | --- | --- |
| ordinary-debug-1 | 222 | 21.475 | 167.435 | 273.040 | 3145627 |
| ordinary-release-1 | 224 | 21.099 | 102.194 | 103.628 | 3145732 |
| ordinary-debug-2 | 221 | 21.424 | 167.975 | 313.351 | 3145606 |
| ordinary-release-2 | 224 | 21.052 | 83.328 | 178.824 | 3145770 |
| ordinary-hardening-debug | 223 | 21.484 | 147.667 | 168.097 | 3145095 |
| installed3-tui | 224 | 21.555 | 84.828 | 103.527 | 3145697 |

The first five rows map to d346650 and the recorded native debug/release binaries.
The last row uses the final packaged release hash above through a debug harness.
Maximum latencies are retained even when greater than the p95 budget. Historical
seven debug misses, five load failures and experimental/VM/Mac evidence are preserved
separately. They are not waived, overwritten or reassigned to this package.

## Helper resources and lifecycle

The final ignored gate recorded 44 observations over the original
180-second sample duration. Each observation includes eight daemon-owned OpenConsole
helpers, the outer harness helper and raw daemon/TUI/fixture memory and handles.
The live inventory is 10 product processes (daemon, TUI and eight helpers), eight
fixture children, one outer helper and the test harness itself.

Charge daemon plus its helpers to the original 512 MiB daemon limit, and conservatively
charge TUI plus the outer harness helper to the original 512 MiB TUI limit. Both retain
the original 32 MiB plateau allowance and minimum 40 samples.

- Daemon with owned helpers maximum: 425713664 bytes.
- Daemon-group steady medians: 425508864 to 425693184 bytes.
- TUI with outer helper maximum: 36851712 bytes.
- Aggregate daemon/helper handles: 1483 before, 1481 after 100 reconnects, including 20 abrupt watchers, within the unchanged tolerance of 16.

Helper identities remained stable across reconnects. The loaded daemon module was
verified at the installed executable directory. Forced exit of a positively identified
synthetic child left seven helpers; normal fixture exit left zero; a fresh shell made
one; daemon shutdown left zero. The outer helper exited after closing its master.
All original memory/plateau/handle checks passed. Numeric helper/PID observations and
independent calculations are in the [shared evidence](evidence/m12-windows-distribution-20260930/README.md).

## Failed attempts and corrections

Every failed/intermediate attempt remains in a separate timestamped private record.
The command ledger retains exit codes, deadlines, elapsed times, source/lock/runtime/
binary identities and raw-log hashes. Pre-driver preparation/inspection failures
have a separate retained ledger. Corrections included missing Rust imports, test-module
ordering, fixture/installer discovery, stale executable-only backup assumptions,
stale system-backend alternate-screen expectations and the resource terminate selector.

The expanded helper gate initially waited for the outer helper while its master was
still open; closing the master before the exit assertion fixed the harness lifecycle
check. A clean build exposed a CRLF license hash versus canonical LF mismatch; the
original source hash is retained and both checkout formats now have a regression test.
A synthetic test hash fixture then required correction. Vendor formatting differences
were corrected and a new pair of artifacts was built and tested.

The first observer waited for native pipeline EOF for 180 seconds while its synthetic
daemon lived. The directly owned observer process was terminated and the exact fixture
daemon was stopped. Bounded one-line JSON capture now completes without waiting for
descendant EOF. WindowsIdentity.Groups omitted a disabled administrative SID; whoami
CSV correctly reports the filtered administrator token. The initial printed true is
not accepted as account evidence. A private driver path typo prevented one final test
sequence from starting; its traceback and empty child directory are preserved.
No test threshold was changed to obtain success.

## Remaining evidence

The [standard-account observation](m12-windows-standard-observation.md) subsequently
passed on 2026-10-01 with this exact retained package. Verified shared write/read,
standard token, exact per-invocation readback, observer/TUI exit 0 and normal fixture
cleanup accompany explicit operator confirmations for input, narrower/wider redraw
and normal restoration. All package/archive/fixture hashes passed return-time checks;
the seven recorded fixture process identities were gone. A preceding integrity-stage
failure remains retained and does not qualify as success. No extra process was killed.
The dedicated account is on the same development host, not an independent machine.
No account creation, policy change, elevation, system installation or rebuild was
performed by this continuation.

Mac/Linux affected dependency, tooling and package regressions remain for their native
hosts. Final-source/global acceptance, clean-machine/runtime-floor independence and
practical trial remain open. Existing unaffected physical, SSH and Windows 11 VM
journeys are reused. Corporate execution remains deferred; no FortiClient causality
or ACL/reputation root cause is established. No push, merge, tag, release or upload
occurred at the verification checkpoint. Subsequent source/evidence branch sharing
was explicitly authorized. See the [Mac/Linux handoff](m12-windows-to-unix-handoff.md).
