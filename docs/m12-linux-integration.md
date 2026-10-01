# M12 Linux continuation integrated on Mac

## Result and source mapping

Work branch `fix/m12-linux-integration` merges the shared Windows handoff at
`b5f886503865a67d6620de47fba3f4d2ed0ec357` and Linux continuation at
`bdedf7463f5688938b43f0ff82b125d94988d73b`. The integration source is
`4e138d28155060d3ed616d3f5f6aec058cf43b7d`. Subsequent documentation and evidence
exports do not change this tested source. No main merge, PR, tag, release or
binary upload is part of this delivery.

Both appended logbook histories are retained verbatim, in their original internal
order. The newer Windows standard-account handoff is preserved. The completed
Linux regression and affected Mac development-profile integration leave the
pending-only queue; global acceptance stays open.

The only Linux build delta is `[profile.dev.package.vt100] opt-level = 1`.
Dependency versions, lockfile, release settings, workloads and acceptance budgets
are unchanged. The reviewed manifest matches 6486da6. Cargo.lock SHA-256 is
`c2a0a6aaaef016e275323aed682313c73e28e1a104561c305ce74d3a49885ec8`.
The release artifacts below keep their original identities and are not rebuilt
or reattributed by this integration.

| Platform | Product source | Executable SHA-256 | Archive SHA-256 |
| --- | --- | --- | --- |
| Linux x86_64 | 6486da6c6005f366586f7ded036708bf3f6d3ccd | 248c36e33d25601cb2dcee5d8dc2760b881dded44c79845fd79d85f5ef027236 | e2e716d3549681b129fc8813c2e4ba7d62f97cadaef8764e96bf75b4b6e9797b |
| Mac arm64 | e712603cc753686675319358f0413946461249e2 | ef20a01284008741b03229f6aa8610403dc60205e3cd534df51cbef09e6b14a4 | 9fb3b5a7567a2682b9d27cd9fbd73bbc6a2142fbec2394958891a431b338d34a |
| Windows x86_64 | f2b3f6f446466f5b3657c3cd91746735cab59d78 | ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa | 039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50 |

The platform reports retain their package inventories, notices, features and
unsigned state. Windows additionally requires its exact bundled ConPTY helpers.
Hosted products are separate new artifacts, not substitutes for these packages.

## Imported Linux evidence review

All nine hashes in the public Linux SHA256SUMS matched. The supplied external
report matched the committed report byte for byte. Recomputing its four numeric
exports produced exactly four sections: three qualified and the original failed
debug section unqualified. Every recomputed sustained section matched the
published numerical evidence. Resource medians and descriptor counts were checked
independently. Original exit 1 and exit 101 attempts remain in the 104-command
ledger, including installer refusals; none is relabeled as a passing load gate.

Linux has no remaining execution in this requested Ubuntu distribution regression.
Its 205-test corrected workspace, exact-installed gates, reproducible artifacts
and unprivileged isolated runtime remain mapped to the
[Linux report](m12-linux-distribution-regression.md).

## Affected native Mac development checks

On the existing macOS 26.5.2 arm64 host, seven serial bounded commands passed at
4e138d2. The ordinary test environment removed CI and test/runtime override
variables. No compilation ran alongside the sustained or resource measurements.

| Check | Result | Elapsed seconds | Limit seconds |
| --- | --- | ---: | ---: |
| Formatting | Passed | 0.77 | 120 |
| Default workspace Clippy, all targets, warnings denied | Passed | 4.71 | 600 |
| All-feature workspace Clippy, all targets, warnings denied | Passed | 3.02 | 600 |
| TUI library tests | Passed | 8.21 | 300 |
| Locked offline workspace prebuild | Passed | 13.42 | 600 |
| Locked offline serial full workspace | 204 passed, 0 failed, 25 ignored | 388.86 | 1200 |
| Explicit ignored resource gate | 1 passed | 189.40 | 360 |

The workspace's hardening and TUI sustained inclusions qualified independently.
Their p95 navigation/echo pairs were 30.200/115.742 ms and 30.183/117.354 ms.
Each retained 100 samples per latency class and the original 120-second sustained
window, minimum rate, maximum sample gap and latency budgets.

The explicit resource gate retained 60 samples, the 180-second window, 512 MiB
per-process ceiling and 32 MiB plateau allowance. Daemon first/last steady medians
were 146112512/146767872 bytes, maximum 370835456. TUI medians were
21217280/21282816 bytes, maximum 22347776. One hundred reconnects included 20
abrupt detachments; daemon descriptors were 48 before and 48 after.

[Public numeric evidence](evidence/m12-linux-integration-20261001/README.md)
contains the allowlisted measurements, exact commands, limits and raw-log hashes.
Raw logs, private drivers and state remain outside Git. An initial resource export
used an incorrect warm-up/median interpretation. It was retained privately and
corrected before publication by matching the source's 20-sample warm-up and upper
median. Every corrected median matches the native assertion output; no measured
value or threshold changed.

## Exact retained Mac restricted runtime delta

The retained e712603 executable, copied into a fresh private runtime, passed
help/version, actual shell discovery, initialization, native IPC and three real
supervised sh PTYs. The profile enforces deny-by-default file contents and
execution allowlists, runtime-only writes and Unix IPC, while denying TCP.
Environment HOME/TMPDIR are synthetic; no provider credentials or user startup
files are supplied. Python drives the test outside the restrictions and is not a
runtime dependency. This uses macOS 26.5.2 on the existing host, not a VM or a
separate OS installation, and does not claim a fresh macOS 14 retained-package run.

Positive controls proved file reading inside/outside the restricted runtime and
TCP connectivity to a temporary owned loopback socket outside it. Negative
controls denied the external synthetic file, repository README, system/Homebrew
Python and Cargo execution, and the same loopback connection. All three supervised
shells independently wrote permitted denial markers for the external file.
The loopback socket closed after its controls. No persistent listener, firewall,
account, privilege or system policy changed.

A task created and transitioned to ready was captured in a live populated backup.
After orderly termination of only the owned sessions/daemon, restoring into a new
private home preserved complete task records, three session names/order, non-null
session/instance/agent-definition identities, launch snapshots and terminal sizes.
Restored former-live instances were correctly lost. Global revision/sequence
metadata advances on startup reconciliation and is not expected to equal the
pre-backup live snapshot. Complete coordination/handover history remains covered
by the earlier mapped runtime and exact-package regression gates; this bounded
delta does not claim a new real-agent trial or physical TUI observation.

Executable SHA-256 matched before and after. The actual private profile hash is
`59f4a582ad6b7d73bcdd3e11b4b329760a9de12b542bf0f90199ad04b91bf35d`.
A public parameterized profile, bounded command ledger and sanitized result are
included in the numeric evidence. The profile's RUNTIME_ROOT parameter replaces
only the owned private absolute path; it has a distinct published hash.

Eight preparatory attempts are retained privately. Attempts 1 and 2 aborted
because the system root directory entry was not readable, so their apparent
negative controls were invalid. Attempt 3 exposed a developer binary reaching
loader startup before its library read was denied; the final profile explicitly
allowlists executable paths. Attempt 4 had a fixture path substitution error.
Attempt 5 lacked native pseudo-terminal permission; attempt 6 lacked supervised
child signal permission. Its three shells were explicitly exited and its owned
daemon stopped through normal IPC. Attempts 7 and 8 compared changing global
revision/sequence metadata with durable task data; the final comparison checks
complete durable records and session metadata instead. Attempt 9 passes every
control and bounded workflow stage. No threshold or product code was changed.

## Current hosted verification

Both workflows were dispatched on 4e138d2, including the committed development
profile. Security completed successfully in
[run 36858094906](https://github.com/fpinero/relayterm/actions/runs/36858094906).
Quality completed successfully with all seven jobs in
[run 36858091674](https://github.com/fpinero/relayterm/actions/runs/36858091674).
The public hosted verification records the exact completed jobs and timestamps.
It includes Linux stable and pinned
Rust, macOS 14 and Windows Server 2022 native gates, plus all three release targets.

Hosted Mac runtime execution on macOS 14 establishes that environment for the
hosted executable. It does not turn the retained developer-host package into a
newly tested macOS 14 artifact or an independently installed OS. Windows Server
2022 execution does not substitute for Windows 10 desktop observation.

## Finite remaining acceptance boundaries

| Boundary | Reused evidence | Genuinely missing action |
| --- | --- | --- |
| Linux runtime | Exact 6486da6 product in unprivileged isolated Ubuntu 24.04, glibc 2.39, without network or build tools | None in this scope. Other distributions, musl and ARM64 are not claimed. |
| Mac runtime | Exact e712603 restricted runtime delta above, installation and package gates on macOS 26.5.2; existing 691a8fb full isolated journey; current hosted macOS 14 product | Reconcile oldest-runtime artifact mapping. Current restricted-runtime evidence is not a separate OS installation or a macOS 14 retained-package run; no new physical or SSH journey is required. |
| Windows runtime | Final f2b3f6f Windows 10 package and ordinary automation; earlier Windows 11 VM journey at its original candidate | Complete the final package's standard-account/real-console delta using the prepared kit. Map its bundled runtime to independent Windows proof; do not relabel the older VM package. |
| Private real-agent trial | Existing prepared synthetic workspace and implementation/test/review handover plan | Actual authorized agent execution with the operator. Version probes do not prove authentication or a successful trial. |
| Global sign-off | Existing M11 AC and phase evidence, current platform regressions and unchanged architecture/privacy/offline contracts | Reconcile the remaining installation, manual, runtime and trial deltas before declaring all 16 ACs complete. |

AC-1, AC-12 and AC-16 retain their M12 clean-install/runtime boundaries. AC-2,
AC-3, AC-4 and AC-8 reuse the completed named-terminal/SSH matrix with explicit
source impact assessment; the final Windows ConPTY console delta remains open.
AC-5, AC-6, AC-7, AC-9 and AC-10 retain coordination, loss/recovery and worktree
regression evidence. AC-11 retains architecture separation checks. AC-13 retains
privacy and security checks; AC-14 retains offline/SSH evidence; AC-15 retains
the unknown-CLI template gate. The private practical trial and final phase/AC
sign-off are separate pending gates. Passing automation is not a blanket M12
completion claim.

Prepare fixtures before any new operator observation. Reuse the
[Windows single-switch handoff](m12-windows-standard-account-handoff.md), which
keeps account creation and password entry with the operator and needs no Codex
installation in the standard account. Keep the corporate ACL case deferred and
separate. Publication requires a later explicit maintainer decision.
