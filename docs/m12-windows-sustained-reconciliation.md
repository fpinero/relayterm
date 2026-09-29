# Windows sustained evidence reconciliation

## Decision on 2026-09-29

The supplied native Windows attempt failed load qualification in all five runs.
M12 remains open. No new latency acceptance pass follows, including the release
runs whose numerical echo p95 is below 250 ms. Keep prior Mac passes, the seven
historical finite-burst debug failures, eight retained-release passes and the
separate 290/294 ms workspace failures visible. Corporate execution stays deferred.

The environment reported here is Windows 10 Pro 10.0.19045, x86_64 MSVC, Intel
Core i7-8665U, 4 cores / 8 threads and approximately 15.75 GiB RAM. It is distinct
from the Windows 11 VM runtime journey. HypervisorPresent=true does not establish
that this OS is a guest or explain the failure. Rust was 1.98.1. No corporate
machine, physical/SSH journey or prior VM functional journey was exercised.

## Identity and outcome

The reports map to base 43ba692680820d5059ee6d5b9ff8033ea76d058f plus the exact
[sustained harness](m12-sustained-performance.md). The LF-normalized patch SHA-256
is 3b8115c7be6753635cbff1c5d1be98199a2da8bae8d77ea62d754138c42687e2.
Both Rust source hashes and the supplied performance document hash match the
Mac handoff. The original Windows patch had CRLF bytes; this transport distinction
does not imply a source change. Product sources, dependencies, budgets and retained
packages were unchanged. Actual Windows executable hashes are in the supplied
report and must be retained with any subsequent diagnostic identity.

Both profiles and the debug hardening target were built before measurements.
The declared order was debug 1, release 1, debug 2, release 2, then hardening debug.
CI and RELAYTERM_TEST_RT were absent. Each run exited 101 without reaching the
360-second outer deadline and collected 100 navigation and 100 echo samples.

Rates below are bytes per second. Echo values are observed p95 milliseconds,
not load-qualified acceptance results.

| Run | Observations | Whole-span rate | Failing intervals | Producer rate | Echo p95 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Debug 1 | 224 | 1,991,672 | 164/223 | 1,918,675 | 254.125 |
| Release 1 | 225 | 2,080,267 | 125/224 | 2,005,301 | 144.614 |
| Debug 2 | 223 | 1,976,220 | 216/222 | 1,905,096 | 318.475 |
| Release 2 | 225 | 2,235,039 | 26/224 | 2,154,521 | 143.042 |
| Hardening debug | 225 | 1,526,562 | 224/224 | 1,471,637 | 254.562 |

All observed spans exceed 120 seconds. Every screen delta is positive; counters
are monotonic and gaps are below 3 seconds. All three load windows fail in every
run. Release 2 passing the average minimum does not waive its failed intervals.
Maximum observer duration is 30.276 ms in debug and 2.461 ms in release. Four
producers averaged below the unchanged 2,097,152 bytes/s minimum; none maintained
the configured 3,145,728 bytes/s target. Producer and daemon counters are different
observations across native terminal transport and must not be equated or used
to infer byte loss without accounting for terminal transformations.

## Interpretation and Windows ownership

The evidence establishes missing sustained-load proof. It does not isolate a
product regression, a native transport limit, pacing behavior, machine load or
parser cost. Low producer output can itself reflect downstream backpressure.
The daemon reader processes output under a terminal-state mutex, but that source
fact alone does not identify the bottleneck. Observer delay is insufficient as
an explanation without a controlled comparison, especially for release.

Continue diagnosis and any supported correction on the non-corporate Windows
host. Compare the same synthetic producer with a drained ordinary pipe, native
PTY drained without terminal processing, and the daemon path. Measure write,
flush, requested/actual sleep, stop-control overhead, reader timing, lock wait
and terminal processing with bounded aggregate instrumentation. Add comparisons
only when evidence warrants them. Keep diagnostic workloads separate from the
unchanged acceptance contract. Do not alter security policy, timer configuration,
dependencies or host settings to manufacture a pass.

The Windows owner may implement a minimal harness or production correction when
the evidence supports it, with proportionate regression tests and native checks.
A production correction creates a new candidate and cannot inherit retained
binary acceptance automatically. Mac should review the resulting patch and
verify affected cross-platform behavior, rather than duplicate Windows edits
while that owner is iterating. Global acceptance remains a separate decision.

## Verification boundary

On Mac, both supplied files were read and every numerical row was parsed.
Recomputed all 1,117 interval deltas, conservative rates and denominators, checked
all 1,122 observations and 500 paired latency samples, and reproduced the reported
whole-span rates, failing counts, observer maxima, producer rates and echo p95.
Checked current patch and all three handoff source hashes against the report.
This is independent arithmetic/source verification, not Windows re-execution.
Original reports and numerical evidence stay outside version control.

The Windows report records successful compilation, formatting, workspace/all-target
Clippy, workspace library tests and eight selected TUI regressions including the
negative load control. Its selection excluded the legacy finite-burst journey;
do not describe that journey as newly passing. Repository/privacy checks also
passed there. Full workspace integration, Linux and hosted CI were not rerun.
These reported successes do not override the five failed native load gates.
