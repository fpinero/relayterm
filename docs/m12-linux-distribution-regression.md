# M12 Linux distribution regression

## Scope and source mapping

The bounded Linux continuation passes after a verified development-profile
correction. This closes M12.LINUX-DISTRIBUTION-REGRESSION, not global M12.

Native validation ran on 2026-10-01 on Ubuntu 24.04.5 LTS x86_64, kernel
7.0.0-34-generic, glibc 2.39, Bash 5.2.21 and Dash 0.5.12. Compiler/linker:
GCC 13.3.0 and GNU ld 2.42. Rust/Cargo 1.98.1 uses LLVM 22.1.8.
The native target is `x86_64-unknown-linux-gnu`; no ARM64, musl, older Ubuntu
or other distribution coverage is inferred.

Fetched delivery `3845420e88bb8f6b3b40f95f4b042f8afb603b87` contains integrated
checkpoint `e712603cc753686675319358f0413946461249e2`. Its later changes are
public documentation/evidence only. The original checkout was clean; its old
work branch, packages, logs, backups and container state were preserved.
Work continues on `fix/m12-linux-distribution-validation`.

A required Linux development-profile correction is committed at
`6486da6c6005f366586f7ded036708bf3f6d3ccd`. It changes only `Cargo.toml` and
contributor documentation: `vt100` uses optimization level 1 in the development
profile. Relayterm crates retain debug settings and dependency debug assertions
remain enabled. Production source, release profile, dependency versions, lock,
protocol, schema, Unix 64 KiB reader, workloads and thresholds are unchanged.
Cargo.lock SHA-256:
`c2a0a6aaaef016e275323aed682313c73e28e1a104561c305ce74d3a49885ec8`.

## Retained failure and supported correction

The original locked offline workspace failed, exit 101 in 193.904 seconds,
inside the hardening sustained test. Its 121.978169-second load span averaged
2,124,383 bytes/s, but individual consumption intervals fell below 2 MiB/s.
Navigation/echo p95 was 41.643/230.893 ms, maximum echo 718.575 ms.
These latencies are observations, not qualified acceptance evidence.
No threshold, producer workload or sampling rule was relaxed.

The existing serial diagnostic measured pipe/native PTY producer rates of
3,145,687/3,145,607 bytes/s, versus 2,428,363 through the daemon.
The existing 16 MiB processing diagnostic measured complete/parser/retention
6,782,415/6,425,491/233,043 microseconds. With only the committed parser-profile
change, those values became 1,480,619/1,205,497/231,176 microseconds.
This identifies parser cost in the development build, rather than a PTY producer
or raw-retention bottleneck. Neither diagnostic is an acceptance workload.

The repeated complete workspace passed: 205 tests, zero failures, 25 explicit
ignores across 48 groups, exit 0 in 438.893 seconds under the original
1,200-second bound. Both sustained inclusions qualified independently.
No build overlapped these measurements.

| Scenario | Consumed bytes/s | Navigation p95 ms | Echo p95 ms |
| --- | ---: | ---: | ---: |
| Corrected debug hardening | 3145693 | 40.603 | 208.177 |
| Corrected debug TUI | 3145596 | 21.080 | 226.600 |

The explicit resource gate passed in 188.538 seconds with 60 observations,
100 reconnects including 20 abrupt, and descriptors 43 to 43, tolerance 16.
Daemon/TUI/fixture maxima were 352,862,208/27,197,440/31,924,224 bytes.
Daemon steady medians were 349,474,816 to 349,532,160 bytes; TUI medians were
27,172,864 to 27,197,440 bytes. The 512 MiB ceilings, 32 MiB plateau allowance,
minimum 40 samples and 180-second window remain unchanged.
Explicit SQLite rollback, worktree cancellation and ignored Git descendant
containment tests passed and executed nonzero selections.

## Tooling and preparation

No Rust installation or Cargo cache survived in the earlier temporary tool root.
Rust 1.98.1 was prepared in a new private directory using `--no-modify-path`.
The first locked offline metadata probe failed because cache inputs were absent.
Explicit `cargo fetch --locked` prepared both workspace and vendor graphs;
subsequent checks and builds use `--locked --offline`.
Child environments remove CI, product overrides, ConPTY selection and reader
metrics/delay variables. Persistent environment and host protections are unchanged.

Both workspace and vendor rustfmt, default/all-feature/all-target Clippy with
warnings denied, the vendored Unix test, and six Python tooling modules passed.
Python ran 31 tests with five Windows-only skips. Six Windows adapter tests were
not run on Linux. Repository contracts and candidate secret negative controls pass.
Gitleaks 8.30.1 and cargo-deny 0.20.2 were prepared from CI's checksum-pinned
archives. The initial scanner invocation failed because Gitleaks was absent;
the prepared scanner passed candidate/control and 290-commit history checks.
The initial offline advisory check failed because its database was absent.
An explicit online audit fetched advisories and passed, followed by a passing
offline audit and license/ban negative controls. This is local security evidence,
not a new hosted Security run. Allowed duplicate-version warnings remain.

The hash-pinned upstream portable-pty-psmux 0.9.7 archive was compared after
line-ending normalization. Unix and serial sources are identical. Shared library
code only adds a Windows-gated export; command-builder differences remove external
test declarations absent from the published archive. Unix runtime behavior is
unchanged by vendoring.

## Isolated runtime and reused observations

The retained Ubuntu 24.04 container uses image digest
`sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`.
It ran the new exact candidate as an unprivileged user with no network, mounts,
capabilities or privilege escalation, without source, Cargo, rustc, Python, C
compiler or Git inside the runtime. External Python orchestration is distinct
from product dependencies. Its userspace uses glibc 2.39; the kernel is shared
with the Linux host. This is independent userspace evidence, not a separate VM.

The earlier corrected private recipe was adapted to new owned paths and the
new candidate hash. Packaged installation/discovery/collision, initialization,
three named/ordered shell PTYs and input, task claim/competing rejection/progress/
handover/successor/completion, authoritative history readback, automated 80 by 24
TUI CURRENT/normal exit/alternate-screen restoration, client-close continuity,
private backup and fresh-home restore/reopen all passed. Restored sessions retain
identity and become honestly lost; task/history/claim readbacks match.
The original installed executable and backup checksums are unchanged.
Both new daemons stopped normally. Before container stop, there were no live
product processes; two exited daemon entries remained unreaped under its sleep
PID 1. The stopped container and all old/new state are retained.

The current Unix/vendor/dependency boundary is therefore exercised against the
new package without relabelling the earlier 691a8fb runtime artifact.
Populated worktree recovery is checked separately by the exact-installed host
backup/worktree gates, because this minimal runtime intentionally has no Git.
Existing Linux physical names/order/Unicode/cursor/conflict/writer-size,
coordination/reopen, disconnected feedback, Info/Error and normal-exit observations
remain mapped to 9cf91f7/15794ad/691a8fb. The later Mac SSH delta is reused at
its original scope. The new terminal-refresh timing and parser-profile boundary
are deterministic and covered by automated PTY/load tests, with unchanged rendering
semantics and runtime restoration. No new uncovered physical behavior was found.
No screenshot battery, SSH configuration change or corporate-host access occurred.

## Reproducible artifacts and exact-installed gates

Two clean detached source copies of 6486da6 used separate fresh outputs, locked
offline dependencies, release profile, empty production features, two build jobs
and the existing path-remapping wrapper. Builds passed in 451.747/451.717 seconds,
each below 900 seconds. Build-record comparison, manifest comparison, direct
binary/archive byte comparisons and both external SHA256SUMS passed. Both source
copies remained clean. No ambient code-generation override was present.
Version is 0.1.0; signing state is unsigned.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Built/extracted/installed rt, both builds | 13053304 | `248c36e33d25601cb2dcee5d8dc2760b881dded44c79845fd79d85f5ef027236` |
| Normalized tar.gz, both packages | 4840159 | `e2e716d3549681b129fc8813c2e4ba7d62f97cadaef8764e96bf75b4b6e9797b` |

Archive inspection/extraction found exactly nine regular members with verified
modes: rt, LICENSE, THIRD_PARTY_NOTICES.txt, THIRD_PARTY_LICENSES.txt, INSTALL.md,
RECOVERY.md, install_release.sh, install_release.ps1 and manifest.json.
ELF inspection identifies x86-64 PIE, interpreter `/lib64/ld-linux-x86-64.so.2`,
libgcc_s.so.1, libm.so.6 and libc.so.6. All resolve through the system loader and
`/lib/x86_64-linux-gnu`. Highest required GLIBC symbol version is 2.39, distinct
from an inference based only on host glibc. Bundled SQLite requires no system
SQLite library. No older runtime claim follows from the ELF kernel ABI field.

Archive smoke passed constrained-PATH help/version, initialization, detached
daemon, real synthetic PTY and orderly shutdown. This developer-host smoke is
separate from the isolated container evidence. The packaged POSIX helper installed
into a new owned directory. Absolute-path help/version and actual Bash/sh command
discovery passed without profile or persistent PATH changes. Reinstallation and
existing-file collisions were rejected without changing bytes. An additional
executable synthetic unrelated rt was discovered/run before and after rejection,
with identical bytes and output. Removal affected only a separate hash-verified
owned copy that never launched a daemon; its foreign marker and external private
state, retained main installation and colliding commands remain intact.

The exact installation passed four serial locked offline selections, with
RELAYTERM_TEST_RT set only in their children:

```sh
cargo test -p relayterm-cli --test tui_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test session_presentation --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test worktree_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test backup_restore --locked --offline -- --nocapture --test-threads=1
```

Fourteen tests passed, zero failed and five helpers/diagnostics were ignored.
The backup fixture is populated, including worktree metadata. Harness executables
are separate development outputs, not package contents. Product hash before/after
every command matches the reviewed installation. The release TUI command passed
in 146.456 seconds under 600 seconds. Its sustained section independently qualifies:
3,145,808 consumed bytes/s, navigation p95 21.040 ms, echo p95 126.230 ms and
maximum echo 145.570 ms, with 100 samples per latency kind and a 122-second span.
No builds, installation probes or other verification workloads overlapped it.

## Evidence and remaining boundaries

The [public evidence index](evidence/m12-linux-distribution-20261001/README.md)
contains the sanitized command ledger, candidate inventory, raw numeric
observations, independent arithmetic and retained-attempt ledger. Raw logs,
drivers, packages, private paths, terminal captures and databases remain outside
Git. The checked-in verifier recomputes four sustained sections: three qualify;
the original failed section remains unqualified. Every accepted command exited
zero; expected installer refusals exited one. No deadline was exhausted.
The initial public numeric export omitted headers printed inline after test names.
The verifier parsed zero sections and an exact-count assertion failed. The incomplete
export is retained privately. Correcting only those headers restored all four
sections, with numeric values and original observations unchanged.
Corrected debug commands ran with the exact manifest delta before it was committed
unchanged. The distributed release is built from that committed source.

Mac/Windows reports retain their original source/artifact identities. The new
profile input changes development measurements, so fresh final-source hosted jobs
must include it; it does not alter the production release settings or invalidate
already passed unchanged Mac/Windows production behavior. Linux evidence cannot
supply a Windows standard-account/console observation, Mac declared-floor runtime,
Windows current-artifact independence or real-agent authentication/trial.

The finite remaining gates are the Windows ordinary-package operator delta,
non-Linux runtime-floor/independence reconciliation, fresh hosted quality/security
and final AC/phase/artifact reconciliation, the prepared private practical trial,
and the later concrete publication decision. Corporate diagnosis remains deferred,
not a new prerequisite to access that host. Existing accepted terminal limits and
all prior failed observations remain unchanged. Use the current
[closure audit](m12-closure-audit.md); no global M12 completion, merge, PR, tag,
release or binary upload is claimed.
