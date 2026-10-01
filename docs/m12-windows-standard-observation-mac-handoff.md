# M12 Windows observation handoff to the Mac orchestrator

## Delivery and source mapping

Fetch origin/fix/m12-windows-standard-observation. Verified Windows evidence is
available at e250ee5a5d160b77add2c1f7c716b1b133a6b0d0; subsequent handoff-only
commits do not change the product or invalidate that observation. The branch
descends from b5f886503865a67d6620de47fba3f4d2ed0ec357 on
origin/fix/m12-final-validation. The original dirty Windows checkout and all
private prior evidence were preserved in place.

| Identity | Revision or SHA-256 |
| --- | --- |
| Retained Windows product source | f2b3f6f446466f5b3657c3cd91746735cab59d78 |
| Unchanged observer source | b5f886503865a67d6620de47fba3f4d2ed0ec357 |
| Prepared wrapper/launcher source | fe726a8dbdd27551fc63abddc32f340371afbf30 |
| Observation/report commit | e32f186341247a0fadc54e3d4d2d9e081be8f15f |
| Portable evidence-manifest correction | e250ee5a5d160b77add2c1f7c716b1b133a6b0d0 |
| rt.exe | ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa |
| Archive | 039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50 |
| conpty.dll | 39fba2713e2495117b1591ae8c32a3b904bea7aa66069cf7815e2844c76d75d8 |
| OpenConsole.exe | b7fd936c2668b87b9ecf7b3366dc6568afc1c6f981874cba3e955a1c35cf8160 |

No binary was rebuilt or uploaded. No production, dependency or Rust test source
changed in this Windows continuation. The new PowerShell/CMD scripts are local
observation fixtures, not product changes or scripts to run on macOS.

## Files to consume

| Repository file | Purpose |
| --- | --- |
| [Observation report](m12-windows-standard-observation.md) | Actual scope, automatic facts, physical confirmations, retained failures and conclusions. |
| [Evidence directory](evidence/m12-windows-standard-observation-20261001/README.md) | Public-safe JSON results, attempt classification, return verification and portable hashes. |
| [Preparation report](m12-windows-standard-kit-preparation.md) | Launcher/token/access controls and explicitly synthetic preparation checks. |
| [Ordinary distribution report](m12-windows-distribution-validation.md) | Original native/package/performance/helper evidence, updated with the bounded operator result. |
| [Candidate inventory](evidence/m12-windows-distribution-20260930/candidate-inventory.json) | Complete 13-file runtime package identity and System32 prerequisite mapping. |
| [Current closure audit](m12-closure-audit.md) | Windows operator delta closed; global gates retained. |
| [Acceptance matrix](acceptance-matrix.md) | Updated Windows checkpoint, without global sign-off. |
| [Pending queue](../TODO.md) and [append-only log](../avances.md) | Remaining work and verified task/provenance records. |
| [Observer](../scripts/observe_windows_package.ps1), [wrapper](../scripts/run_standard_observation.ps1) and [launcher](../scripts/run-observation.cmd) | Reviewable capture and classification contracts, no Mac execution required. |

The evidence directory contains observation.json, failed-integrity-attempt.json,
attempt-index.json, return-verification.json and SHA256SUMS. Original shared
attempt byte hashes are provenance fields in the index. Public JSON uses canonical
LF without BOM; its committed hashes are in SHA256SUMS. The first post-push blob
check found CRLF-normalization mismatches, which e250ee5 corrected. Keep that
correction and its failure history rather than using the earlier manifest.

After obtaining the files in a clean worktree, verify them on macOS without
executing any product or Windows fixture:

```sh
cd docs/evidence/m12-windows-standard-observation-20261001
shasum -a 256 -c SHA256SUMS
```

## Verified conclusions and limits

Successful synthetic run ff9e4a7c74994bdf8f943801881da33a records an actual
standard account on the same Windows 10 host, OS 10.0.19045.0. The account
already existed. Shared-folder write/read, token inspection excluding even
disabled Administrators membership, package/fixture hashes and the actual
System32 runtime prerequisite passed. The unchanged observer and actual TUI
both exited 0. Readback was bound to this invocation's fresh private fixture.

The operator explicitly confirmed typed M12-RUNTIME-OK input, usable narrower/
wider redraw with fresh output and normal outer-console restoration. Numeric
144 by 32 dimensions before/after are supporting measurements, not a substitute
for those confirmations. No screenshot was supplied or inferred. Direct manual
observer launch without its mandatory arguments did not qualify as evidence;
the successful wrapper had already run it automatically with those arguments.

All 13 package hashes, four kit-file hashes and the archive hash matched on return.
All seven recorded fixture PID/creation-time identities were absent, and no rt.exe
or OpenConsole.exe remained. Normal cleanup required no forced termination; the
returning agent killed no process. Unrelated shells and the standard-account
desktop session were left open. Private state, raw observation and every attempt
remain local, outside Git.

Retain these separate unsuccessful boundaries:

- Two expected development-account refusals, standard_account=false and
  elevated=false, prove the filtered-administrator preflight control only.
- Standard-account run 1a4b25886cf04a6e9ffac6a5597c91da failed at integrity with
  diagnostic -2146233087 before the product ran. The available stage and generic
  diagnostic do not establish the precise cause. Do not claim every launcher
  invocation passed or reinterpret this attempt as a product runtime failure.
- The successful wrapper's parent-shell version was not recorded. The observer
  launch itself was pinned to system Windows PowerShell. Do not attribute the
  pass to a specific parent shell or to the preceding unsuccessful invocation.
- Preparation lookup, binding and synthetic redirected-console failures remain
  in the preparation report and private records. Synthetic collector controls
  are not real account or physical evidence.

This closes the mapped Windows operator delta only. A second account on the
developer host is not an independent clean Windows installation or new runtime-floor
proof. Do not infer fresh hosted coverage, Linux success, a new VM journey,
global latency waiver, practical agent-trial completion or global M12 acceptance.
Affected runtime-floor/independence, Linux/hosted and global reconciliation remain
with the orchestrator. Corporate-host probes remain deferred. No completed physical,
performance, SSH, VM or complete platform battery was repeated here.

## Bounded reconciliation for the orchestrator

Inspect Git status and preserve current Mac/Linux work before fetching the Windows
branch. Use a separate clean worktree if needed. Compare branch ancestry and its
change list to the b5f8865 base, verify the evidence hashes and review the result
predicates against the observer/wrapper source. Consume the result without running
Windows scripts on Mac or replacing the retained package.

Reconcile only the passed Windows scope into the latest three-platform audit.
The branch's TODO/audit files are a Windows checkpoint based on b5f8865, so do not
overwrite newer Linux/Mac conclusions or log entries with whole-file copies.
Preserve append-only history and pending-only tasks. Reuse current platform reports
and explicit artifact/source mappings; source/report descendants do not reassign
ancestor binary measurements. Address only actually missing proof.

This handoff requests read/reconciliation work. It does not authorize main merges,
PRs, tags, releases, binary uploads, new hosted test dispatches, security changes
or corporate-host access. Any other existing operator authorization must be assessed
separately, rather than inferred from this document.

## Copy-paste prompt

```text
Reconcile the completed Windows M12 standard-account observation into your
three-platform orchestration. Fetch origin/fix/m12-windows-standard-observation,
preserving all local Mac/Linux changes and using a separate clean worktree if needed.
Verified evidence is at e250ee5; later handoff-only commits do not change its scope.

Read AGENTS.md, relevant CLAUDE.md, PROJECT_VISION.md, MVP_TECHNICAL_SPEC.md,
README.md, TODO.md, latest avances.md and
docs/m12-windows-standard-observation-mac-handoff.md. Follow its file inventory
and verify the canonical LF evidence manifest before consuming the result.

The unchanged f2b3f6f Windows package passed actual standard-account token/access,
exact-invocation readback, observer/TUI exit 0, normal fixture cleanup and explicit
operator confirmations for input, narrow/wide redraw and console restoration.
Return-time package/fixture/archive hashes matched and all seven recorded fixture
process identities were gone. Preserve the integrity-stage failed attempt, filtered
administrator refusals, unavailable failure cause and parent-shell-version boundary.

Close only the mapped Windows operator delta. This account is on the same developer
host, not an independent clean installation or new runtime-floor proof. Keep product,
package, observer, fixture and report identities separate. Reconcile against your
latest Linux/Mac evidence without overwriting newer queues, conclusions or append-only
log entries. Do not rebuild, rerun Windows scripts on Mac or request another completed
physical/performance/SSH/VM round. Identify only genuinely missing global proof.

No main merge, PR, tag, release, binary publication, new hosted dispatch, corporate
probe or system-security change is authorized by this handoff. Communicate in Spanish;
write public documentation and code comments in English. Report the updated bounded
acceptance conclusions and remaining gates without declaring global M12 complete.
```
