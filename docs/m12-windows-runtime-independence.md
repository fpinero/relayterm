# M12 Windows runtime independence verification

## Result and scope

The retained f2b3f6f Windows package passed the bounded automatic development
environment independence check on 2026-10-01. The product ran outside the checkout
with a fresh allowlisted environment, profile, TEMP, workspace and private state,
using only its complete package and audited Windows dependencies. This closes
the Windows dependency/environment independence delta only.

The earlier exact-package Windows 10 22H2 floor evidence, OS 10.0.19045.0,
and [standard-account console observation](m12-windows-standard-observation.md)
remain valid and were not repeated as separate journeys. This automatic audit
used a non-elevated filtered development-account token, standard_account=false.
It does not reclassify that token or claim filesystem/security isolation.
The plan permits an audited user environment; a second computer or Windows
installation is not required for this independence proof.

macOS 14 execution of the retained Mac binary, the real-agent trial, remaining
cross-platform reconciliation and global M12 acceptance remain open. No CI,
corporate probe, account/security change, push, merge, PR, tag, release or binary
publication occurred at the runtime verification checkpoint. The operator later
authorized sanitized evidence/fixture delivery on the Windows result branch.

## Exact identities

| Item | Revision or SHA-256 |
| --- | --- |
| Product source | f2b3f6f446466f5b3657c3cd91746735cab59d78 |
| rt.exe | ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa |
| Prior package archive mapping | 039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50 |
| Supplied handoff ZIP | 0bac0b072ba220393b9574e420223134ae2a0a69d2942ef14326489f2bfc3adc |
| Original runner | 717bed70b3318def0c8e0181568a437192f82003fccfa5e8a5ae44c19b0d0a4f |
| Final runner | 5f217292a2d533e5b35cebcdb6a02ff5d719b4d054a33a43530ff6cd398c95a5 |
| Final generated collector bytes | 60cdac0e9b9dee6b7b6fc15b52c0bd092408d5e6430ad699f570e3e404244dbf |
| Synthetic tests | 47f42e96802fc31cc826a5b82066c888446c1caf6b51c609f0c9c3045866261c |

The ZIP was extracted into a new private directory outside both checkout and
candidate, and every supplied SHA256SUMS entry passed before use. The original
bundle remains unchanged. A separate fixture-correction directory retains the
finally executed script and collector identities. No package file was substituted
or rebuilt; all 13 original and copied hashes passed before/after product execution.
The archive hash above is retained provenance, not a claim of an archive rehash
in this new audit. See the [candidate inventory](evidence/m12-windows-distribution-20260930/candidate-inventory.json).

## Commands and native outcomes

Python was verified as 64-bit. These are the actual command forms, with private
directory arguments replaced by synthetic labels. Exact native arguments and
all raw stdout/stderr remain in private records, outside Git.

```text
python -m unittest scripts/test_windows_runtime_independence.py
python scripts/check_windows_runtime_independence.py --candidate <complete-retained-package> --inventory docs/evidence/m12-windows-distribution-20260930/candidate-inventory.json --output <fresh-private-attempt-01>
python -m unittest scripts/test_windows_runtime_independence.py
python scripts/check_windows_runtime_independence.py --candidate <same-complete-retained-package> --inventory docs/evidence/m12-windows-distribution-20260930/candidate-inventory.json --output <fresh-private-attempt-02>
```

The first synthetic invocation used the verified original bundle, the second
used the corrected fixture. Each ran nine tests: eight passed and one non-Windows
execution control was explicitly skipped on Windows. The symbolic-link rejection
control ran successfully. Original and corrected generated collectors parsed with
system Windows PowerShell's native language parser without errors.

Attempt 01 exited 1 at module_inventory because Get-FileHash was unavailable to
the system PowerShell subprocess inheriting the Python driver's PSModulePath.
Initialization, status/IPC, definition registration and real session creation had
already returned accepted JSON. Failure cleanup requested only the owned daemon's
stop with terminate-sessions, which returned stopped. A subsequent scoped collector
confirmed no fixture process remained. The complete failed state/result/logs remain.

The diagnosis reproduced cmdlet discovery failure with the inherited driver
environment (exit 1) and successful discovery after omitting only that driver's
PSModulePath (exit 0). Neither probe ran the product or changed persistent settings.
The bounded fixture correction uses framework SHA-256 and OpenRead in the collector
instead of cmdlet auto-loading, with finally disposal. It does not change the
product environment, module-origin/hash predicates, required processes or cleanup.

Attempt 02 exited 0. It verified help/version, accepted fresh initialization,
administrative IPC, a running native cmd.exe PTY, accepted input of the synthetic
echo/exit commands, session status exited with exit code 0, and daemon lifecycle
stopped. All commands were bounded; output went to retained private files rather
than inherited EOF-sensitive pipe capture. No physical input or output-readback
claim is inferred from this administrative PTY check.

The product received only SystemRoot/WINDIR, System32 PATH/COMSPEC and fresh
TEMP/TMP/USERPROFILE/HOME/LOCALAPPDATA/APPDATA values. The definition's environment
allowlist was empty and cmd.exe used /D to disable AutoRun. Python and PowerShell
were external drivers; compiler, checkout, provider variables, diagnostic runtime
overrides and inherited startup/profile configuration were excluded from the
product environment.

## Loaded dependencies and cleanup disposition

The live snapshot bound three owned processes by exact copied executable/fixture
arguments and descendant identity: rt.exe (26 module records), OpenConsole.exe
(34), and cmd.exe (10). Their 70 module records represented 43 unique files.

- Three package-local files matched the pinned inventory: rt.exe, conpty.dll and
  OpenConsole.exe. The two Microsoft runtime files had valid Microsoft signatures.
  The unsigned rt.exe is the exact retained unsigned candidate, not a new artifact.
- Forty unique Windows files were loaded solely from the Windows directory.
  Independent post-snapshot hashes matched every captured hash. Every Windows
  file had a valid Microsoft catalog or Authenticode signature. No checkout,
  compiler/tool directory, provider path or undeclared external DLL was loaded.
- VCRUNTIME140.dll came from System32 with the inventory's exact hash
  d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066.
  Its mapped version remains 14.44.35211.0. No runtime install or downgrade occurred.

Normal shutdown left zero recorded processes, including after checking recorded
PID/UTC creation-time pairs independently of parent survival. Return-time process
inspection again found all three identities absent. The failed attempt's scoped
return inventory was also empty. No process was killed by name or forcibly stopped.
Both copied candidate packages, fixture databases, raw snapshots and logs remain
private and retained.

The [sanitized evidence](evidence/m12-windows-runtime-independence-20261001/README.md)
contains the reviewed result.json, failed attempt, exact script/bundle identities,
signature/hash disposition and numeric cleanup verification. Raw module paths,
account identities, shell output and databases are excluded. Microsoft signatures
support the dependency review; this is not a general OS security certification.

## Handover to the orchestrator

Consume the sanitized evidence and final runner by their hashes. Keep the failed
attempt and fixture-only correction mapped separately. Reconcile the current
Windows gate as passed under the explicitly permitted audited user-environment
definition, preserving the already completed runtime floor and physical evidence.
Do not demand another computer, account switch, build, performance round, full
battery or VM journey for this completed delta.

Keep platform artifact identities separate and do not close the retained Mac
macOS 14 execution, real-agent trial or global M12 acceptance. Reconcile newer
Mac/Linux queues without overwriting them. See the subsequent
[Mac delivery handoff](m12-windows-runtime-independence-mac-handoff.md) for the
authorized Windows result-branch delivery and remaining-check audit. Neither
document authorizes new CI/security/system actions, main merges or releases.
