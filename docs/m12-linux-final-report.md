# M12 Linux final validation report

Current continuation: [Linux final delta only](m12-linux-final-delta-continuation.md).
The original decision and findings below are historical. Read the subsequent
diagnosis and completed physical correction retest before scheduling any work.
Only later presentation changes justify new local physical observation; do not
repeat the original journey or the completed 15794ad round.

## Decision

The required Linux observation attempts and authorized focused SSH journey are
complete for this validation round. Acceptance is not granted: interrupted SSH
terminal restoration failed, the documented detach shortcut failed, and other
UI findings require diagnosis. No production correction, global M12 sign-off,
release freeze, push, or cross-platform acceptance is implied.

This report summarizes the [detailed Linux observations](m12-linux-manual-observations.md)
against the [original continuation contract](m12-native-continuation.md),
[usability procedure](m12-usability-verification.md), and
[installed quick start](quick-start.md). Earlier pending statements in the
chronological record are superseded only by their later recorded observations.

## Candidate and environment

Validated on 2026-09-21, Ubuntu 24.04.5 LTS x86_64, GNOME Terminal 3.52.0,
VTE 0.76.0, Spanish keyboard, Bash 5.2.21 and Dash 0.5.12. Native target:
`x86_64-unknown-linux-gnu`. Source:
`9cf91f7164235d35beda81d9c8c2d4200b703aad`. Rust/Cargo 1.98.1,
LLVM 22.1.8, GCC 13.3.0, GNU ld 2.42, glibc 2.39.
Version 0.1.0, empty default production features, unsigned.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Installed executable | 13,030,408 | `b2bbc872bf4b7c8175f07dfd66b64d2cc8a02bf66ae6c409ee33542c0a9fcfd4` |
| Release archive | 4,826,579 | `0f7add246e4406eb7a30a955844a9a04d51026014ca69301232d3d536cd3450b` |

Two isolated clean builds and packages were byte-identical. Archive inventory,
checksums, native ELF dependencies, extracted smoke, and dedicated installation
passed. All observed shared libraries resolve; maximum symbol requirement is
GLIBC_2.39. This is not evidence for older distributions. Artifacts and runtime
evidence remain outside Git.

## Results

| Check | Result and evidence boundary |
| --- | --- |
| Installed executable gates | Passed with the corrected test-only resize synchronization harness: ten tests, one helper ignored. Original synchronization and loaded/debug echo-budget failures remain recorded. |
| Source and tooling checks | Serial workspace tests, formatting, Clippy, repository checks, release tooling, secret scan, and audit negative controls passed as recorded. No new full advisory audit is claimed. |
| Installation safety | Collision refusal preserved a synthetic unrelated command. Scoped removal passed for a separate owned install with no daemon running from it. |
| Session presentation | Operator observed neutral definitions, ordered distinct identities, duplicate names, cleared name, and persistence across client reopen. Extra and terminated sessions remain recorded. |
| Form editing | Operator confirmed ASCII/wide/combining sample editing, movement, insertion, Backspace, Delete, field navigation, multiline text, Ctrl-U, resize preservation, validation rejection, and cancellation. Screenshots do not independently establish Unicode code points. |
| Minimum layout | All six screens visited at the measured minimum; complete header and Tasks/Sessions footer observed. Below-minimum notice and enlargement recovery passed. Full Help content accessibility was not established. |
| Stale rename | Local and SSH rejection, retained draft, two-step review, and discard passed. Stale context label remains a finding. |
| Input exclusion and size | Local and SSH first acquisition, rejected competing acquisition, read-only resize isolation, both transfer directions, and fresh long-line wrapping passed. Independent live snapshots confirmed writer dimensions. |
| Detach and reopen | Implemented Ctrl-] release followed by Esc detach passed; new Session 6 remained running. Ctrl+AltGr+] was confirmed on this operator's keyboard. Documented Ctrl-Space failed. |
| Task handover | Neutral-session claim, competing rejection, progress, immediate CURRENT/handover_ready/unowned state, saved history, distinct successor, completion, and quit/reopen passed with independent readback. |
| Live backup and restore | Authorized original daemon stop and fresh-home restore passed. Stable identities and durable records retained; prior running sessions became lost. Original home and backup preserved. Physical history paging passed. |
| SSH loss and reconnection | Scoped SSH client SIGTERM, child survival, lease reacquisition, fresh connection, and stable identities passed. This was a client interruption, not a network-blackhole test. |
| Terminal recovery | Normal local and SSH exit and disconnected local exit passed. Interrupted SSH left stale TUI content over the outer shell; manual reset was needed. Automatic visual recovery failed. |

## Findings requiring follow-up

1. Interrupted SSH visual recovery failed. Determine responsibility across the
   SSH client, terminal, and Relayterm before choosing a correction; preserve
   the original failed result and retest the same interruption scenario.
2. Quick start prescribes Ctrl-Space detach, but the candidate handler implements
   Ctrl-] release and Esc detach. Align the intended behavior, documentation,
   help, and regression coverage, then verify physically.
3. After daemon stop, DISCONNECTED coexists with stale INPUT/WRITER and
   Attaching indicators. Repeated transport-loss diagnostics fill Events.
4. The release chord switched a disconnected client to Events without an
   operator 5 keypress. Capture native key events and reproduce before assigning
   a cause. Successful release while connected does not close this finding.
5. Stale rename reconciliation retains an old context label despite authoritative
   value review. Investigate the context refresh separately from data integrity,
   which passed.
6. Older content appeared clipped after wide-to-narrow transfer. Fresh command
   wrapping and live dimensions passed; historical display behavior still needs
   comparison with the intended terminal model.

## Limitations and retained state

The optional manual Git worktree journey was not performed; the installed
worktree gate passed. The restored fixture contained no worktrees, so populated
worktree metadata restoration was not physically established. No raw screenshots,
transcripts, private identities, runtime databases, or credentials are committed.

The original daemon was stopped with explicit authorization. The restored
workspace and its new running Session 6 are retained at the final operator
checkpoint. The temporary authorized loopback SSH service and private fixtures
are not claimed removed by this report. Cleanup must target only these owned
resources and preserve the backup and candidate evidence.

Windows and macOS results retain their original source mappings. No unavailable
platform check, global acceptance, or failed recovery check is marked passing.

## Proposed next work

Review and prioritize the findings above, then implement focused corrections on
a work branch. Retest affected paths on a newly identified candidate, preserving
this baseline report and its failures. Reconcile the remaining macOS and global
matrix before freezing M12. Publishing evidence or creating a pull request
requires a separate operator request.

## Subsequent diagnosis on the correction branch

The original results above remain unchanged. A synthetic loopback SSH control
experiment, without Relayterm, entered alternate screen and installed remote
exit cleanup. Normal completion delivered both enter and leave sequences;
SIGTERM of the locally owned SSH client delivered entry but no restoration
sequence. Both assertions passed. This isolates the interrupted-client visual
residue from Relayterm rendering and establishes a local recovery limitation,
not a passing automatic restoration result. The TUI guide now describes it.

Source inspection identified fixed-grid resizing in the terminal dependency:
existing rows are resized, not reflowed. Historical clipping is therefore a
terminal-model limitation; newly entered command wrapping remains a separate
requirement. Correction-branch tests exercise both behaviors.

## Correction-branch verification

The correction branch prevents modified digits from selecting global screens,
consumes the release chord in navigation, moves disconnected terminals out of
input focus while preserving uncertain lease identity, suppresses consecutive
identical transport diagnostics, and refreshes rename guidance from the reviewed
snapshot without changing the draft or its explicit submission contract.
The quick start now matches the physically verified release/detach sequence.

`cargo test --workspace --release --locked --offline -- --test-threads=1`
passed: 198 reported test executions, zero failures, and 16 ignored helpers or
opt-in tests. The new native PTY regression stops its own daemon while writing,
verifies navigation and non-writer presentation, exercises Ctrl-5 without an
Events transition, checks diagnostic deduplication, and verifies alternate-screen
restoration on exit. The existing two-client test also verifies the refreshed
rename context after adopting the reviewed revision. The fixed-grid test checks
historical clipping separately from fresh output wrapping. Formatting passed.

These checks are automated Linux evidence for the correction branch, not a
physical retest of the operator's keyboard or proof for macOS or Windows.
The original candidate observations and failed SSH interruption result remain
preserved. Retest the affected physical paths and reconcile platform acceptance
before freezing a replacement candidate.

Workspace Clippy with all targets in release mode and `-D warnings` passed.
No production schema, daemon protocol, or dependency change was introduced.

## Physical Linux correction retest completed

The operator completed the affected checks on source 15794ad using executable
SHA-256 `d580683ad921e304d4204e522de805226df8c3734ba667849f9726d187e2f926`
in a separate private workspace. Input release, repeated release while already
read-only, disconnected indicators, release while disconnected without an Events
transition, non-repeating transport diagnostics, and clean disconnected exit all
passed physically. The two-step stale rename review now shows the authoritative
label while retaining the draft; final discard and independent revision-5
readback confirm no overwrite.

The correction workspace daemon remains running after the rename check, with its
single test session terminated. Original candidate resources remain separate.
The generic Error prefix on informational revision-adoption guidance is a
remaining presentation issue, not failed adoption. These results close the
targeted Linux correction retests, not global M12 acceptance. macOS/Windows
verification and acceptance treatment of the documented terminal/SSH limits
remain pending. No remote publication is implied.

The operator subsequently confirmed final client closure with q. The open
outer terminal is not needed for additional observations in this round. See
[the continuation matrix](m12-cross-platform-continuation.md) for remaining
visual, packaging, security, clean-runtime and global acceptance requirements.

## Final delta continuation on 2026-09-23

The initially clean Linux checkout at 7f18686 was an ancestor of fetched delivery
25c1092849600dc833afefa4cfd58f3a6d1861fb. Work continues on
fix/m12-linux-final-delta. No existing changes were discarded or hidden.

Product source remains 691a8fbb45980658b98d647a85ea8305b2325938.
Inspection of the production diff from 15794ad confirms typed in-memory
Info/Error feedback only. The only crate change between the product and delivery
is the TUI test harness; e4431f7 through delivery changes documentation only.
Protocol, schema, submission, leases and writer sizing retain previous evidence.
The complete 9cf91f7 journey and physical 15794ad correction round are reused.
Local operator scope is limited to Info adoption and real validation Error with
readable guidance and retained cursor/form, plus independent no-write readback.

Read-only GitHub queries reconfirmed success of all seven jobs in
[Quality 35637866692](https://github.com/fpinero/relayterm/actions/runs/35637866692)
and the audit job in
[Security 35637869615](https://github.com/fpinero/relayterm/actions/runs/35637869615),
both at e4431f7b357fe1388963fa94675a567875492fc3. No workflow was dispatched.
This coverage is reused for unchanged code, including the delivery harness.

Retained local artifact inventory contained the complete 9cf91f7 packages and
logs, not final 691a8fb packages. New isolated clean product checkouts and fresh
outputs are therefore required. Package identity and installed checks will be
recorded separately from hosted and physical results below.

The earlier SSH evidence covers 9cf91f7, not the later presentation fixes.
The former temporary listener is absent and its temporary connection resources
are unavailable. No SSH service or security configuration was changed. The
bounded presentation delta needs an available authorized connection; prior
size, survival and continuity evidence remains valid. Deliberate SSH-client
termination is not repeated. Automatic restoration failure and fixed-grid
historical clipping remain explicit acceptance decisions.

### Final native artifacts

Two clean detached checkouts of 691a8fb built offline with Rust/Cargo 1.98.1,
LLVM 22.1.8, GCC 13.3.0, GNU ld 2.42 and glibc 2.39 on Ubuntu 24.04.5
x86_64. The checked-in build wrapper used separate fresh outputs and default
production features, with public path remapping. Both clean source checkouts
remained unchanged. Executable and normalized package comparisons passed.
Version is 0.1.0; signing state is unsigned. No cross-host identity is claimed.

| Final Linux artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Built, extracted and installed rt | 13,034,640 | `26f7f90e27da12239c3130e1163951a4cfe2f08e448fc8b28f6a46cec642d8d7` |
| Normalized archive, both copies | 4,838,446 | `a9e5bb84b474ab8f37e9778046ec530b3833df67261b294549d1352df63a3285` |

Both external SHA256SUMS files verified their archive and manifest before
extraction. Inspection confirmed exactly nine regular members. Native inspection
confirmed x86-64 PIE, interpreter /lib64/ld-linux-x86-64.so.2 and dependencies
libgcc_s.so.1, libm.so.6, libc.so.6 and the loader, all resolved. The maximum
observed GLIBC symbol requirement is 2.39; no older baseline is claimed.

The first extracted smoke failed in the agent's restricted execution environment
at workspace initialization. A separate probe returned daemon_spawn_failed.
Native execution outside that restriction then passed help, version, workspace
initialization, detached daemon, real synthetic PTY and orderly shutdown, without
changing the artifact. Preserve the restricted failure as an environment result,
not a passing invocation or an established product defect.

The packaged helper installed the extracted executable into a new dedicated
user-local directory. Its hash matched the build and archive. Bash without
profile files resolved that exact installation and passed version/help. Earlier
installs, archives, homes and backups were preserved; no global PATH changed.

### Exact-installed automated gates

Using delivery harness 25c1092 and RELAYTERM_TEST_RT set to the dedicated
installation above, the following command passed natively after all compilation
work finished:

```text
cargo test -p relayterm-cli --test session_presentation --test tui_gate --test worktree_gate --test backup_restore --locked --offline -- --nocapture --test-threads=1
```

Twelve tests passed, zero failed and one fixture helper was ignored. The harness
and fixture executables were separate debug outputs, not shipped product files.
Installed release startup p95 was 108 ms, navigation p95 23 ms, input echo p95
154 ms and flood throughput 16,380,700 bytes/s. The unchanged 250 ms input
reference passed. Both worktree gates and backup/restore passed. These results
do not erase earlier Linux or Windows debug/load failures.

### Independent Linux clean runtime

The verified extracted package was copied into a disposable Ubuntu 24.04
container identified by image digest
`sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`.
The container used an unprivileged user, no network, no host mounts, dropped
capabilities and no-new-privileges. No repository, Cargo, rustc, rustup, Python,
C compiler or Git was present. Python orchestration remained outside the
container; product commands and shell children ran inside it.

Runtime inventory: glibc 2.39-0ubuntu8.9, libgcc-s1 14.2.0-4ubuntu2~24.04.1,
Bash 5.2.21 and Dash 0.5.12. The packaged helper installed into a fresh owned
directory. Exact hash, Bash resolution, help/version and preservation of an
unrelated synthetic command on collision passed.

The installed runtime passed initialization, detached daemon operation, three
neutral shell PTYs with stable order/names and input, task creation/readiness,
exclusive claim with competing rejection, progress, handover, successor claim
and completion. An automated 80 by 24 PTY observed CURRENT and successful TUI
exit with alternate-screen restoration; session snapshots were unchanged by
client closure. Live backup, original daemon stop, fresh-home restore and reopen
passed. Session identities/order/names were preserved and prior live sessions
became lost. Task content/status and complete progress/handover/claim entries
matched the pre-backup readbacks. Both test daemons stopped orderly.

The private runner initially omitted the required history revision, omitted a
terminal cursor-position response, selected a backup parent without private
permissions, and expected an items field instead of history entries. Those
failed attempts are retained. Corrected checkpoint continuations used the same
fixture without discarding state or repeating completed stages. The terminal
probe passed after the emulator answered the standard cursor-position query;
backup passed with an owned private parent. No product change was required.
The consolidated corrected recipe was syntax-checked, not claimed as an
additional fresh complete execution.

This closes the independent Linux runtime preparation row. It is automated
installation/quick-start evidence, combined explicitly with unchanged native
9cf91f7 and 15794ad interaction evidence and the new local feedback observation
below. It is not a new full physical journey or evidence for macOS/Windows clean
runtimes. Optional Git worktrees remain covered by the exact-installed host gate;
Git was deliberately absent from this minimal runtime. The stopped container,
original state and backup are retained.

### Local physical feedback delta completed

The operator confirmed the two requested checkpoints in GNOME Terminal using
the exact installed 691a8fb executable. Client A retained Draft A while client B
saved Saved B. The conflict and two-step review were setup for the changed Info
presentation, not a repeated conflict acceptance battery.

After adoption, the operator confirmed readable Info guidance requiring explicit
Ctrl-S, Current label: Saved B, the retained Draft A and its visible cursor.
Independent complete ordered snapshots before and after adoption were identical
at revision 4 and event watermark 4. Adoption did not write automatically.
After safe discard, another complete snapshot was identical to that winner.

The operator then confirmed Error: Title is required. with the empty task form
still open and its title cursor visible. Independent task-list snapshots before
and after validation were identical and empty. No task was created. Cancellation
and final readback preserved the empty task list and winning session snapshot.

Both clients subsequently exited with status zero and the owned daemon stopped.
Process inspection found zero running processes using the installed candidate;
its hash remained unchanged. No new physical size, input-transfer, disconnect,
diagnostic, handover, Unicode, backup or SSH-interruption round was requested.
Native package and physical evidence are complete for this bounded Linux delta;
M12 remains globally open.

### Finite remaining requirements

| Responsible environment or owner | Exact outstanding proof |
| --- | --- |
| One authorized SSH connection | Available authorized destination first, then only corrected rename review/context/Info, disconnected navigation/read-only feedback, repeated release without Events, stable diagnostics, normal exit and reconnect identity. Reuse prior sizes, survival and continuity; do not kill the SSH client again. |
| macOS and Windows clean runtimes | Install each retained final package in an identified independent runtime, verify prerequisites/hash/discovery/daemon/PTY and applicable quick-start commands. Reuse their completed physical observations. Linux container evidence cannot certify these targets. |
| Windows debug environment | Investigate/dispose the retained 290/294 ms input-p95 misses against 250 ms, separately from passing release and hosted results. No further physical keyboard round is implied. |
| Maintainer acceptance | Explicit disposition of interrupted-SSH automatic restoration failure and fixed-grid historical clipping against AC-4/AC-8, retaining original failures and other documented evidence limits. |
| Final M12 reconciliation | After those prerequisites, reconcile all 16 ACs and phase gates, audit/freeze the mapped three-platform artifact inventory and complete the local handoff. Publication policy remains separate; no publishing action is authorized here. |

No M13, push, merge, tag or release was performed. The SSH prerequisite remains
bounded to connection availability; no service or security setting was changed.
