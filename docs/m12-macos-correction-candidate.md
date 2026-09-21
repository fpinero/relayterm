# M12 macOS correction candidate

## Source and scope

Prepared from Linux handoff `7f1868620ef537f27927fe22224e60d3a3babb8c`.
Candidate source: `691a8fbb45980658b98d647a85ea8305b2325938`.
Branch: `fix/m12-macos-final-candidate`. Version 0.1.0, empty default
production features, unsigned. No remote delivery or global acceptance is
claimed. Later documentation-only descendants may reuse these behavior checks.

The only additional production change after Linux source 15794ad is typed
in-memory form feedback. Refresh/review/adoption guidance uses Info; errors
retain Error. There is no change to protocol, schema, terminal size, input
leases, conflict checks, automatic replay policy or explicit submission.

## Verification boundary

The 28 TUI unit tests and the native two-client input/rename regression passed.
The latter verifies the Info prefix and unchanged authoritative value/revision
after adoption. Cursor layout is checked at widths 76, 30 and 8. Full local
checks and native artifact preparation passed as recorded below.
Completed and pending physical observations are recorded in the macOS report.

## Minimal remaining operator scope

1. macOS minimum layout is complete: six-screen header/footer at 80 by 24,
   explicit warning at 79 by 24 and recovery. See the operator observation report.
2. macOS writer-size checks are complete: first acquisition, both transfers,
   independent live dimensions, observer resize isolation and fresh input wrapping.
3. macOS connected/disconnected release, non-writer indicators, diagnostic
   stability and native exit are complete. Windows has also completed these
   focused checks on installed 691a8fb; see the Windows manual observations.
   The operator authorized necessary validation process lifecycle operations
   on this Mac; no repeat authorization is needed within that scope.
4. macOS form checks are complete: stale rename rejection, two-step review,
   updated context, retained draft/cursor, Info guidance, validation Error and safe
   discard. Windows has also completed these focused observations on installed
   691a8fb, with independent persisted-state readback.
5. Linux: only changed informational form presentation and cursor/error contrast.
   Do not repeat the completed 15794ad correction round.
6. One authorized SSH connection: changed review and disconnected feedback,
   normal exit and reconnect identity. Reuse unaffected width/continuity evidence.

Earlier names, order, Unicode editing, handover, recovery and installation
observations retain their documented source boundaries. Windows header and
writer-size checks already passed on 0bc4b63/9cf91f7 and are unchanged here.

## Final gates, separate from visual repetition

| Gate | Concrete outstanding proof |
| --- | --- |
| Final native Quality | Final source on Linux, macOS and Windows, stable compiler jobs with each inherited gate repetition separate, plus pinned Linux compiler. Hosted execution requires remote delivery authorization. |
| Security | Final-source dependency/license/source audit, history and candidate scans, negative controls, and no unreviewed critical/high finding. Local results and hosted results must remain distinct. |
| Artifacts | Two clean native builds and normalized packages per target, retained inventory/hash/source mapping, native runtime inspection and exact extracted/installed gates. |
| Clean runtime | Identified runtime or disposable user environment without reliance on source/build tools, audited prerequisites and final-artifact quick start on all targets. Developer-host PATH masking alone is insufficient. |
| AC-1, AC-14, AC-16 | Final installation, executable resolution, collision preservation, clean-runtime initialization and quick start. Reuse unchanged completed interaction observations with explicit mapping. |
| AC-2 through AC-10 | Reuse M11 and later candidate evidence for unchanged behavior; exact installed gates plus affected forms/input/reconnect observations. No new full manual task/handover/backup journey solely for Info labels. |
| AC-11 | Final core-only checks and unchanged dependency direction. |
| AC-12, AC-13 | Final native Quality/Security, source and artifact privacy/notices. |
| AC-15 | Unchanged neutral configuration plus final installed/native regression coverage. |
| Phase 5 and acceptance | Reconcile all 16 criteria, fixed resource budgets and the two documented limits below. Do not mark M12 complete before mandatory rows pass. |

Interrupted local SSH-client termination can prevent remote restoration bytes
from arriving, as reproduced without Relayterm. Preserve the failed automatic
visual-restoration result and document local reset recovery. Fixed-grid history
does not reflow when narrowed; fresh input must wrap at the current writer width.
These limits need explicit acceptance disposition against AC-4/AC-8 and the TUI
contract, not a false passing result or an unrequested terminal reflow feature.

Signing, notarization, tags and public release remain publication decisions.
They are distinct from completing M12 and any subsequent authorized merge.

## Local source verification on macOS

Native macOS 26.5.2 (25F84), arm64, Rust/Cargo 1.98.1, Apple Clang
21.0.0 and SDK 26.5. The following commands passed on candidate 691a8fb:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo build --workspace --locked
cargo deny --locked check advisories licenses bans sources
python3 -m unittest scripts/test_build_release.py scripts/test_package_release.py scripts/test_install_release.py scripts/test_smoke_release.py
python3 scripts/check_audit_controls.py
gitleaks git --redact --no-banner --exit-code 1
git diff --check
```

The workspace suite reported 198 passing executions, zero failures and 16
ignored helpers/opt-in tests. The ordinary suite does not run feature-gated
SQLite interruption/cancellation or opt-in sustained-resource/descendant tests;
the complete native Quality workflow and its two separate repetitions remain
required. Release tooling reported 22 tests, including two Windows-only skips.
Cargo-deny passed advisories, bans, licenses and sources. No security exception
was introduced.

Both scripts/check_repository.py and scripts/check_secrets.py passed in a clean
candidate-only clone. The ordinary working-tree repository check failed on
trailing whitespace in an unrelated, preserved untracked reviewer document;
that file was neither altered nor included in the candidate. The private
vulnerability reporting API returned enabled on 2026-09-21. Read-only inspection
found no PR or workflow runs for fix/m12-linux-findings at this checkpoint.
These are local checks, not final hosted CI results.

## Native artifact and installation results

Two fresh detached clean checkouts built source 691a8fb offline with the checked-in
release wrapper. Both executable copies and both normalized archive copies are
byte-identical within this native environment. Mach-O inspection reports arm64,
minimum deployment field 11.0, SDK 26.5, and only libiconv.2.dylib and
libSystem.B.dylib. The declared macOS 14 minimum remains unchanged; this run
was on macOS 26.5.2.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 11,050,320 | `4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060` |
| Archive | 4,386,181 | `4d8c1dda4d7fa9ea412928365e8dbc9b550f02d0f3766b3462c1cd7832955007` |

Executed build_release.py build twice with --target aarch64-apple-darwin and
--offline, then build_release.py compare; package_release.py create twice,
compare, inspect and extract; shasum -a 256 --check SHA256SUMS; and
smoke_release.py against the archive. Every command passed. The exact nine-file
inventory and external checksums passed. Extracted help/version, initialization,
detached daemon, real shell PTY, clean child exit and orderly shutdown passed.
Both packages, manifests, build records, extracted files and private logs are
retained outside Git.

The packaged POSIX helper installed into a fresh dedicated user-local directory;
its checksum matches the archive executable. Zsh and Bash discovery found no
existing rt resolution. Absolute help/version passed with PATH limited to
/usr/bin:/bin and outside the source checkout. A synthetic collision was refused
and preserved byte-for-byte; a separate spaces/Unicode installation retained
the same checksum. No shell profile or global PATH was changed. These are
assistant-driven developer-host observations, not independent clean-runtime proof.

The exact retained installed executable passed:

```text
RELAYTERM_TEST_RT=<absolute-installed-rt> cargo test -p relayterm-cli --test session_presentation --test tui_gate --test worktree_gate --test backup_restore --locked -- --nocapture --test-threads=1
```

Eleven tests passed, zero failed, one fixture helper ignored. Startup p95 was
74 ms, navigation p95 30 ms, input-to-rendered-echo p95 107 ms and flood output
60,759,728 bytes/s. Native source and installed gates include disconnected
navigation/diagnostic restoration and the independent two-client rename/size
checks. They do not establish physical keyboard or Terminal.app observations.

A new private workspace with one neutral /bin/sh session and two checksum-checked
launchers is ready for the focused operator checks. Launcher shell syntax passed
zsh -n; startup and session identity were read back administratively. Existing
operator workspaces, sessions, backups and SSH configuration were preserved.
No operator-owned daemon was stopped. Physical observation has not yet begun.

## Native physical round completed

The [macOS observations](m12-macos-manual-observations.md) record every focused
native check as passed on 691a8fb. Both clients exited cleanly, and no process
from this candidate remains running. Do not repeat this native visual round
for documentation-only descendants. The cross-platform continuation remains
authoritative for SSH, clean-runtime, final CI and acceptance requirements.
