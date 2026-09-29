# M12 sustained performance verification

## Contract and separation

The historical `tui_initializes_launches_detaches_and_reopens_without_stopping_children`
scenario remains a functional and finite-burst regression. Its latency figures
must not be described as proof of the sustained M11 workload. Historical Windows
failures remain visible; a new harness is not a retrospective fix or waiver.

The new `sustained_load::sustained_output_navigation_and_echo_meet_declared_contract`
scenario uses three synthetic native sessions: a paced 3 MiB/s producer (measured
minimum 2 MiB/s), a full-screen ANSI producer at a configured 10 frames/s, and an
echo producer. The configured rates are targets, not observed throughput.
The output remains active for at least 120 seconds. One hundred navigation
samples and 100 parsed visible echo samples are spread across separate windows
inside this interval. Producer readiness precedes measurement; private stop
controls and a 240-second producer safety limit bound the fixture lifetime.

A separate monitor uses one persistent IPC connection to read daemon snapshot
raw byte offsets for both producers via one-cell viewport responses. It does
not spawn a CLI or transfer full terminal snapshots per sample. The source
viewport dimensions and input ownership are unchanged by these cropped reads.
It checks monotonic consumed flood deltas, a minimum 2 MiB/s for each observed
interval and each measurement window, bounded sampling gaps, and continuing
full-screen output. The elapsed denominator conservatively includes snapshot
request latency. The monitor itself generates administrative load, consistently
in both profiles. These counters measure bytes consumed by the daemon, not bytes
painted on the physical terminal. Producer byte/frame totals are also reported.
Synthetic negative controls reject a finite burst, a stopped screen, insufficient
rate, missing overlap and large sampling gaps.

Thresholds remain reference p95 navigation <=100 ms and echo <=250 ms; hosted
classification retains <=500 ms and <=1 second and reports reference misses.
Do not set CI to obtain a pass on a local host. Keep product override, harness
profile, host, load rate and candidate identity distinct.

Administrative capture now bounds process exit and the first JSON response line
separately. Waiting for EOF after a detached daemon inherits a pipe is not a
completion condition. A timed-out or uncertain mutation must not be retried
blindly. No production API, dependency, budget or retained package is changed.

## Native commands

Use a non-corporate development environment with the pinned toolchain already
available. Do not install tools on the independent VM or access the corporate
laptop as an implicit extension of these commands. Preserve existing environment
settings; check CI and RELAYTERM_TEST_RT first. For same-profile measurements,
both must be absent in the measurement process. Keep stdout/stderr in private
files and transfer only sanitized numerical summaries.

Build both profiles before measuring so compilation is not concurrent load:

```sh
cargo test -p relayterm-cli --test tui_gate --locked --no-run
cargo test -p relayterm-cli --test tui_gate --release --locked --no-run
```

Declare two repetitions per profile, ordered debug, release, debug, release.
Run serially with no overlapping compiler, linter or other validation job:

```sh
cargo test -p relayterm-cli --test tui_gate sustained_load::sustained_output_navigation_and_echo_meet_declared_contract --locked -- --exact --nocapture --test-threads=1
cargo test -p relayterm-cli --test tui_gate sustained_load::sustained_output_navigation_and_echo_meet_declared_contract --release --locked -- --exact --nocapture --test-threads=1
```

Use an external 360-second deadline per focused run. A timeout is a failure to
complete, not a performance sample. Preserve every attempt, including failure.
The same source is included in hardening_gate under the prefix `tui::`; its
native execution must also pass before claiming the repository gate verified.
Do not infer native Windows behavior from Mac compilation or execution.

For a retained release comparison, explicitly set RELAYTERM_TEST_RT only for
that separate run and record its SHA-256. Do not call a debug harness running a
release product a debug-product measurement. Record git HEAD, dirty patch hash,
SHA-256 of both harness source files and actual product/harness executables,
Rust version, OS/architecture, CPU/RAM and whether the host is virtualized.

## Windows handoff

Use the same focused commands in an ordinary PowerShell development session.
Keep compiler prerequisites separate from the independent runtime trial, which
already passed. The Windows owner should preserve the seven historical debug
misses and eight release passes, inspect their original logs, and compare the
new load-qualified results with explicit harness identities. If a miss recurs,
profile the observed bottleneck before proposing a product change. If it does
not recur, report non-reproduction rather than a demonstrated historical fix.

Corporate host work is deferred by the operator to avoid repeated security
alerts. Do not probe it, change policy, retry installation, disable controls or
request an exception from SOC. Possible application-reputation enforcement and
the observed ACL API failure remain separate hypotheses until evidence relates
them. Public acceptance and any supported-environment limitation require an
explicit disposition; this deferral is not a waiver.

## Local results

Harness verification, measured product performance, native Windows verification
and global M12 acceptance are separate gates. The practical local trial may
proceed with a clearly identified executable without claiming release readiness.

Development attempts are not pooled with the final harness measurements. The
first attempt also overlapped compilation and is excluded from performance
claims. A monitor that repeatedly started administrative CLI processes failed
short-interval load checks in debug, although the whole-window rate was about
3.13 MB/s. A persistent connection transferring full snapshots passed one debug
and both release repetitions but failed one debug load check. Those observations
do not establish a product throughput failure: the conservative interval also
includes observer delay. The final one-cell response reduces transfer overhead
without changing the workload, sampling-gap limit or latency budgets. All
attempts and their available source identities remain in private local evidence.

Final measurements on 2026-09-28 used native macOS 26.5.2 arm64, Apple M1 Pro,
16 GiB RAM and Rust 1.98.1. Both profiles were compiled before the serial order
debug 1, release 1, debug 2, release 2. CI and RELAYTERM_TEST_RT were absent.
Each run passed the 120-second overlap and interval checks, continued full-screen
output, 100 navigation samples and 100 echo samples. The observed load interval
was approximately 122 seconds. No compiler or other verification job ran in
parallel with these measurements; ordinary user applications were left intact.

Latency columns are median / p95 / maximum in milliseconds. Rate is the
conservative whole-window consumed flood rate in bytes per second.

| Profile and repetition | Navigation ms | Echo ms | Consumed bytes/s | Result |
| --- | --- | --- | ---: | --- |
| debug 1 | 26 / 30 / 31 | 137 / 147 / 191 | 3,145,650 | Passed |
| release 1 | 28 / 31 / 78 | 108 / 116 / 119 | 3,145,487 | Passed |
| debug 2 | 27 / 30 / 30 | 135 / 145 / 155 | 3,145,694 | Passed |
| release 2 | 28 / 30 / 43 | 111 / 134 / 143 | 3,145,711 | Passed |

These are local product measurements with the final harness, not a Windows
latency fix. They do not replace retained-binary evidence or historical failures.
Native Windows and Linux execution and hosted CI duration remain unverified for
this new scenario. Reuse completed functional/physical/SSH journeys.

The source base is 43ba692680820d5059ee6d5b9ff8033ea76d058f with the harness patch.
Production sources and Cargo dependencies are unchanged from 691a8fb. The private
measurement inventory records every executable hash and raw sample. The final
harness source SHA-256 values are:

- `crates/relayterm-cli/tests/tui_gate.rs`: `a8b264fa2ae2671207724998e8172dd8ef9f40c3da6e9b7841beed62ea0cce95`.
- `crates/relayterm-cli/tests/support/sustained_load.rs`: `ee80ac913bd775784a5e7f7e289202885c1c359ea0c3a805d238472723cbf244`.


The same final scenario also passed once through hardening_gate in local debug:
237 load observations, 3,145,725 consumed bytes/s, navigation p95 30 ms and echo
p95 149 ms. This is an integration check, separate from the declared four-run
comparison. The remaining tui_gate selection passed nine tests with one ignored
fixture helper. Formatting, workspace/all-target Clippy with warnings denied,
and workspace library tests passed. The full workspace integration suite was
not rerun; native Windows/Linux and hosted execution remain pending.
