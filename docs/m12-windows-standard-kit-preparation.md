# M12 Windows standard-account kit preparation

## Scope

Prepared on 2026-10-01 from the fetched b5f8865 handoff in a separate Windows
worktree. The original dirty checkout and retained validation artifacts remain
untouched. This verifies preparation only. Actual standard-account access,
interactive input, narrow/wide redraw and normal console restoration remain
pending. A second account on this host is not an independent clean installation.

## Local kit

The dedicated Public Documents kit contains the exact retained archive, complete
13-file candidate, unchanged observer, candidate inventory, hashed fixture
identity, double-click launcher, Spanish operator instructions and per-attempt
results. No binary was rebuilt or uploaded. No account, credential, system
protection, persistent execution policy, PATH or ACL was changed.

The package remains mapped to f2b3f6f446466f5b3657c3cd91746735cab59d78.
Executable SHA-256:
ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa.
Archive SHA-256:
039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50.
The [shared inventory](evidence/m12-windows-distribution-20260930/candidate-inventory.json)
binds all support files, notices, provenance and the actual System32 prerequisite.

[The wrapper](../scripts/run_standard_observation.ps1) rejects elevation and any
Administrators SID, including disabled membership reported by system whoami CSV.
It tests shared-folder write/read before launching the product, validates every
package/fixture hash, checks the mapped System32 VCRUNTIME140.dll and requires
interactive console handles. The launcher uses only system PowerShell with
process-scoped RemoteSigned, anchored and quoted paths, and a visible final pause.

Each invocation owns a fresh native-TEMP root. Only that root's single new fixture
can supply the original observation JSON. Raw state and observation remain private.
The shared result contains hashes, numeric outcomes, token classification,
automatically checked cleanup, and three separate operator confirmations.
An interrupted run remains in progress, never a pass. A 30-minute observer limit,
missing JSON, invalid binding, nonzero TUI exit or missing confirmation cannot pass.

Process snapshots bind fixture candidate paths and scoped arguments, then live
parent PID/creation-time relationships. Failure cleanup targets only those numeric
process identities after rechecking creation time. It never kills by name. Normal
cleanup must finish without forced termination to qualify the observation.
Numeric process identities are retained for return-time reconciliation.

## Verification

- Retained archive hash and source inventory passed before copying; all 13 copied
  hashes and the internal manifest contents passed afterward. Observer bytes match
  the checked-in b5f8865 source. Fixture bindings passed after correction.
- System32 VCRUNTIME140.dll matches the recorded hash and version 14.44.35211.0.
- Windows PowerShell syntax parsed without errors. The actual CMD launcher passed
  quoting/invocation checks and refused the filtered administrator token with exit
  1, standard_account=false and elevated=false, before product execution.
- The unchanged observer passed bounded PrepareOnly with exit 0 against the exact
  candidate. No process with that preparation fixture's scoped arguments remained.
  This is administrative fixture preparation, not physical observation.
- Six private synthetic collector controls passed using paths with spaces: valid
  readback, nonzero TUI exit, missing result, missing physical confirmation,
  observer timeout and wrong candidate binding. The positive control mocked token,
  console and observer data; it does not prove a real standard-account run.
- Existing Public Documents permissions provide inherited interactive access.
  No ACL was changed. Actual access is checked again in the standard-account run.

Retained preparation failures include a read-only lookup of the archive extraction
parent instead of its nested package, a dotted JSON-property binding update, and
a synthetic control's unavailable redirected console dimensions. All original
attempt folders, outputs and refusal results remain local. Corrected controls
passed; none of these attempts is relabelled as physical acceptance.

## Return and delivery

After the operator returns, inspect every shared attempt without selecting a stale
newest file. Require the kit identity, run-directory identity, standard token,
zero observer/TUI exits, all three physical confirmations, exact readback, unchanged
package and zero remaining recorded fixture processes. Keep failures and private
state. Review any cropped screenshot before public export.

Only then update the Windows report, current closure table and pending queue for
the actual passed scope. Publish sanitized verified results on the Windows branch
with a normal push and verify local/remote SHA equality. Main merge, PR, tag,
release and binary publication remain excluded.
