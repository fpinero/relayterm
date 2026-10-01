# M12 macOS distribution regression

## Result and source identity

On 2026-10-01, native Apple Silicon macOS 26.5.2 (build 25F84) passed the
affected Unix continuation of the ordinary Windows runtime integration at
`e712603cc753686675319358f0413946461249e2`. The original checkout, unrelated
local document, retained packages, practical-trial state and historical failures
were preserved. A separate checkout used `fix/m12-final-validation` at the exact
requested commit. No production code, dependency, threshold or host setting changed.
M12 remains open; this closes the local Mac distribution regression task.

The [Windows package](m12-windows-distribution-validation.md) still maps to
product source `f2b3f6f446466f5b3657c3cd91746735cab59d78` and executable
`ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa`.
Its evidence is imported Windows execution, never a claim about this Mac build.
All 21 shared evidence hashes passed, and independent parsing reproduced
15 logs, 11 sustained sections and 11 qualified sections. Failed overall Windows
commands remain failed even when an individual section qualified.

Public-safe observations, command outcomes and hashes are available in the
[Mac evidence index](evidence/m12-macos-distribution-20261001/README.md).
The [Linux handoff](m12-linux-distribution-handoff.md) defines the affected next
native continuation; this sharing adds no new product execution.

## Native verification

All 16 bounded native driver commands passed without timeout:
workspace/vendor rustfmt; default and all-feature/all-target Clippy with warnings
denied; vendored library tests; the six Python distribution/evidence test modules;
repository contracts; candidate secret scan and synthetic negative control;
offline cargo deny and license/ban negative controls; prebuilt workspace tests;
complete workspace; explicit resource gate; SQLite kill; Git cancellation;
and ignored Git descendant containment tests.

The workspace ran 204 passing tests, zero failures and 25 explicit ignores across
48 test/doc-test groups in 392.457 seconds
with a 1,200-second external bound. The native vendored library ran one Unix test;
six Windows adapter tests were not executed on Mac. Python ran 31 tests with
five Windows-only skips. Explicit resource/fault/descendant checks ran separately.
Git-history Gitleaks scanned 289 commits and found no leaks. Audit results use
cached advisories, with allowed duplicate-version warnings, not fresh hosted evidence.

The Unix and serial adapter sources match the cached upstream crate after
line-ending normalization. The library adds a Windows-only preflight export;
the command-builder difference removes unavailable external upstream test paths.
The Unix reader remains 64 KiB; Windows remains 16 KiB. No native Windows or
Linux coverage is inferred from Mac execution.

## Sustained and resource results

Independent printed-microsecond arithmetic recomputed all intervals/windows,
producer records and 100 navigation plus 100 echo samples per section. Native
assertions use nanoseconds. Every section met the unchanged 2 MiB/s load,
at least 120 seconds, maximum three-second sample gap and 100/250 ms p95 limits.

| Scenario | Load observations | Navigation p95 ms | Echo p95 ms | Maximum echo ms | Whole-span bytes/s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Native debug hardening | 239 | 25.324 | 101.164 | 101.228 | 3145629 |
| Native debug TUI | 237 | 28.213 | 110.787 | 112.884 | 3145642 |
| Extracted release TUI | 238 | 29.334 | 77.706 | 113.136 | 3145856 |

Native measurements ran without competing builds. Exact-package tests also ran
after both release builds completed. A separate isolated installer check briefly
overlapped the extracted-release sustained test;
this overlap is retained rather than described as completely idle execution.
No build overlapped any sustained scenario and all unchanged assertions passed.

The ignored resource gate passed 60 observations over its original 180-second
sampling duration, 100 reconnects including 20 abrupt, and daemon descriptors
48 to 50 within tolerance 16. Daemon maximum was 379,912,192 bytes; TUI maximum
21,544,960 bytes; fixture-child maximum 20,758,528 bytes. Daemon first/last steady
medians were 160,301,056 and 156,942,336 bytes; TUI medians were 21,184,512 and
21,086,208 bytes. Original 512 MiB ceilings, 32 MiB plateau allowance and minimum
40 samples remain unchanged. Windows helper accounting remains Windows evidence.

## Exact Mac artifacts and installation

Two clean source copies and fresh build directories used Rust/Cargo 1.98.1,
locked offline dependencies, target `aarch64-apple-darwin`, release profile,
empty production features and the existing path-remapping build wrapper.
Both executables and both normalized archives matched byte for byte.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Production rt | 11050240 | `ef20a01284008741b03229f6aa8610403dc60205e3cd534df51cbef09e6b14a4` |
| tar.gz archive | 4387453 | `9fb3b5a7567a2682b9d27cd9fbd73bbc6a2142fbec2394958891a431b338d34a` |

The unsigned archive contains exactly nine regular files with notices/licenses
and verified modes. Mach-O is ARM64, deployment target 11.0, dynamically linked
only to system libiconv and libSystem. The declared macOS 14 floor remains a
separate runtime boundary; this run was on macOS 26.5.2.
Constrained-PATH smoke passed help/version, initialization, detached daemon,
synthetic PTY and shutdown without development tools on the product PATH.

The extracted executable passed the complete `tui_gate`, `session_presentation`,
`worktree_gate` and populated `backup_restore` selections. RELAYTERM_TEST_RT was
set only in these child environments. Executable hashes matched before/after.
The real POSIX installer passed clean installation, byte-preserving collision
and reinstallation rejection, process-local PATH discovery and constrained help.
Removal affected only the new hash-verified disposable copy; a foreign marker,
the colliding synthetic command, extracted package and private state were preserved.

Reproduce with the native commands in the
[Unix handoff](m12-windows-to-unix-handoff.md), explicit resource/fault selections
in the [integrated Mac report](m12-macos-integrated-validation.md), and two clean
build/package commands in [release builds](release-builds.md). Use this exact
commit for builds and the artifact hash above for extracted tests. Documentation-only
descendants reuse this mapped artifact; a different rebuild needs its own identity.

## Retained attempts and remaining gates

One initial evidence-verifier command used the wrong working directory; the
shell rejected its glob before Python/product execution. Repeating from the
repository root passed. Read-only missing-file/early-log lookups remain recorded.
Computer Use rejected Terminal.app access before interaction for safety reasons.
No new GUI screenshot or native physical observation is claimed. Existing Mac,
SSH and Windows 11 VM observations are retained within their stated source scope.
The Windows ConPTY change needs the new Windows real-console observation;
it does not justify repeating the unchanged Mac naming/cursor/writer journey.

Raw command logs, drivers, source/binary identities, all attempts, packages and
synthetic runtime state remain private outside Git. Final repository, secret,
append-only and whitespace checks verify only the resulting public documents.
No push, merge, PR, tag, release, binary upload, corporate probe, security setting
change or provider trial was performed.

Use the current table at the top of the [closure audit](m12-closure-audit.md).
Native Linux regression and new hosted coverage are still missing for this source.
Windows needs actual standard-account and real-console observations of its mapped
package, followed by runtime-floor/independence reconciliation. The private
real-agent trial, final AC/phase reconciliation and publication decision stay separate.
Do not transfer passes between artifacts or restart completed journeys because a
report-only commit changes HEAD.
