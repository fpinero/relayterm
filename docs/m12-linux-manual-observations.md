# M12 Linux native observations

## Scope and candidate

Status: In progress. No physical M12 checkpoint has passed yet.

The evidence branch was created from the fetched Windows continuation branch
with an initially clean working tree. Production source is exactly
`9cf91f7164235d35beda81d9c8c2d4200b703aad`. The evidence descendant changes
only documentation. Two separate clean detached source clones and fresh build
outputs are used outside the repository. Both production clones remained clean after building and packaging.

## Environment

Preparation date: 2026-09-21. Ubuntu 24.04.5 LTS, x86_64; this is within the
declared Ubuntu 24.04 baseline. GCC 13.3.0, glibc 2.39 (Ubuntu package
2.39-0ubuntu8.9), Bash 5.2.21, Dash 0.5.12-6ubuntu5, GNOME Terminal 3.52.0,
and VTE 0.76.0. The operator selected GNOME Terminal and a Spanish keyboard.
Rust/Cargo 1.98.1 was prepared in an isolated temporary toolchain location;
no shell profile was changed. Public locked dependencies were fetched before
starting the offline release builds.

## Built and installed artifacts

Both native release builds passed from separate clean clones and fresh output
directories using `scripts/build_release.py build --target
x86_64-unknown-linux-gnu --output <fresh-output> --offline`. Build-record
comparison and package-manifest comparison passed with byte-identical copies.
Rust 1.98.1 uses LLVM 22.1.8; Cargo is 1.98.1. Native linker: GNU ld 2.42.
Version 0.1.0, empty default production features, unsigned.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 13,030,408 | `b2bbc872bf4b7c8175f07dfd66b64d2cc8a02bf66ae6c409ee33542c0a9fcfd4` |
| Archive | 4,826,579 | `0f7add246e4406eb7a30a955844a9a04d51026014ca69301232d3d536cd3450b` |

External archive and manifest checksums passed before the extraction used for
installation. Inventory inspection confirmed exactly nine expected regular
members. `file`, `readelf`, and `ldd` confirmed x86-64 PIE, interpreter
`/lib64/ld-linux-x86-64.so.2`, and direct dependencies on libgcc_s, libm, libc,
and the loader. All resolved locally; the maximum observed glibc symbol version
is GLIBC_2.39. This does not establish support below Ubuntu 24.04.

The extracted smoke passed help, version, initialization, detached daemon,
real synthetic PTY, and orderly shutdown with constrained runtime PATH.
The packaged helper installed a fresh dedicated user-local copy. Its checksum,
help, and version passed. Both archives, manifests, checksums, build records,
and native inspection are retained in a dedicated user artifact directory
outside Git. No global PATH or shell profile was changed.

Assistant-driven installation checks verified an existing synthetic unrelated
`rt` was refused and preserved byte-for-byte, and independently resolved that
sentinel in an isolated Bash PATH. A separate space/Unicode install retained the
candidate checksum. That owned copy ran only version, never a daemon, and its
executable and empty directory were removed explicitly; the sentinel remained.
These are automated observations, not operator keyboard confirmations.

## Installed gate investigation

With RELAYTERM_TEST_RT pointing to the exact installed executable, the command
`cargo test -p relayterm-cli --test session_presentation --test tui_gate
--test backup_restore --test worktree_gate --locked --offline -- --nocapture
--test-threads=1` passed backup/restore and session presentation, but failed the
TUI input-ownership scenario. Five other TUI tests passed and one fixture helper
was ignored. The failed assertion did not observe `Another client controls input`
within 45 seconds after the observer acquired unsuccessfully and switched to
Events at 80 by 24. This remains a failure under investigation, not a passing
writer-size gate. Cargo did not reach worktree_gate in that invocation. Its previously compiled
harness was then invoked separately with the same RELAYTERM_TEST_RT; both
worktree tests passed, including two isolated worktrees, three sessions, two
claims, restart recovery, and the non-Git boundary with Git 2.43.0.

The diagnostic reproduction also failed waiting for READ ONLY or WRITER after
an immediate resize/input sequence. The original test sent resize, attachment,
acquisition, and screen navigation without waiting for the new frame. A passing
diagnostic variant alone was not accepted as a fix. The retained test correction
sizes its parser before notifying the child, waits for the footer at its new
row, waits for discard confirmation to close, and observes read-only attachment
before acquisition. It retains both inline and Events notice checks and all
live snapshot dimension assertions. Production code and installed binary remain
unchanged; this evidence branch changes the test harness only. The subsequent serial complete gate
passed, and the initial failed results remain recorded.

The first complete rerun with the corrected harness passed input ownership but
failed the separate echo reference budget: p95 was 299 ms against 250 ms, with
navigation p95 27 ms and startup p95 400 ms. Assistant-started Clippy compilation
was concurrent on the same host. This loaded result is retained as failed;
a serial rerun with no build workload is required. No budget was relaxed.

After Clippy finished, the complete four-gate command passed serially through
the unchanged installed binary: ten tests passed and one fixture helper was
ignored. Startup p95 was 82 ms, navigation p95 24 ms, input echo p95 179 ms, and
flood throughput 23,500,094 bytes/s. The original 250 ms local echo budget passed.
Input ownership verified initial live dimensions 21 by 88, transfer to 17 by 78,
read-only resize isolation, rejected-acquisition isolation, and return to 28 by
118. Both worktree tests passed in the same invocation. This validates the
installed candidate with the corrected test harness, not a rebuilt executable.

`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
passed on the isolated source with the same harness correction. Formatting
passed in the evidence working tree. The first full workspace run without RELAYTERM_TEST_RT failed the debug-build
hardening echo reference check at p95 470 ms against 250 ms; navigation p95 was
23 ms and the other eight hardening tests passed (one helper ignored). This
unoptimized-binary performance failure is not relabelled as passing. A subsequent serial
`cargo test --workspace --locked --offline -- --test-threads=1` completed with
exit 0 and RELAYTERM_TEST_RT selecting the installed production candidate in
supported gates. Original budgets are unchanged. Ignored helpers and isolated
opt-in gates remain ignored, not newly passing evidence. The corrected test
file was byte-compared with the evidence working tree after validation.

The passing integrated TUI scenario reported startup p95 148 ms, navigation
p95 25 ms, input echo p95 172 ms, and flood throughput 10,982,501 bytes/s.
A separate diagnostic checkout is used to inspect the failure without modifying
the installed candidate or the clean production checkouts.

## Initial operator checkpoint

The operator reported an outer terminal of exactly 24 rows by 80 columns and
TERM=xterm-256color. The initial discovery command accidentally checked `rt.`
with a trailing period. The corrected `type -a rt` subsequently reported no
command resolution. This is terminal preparation, not TUI layout evidence.

The operator supplied three local screenshots showing the installed candidate
in Overview, Tasks, and Sessions. The assistant inspected them without copying
them into Git. At the reported 80 by 24 size, all six tab names, the complete
CURRENT label, and the Relayterm footer with keyboard hints are visible.
Tasks and Sessions are empty. Independent installed workspace readback confirms
revision 1, ready daemon, and zero definitions, instances, and tasks. Platform
warning confirmation and the remaining screens/resize observations are pending.

The next operator screenshot showed one running Session 1 with CURRENT and
full, distinct session/instance identities. Independent ordered readback matched
both identities, creation ordinal 1, the enabled Shell A definition with
`/bin/sh`, empty arguments/environment allowlist, terminal capability, and no
task assignment at revision 5 and watermark 5. The remaining two intended definition launches are
pending operator observation. A subsequent screenshot showed Shell B disabled
while Shell A remained selected and was also disabled. Readback at revision 10
and watermark 10 showed Session 1 still running and a second Shell A session
terminated with exit code 1. This deviation is retained; it does not establish
a Shell B launch or the intended three-session journey. Guidance was narrowed
to one explicit selection and launch at a time, without deleting history.

A subsequent Sessions screenshot and ordered readback at revision 15 and
watermark 15 confirmed four stable rows: Session 1 running from Shell A,
Session 2 terminated from Shell A, and Sessions 3 and 4 running from Shell B.
Thus three children are live, but the planned third distinct definition has
not yet launched. Both Shell B launches are retained with distinct session and
instance IDs. The operator is being guided through Shell C creation separately
from selection, enablement, and launch.

## SSH availability

The M11 record identifies a temporary loopback server that was stopped after
observation. Read-only inspection confirmed that the global SSH service and
socket are inactive and disabled, with no initial listener on ports 22 or 22222.
OpenSSH client/server 9.6p1 Ubuntu-3ubuntu13.19 is installed.

The operator explicitly authorized a new temporary loopback SSH server.
A user-owned server bound to loopback port 22222 was prepared with isolated
synthetic Ed25519 host/client keys, pinned host verification, public-key-only
authentication, and forwarding disabled. StrictModes is disabled only in this
temporary configuration because its synthetic authorized-key file is below the
sticky temporary directory, matching the M11 fixture boundary. No global SSH
configuration or service setting was modified. A fresh batch connection returned
the expected synthetic marker. This proves connection availability only;
interactive SSH cursor, conflict, writer-size, disconnect/reconnect, identity,
and terminal recovery observations remain pending.

## Preparation checks

Assistant checks passed:

- `python3 -m unittest discover -s scripts -p 'test_*release.py'`: 22 cases,
  20 passed and two Windows-only cases skipped.
- `cargo fmt --all -- --check` on the exact clean candidate.
- `python3 scripts/check_repository.py`: style, links, queue, and boundaries.
- `python3 scripts/check_secrets.py`: candidate scan and synthetic negative
  control, using the workflow-pinned Gitleaks 8.30.1 after checksum validation.
- `python3 scripts/check_audit_controls.py`: license and dependency-ban negative
  controls with workflow-pinned cargo-deny 0.20.2 after checksum validation.

The initial scanner and audit-control invocations could not start because their
required tools were absent from PATH. The isolated tools and explicit toolchain
PATH resolved those preparation failures; they were not product failures or
passing scans. The full dependency advisory audit is not claimed by these
negative-control checks.

## Observation boundary and pending checkpoints

Assistant automation and operator observations will be recorded separately.
No screenshots, raw terminal captures, databases, private paths, credentials,
or personal host/account identifiers belong in this record.

- Resolve the installed TUI gate failure and finish exact-installed integration
  gate evidence; retain the original failed result.
- Observe the three-session quick start, IDs/order/names, Unicode form cursor,
  exact 80 by 24 layout, below-minimum recovery, and two-client stale review.
- Observe first writer acquisition, transfers in both directions, read-only
  resize isolation, detach/reattach, and client quit/reopen.
- Observe task-neutral claim, conflict, progress, immediate CURRENT and
  unclaimed handover_ready, successor claim, completion, and retained history.
- Verify live backup and fresh-home restore after explicit operator agreement
  to terminate the disposable sessions; retain original home and backup.
- Complete focused SSH physical observations.
- Independent clean-machine evidence, affected macOS retests, and global M12
  reconciliation remain separate open requirements.
