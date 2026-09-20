# M12 native continuation instructions

## Shared contract

Continue from the fetched `feature/m12-windows-native` branch. Read AGENTS.md, PROJECT_VISION.md,
MVP_TECHNICAL_SPEC.md, README.md, TODO.md, docs/M12_details.md,
docs/m12-candidate-handoff.md, docs/m12-macos-manual-observations.md,
docs/m12-usability-verification.md, docs/release-builds.md, docs/install.md,
and docs/quick-start.md before execution. Communicate with the operator in
Spanish; write repository evidence and comments in English. Follow the pending
TODO and append-only avances workflow. Do not edit main or discard user changes.

The current production candidate source is
`0bc4b6384adf37dbdd9939e840d9c3e14e86d9e1`. It retains the handover correction
from `d760554` and fixes the minimum-width header by moving the product name to
the footer and reserving space for the full freshness label. Later evidence-only
commits do not replace that artifact identity. Inspect the source diff before
reusing artifacts. The original candidate at 491d5f1 has a confirmed handover
projection failure and must not be used to sign off the corrected workflow.

Read [the Windows observations](m12-windows-manual-observations.md) before
continuing. The prior Windows manual journey used d760554; its completed backup,
handover, and session checks are not physical proof of the new header. The new
header passed all 24 TUI unit tests, including every screen and all eight
freshness states at 80 by 24. The operator subsequently confirmed the header and
footer on Windows in Overview, Tasks, and Sessions. Physical header/footer
verification remains open on macOS and belongs in the Linux native journey. The
separate command-line clipping defect on writer acquisition remains open as
M12.WIN-SIZE. Do not describe this candidate as fixing that defect or freeze M12.

Build in clean isolated checkouts at the corrected source, outside the evidence
working tree. Inspect the checked-in release scripts and their help rather than
inventing flags. Use the pinned Rust toolchain and locked dependency graph; fetch
missing public dependencies before offline builds when necessary. Build twice
into separate fresh output directories, compare build records, package twice,
compare manifests, inspect archive inventory and native linkage, and run the
extracted smoke. Record exact commit, target, toolchain, native SDK/compiler,
OS, sizes, executable and archive SHA-256, features, and signing state. Retain
archives, manifests, checksums, and build records outside Git. Historical CI
archives were not retained and cannot be downloaded as though they were.

Install the extracted candidate in a fresh dedicated user-local directory with
the packaged helper. Preserve unrelated commands and existing installs. Verify
checksum, target, help, version, runtime prerequisites, actual command resolution,
and TUI startup. Run session_presentation, tui_gate, and backup_restore through
that exact installed executable using RELAYTERM_TEST_RT, not a Cargo substitute.
Run the appropriate source regressions and checks for any new code correction.

Guide the operator through small numbered steps, with one meaningful checkpoint
at a time. Keep platform preparation autonomous; physical terminal observations
must come from the operator. Use synthetic names and short disposable private
workspace paths. Never commit screenshots, raw transcripts, databases, private
paths, environment dumps, or personal details. Record assistant automation and
operator observation separately. A missing prerequisite or host is pending
work, not a passing result.

Execute the consolidated native observation and installed quick start: three
neutral sessions; distinct stable IDs and creation order; duplicate and cleared
names; ASCII, wide and combining Unicode form cursor; resize; two-client stale
rename rejection and two-step Ctrl-R review; competing input warning and explicit
writer transfer; detach, reattach, and client quit/reopen. At exactly
80 by 24, verify all six tabs, the complete freshness label, and the footer name
and shortcuts in Tasks and Sessions, then shrink below minimum and enlarge.
The task claim status is active, not in_progress. Observe competing claim rejection in Events when
Tasks has no inline warning. Input release is Ctrl-]; Ctrl-5 was not equivalent
on the observed Spanish Mac keyboard, so verify actual native keys.

The critical corrected journey is a claim from a task-neutral session, progress,
and Ctrl-S handover. Before any new claim, confirm CURRENT, handover_ready, no
owner, and the saved handover immediately, without needing refresh or restart.
Then select a different running session, claim, verify a different owner and
retained history, complete with d, and quit/reopen to verify done and CURRENT.
Read back task state and history independently. Session IDs and instance IDs
are different identities; task owners refer to instances.

Verify live backup and fresh-home restore. Obtain explicit operator agreement
before terminating sessions or stopping a daemon with termination. Restored
running sessions cannot adopt old PTYs and should be reported lost. Retain the
original backup and home. Verify collision refusal preserves a synthetic
unrelated command, and test scoped removal using an owned disposable install
with no daemon running from it. Do not purge state or alter global protections.

Write a platform-specific sanitized observation document and update the evidence
matrix and append-only logbook. Preserve the original failure and macOS impact
mapping. Keep TODO pending-only and leave global tasks open until all applicable
platform and SSH rows pass. Do not merge, tag, publish a release, delete branches,
or push without explicit operator authorization in the destination task. If
Windows and Linux run concurrently, use distinct feature branches and evidence
files; reconcile shared documents without overwriting the other platform's work.

## Windows task prompt

Continue Relayterm M12 native acceptance on this Windows host using the shared
contract in docs/m12-native-continuation.md. Work on an appropriate feature branch
based on the fetched feature/m12-windows-native evidence. Build the exact
candidate source 0bc4b6384adf37dbdd9939e840d9c3e14e86d9e1 natively for
x86_64-pc-windows-msvc; do not use WSL as Windows evidence. Inspect the actual OS,
architecture, terminal, PowerShell and cmd.exe versions and MSVC toolchain.
Verify the Visual C++ x64 runtime dependency from the candidate's PE imports.
Do not install system prerequisites or disable reputation protections without
following the operator's authorization. Record actual warnings and prerequisites.

Guide native terminal observations in PowerShell and verify discovery and
absolute invocation in cmd.exe as documented. Use neutral native shell child
commands from the quick start. Verify real ConPTY behavior and terminal recovery,
not only subprocess output. Record results in docs/m12-windows-manual-observations.md.
Complete every available native step, fix and empirically retest in-scope defects,
and report only concrete missing evidence. Keep Linux and SSH rows pending unless
independently completed. Finish with tested artifact identities, checks performed,
remaining limitations, and a focused evidence commit ready for review.

## Linux task prompt

Continue Relayterm M12 native acceptance on this Linux host using the shared
contract in docs/m12-native-continuation.md. Work on an appropriate feature branch
based on the fetched feature/m12-windows-native evidence. Build the exact
candidate source 0bc4b6384adf37dbdd9939e840d9c3e14e86d9e1 natively for
x86_64-unknown-linux-gnu. Inspect actual distribution, architecture, terminal,
shell, compiler, ELF interpreter, shared libraries, and glibc requirements.
Ubuntu 24.04 is the declared baseline; do not silently generalize another host
into baseline proof. Verify native PTY and terminal recovery through the installed
candidate and record results in docs/m12-linux-manual-observations.md.

Also perform the focused SSH row in docs/m12-usability-verification.md if an
already authorized SSH service and connection are available. Ask only for missing
access or explicit service-change authorization; do not enable, expose, or
reconfigure SSH automatically. Repeat Unicode cursor, stale rename review,
competing-input transfer, connection loss, reconnection, stable session identity,
and terminal recovery with the same candidate. Keep hostnames and credentials
out of public evidence. If SSH is unavailable, finish independent native work
and leave that row explicitly open. Finish with tested artifact identities,
checks performed, remaining limitations, and a focused evidence commit ready
for review. Do not mark Windows or global M12 acceptance complete.
