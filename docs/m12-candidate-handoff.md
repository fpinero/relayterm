# M12 local candidate handoff

## Status

This is a technical candidate handoff, not a release publication record. Source `491d5f1037450c362525b289fa6361d834fc0b6f` passed the complete native Quality and Security workflows. Before the correction described below, its documentation-only descendants affected the quick start, evidence documents, TODO, and the append-only logbook. The subsequent production correction at `d760554` supersedes that original executable for continued acceptance testing; the original hashes and workflow results below remain historical evidence.

The implementation is available in draft PR [#24](https://github.com/fpinero/relayterm/pull/24). It has not been merged, tagged, uploaded as a package, or published as a release. The CI archives were tested before their ephemeral workspaces ended and were not uploaded. The local macOS archive described below remains outside version control.

M11 is complete at merge `dc5b2e6067d50345828ac175858029434ce3c081`. Its [final review](m11-final-review.md) and [acceptance matrix](acceptance-matrix.md) remain the source for behavior unaffected by M12. M12 changes session presentation, ordered paging, form cursor geometry, stale-form feedback, input-owner feedback, release tooling, installation, and release documentation. Those affected manual rows require candidate-specific evidence.

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
recorded below; its manual journey, the Linux corrected build and journey, and
focused SSH observation remain required. Use
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
retained outside Git. This is assistant-driven preparation on a developer host;
physical terminal acceptance remains pending. See
[the Windows record](m12-windows-manual-observations.md) for exact boundaries.

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

1. Complete the Linux and Windows consolidated native observations against corrected retained candidates, and reconcile the macOS source-impact mapping recorded above.
2. Complete the focused SSH observation through an already authorized SSH service.
3. Retain corrected Windows and Linux artifacts and execute checksum verification, collision-safe user-local installation, the complete interactive quick start, upgrade/backup/fresh-home restore, and scoped removal on those targets. The local macOS evidence is recorded above. CI proved extracted execution, but its runners are not the required operator-controlled clean environments and their archives were not uploaded.
4. Reconcile AC-1 through AC-16 and the final artifact inventory after those rows pass.
5. Obtain the maintainer's publication choices. A tag, package upload, and public release remain outside this handoff.

The earlier macOS probe found no authorized local SSH listener and computer
control refused Terminal.app automation. A native Windows 10 host is now
available for the continuation recorded above. Linux and focused SSH evidence,
Windows operator observations, and final reconciliation remain pending.

## Publication choices

After the remaining evidence passes, the maintainer chooses in one decision:

- Keep candidate version `0.1.0` or select a different version and rebuild.
- Keep the candidate local, merge the implementation PR, or later create a GitHub release.
- Publish unsigned archives with explicit warnings, or defer distribution until signing and macOS notarization are arranged.
- Adopt a code of conduct or release policy now, or defer either governance document without changing the technical gate.

The established sole-maintainer approval with assistant-assisted technical review is sufficient. No second GitHub account or claimed independent human review is required.
