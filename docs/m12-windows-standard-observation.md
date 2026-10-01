# M12 Windows standard-account observation

## Result and scope

The mapped f2b3f6f Windows package passed the bounded standard-account and
real-console observation on 2026-10-01. This closes the Windows operator delta
only. The account is a dedicated existing local standard account on the same
Windows 10 development host, OS version 10.0.19045.0. It is not an independent
clean machine, a new Windows 11 journey or proof of another runtime floor.
Global M12 acceptance remains open.

The exact executable SHA-256 remains
ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa.
The archive SHA-256 remains
039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50.
No rebuild, replacement executable or binary upload was performed. See the
[ordinary distribution report](m12-windows-distribution-validation.md),
[kit preparation](m12-windows-standard-kit-preparation.md) and
[sanitized evidence](evidence/m12-windows-standard-observation-20261001/README.md).

## Automatic verification

Successful run ff9e4a7c74994bdf8f943801881da33a used the unchanged observer at
b5f886503865a67d6620de47fba3f4d2ed0ec357 and the wrapper/launcher identities
prepared at fe726a8dbdd27551fc63abddc32f340371afbf30. The per-run result binds
their file hashes separately from the product source. It records:

- Shared result-folder write/read passed under the actual standard token.
- standard_account=true, elevated=false and no Administrators SID, including
  disabled membership, from the system whoami CSV inspection.
- Package and fixture integrity passed before launch and package integrity after
  use. The actual System32 VCRUNTIME140.dll matches the recorded prerequisite.
- The unchanged observer completed with exit 0 and the actual TUI exited with 0.
  Its required administrative commands returned accepted JSON and exit 0 before
  its final observation file was written. These outcomes are derived from the
  unchanged observer's checked command contract and exact invocation readback.
- Readback used the single new fixture in that invocation's fresh native-TEMP
  root. The raw observation hash is retained; its private-state path is excluded
  from shared/public evidence.
- Normal cleanup completed with zero recorded fixture processes and no forced
  cleanup. Numeric PID/creation-time identities bind seven observed candidate
  and descendant processes to the synthetic fixture.

Return-time verification recomputed all 13 candidate file hashes, all four fixture
file hashes and the retained archive hash. Every hash matches the prepared kit and
reported identities. All seven recorded PID/creation-time identities were absent;
no rt.exe or OpenConsole.exe process remained in the process inventory. Unrelated
PowerShell consoles and the standard-account desktop session were left open.
No additional process termination was needed or performed.

The observer itself launches using system Windows PowerShell. The successful
wrapper's parent shell version was not recorded, so the result is not attributed
to a particular parent-shell version or to the preceding CMD attempt.

## Physical confirmations

The operator explicitly confirmed all three per-run prompts:

- Typed input produced the synthetic M12-RUNTIME-OK marker correctly.
- Narrower and wider redraw and fresh output remained usable.
- Normal exit restored the outer console without reset or forced closure.

The operator subsequently reported successful checks after returning to the normal
account. Console dimensions recorded before and after were 144 by 32. Those
dimensions alone do not prove redraw or restoration; the physical confirmations
provide that evidence. No screenshot was supplied or inferred. Names/order/cursor,
conflict, full SSH, VM and sustained-performance rounds were not repeated.

## Retained failures and boundaries

Two earlier development-account preflight attempts were correctly refused with
standard_account=false and elevated=false. They are expected refusal controls,
not standard-account runs.

Standard-account attempt 1a4b25886cf04a6e9ffac6a5597c91da failed at integrity,
diagnostic -2146233087, before observer or TUI launch. Its sanitized result and
original shared attempt remain preserved. The recorded stage and generic numeric
diagnostic do not establish the precise cause. The successful invocation did not
erase or reattribute that failure; return-time hashes match exactly. This result
does not claim that every invocation of the launcher succeeded.

The operator also attempted to launch the parameterized observer directly without
knowing its required arguments. That manual invocation has no qualifying result.
The successful wrapper already supplied those arguments and executed the unchanged
observer, so another physical run or account switch is unnecessary.

Raw synthetic state and the original operator-observation.json remain in the
standard profile. Public evidence contains only reviewed hashes, numeric outcomes,
synthetic run IDs, classifications and explicit physical confirmations. No account
names, native paths, credentials, transcripts, binaries or screenshots are exported.
No account, system protection, persistent policy, PATH or ACL was changed.

Affected native Linux/hosted gates, runtime-floor/independence reconciliation,
the practical agent trial and global distribution/acceptance remain governed by
the [current closure audit](m12-closure-audit.md). No global latency waiver follows.
The [Mac orchestrator handoff](m12-windows-standard-observation-mac-handoff.md)
provides the source mapping, evidence inventory, checksum procedure and bounded
reconciliation prompt for the three-platform continuation.
