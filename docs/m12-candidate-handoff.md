# M12 local candidate handoff

Current decision: see the [M12 closure audit](m12-closure-audit.md). It consolidates
all three platform deliveries and identifies the exact remaining closure gates.
M13 has no definition in the inspected roadmap.

Latest runtime update: the [isolated macOS installed journey](m12-macos-final-runtime.md)
passed on the retained candidate. The remaining native runtime row is Windows;
SSH presentation and Windows debug latency remain separate open requirements.

## Status

This is a technical candidate handoff, not a release publication record.
Current product source is `691a8fbb45980658b98d647a85ea8305b2325938`.
The final native Linux artifacts, installed gates, independent Ubuntu runtime
and focused physical feedback delta passed on 2026-09-23. The macOS and Windows
focused physical rounds and final hosted Quality/Security coverage are complete
as mapped below. Follow [the current continuation](m12-cross-platform-continuation.md)
and the finite remaining requirements in [the Linux report](m12-linux-final-report.md).
Older hashes and workflow results remain historical evidence. SSH presentation,
macOS/Windows clean runtimes, Windows debug latency and global acceptance remain
open; no artifact freeze or publication is implied.

The implementation is available in draft PR [#24](https://github.com/fpinero/relayterm/pull/24). It has not been merged, tagged, uploaded as a package, or published as a release. The CI archives were tested before their ephemeral workspaces ended and were not uploaded. The local macOS archive described below remains outside version control.

M11 is complete at merge `dc5b2e6067d50345828ac175858029434ce3c081`. Its [final review](m11-final-review.md) and [acceptance matrix](acceptance-matrix.md) remain the source for behavior unaffected by M12. M12 changes session presentation, ordered paging, form cursor geometry, stale-form feedback, input-owner feedback, release tooling, installation, and release documentation. Those affected manual rows require candidate-specific evidence.

## Focused macOS native round completed

All focused physical macOS checks pass on 691a8fb, including the informational
feedback correction, measured minimum layout, both writer-size transfers,
release/navigation, disconnect diagnostics and native exit. Both clients exited,
and the dedicated test daemon is stopped. See the [candidate report](m12-macos-correction-candidate.md)
and [operator observations](m12-macos-manual-observations.md). No further native
visual repetition is required unless behavior changes. Current outstanding SSH, macOS/Windows clean-runtime, debug-latency and
global acceptance requirements are listed in the final Linux report.

## Current macOS correction preparation

The Linux handoff is now followed by candidate source
`691a8fbb45980658b98d647a85ea8305b2325938`, correcting informational form
feedback without changing submission, protocol, persistence or lease behavior.
See [the macOS correction candidate](m12-macos-correction-candidate.md) for
current source verification, artifact preparation and the minimal pending matrix.
Earlier artifacts remain attributed to their original source and are not replaced
by this source identity. Global M12 acceptance remains open.

## Corrected candidate after macOS observation

The original installed manual journey exposed a committed handover that left
workspace snapshots invalid and the TUI stale. Source
`d7605541bfa829cfab6d8a3c53b87ad6e0f7b4fa` restricts projected handovers to the
projected task IDs. It preserves full history. The failing persistence regression
passed after the correction, and two-client protocol coverage now refreshes
before continuation. Full workspace tests, Clippy, formatting, and source checks
passed locally. The historical hosted workflows below do not certify this fix.

The corrected unsigned macOS arm64 build was produced twice on macOS 26.5.2 with
Rust 1.98.1 and SDK 26.5. Both executable and normalized archive copies matched.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 11,051,968 | `4de8d6f45c8cc375c3b71c3ad05515cbdbc91f282120d0fb351bdaeae98bc7c7` |
| Archive | 4,388,092 | `129c925e7d03c2acd2a333ad5832927672dc809cfaf704aa6a4f71fff40c27bf` |

The extracted smoke and exact-installed session presentation, TUI, and backup
restore gates passed. The operator's affected handover, second-session claim,
completion, and client-reopen retest passed after fresh-home backup restoration.
The macOS record also covers original-candidate names, order, Unicode cursor,
stale rename review, competing input, detach, reattach, installation, discovery,
and corrected-candidate scoped removal. See [the detailed observations](m12-macos-manual-observations.md)
for source mapping and the distinction between operator and assistant checks.
Archives and metadata are retained outside Git. Corrected Windows preparation is
recorded below. The Windows manual journey on d760554 is recorded separately
from the completed header and writer-size retests; the Linux current-candidate build and journey,
and focused SSH observation remain required. Use
[the continuation instructions](m12-native-continuation.md).

### Corrected Windows candidate

On 2026-09-20, two clean native Windows 10 Pro 22H2 x64 builds from `d760554`
produced identical executables and normalized ZIP archives. Rust and Cargo
1.98.1, MSVC compiler 19.29.30158, linker 14.29.30158.0, and Windows SDK
10.0.19041.0 were used. Version remains 0.1.0, with empty default production
features and no signature.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 11,880,448 | `df4f0b7285432a57855aee862aad71879af416bfb4e301aac532fca05298b9f8` |
| ZIP archive | 4,511,930 | `b4030f4a7dc6c97ad1d69a33265635d437bfa5aa6b92dd5bc2f61271640ec652` |

PE inspection, nine-entry inventory, archive/manifest checksums, extracted smoke
with real ConPTY, dedicated installation, absolute PowerShell and cmd.exe
invocation, collision preservation, and a spaces/Unicode install passed.
The exact installed executable also passed session presentation, backup/restore,
and all six TUI scenarios (one fixture helper ignored). The system has the
required Visual C++ x64 runtime. Artifacts and metadata are
retained outside Git. The operator subsequently completed the handover,
session, editing, client reopen, and restored-state journeys on this developer
host. Width testing exposed header clipping and command clipping on writer
acquisition. The later candidates and affected Windows retests below address
both defects. See
[the Windows record](m12-windows-manual-observations.md) for exact boundaries.

## Minimum-width candidate

Source `0bc4b6384adf37dbdd9939e840d9c3e14e86d9e1` moves Relayterm to the
footer and reserves header space for the full freshness label. On the same
Windows 10 host and toolchain documented above, two fresh clean detached clones
built offline and produced identical executables and normalized ZIP archives.
Version remains 0.1.0 with default empty features and no signature.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 11,882,496 | `8a6e231f253c83321aaabbd97318a613e13c68d56eb3859c1f204aa0436b1fd4` |
| ZIP archive | 4,511,842 | `e3008b927f077b768c261a2fe0568e8a0a73db96d8492293640d03d5b86a9128` |

Build and package comparisons, nine-member inspection, external manifest and
archive checksums, native x64 PE inspection, and extracted ConPTY smoke passed.
PE imports retain VCRUNTIME140.dll and UCRT dependencies. Installation into a
new dedicated directory preserved the earlier candidate and matched the new
hash. All 24 TUI tests and Clippy passed before the clean builds.
The exact installed executable passed session presentation, all six TUI
scenarios, private backup/restore, and both worktree gates: ten tests passed,
with one fixture helper ignored. Startup p95 was 206 ms, navigation p95 21 ms,
and input echo p95 126 ms. The detailed Windows record lists commands and
observation boundaries.

This candidate remains retained as header evidence, not frozen or released.
The Windows operator subsequently confirmed its header/footer at 80 by 24 in
Overview, Tasks, and Sessions.
Linux must build the current source identified above and complete its native
journey; macOS needs affected header and writer-size retests. M12.WIN-SIZE was
subsequently corrected in source at 9cf91f7 and passed Windows physical acceptance.
Follow the existing
[Linux task prompt](m12-native-continuation.md#linux-task-prompt).

## Writer-size candidate

Source `9cf91f7164235d35beda81d9c8c2d4200b703aad` invokes the existing
lease-owned resize after successful input acquisition. Two clean detached
offline builds on the same Windows 10, Rust/Cargo 1.98.1, MSVC, and SDK environment
produced byte-identical executables and normalized archives. Build parallelism
was limited to four Cargo jobs per copy; production flags and features were
unchanged. Version remains 0.1.0, unsigned.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 11,803,648 | `65da79d61ba031cfef02c3971ae65a35d706e88c743d0f68fc8139f334325045` |
| ZIP archive | 4,497,240 | `0409cc32358acd4cd2526309ddcc5a3b8246d8abce13263df704c2e2120c3d4c` |

Build/package comparisons, checksum verification before extraction, nine-member
inventory, PE inspection, extracted ConPTY smoke, and separate installed
help/version/hash checks passed. The regression failed against 0bc4b63 and
passed after the correction; full source TUI integration, 24 TUI unit tests,
and TUI/CLI Clippy with denied warnings passed. Windows physical first
acquisition, transfers in both directions, and read-only resize isolation passed.
Independent snapshot reads retained 17 by 78 for the narrow writer after the
observer was resized. Other-platform affected retests remain required. Prior
header observations are unaffected by this source change.
The exact installed candidate passed all ten session presentation, TUI,
backup/restore, and worktree tests, including the new dimension regression;
one fixture helper was ignored. Startup p95 was 186 ms, navigation p95 21 ms,
and input echo p95 146 ms.

## Original native artifact evidence

Version: `0.1.0`. Production features: default empty feature set. Toolchain: Rust 1.98.1. Signing: unsigned. Candidate artifacts use source `491d5f1037450c362525b289fa6361d834fc0b6f`.

| Target | Binary bytes and SHA-256 | Archive bytes and SHA-256 | Native release gate | Manual installed workflow |
| --- | --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | 13,028,760, `264d8b32a92e222cedb3d99c5418c76d6ace6f3cf6e2b8d2510a6c69a3151e4d` | 4,841,684, `34fe66c00d8e88ef426b94204d3d6dd189f9e530b7801d5ca702d6bc5f8f629e` | [Passed](https://github.com/fpinero/relayterm/actions/runs/34947043434/job/104308988311) | Pending |
| `aarch64-apple-darwin` | 11,051,968, `1acef036ef5cbbdc34ac9857a51aff1987554e7beff7f81657e8f32844de193c` | 4,374,946, `bdc16f0d0df761ed41da8a6bb4f9d0392adb3eebccb486a8e0721dc0a0a1b684` | [Passed](https://github.com/fpinero/relayterm/actions/runs/34947043434/job/104308988219) | Original manual journey exposed handover defect; corrected retest above |
| `x86_64-pc-windows-msvc` | 11,825,664, `0b44c59a89524ff75caa9fa1d89a28a2dc4ee241e71d4bc872f9488497aa0766` | 4,485,283, `73509264e38a89a1d27b17c9e38493cd87a3101a2455a8788c0321a2da263907` | [Passed](https://github.com/fpinero/relayterm/actions/runs/34947043434/job/104308988407) | Pending |

Every native runner built the executable twice from clean isolated target directories with the locked offline graph. Both executable hashes matched on each target. Each runner then created the normalized nine-entry archive twice, and both archive hashes matched. The jobs inspected the exact inventory and ran help, version, workspace initialization, detached daemon startup, a real PTY or ConPTY shell, clean child exit, and orderly shutdown from the extracted archive with the build toolchain absent from `PATH`. They also ran the complete TUI, session presentation, real worktree, and backup/restore gates through that extracted executable.

The Linux artifact is an x86-64 PIE with interpreter `/lib64/ld-linux-x86-64.so.2` and direct dependencies on `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and the loader. The symbol inventory reaches GLIBC 2.39 for weak process symbols. Ubuntu 24.04 is the tested and initially declared baseline.

The macOS artifact is arm64, has deployment target 11.0, and links only `/usr/lib/libiconv.2.dylib` and `/usr/lib/libSystem.B.dylib`. macOS 14 is the native CI baseline. A separate macOS 26.5.2 build from the same source produced an 11,051,872-byte executable with SHA-256 `6d16cae5733c458f6749e9a5dbaa9f17f64a41e773d9b5ee727107a332dd84da` and a 4,390,446-byte archive with SHA-256 `6e9e524d044383a4edc0483434a955f9d834c4c6a92ec5854e6d38af495b1591`. The different native SDK and linker environment produce a different artifact, so no cross-host byte identity is claimed.

The Windows artifact is an x64 PE. Its import table includes Windows system libraries, Universal CRT API sets, and `VCRUNTIME140.dll`. The supported Microsoft Visual C++ v14 Redistributable for x64 is therefore an explicit runtime prerequisite. The fixed session presentation gate completed in 0.69 seconds after an earlier five-minute failure exposed inherited pipe-handle waiting.

## Validation

[Quality run 34947043434](https://github.com/fpinero/relayterm/actions/runs/34947043434) passed all seven jobs on `491d5f1`: stable Linux, macOS, and Windows, pinned Rust 1.98.1 on Linux, and the three release targets. Every inherited M11 repetition remained independently reported. [Security run 34947043380](https://github.com/fpinero/relayterm/actions/runs/34947043380) passed dependency, license, source, audit, and secret controls.

The hosted TUI repetitions met their 500 ms navigation and one-second input guardrails. Linux reported navigation p95 21 ms and input p95 164 ms. Windows reported navigation p95 22 ms and input p95 210 and 208 ms. macOS reported navigation p95 168 ms and input p95 267 and 269 ms, so its stricter 100 ms and 250 ms reference indicators remained visibly false while the documented shared-runner guardrails passed. A local installed macOS run reported startup p95 77 ms, navigation p95 30 ms, input p95 122 ms, and 64,108,771 output bytes/s.

Local verification on `491d5f1` passed formatting, all-target checks, Clippy with denied warnings, the full workspace suite, core-only tests, workspace build, cargo-deny, 22 release-tooling tests with two Windows-only skips, audit negative controls, candidate secret scanning, Gitleaks history scanning, candidate-only repository checks, and whitespace validation. The locally installed macOS executable also passed session presentation, the complete TUI gate, two real Git worktree journeys using Git 2.53.0, and private backup/restore. Installation into a path with spaces and Unicode preserved the exact executable hash, and a second installation refused to overwrite it.

The ordinary repository check remains affected only by the preserved unrelated untracked reviewer document. Candidate-only validation passed without deleting, hiding, staging, or publishing that file. The private vulnerability reporting setting returned `enabled: true` through a read-only GitHub API query on 2026-09-14. No report or message was sent.

## Evidence still required

Use [the current platform checklist and destination prompt](m12-cross-platform-continuation.md)
as the authoritative pending scope. Linux original observations and targeted
15794ad physical correction retests are complete, with limits and failures
preserved in [the Linux report](m12-linux-final-report.md).

Remaining gates are the informational form-feedback issue, final-source native
Quality/Security and release packages, affected macOS/Windows and SSH checks,
independent clean-runtime proof, and final AC/artifact reconciliation. The
macOS header and writer-size observations remain required; Windows passed those
at their documented source boundaries. Historical CI and 9cf91f7 packages do
not certify the later source. Publication remains a separate maintainer decision.

## Publication choices

After the remaining evidence passes, the maintainer chooses in one decision:

- Keep candidate version `0.1.0` or select a different version and rebuild.
- Keep the candidate local, merge the implementation PR, or later create a GitHub release.
- Publish unsigned archives with explicit warnings, or defer distribution until signing and macOS notarization are arranged.
- Adopt a code of conduct or release policy now, or defer either governance document without changing the technical gate.

The established sole-maintainer approval with assistant-assisted technical review is sufficient. No second GitHub account or claimed independent human review is required.

## Linux continuation artifact

On 2026-09-21 exact source `9cf91f7164235d35beda81d9c8c2d4200b703aad`
was built twice from clean detached clones on Ubuntu 24.04.5 x86-64 with
Rust/Cargo 1.98.1, GCC 13.3.0, GNU ld 2.42, and glibc 2.39. Both executables
and both normalized archives are byte-identical within that environment.
Version remains 0.1.0, default empty features, unsigned.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 13,030,408 | `b2bbc872bf4b7c8175f07dfd66b64d2cc8a02bf66ae6c409ee33542c0a9fcfd4` |
| Archive | 4,826,579 | `0f7add246e4406eb7a30a955844a9a04d51026014ca69301232d3d536cd3450b` |

Native ELF inspection, external checksums, nine-member inventory, extracted PTY
smoke, dedicated installed identity/help/version, synthetic collision refusal,
and scoped removal passed. The serial exact-installed gates pass ten tests
with one helper ignored using the test-harness correction documented in
[the Linux record](m12-linux-manual-observations.md). Original failed runs remain
recorded. The test-only correction does not replace the production artifact.
Packages, checksums, manifests, build records, and inspection are retained outside
Git. The native journey and targeted correction retests are now recorded in
[the final Linux report](m12-linux-final-report.md); its limits and original
SSH failure remain explicit. Final-source artifacts, independent clean-machine
proof, and global reconciliation remain open. This developer-host run does not certify an independent clean machine.

## Windows final-source artifact preparation

On 2026-09-21 Windows 10 Pro 22H2 x64 built exact production source 691a8fb
twice with Rust/Cargo 1.98.1 and MSVC 14.29. Both binaries and normalized ZIPs
are byte-identical. The [Windows report](m12-windows-manual-observations.md)
contains complete hashes, runtime inventory, installation and test commands.
The retained executable SHA-256 is
`2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708`;
the ZIP SHA-256 is
`875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94`.
Twelve exact-installed tests passed with a local test-only correction for
physical-row footer indexing and native exit assertions. No product rebuild
was substituted. The focused physical correction round passed on that same
installation, including both native shell exits. Clean-runtime acceptance and
local debug latency remain pending. Security passed; hosted Quality failed its two Windows jobs on
the original harness. Its failures require a later authorized verification run.

That authorized verification is now complete: Quality 35637866692 passed all
seven jobs and Security 35637869615 passed on e4431f7. Both Windows jobs and
independent repetitions passed. See the [Windows final report](m12-windows-final-report.md)
for artifact identities and the [Linux delta continuation](m12-linux-final-delta-continuation.md)
for the remaining bounded work. Do not repeat completed physical journeys for
documentation-only descendants or treat hosted success as clean-runtime proof.

## Linux final candidate checkpoint

On 2026-09-23, Linux completed the bounded 691a8fb continuation documented in
the [final report](m12-linux-final-report.md). Two clean native builds and
normalized packages matched. The installed executable hash is
`26f7f90e27da12239c3130e1163951a4cfe2f08e448fc8b28f6a46cec642d8d7`;
the archive hash is
`a9e5bb84b474ab8f37e9778046ec530b3833df67261b294549d1352df63a3285`.
Delivery harness 25c1092 passed 12 exact-installed tests with one ignored helper.
The operator confirmed Info/Error and cursor retention; independent readbacks
proved no automatic write, safe discard and no task creation on validation.

An isolated, unprivileged Ubuntu 24.04 runtime without network, source mounts or
build tools passed installation/discovery/collision, daemon, three PTYs, automated
TUI entry/exit, task coordination and backup/restore. Unchanged physical journeys
remain mapped to their original candidates. Existing successful Quality/Security
runs on e4431f7 were reconfirmed without dispatching CI.

These results advance the Linux portions of AC-1, AC-4, AC-12, AC-14 and AC-16;
they do not grant global M12 acceptance. The report lists the finite remaining
SSH, macOS/Windows runtime, Windows debug latency and acceptance requirements.
No additional native visual round is required for this documentation-only change.
