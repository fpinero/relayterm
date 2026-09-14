# M12 local candidate handoff

## Status

This file is the prepared local handoff, not a publication record. The implementation branch has not been pushed, merged, tagged, uploaded, or released under the current authorization. The final source commit and all three artifact hashes remain to be frozen after local source verification and native target evidence.

M11 is complete at merge `dc5b2e6067d50345828ac175858029434ce3c081`. Its final review and acceptance matrix remain the source for behavior unaffected by M12. M12 changes session presentation, ordered paging, form cursor geometry, stale-form feedback, input-owner feedback, release tooling, installation and documentation. Those affected rows require candidate-specific evidence.

## Prepared inventory

Version: `0.1.0`. Production features: default empty feature set. Toolchain: Rust 1.98.1. Signing: unsigned. Publication: not authorized.

| Target | Final source | Binary SHA-256 | Archive SHA-256 | Native release gate | Manual installed workflow |
| --- | --- | --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Pending | Pending | Pending | Pending | Pending |
| `aarch64-apple-darwin` | Pending | Pending | Pending | Prototype passed, final pending | Pending |
| `x86_64-pc-windows-msvc` | Pending | Pending | Pending | Pending | Pending |

The prototype macOS arm64 build at source `087c505e33c30d74a5a30e1aad6b4d449af0886a` produced two identical 11,051,872-byte binaries with SHA-256 `fef20c23a7b14ba9f4429b91b86112b8c855a56311924f46544cb7a34a864681`. Its extracted smoke passed help, version, initialization, detached daemon startup, a real shell PTY, clean exit, and orderly shutdown with the product `PATH` restricted to `/usr/bin:/bin`. It linked only `/usr/lib/libiconv.2.dylib` and `/usr/lib/libSystem.B.dylib`, with Mach-O arm64 deployment target 11.0. This prototype predates the final nine-file archive and is not the release candidate.

The exact archive inventory is documented in [release builds](release-builds.md). It excludes databases, logs, sockets, source, terminal captures, test executables, and debug dumps. `SHA256SUMS` and the target manifest remain beside the archive. The checksum is an integrity mechanism, not independent publisher authentication.

## Evidence still required

1. Freeze one final implementation source after all local code and documentation changes.
2. Run the complete local M12.06a checks on that source.
3. Build and package twice on native Linux, macOS, and Windows. Compare hashes, inspect ELF, Mach-O, and PE dependencies, then run the extracted smoke.
4. Run the required Quality and Security jobs without hidden retry or combined failure masking.
5. Complete one consolidated names/order/cursor/stale/input observation per OS and one focused authorized SSH observation.
6. Execute collision-safe installation, complete installed quick start, upgrade, backup, fresh-home restore, and removal procedures on each target.
7. Reconcile AC-1 through AC-16 and every phase exit criterion against the frozen source and archives.
8. Replace pending cells with actual evidence, audit the final inventory, and obtain the maintainer's publication choices.

The current macOS host cannot supply a Terminal.app observation through the available computer-control tool, and no authorized local SSH listener was present. Linux and Windows hosts are unavailable in this local checkout. These facts leave the named rows pending and do not weaken them.

## Publication choices

After the local candidate is technically complete, the maintainer chooses in one decision:

- Keep candidate version `0.1.0` or select another pre-release label before rebuilding.
- Keep the candidate local, deliver it through a pull request, or later create a GitHub release.
- Publish unsigned archives with explicit warnings, or defer public distribution until signing and macOS notarization are arranged.
- Adopt a code of conduct or release policy now, or defer either governance document without changing the technical gate.

The established sole-maintainer approval with assistant-assisted technical review is sufficient. No second GitHub account or claimed independent human review is required.
