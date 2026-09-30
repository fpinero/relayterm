# M12 Windows distribution continuation

## Objective and scope

Continue on the non-corporate Windows development computer. Implement and verify
the Windows distribution work that remains after the successful experimental
ConPTY evaluation. Complete all possible autonomous engineering and checks before
presenting one consolidated operator checklist. Diagnose and fix in-scope Windows
failures locally; do not use Mac as an intermediary for routine Windows fixes.
Communicate with the operator in Spanish. Write documents, scripts and comments
in English without Unicode U+2014. M12 remains open until cross-platform and
global acceptance evidence has been reconciled.

The delivery branch is `fix/m12-final-validation`. The accompanying transfer
manifest or operator prompt identifies the exact handoff commit. Product source
verified on Mac is `6e573326180acbbc5fdc6c706a610c27d22eda31`; the handoff adds
only documentation. Verify this relationship with Git before using the evidence.

Read repository instructions and these documents completely before implementation:

- [Project vision](../PROJECT_VISION.md), [technical specification](../MVP_TECHNICAL_SPEC.md) and [README](../README.md).
- [Pending work](../TODO.md) and the latest entries in [the logbook](../avances.md).
- [Closure audit](m12-closure-audit.md).
- [ConPTY reconciliation](m12-conpty-evaluation.md).
- [Integrated Mac verification](m12-macos-integrated-validation.md).
- [Release build contract](release-builds.md).
- [Experimental adapter instructions](../scripts/conpty_evaluation/README.md), its preparation/runner scripts and runtime patch.
- [Sustained harness contract](m12-sustained-performance.md) and [Windows historical failures](m12-windows-sustained-reconciliation.md).

The previous Windows five-pass experiment and Mac passes are evidence for their
mapped source/runtime, not proof that the current ordinary Windows archive works.
The final integrated reader is 16 KiB on Windows and 64 KiB elsewhere. Preserve
the Unix guard and TUI fix. Do not reapply the earlier evaluation patch bundle.

## 1. Preserve and reconcile the Windows checkout

Inspect branch, tracked and untracked changes, local commits and previous private
artifacts first. Fetch the delivery branch and verify the exact handoff commit.
Do not pull over local experimental changes, reset, clean, stash or overwrite them.
Prefer a new clean local clone or suitable isolated worktree if the earlier
checkout contains evaluation changes. Compare old work with the integrated source;
retain unique useful changes without duplicating already integrated patches.
Use an appropriate `fix/` work branch, never main/master.

Record native OS edition/build and architecture, compiler, source, dependency and
runtime identities privately. Verify the automation account/token and whether it
is an actual standard account or a non-elevated token of an Administrators member.
A sandbox account or filtered administrator token does not establish an
interactive standard-account journey. Do not dump account names or machine data
into public documentation.

Plan changes in TODO.md before implementation and append only verified outcomes
to avances.md. Preserve every failed or blocked attempt, with its own identity.

## 2. Integrate the runtime into an ordinary candidate

Start from the pinned, already evaluated Microsoft runtime and adapter, rather
than upgrading dependencies opportunistically. The evaluation uses
Microsoft.Windows.Console.ConPTY 1.24.260710001 and portable-pty-psmux 0.9.7.
Use the hashes, Microsoft signature checks and ABI reference in prepare.py.
Downloads of the pinned inputs into isolated project-local/private artifact
storage are part of preparing the candidate; do not install a global runtime,
replace system DLLs or change persistent host configuration.

Make and document a conservative engineering decision for an ordinary x64
package, then implement it and test it locally. Address:

- Locked, reproducible adapter/dependency integration without an external private
  Cargo path override. Review the bounded DA1 response, input-handle lifetime,
  exported function names and flags against the pinned header.
- Package-local runtime discovery anchored to the executable, independent of the
  current directory and ambient PATH. Define and test trust/integrity checks for
  conpty.dll and OpenConsole.exe, including dependent DLL lookup and helper launch.
  Never search an unrelated writable directory or silently accept altered inputs.
- Explicit failure policy for missing, corrupt, incompatible or untrusted runtime
  files. Preserve actionable numeric diagnostics. Do not silently fall back to an
  unqualified system backend and then claim sustained acceptance.
- Both runtime files, licenses/notices, hashes and provenance in the declared
  archive manifest/inventory. Update archive inspection, extraction, installation,
  collision handling and removal tests coherently. The existing exact nine-file
  inventory and single-executable installer are not sufficient for a bundled
  runtime. Keep owned files together and preserve unrelated files and state.
- Explicit version/update ownership with no runtime network download or automatic
  updater. Review upstream redistribution terms from the package and official
  sources. If redistribution permission cannot be established, report the precise
  blocker rather than inventing permission.
- Windows x64 and the declared OS support floor only. Do not claim ARM64/x86 or
  narrow the supported OS contract silently. Reconcile documentation/specification
  claims about a single executable with any approved package-local support files.
  Keep the domain, daemon and clients separate; preserve Mac/Linux behavior.

Choose routine reversible implementation details autonomously. Record significant
architecture choices and their tests. Do not change security policy, trust stores,
accounts, antivirus exclusions, timer settings, or global PATH to make tests pass.
If a prerequisite needs elevation or a system installation, collect the exact
requirement and official installer/source for the final operator checklist.
Continue independent work first. Do not treat private copies of VCRUNTIME140
inside unrelated tools as proof of the system x64 redistributable.

## 3. Run autonomous native checks

Keep raw output in a new private directory outside tracked source. Automate the
sequence with logs, timestamps, source/lock/runtime/binary hashes, exit codes,
bounded deadlines and final status. Stop dependent stages on a failed prerequisite,
continue independent checks, and fix in-scope defects before retesting. A failed
attempt must remain in the report; do not repeat blindly until a green result.

Run formatting, default and all-feature/all-target linting, applicable tooling
unit tests and the complete locked workspace tests. Exercise native PTY, IPC,
permissions, daemon lifecycle, TUI refresh/input, coordination, backup/restore,
worktrees and affected fault/security regressions. Add proportionate tests for
runtime discovery, integrity, wrong architecture, missing helper, installation
collisions, rollback and owned removal. No elevated execution for acceptance.

Prebuild all relevant default production profiles and test harnesses before
performance measurement. Run serially with CI, RELAYTERM_TEST_RT, experimental
runtime selection and reader diagnostic/delay variables absent for ordinary native
acceptance. Verify which runtime actually loads; its presence beside rt.exe is
not sufficient evidence. Test hooks are diagnostic only, never release features.

Execute two debug and two release sustained runs in debug/release/debug/release
order, followed by the debug hardening inclusion. Preserve the unchanged harness:
minimum 2 MiB/s consumed in every required interval/window, at least 120 seconds,
100 navigation samples, 100 echo samples, sampling-gap bounds, 100 ms navigation
p95 and 250 ms echo p95 on the reference native environment. Use 360-second
external limits for each focused sustained run. A load failure invalidates latency
acceptance. Recompute numerical evidence independently from observations.

Run the explicit ignored resource gate and extend the process inventory to include
OpenConsole and any other owned runtime helpers: separate and aggregate memory,
handles, process count and lifecycle, including reconnects, abrupt exits and daemon
shutdown. Retain the original resource limits; do not hide helper consumption.
Report how helper accounting maps to the existing budget, and leave acceptance
open if that mapping is unresolved. Terminate only positively identified synthetic
owned processes after timeouts, never unrelated processes by generic name.

Use the actual test selectors from the current source, including:

```text
cargo test -p relayterm-cli --test tui_gate --locked sustained_load::sustained_output_navigation_and_echo_meet_declared_contract -- --exact --nocapture --test-threads=1
cargo test -p relayterm-cli --test hardening_gate --locked tui::sustained_load::sustained_output_navigation_and_echo_meet_declared_contract -- --exact --nocapture --test-threads=1
cargo test -p relayterm-cli --test resource_runtime_gate --locked sustained_output_memory_and_reconnect_resources_are_bounded -- --ignored --exact --nocapture --test-threads=1
```

Add --release for the release profile and --offline when all locked inputs are
available. The experimental run.ps1 always selects the experimental runtime, so
it must not be reused unchanged to certify the ordinary package.

## 4. Build and test the actual package

Use existing release tooling, updated for the reviewed Windows inventory. Its
clean-source rule must remain intact. A local verification commit in an isolated
work branch/clone is permitted for reproducible builds after reviewing and
sanitizing the exact diff. Record its SHA and do not push it automatically.
Do not include raw evidence, private paths, binaries or retained artifacts in Git.

Produce two clean native x64 production builds and archives from identical inputs
and separate build directories. Compare bytes, inspect PE imports/architecture,
validate notices and exact inventory, and record SHA-256 values. No test hooks or
absolute private dependency paths may remain. Do not substitute an untested build.

Extract the actual archive and run constrained-PATH smoke, clean isolated install,
collision/reinstallation rejection preserving bytes, help/version/init, synthetic
sessions, TUI, worktree and populated backup/restore gates. Set RELAYTERM_TEST_RT
only in those extracted-package test children and record that they measure a
release product through a test harness. Test sustained acceptance through this
exact packaged executable as well as native debug/release gates. Remove experimental
runtime variables; prove loading from the installed candidate. Test owned removal
and preserve foreign markers and private workspace state.

Use RemoteSigned only in dedicated PowerShell processes if needed, with no
persistent execution-policy change and no blanket file unblocking. Do not weaken
Group Policy or security controls. Identify runtime prerequisites honestly and
separate an absent prerequisite from an application failure.

## 5. Ask the operator last

After all autonomous work, present one short ordered list containing only missing
human actions. Each item must identify the exact candidate/hash, terminal/account,
command or keystrokes, expected observation and evidence to return. Prepare scripts
for command-heavy steps and synthetic test data beforehand. Keep secrets and
personal folders out of screenshots; request only the relevant application area.

Potential items, only if not already established by current evidence:

- A prerequisite requiring explicit operator installation/UAC, with official source
  and verification steps. Resume blocked automated checks after it is satisfied.
- Launching the installed candidate from an actual interactive standard account
  when automation cannot demonstrate that account/token context. Do not create
  or modify accounts automatically.
- A minimal real-console observation affected by the packaged runtime, such as
  input, redraw/resize, normal exit and shell restoration. Reuse prior physical,
  SSH and Windows 11 VM evidence where behavior is unaffected. Do not repeat the
  entire historical functional journey just to obtain new screenshots.

Screenshots support visual observations, not numerical latency or throughput proof.
Do not ask for provider credentials or launch paid/real agent tasks for these
synthetic acceptance checks. Do not access the corporate laptop or investigate
its security tooling. Its invalid_location failure remains separate and unresolved;
no FortiClient causality or ACL/reputation root cause is established here.

## 6. Deliver a reviewable result

Update public-safe repository documentation, TODO.md and append-only avances.md.
Prepare outside tracked source:

- report.md: source/runtime/artifact identities, changes, exact checks and outcomes,
  all failures, account/token scope, limitations and remaining manual items.
- numerical-evidence.json or .md: every sustained observation, interval delta,
  producer count, latency sample and helper-resource observation, with a verifier.
- MAC-HANDOFF.md: changed files, compatibility implications, exact baseline and
  final source mapping, and affected Mac/Linux checks to run next.
- APPLY.md plus patches or a Git bundle sufficient to reproduce all relevant
  local commits and remaining tracked/untracked source additions. Verify applying
  to the exact delivered baseline in a clean disposable checkout. Include hashes
  and no raw private logs, runtime databases, credentials or host inventories.
- A separate reviewed candidate artifact inventory; keep actual packages private
  and do not publish or upload them automatically.

Do not push, merge main, open a PR, tag or publish a release without a subsequent
explicit operator request. Do not close M12 globally. Do not claim a Windows 11
run if the actual machine reports Windows 10, or a clean-machine/OS-floor test
from a development host. Report completed automation first and the short remaining
operator checklist last, then continue from the operator's observations.
