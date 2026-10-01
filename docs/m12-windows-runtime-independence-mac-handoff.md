# M12 Windows runtime handoff to the Mac orchestrator

## Delivered scope

Consume origin/fix/m12-windows-standard-observation. The branch retains the
standard-account evidence at e250ee5 and its earlier orchestration handoff at
ff7cc8097479469826b00bde60b20b9bdf10fa9c. This continuation adds the completed
automatic runtime independence audit, its bounded fixture correction and reviewed
sanitized evidence. It changes no product, dependency, retained binary or budget.

Read the [runtime report](m12-windows-runtime-independence.md),
[evidence directory](evidence/m12-windows-runtime-independence-20261001/README.md),
[ordinary distribution report](m12-windows-distribution-validation.md),
[standard-account observation](m12-windows-standard-observation.md), current
closure audit, acceptance matrix and append-only log. The final
[runner](../scripts/check_windows_runtime_independence.py) and
[synthetic tests](../scripts/test_windows_runtime_independence.py) are included
for source review; do not execute the native runner on Mac as runtime evidence.

## Verified runtime result

The complete unchanged 13-file package maps to product source
f2b3f6f446466f5b3657c3cd91746735cab59d78, with rt.exe SHA-256
ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa.
The existing archive hash remains provenance only for this new audit; no archive
rehash or rebuild is claimed. Original/copied package hashes were checked before
and after automatic execution in fresh allowlisted profile/TEMP/project/state.

Final runner SHA-256:
5f217292a2d533e5b35cebcdb6a02ff5d719b4d054a33a43530ff6cd398c95a5.
Generated Windows collector SHA-256:
60cdac0e9b9dee6b7b6fc15b52c0bd092408d5e6430ad699f570e3e404244dbf.
Source hashes identify the executed fixture separately from product identity.

Python was 64-bit. Nine synthetic tests ran, eight passed and the non-Windows
execution control was explicitly skipped. Original/corrected generated PowerShell
collectors passed native syntax parsing. The first automatic attempt failed at
module_inventory: inherited PSModulePath prevented Get-FileHash discovery by
system Windows PowerShell. Driver-only probes reproduced that failure. The
collector now uses framework SHA-256 instead of cmdlet auto-loading, preserving
every module-origin, module-hash, process and cleanup predicate. The failed
attempt's accepted owned-daemon shutdown and empty scoped return inventory are
retained. Neither failure nor correction changed system protections or the product.

The second automatic attempt passed with help/version, accepted initialization,
IPC, one real cmd.exe PTY, input/normal session exit 0, orderly shutdown and zero
remaining recorded process identities. It used a non-elevated filtered development
token, standard_account=false. Reuse the separate actual standard-account pass;
do not relabel this token. Forty Windows files and three package files represented
43 unique loaded files, from 70 records across three owned fixture processes.
Every post-snapshot hash matched. All 40 Windows files and both packaged Microsoft
runtime files had valid Microsoft signatures; rt.exe remains the pinned unsigned
candidate. System32 VCRUNTIME140.dll matches its declared hash/version. No undeclared
DLL, developer checkout/tool path or inherited product configuration was used.
PID/creation-time return checks found all three fixture identities absent.

The plan permits an audited user environment for independence. Another computer
or Windows installation is not required. The exact Windows 10 22H2 floor,
10.0.19045.0, was already proven and its earlier journey was not repeated.
This is not a filesystem/security sandbox or global M12 acceptance.

## Remaining Windows check audit

| Windows item | Disposition and next action |
| --- | --- |
| Declared oldest Windows runtime | Exact mapped 19045 evidence exists. Reuse it; no new floor journey is needed. |
| Actual standard account and real console | Passed with exact package, automatic readback/cleanup and operator confirmations. Reuse input/redraw/restoration proof. |
| Development-environment independence | Now passed with reviewed loaded dependencies and fresh product configuration. Consume this result. |
| Ordinary native quality, sustained budgets and helpers | Already passed at the source/package mappings in the ordinary distribution report. Historical failures retain their scope; do not repeat measurements or infer a global waiver. |
| Package, install/discovery/collision/removal and runtime negatives | Already passed in the ordinary distribution report. No new product change invalidated them. |
| Original VM initialization response-capture gap | Keep the old gap historical. The current f2b3f6f audit captures accepted initialization as its own mapped proof; do not reattribute it to the older VM binary or repeat the VM journey. |
| ACL/1355 diagnostic and proposed correction | Still separate pending engineering/evidence work. The requested instrumented handle-based diagnostic is not present in this delivery, and the actual blocked host condition is not available in the successful audited environment. Corporate execution remains deferred. Native product initialization success does not reproduce or explain 1355; synthetic injection cannot establish its cause. A correction requires diagnostic evidence, an explicit engineering scope and a newly mapped candidate/regression plan. |
| Cross-platform/hosted acceptance and real-agent trial | Not closed by Windows. The orchestrator owns the next bounded step and any separately authorized CI or provider interaction. |

No further outstanding automatic runtime test for the retained Windows package
is executable in this bounded continuation without repeating completed evidence
or opening that separate ACL/product engineering scope. This is not a declaration
that every Windows roadmap item is complete. The ACL investigation remains pending,
as do global source/AC/phase reconciliation and the applicable real-agent trial.
No corporate access, forced reproduction, security change or speculative product
correction was attempted.

## Evidence consumption and next orchestration step

Fetch the branch while preserving newer Mac/Linux work; use a separate worktree
if necessary. Verify the public manifest in a clean checkout:

```sh
cd docs/evidence/m12-windows-runtime-independence-20261001
shasum -a 256 -c SHA256SUMS
```

The public files are result.json, failed-attempt-01.json, module-review.json,
cleanup-review.json and fixture-identities.json, with canonical LF and no BOM.
Keep raw logs, native paths, accounts, original states and binaries out of public
integration. Do not use whole-file Windows TODO/audit copies to overwrite newer
Mac/Linux conclusions or append-only entries. Preserve the initial PSModulePath
failure and all earlier source/artifact distinctions.

Close only the mapped Windows independence delta. Decide the next actually
missing cross-platform proof from the latest orchestrator state. Retained Mac
binary execution on macOS 14 and the real-agent trial are not closed here.
The user requested returning control to the Mac orchestrator after delivery.
The authorized action here is this sanitized Windows-branch push, not a main merge,
PR, tag, release, binary publication, new CI dispatch or message to another chat.
