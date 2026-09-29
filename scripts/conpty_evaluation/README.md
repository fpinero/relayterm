# Isolated ConPTY evaluation

This Windows x64 experiment selects Microsoft's independently packaged ConPTY
runtime through a small patch to portable-pty-psmux 0.9.7. It does not install a
runtime globally or change the default repository dependency. It requires
Python 3.12 or later, PowerShell 7, Git and the existing Rust toolchain.

The operator must explicitly authorize downloading and evaluating dependencies.
Use a separate working copy when preserving the current Cargo.lock is necessary:
the first Cargo build with the path override updates that file. Keep all failed
attempts and private output. Do not promote this experiment to a distributed
product without reviewing runtime packaging, updates and native regressions.

## Prepare

From the repository root:

```powershell
python scripts/conpty_evaluation/prepare.py "$env:LOCALAPPDATA/relayterm-validation/conpty-evaluation"
```

The destination must be new and outside the checkout. Preparation verifies the
crate and NuGet package SHA-256 values, applies the dependency delta and checks
Microsoft Authenticode signatures. The runtime version is 1.24.260710001.
No system DLL, PATH, security setting or timer configuration is changed.

The adapter loads only an explicitly supplied absolute path. Its absence keeps
the original system backend. A supplied but invalid path fails instead of
silently switching backends. The Microsoft runtime exports Conpty-prefixed
functions; its flag 0x8 specifies glyph width, so the dependency's purported
passthrough flag is not forwarded. The experiment uses default flags (zero),
without cursor inheritance. Process creation stays unchanged. The master retains
an input handle so dropping the writer cannot prematurely terminate the console.
A bounded reader recognizes the initial DA1 query across chunk boundaries and
responds once with basic VT100 capabilities. All output bytes remain intact.
The matcher stops after the first response or 4096 inspected bytes. This adapter
has only been executed on x64.

## Build and measure

Assign the prepared directory, then build every profile before measurement:

```powershell
$evaluation = "$env:LOCALAPPDATA/relayterm-validation/conpty-evaluation"
./scripts/conpty_evaluation/run.ps1 -EvaluationRoot $evaluation -Scenario acceptance -Profile debug -BuildOnly
./scripts/conpty_evaluation/run.ps1 -EvaluationRoot $evaluation -Scenario acceptance -Profile release -BuildOnly
./scripts/conpty_evaluation/run.ps1 -EvaluationRoot $evaluation -Scenario hardening -Profile debug -BuildOnly
```

Run acceptance serially in debug, release, debug, release order, followed by
hardening debug. Omit BuildOnly from the corresponding commands. Do not compile
or run other checks concurrently. The original 120-second workload, 100 samples
of each latency, load minimum, sampling gaps and latency budgets are unchanged.
Each acceptance command has a 360-second external deadline.

For preliminary measurements, use Scenario diagnostic or native-long. Build that
scenario first. These scenarios enable reader test hooks; acceptance and
hardening omit them. Diagnostics have a 180-second deadline and their success
only records completion, not acceptance. Build deadlines are 900 seconds.

All commands keep CI and RELAYTERM_TEST_RT absent in child processes. Runtime
selection is child-local. Logs, identities, Cargo artifacts and numerical reader
reports remain in the private evaluation directory. Inspect owned synthetic
processes after a timeout before retrying because a daemon can be detached from
the terminated process tree. Do not terminate unrelated processes.

## Sources

- [Microsoft runtime package](https://www.nuget.org/packages/Microsoft.Windows.Console.ConPTY/1.24.260710001)
- [Microsoft public header](https://github.com/microsoft/terminal/blob/main/src/winconpty/winconpty.h)
- [Dependency source](https://docs.rs/crate/portable-pty-psmux/0.9.7/source/src/win/psuedocon.rs)

The pinned package's included conpty.h is the ABI reference for this experiment;
the repository links provide background and may evolve.
