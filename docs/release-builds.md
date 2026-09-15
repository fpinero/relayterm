# Release build contract

## Candidate scope

Relayterm's first local release candidate keeps package version `0.1.0`. A version does not identify an artifact by itself. Every manifest and handoff must also identify the exact source commit and target triple. No tag, signed artifact, notarization, package upload, or published release exists merely because this recipe succeeds.

The production deliverable is one executable, `rt` on Unix and `rt.exe` on Windows. The `relayterm-cli` package has no feature flags. Production builds use its default feature set and must not enable dependency test hooks or build integration-test executables into the archive.

## Supported artifact targets

Support claims are limited to these three native artifacts and tested runtime baselines:

| Target | Architecture | Tested runtime baseline | Current evidence boundary |
| --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | x86-64 | Ubuntu 24.04 | M11 native CI and manual GNOME Terminal/OpenSSH behavior passed. M12 build, linkage, installed workflow, and affected usability evidence are pending. Other distributions and musl are not claimed. |
| `aarch64-apple-darwin` | Apple arm64 | macOS 14 | M11 native CI passed on macOS 14. M11 manual behavior passed on a newer native macOS arm64 host. M12 local automated behavior passes on macOS 26.5.2; final artifact and manual evidence are pending. Intel and universal binaries are not claimed. |
| `x86_64-pc-windows-msvc` | x86-64 | Windows 10 22H2 for desktop behavior, Windows Server 2022 for CI | M11 native CI and manual Windows Terminal, PowerShell, cmd.exe and OpenSSH behavior passed. M12 build, PE inventory, installed workflow, and affected usability evidence are pending. Windows arm64 is not claimed. |

The tested baseline is also the minimum supported version for the first candidate because no older runtime has been tested. Future evidence may lower that minimum. A compiler target existing does not establish runtime support.

## Fixed build inputs

Use all of these inputs for each build:

- Exact clean source commit.
- Checked-in `Cargo.lock`.
- Rust 1.98.1 with the minimal profile from `rust-toolchain.toml`.
- One target triple from the table above.
- Cargo release profile as committed, with no environment override that changes code generation.
- Default production features only.
- A fresh target directory outside any prior build output.

The native build command is:

```text
cargo build --locked --release -p relayterm-cli --bin rt --target <target>
```

The checked-in wrapper enforces these inputs and writes a bounded public-safe build record:

```text
python3 scripts/build_release.py build --target <native-target> --output <empty-output-directory> --offline
python3 scripts/build_release.py compare <first>/build-record.json <second>/build-record.json
```

The wrapper rejects non-native targets, dirty tracked source, nonempty output directories, missing regular executables, changed build inputs and private build paths in the executable. It records the source commit and timestamp, exact toolchain, target, feature set, public path-remapping flags, command, binary size and SHA-256. The isolated build remaps the source checkout, Cargo home and build-user home so public binaries do not disclose native usernames or private build paths. It never follows or replaces a destination link and never publishes an artifact.

Packaging uses the corresponding checked-in tool after both native builds pass:

```text
python3 scripts/package_release.py create --build <first> --output <first-package>
python3 scripts/package_release.py create --build <second> --output <second-package>
python3 scripts/package_release.py compare <first-manifest> <second-manifest>
python3 scripts/package_release.py inspect <archive>
python3 scripts/smoke_release.py <archive>
```

The package tool reads the locked, target-specific normal Cargo graph. It includes each dependency's SPDX expression and packaged license files, calls out bundled SQLite, and conservatively retains proc-macro packages even though they are build-time tooling. It excludes test-only and Cargo build-dependency edges. Missing license metadata or text fails packaging. Notice material is bounded at 8 MiB.

The crates.io archive for `windows-permissions 0.2.4` declares MIT in its package metadata but omits a license file. Relayterm carries a version-pinned supplemental copy at `third_party/licenses/windows-permissions-0.2.4.txt`, derived from that declaration and the upstream authorship metadata. The package inventory labels it `relayterm-supplement` instead of presenting it as an upstream file. This exception applies only to version 0.2.4; another version without packaged text fails closed.

After dependencies are fetched explicitly, repeat with Cargo offline. Record `rustc -vV`, `cargo -V`, the operating-system version, source commit, target, command, relevant code-generation environment, executable size, and SHA-256. Keep hostnames, usernames and native build paths out of public manifests.

Build twice from separate clean checkouts and separate target directories with identical declared inputs. Compare binary and normalized archive SHA-256 values. Windows builds request deterministic linker output and record only the PDB filename in the PE debug entry, so isolated target-directory names do not become artifact inputs. Equal hashes prove byte identity only for those two observed builds. Different hashes require inspection and an exact explanation; a usable repeatable recipe alone is not a byte-reproducibility claim.

An initial native macOS repetition produced equal binaries and archives, but a subsequent privacy inspection found native build-user paths in dependency diagnostics. Those artifacts were rejected. Source `b6559156ce0d20b89590edb35e52eff5ebd7cc92` added path remapping and rejection, then produced two 11,051,872-byte binaries with identical SHA-256 `6d16cae5733c458f6749e9a5dbaa9f17f64a41e773d9b5ee727107a332dd84da`. Its two normalized nine-entry archives were also byte-identical, with size 4,389,579 bytes and SHA-256 `35329a29a5cc4ca12fa6b3699c7ff4c1e5d551e03d1b567b018e0e316c647c49`. Inspection found none of the original source, Cargo-home or build-user prefixes. This proves byte identity and the stated privacy inspection for those two macOS observations only. Linux and Windows repetitions remain pending until native CI can run an authorized published branch.

## Runtime inspection boundary

Inspect the built file with native tools before packaging:

- Linux: identify the ELF target, interpreter, required shared libraries and observed glibc symbol floor.
- macOS: inspect Mach-O architecture, minimum deployment target, load commands and dynamic libraries.
- Windows: inspect PE architecture and imported DLLs, including any Visual C++ runtime dependency.

Bundled SQLite removes a separate SQLite installation requirement but does not prove that the executable has no dynamic operating-system dependencies. Git remains required only for explicit worktree features. User-selected shells and agent commands remain their own runtime prerequisites. The installed `rt` must run help, version, workspace initialization, detached daemon startup, one synthetic PTY and orderly shutdown without Cargo, rustup, Python, a source checkout, or a hosted service on PATH.

The release CI extracts the inspected archive and routes the session-presentation, full TUI, two-worktree, and private backup/restore gates through that exact executable. Each gate is a separate step, so one failure cannot be hidden by a later command. The harness binaries and fixtures remain Cargo test outputs outside the archive; only the production `rt` comes from the candidate package.

On pull requests, general quality jobs validate GitHub's proposed merge commit while the release jobs explicitly check out the pull request head. This keeps integration coverage and makes each build record identify the stable candidate source SHA. Push-triggered release jobs use the pushed commit. Native release jobs print and validate ELF, Mach-O, or PE architecture and runtime dependency metadata before packaging. Every installed command in the extracted smoke has a 30-second bound, and the complete smoke step has a three-minute bound.

The observed prototype macOS executable is Mach-O arm64 with deployment target 11.0. It links `/usr/lib/libiconv.2.dylib` and `/usr/lib/libSystem.B.dylib`; SQLite is bundled. The declared macOS 14 floor remains the oldest tested runtime, regardless of the lower linker deployment field. The extracted smoke passed on macOS 26.5.2 with only `/usr/bin:/bin` on `PATH` for the product process. A short private test root under `/tmp` was required because Unix-domain socket paths are length-bounded. Production default private locations are compact; an excessively long explicit `--home` can fail with `daemon_unavailable` and should be replaced with a shorter private path.

## Portable archive inventory

Each archive has one target-specific root and exactly these nine regular files:

- `rt` or `rt.exe`.
- `LICENSE`.
- `THIRD_PARTY_NOTICES.txt` and `THIRD_PARTY_LICENSES.txt`.
- `INSTALL.md` and `RECOVERY.md`.
- `install_release.sh` and `install_release.ps1`.
- `manifest.json`.

The executable and POSIX installer have mode 0755; other entries have mode 0644. Tar and ZIP entry order, timestamps, ownership fields, modes, compression settings, and root name are normalized. The inspector rejects links, directories, absolute paths, traversal, duplicate or extra inventory, and an unexpected binary name. The package output also contains an external target manifest and `SHA256SUMS`; those are verified before extraction and are intentionally outside the self-referential archive hash.

## Artifact freeze rule

Do not freeze or certify release artifacts until M12.00h has complete affected usability evidence. Any source change after an artifact build requires an impact assessment. A behavior change requires rebuild and affected retest. A documentation-only descendant may reuse a binary with an explicit source mapping, but an untested rebuild may not replace the reviewed artifact.
