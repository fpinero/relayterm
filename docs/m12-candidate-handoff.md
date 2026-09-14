# M12 local candidate handoff

## Status

This file is the prepared local handoff, not a publication record. The implementation branch has not been pushed, merged, tagged, uploaded, or released under the current authorization. Final artifacts must be frozen from a clean commit after all release privacy controls pass. Linux, macOS and Windows final artifact hashes remain pending.

M11 is complete at merge `dc5b2e6067d50345828ac175858029434ce3c081`. Its final review and acceptance matrix remain the source for behavior unaffected by M12. M12 changes session presentation, ordered paging, form cursor geometry, stale-form feedback, input-owner feedback, release tooling, installation and documentation. Those affected rows require candidate-specific evidence.

## Prepared inventory

Version: `0.1.0`. Production features: default empty feature set. Toolchain: Rust 1.98.1. Signing: unsigned. Publication: not authorized.

| Target | Final source | Binary SHA-256 | Archive SHA-256 | Native release gate | Manual installed workflow |
| --- | --- | --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Pending | Pending | Pending | Pending | Pending |
| `aarch64-apple-darwin` | Pending clean rebuild | Pending | Pending | Prior functional gates passed, privacy rebuild pending | Interactive observation pending |
| `x86_64-pc-windows-msvc` | Pending | Pending | Pending | Pending | Pending |

The first macOS arm64 build proved functional packaging, extraction, smoke, installation collision handling and the complete automated TUI gate, but artifact inspection found native build-user paths in dependency diagnostics. That artifact is rejected. The build wrapper now remaps private source, Cargo and user-home prefixes and rejects an executable that still contains them. Final sizes and hashes will be recorded only after a clean rebuild from the committed correction.

The exact archive inventory is documented in [release builds](release-builds.md). It excludes databases, logs, sockets, source, terminal captures, test executables, and debug dumps. `SHA256SUMS` and the target manifest remain beside the archive. The checksum is an integrity mechanism, not independent publisher authentication.

## Evidence still required

1. Build and package twice on native Linux and Windows. Compare hashes, inspect ELF and PE dependencies, then run the extracted smoke. Repeat macOS only if executable or packaged input changes.
2. Run the required Quality and Security jobs without hidden retry or combined failure masking.
3. Complete one consolidated names/order/cursor/stale/input observation per OS and one focused authorized SSH observation.
4. Execute collision-safe installation, complete installed quick start, upgrade, backup, fresh-home restore, and removal procedures on each target. The macOS automated smoke and collision check are complete; its interactive portions remain.
5. Reconcile AC-1 through AC-16 and every phase exit criterion against the frozen source and archives.
6. Replace pending cells with actual evidence, audit the final inventory, and obtain the maintainer's publication choices.

Local source verification through `3b39f03` passed formatting, all-target checks, Clippy with denied warnings, the full workspace suite, core-only tests, workspace build, cargo-deny, audit negative controls, candidate secret scanning, Gitleaks history scanning, candidate-only repository checks, and whitespace validation. The later artifact privacy finding requires a new committed source and candidate before final handoff. The ordinary repository check remains affected by the preserved unrelated untracked reviewer document and was not used to hide or delete it.

The private vulnerability reporting setting returned `enabled: true` through a read-only GitHub API query on 2026-09-14. No report or message was sent.

The current macOS host cannot supply a Terminal.app observation through the available computer-control tool, and no authorized local SSH listener was present. Linux and Windows hosts are unavailable in this local checkout. These facts leave the named rows pending and do not weaken them.

## Publication choices

After the local candidate is technically complete, the maintainer chooses in one decision:

- Keep candidate version `0.1.0` or select another pre-release label before rebuilding.
- Keep the candidate local, deliver it through a pull request, or later create a GitHub release.
- Publish unsigned archives with explicit warnings, or defer public distribution until signing and macOS notarization are arranged.
- Adopt a code of conduct or release policy now, or defer either governance document without changing the technical gate.

The established sole-maintainer approval with assistant-assisted technical review is sufficient. No second GitHub account or claimed independent human review is required.
