# M12 integrated macOS validation

## Scope

On 2026-09-30, this checkpoint verified committed candidate
`6e573326180acbbc5fdc6c706a610c27d22eda31` on native Apple Silicon macOS 26.5.2.
The product retains the 64 KiB Unix reader, 16 KiB Windows reader and corrected
TUI refresh ordering. No product source, dependency, budget or workload changes
are part of this checkpoint. Previous failures remain in the
[ConPTY evaluation reconciliation](m12-conpty-evaluation.md).

## Execution contract

Run verification serially with Rust 1.98.1 and the locked dependency graph.
Remove CI, executable override, experimental ConPTY and reader metrics variables
from child environments. Build test executables before sustained measurements.
Keep command logs, deadlines, exit codes and timings privately. A timeout needs
owned-process review, not a silent retry. Do not replace failed attempts.

The default workspace test command is:

```sh
cargo test --workspace --locked --offline -- --nocapture --test-threads=1
```

Explicitly enable the ignored resource scenario separately:

```sh
cargo test -p relayterm-cli --test resource_runtime_gate --locked --offline sustained_output_memory_and_reconnect_resources_are_bounded -- --ignored --exact --nocapture --test-threads=1
```

Also execute SQLite kill and worktree cancellation gates with `test-hooks`,
and the ignored `relayterm-git` descendant-containment tests. Test hooks are
excluded from distributable builds.

Build ordinary production release binaries in two separate clean local clones
of the committed candidate, each with a fresh target directory, using the
[release build contract](release-builds.md). Compare both binaries and archives,
inspect Mach-O dependencies and archive inventory, then run the constrained-PATH
release smoke check. Run `tui_gate`, `session_presentation`, `worktree_gate` and
`backup_restore` with `RELAYTERM_TEST_RT` set only to the extracted release binary.
Those tests exercise a release product even though their harness uses debug.
Preserve the old retained packages and practical-trial executable.

## Results

The complete default workspace suite passed: 204 tests passed, no failures and
25 ignored helper or explicit diagnostic/resource selections across 48 test and
doc-test groups. The ignored resource and fault selections are handled separately.
The command completed in 521.705 seconds, within its external 1,200-second limit.

Both included sustained gates passed with 100 navigation and 100 echo samples
each, 237 load observations and more than 122 seconds of load. CI and product
override were absent, so these are native debug-product results.

| Debug inclusion | Navigation p95 ms | Echo p95 ms | Whole-load bytes/s |
| --- | ---: | ---: | ---: |
| Hardening | 30 | 110 | 3,144,929 |
| TUI | 30 | 110 | 3,145,535 |

Every required interval and navigation/echo load window qualified against the
unchanged 2 MiB/s minimum. Earlier failed attempts remain historical evidence.

Formatting, all-feature/all-target Clippy with warnings denied and release
tooling unit tests passed. The Python selection ran 22 tests with two
platform-specific skips. The explicitly enabled resource scenario passed in
191.989 seconds: 60 observations, daemon maximum 367,329,280 bytes, TUI maximum
21,757,952 bytes and fixture-child maximum 17,416,192 bytes. Daemon descriptors
were 48 before and after 100 reconnects, including 20 abrupt disconnects.
SQLite kill recovery, worktree cancellation and both ignored Git descendant
containment regressions passed.

The two clean release builds and packages were byte-identical. Both have version
0.1.0, target `aarch64-apple-darwin`, default production features and unsigned
status. The exact archive inventory contains the nine expected regular files.
Mach-O inspection found ARM64 and only system libiconv/libSystem dependencies.
The manifest's macOS 14 baseline is a declared target, not the observed OS of
this run. No macOS 14 runtime claim is added here.

| Artifact | SHA-256 |
| --- | --- |
| Production rt, 11,050,320 bytes | `8e389b25463d0f14516e9e18ea2c34a4302c72565fe03adf403264364cb43ee7` |
| Native tar.gz package | `c69074311a8d35743dfa9c86913345e7b5f1f0360484a72d31b0e6c476944947` |

The extracted-package smoke passed help/version, initialization, detached daemon,
synthetic PTY and orderly shutdown with PATH restricted to system directories.
The actual POSIX installer passed isolated clean installation, byte-preserving
collision/reinstallation rejection and executable resolution through a
process-local PATH. Removal deleted only the newly installed, hash-verified rt;
a synthetic unrelated marker and colliding command remained intact.

The extracted release passed `tui_gate` (10 tests, five explicit helpers or
diagnostics ignored), `session_presentation` (one), `worktree_gate` (two) and
`backup_restore` (one). Its sustained inclusion collected 238 observations over
122.003 seconds and 100 samples of each latency category. Navigation p95 was
31 ms and echo p95 78 ms; every required load interval/window qualified, with
whole-load consumption 3,145,396 bytes/s. These are release-product results
through a debug harness with `product_override=true`, not debug-product timing.
No builds or other verification jobs overlapped sustained measurements.

The serial driver completed 34 bounded commands with exit zero and no timeout.
The actual-package installer checks were executed separately after that driver.
Repository contracts, candidate secret scanning and its synthetic negative
control passed. Offline `cargo deny` passed advisories, licenses, bans and
sources against the locally available advisory database, with duplicate-version
and unused license-allowlist warnings. License-rejection and dependency-ban negative controls passed. This
is local cached-database evidence, not a claim of a freshly fetched security
advisory database or new hosted CI coverage.

## Reproduction commands

Use the release build, package comparison, inspection and smoke commands from
[release builds](release-builds.md), with separate empty output directories in
two clean clones of the source commit above. After extracting the reviewed
archive, set `RELAYTERM_TEST_RT` to its absolute executable path only in the child
environment, then run:

```sh
cargo test -p relayterm-cli --test tui_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test session_presentation --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test worktree_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test backup_restore --locked --offline -- --nocapture --test-threads=1
```

Explicit fault/resource checks and the default workspace run must use their
normal product selection without that override. Every raw command, deadline,
exit status, source/binary identity and observation is retained privately.
The unrelated local reviewer document was preserved and excluded from the
candidate contract/secret checks through isolated clean source copies. Final
document checks passed for 78 Markdown files and eight ADRs; the final candidate
secret scan, Git-history secret scan and whitespace checks also passed.

## Remaining boundaries

Native Mac success cannot certify Linux, Windows, the declared macOS 14 floor,
or a machine independent of the development host. The ordinary Windows runtime
is still not the experimental ConPTY distribution. Windows integration,
standard-account package checks, helper resources, hosted checks, candidate-wide
reconciliation and global M12 acceptance remain separate gates.

Completed physical, SSH and Windows VM journeys do not need repetition without
an affected behavior or missing proof. The real-agent practical trial requires
the operator and remains deferred. Corporate-host probes remain deferred.
No commit, push, tag, release, dependency installation or host policy change is
part of this verification.
