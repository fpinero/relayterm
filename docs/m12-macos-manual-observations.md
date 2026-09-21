# M12 macOS manual observations

## Candidate and environment

Observation date: 2026-09-19. Native macOS 26.5.2 (25F84), arm64,
Terminal.app 2.15, zsh 5.9, Spanish Mac keyboard. Initial terminal size:
120 columns by 30 rows. Child commands: three independent `/bin/sh` sessions.

Source: `491d5f1037450c362525b289fa6361d834fc0b6f`. Version: `0.1.0`.
Target: `aarch64-apple-darwin`. The unsigned local artifact is the separate
macOS build identified in [the candidate handoff](m12-candidate-handoff.md).

- Executable SHA-256: `6d16cae5733c458f6749e9a5dbaa9f17f64a41e773d9b5ee727107a332dd84da`.
- Archive SHA-256: `6e9e524d044383a4edc0483434a955f9d834c4c6a92ec5854e6d38af495b1591`.

The assistant checked archive and manifest checksums, executable identity,
version, and administrative readback. The operator performed the terminal
interactions and reported observations. Screenshots were inspected in the
conversation only; no screenshots, terminal captures, private paths, or raw
runtime data are included here.

## Verified observations on the original candidate

- The operator's zsh resolved no existing `rt`. Installation into a fresh,
  dedicated user-local directory succeeded without changing PATH. The installed
  executable retained the expected checksum and reported `rt 0.1.0`.
- Interactive initialization opened an empty workspace outside the repository.
  The initial interface rendered correctly in Terminal.app.
- Three neutral definitions were created, validated, enabled, and launched.
  Sessions appeared as `Session 1`, `Session 2`, and `Session 3`, oldest first,
  with distinct abbreviated identities and running status.
- Duplicate synthetic names preserved identity and order. Clearing the second
  name restored `Session 2`. Administrative readback confirmed all three running
  instances and creation ordinals; the first session's full session and instance
  IDs matched the displayed Details values.
- The operator confirmed a visible block cursor, ASCII insertion and deletion,
  wide and combining Unicode input, navigation to the beginning and end,
  field switching, and editing at 80 by 24 followed by enlargement. A separate
  subminimum 68 by 26 resize displayed the minimum-size guidance. Cancelling the
  draft left the task list empty.
- A controlled two-client rename rejected the older draft, preserved it,
  exposed the authoritative name on the first Ctrl-R, restored the original
  draft on the second Ctrl-R, and preserved the saved name after discard.
  An earlier attempt saved normally and did not establish a stale conflict;
  the controlled repetition established each state separately.
- With two clients attached to one child, A acquired WRITER and B remained
  READ ONLY. B's competing acquisition displayed the inline owner guidance.
  A retained input and both clients continued receiving output. Explicit release
  in A followed by `i` in B transferred input and cleared the warning. Both
  clients received the executed synthetic marker from B.
- Ctrl-] released input. Ctrl-5 did **not** substitute for Ctrl-] in the observed
  terminal and keyboard configuration, despite the application accepting that
  decoded key combination. Do not advertise it as verified on this environment.
- Detaching both clients preserved all three running sessions. Quitting both
  TUIs restored normal shell input and output. Reopening preserved names, IDs,
  order, running children, and terminal history. A new marker executed after
  reattachment.
- A task was created, made ready, and claimed by the first instance with status
  `active`. A competing claim retained the original owner; rejection diagnostics
  appeared in Events, without an inline Tasks notice. Progress summary and
  verification were saved and visible in task detail.

## Blocking handover refresh failure

Submitting a handover with Ctrl-S closed the form, but the TUI remained STALE
and continued to display the task as active with the original owner. An explicit
uppercase R refresh did not recover the view. Events accumulated generic
rejection diagnostics claiming that state had been refreshed.

Read-only administrative inspection confirmed that the operation had committed:
the task was `handover_ready`, its owner was null, its claim was closed for
handover, and the handover fields were intact. A read-only protocol probe isolated
the failure: `workspace.get_snapshot` for `workspace` returned `invalid_reference`;
the definitions, tasks, claims, progress, and handovers collections remained
readable at revision 29 and event watermark 32.

The operational projection loaded latest handovers across the workspace even
when their tasks and closed claims were outside the projection. A task-neutral
session completing a handover therefore broke reference validation in the
workspace overview used by a full client refresh. The persistence continuity
regression was extended to check this exact boundary before another session
resumes the task; it failed with `Reference` on the original implementation.

The source correction scopes projected handovers to projected task IDs while
preserving full-history reads. Regression coverage also verifies that a task-scoped
projection retains the handover and closed claim, and that two protocol clients
refresh successfully before the next claim. Source validation passed the full
workspace test suite, the updated targeted protocol journey, Clippy with denied
warnings, formatting, secret checks, audit negative controls, candidate-only
repository checks, and whitespace checks. The diagnostic probe did not modify the original installation or workspace.
The separately identified corrected-candidate retest is recorded below; the
original candidate failure remains part of the evidence.

## Corrected candidate preparation

On 2026-09-19 source `d7605541bfa829cfab6d8a3c53b87ad6e0f7b4fa` was built
twice offline using the release wrapper, two clean detached clones, and separate
fresh target directories. Both binaries and both normalized archives matched.
The unsigned arm64 candidate retains version `0.1.0` and empty production features.

- Executable: 11,051,968 bytes; SHA-256 `4de8d6f45c8cc375c3b71c3ad05515cbdbc91f282120d0fb351bdaeae98bc7c7`.
- Archive: 4,388,092 bytes; SHA-256 `129c925e7d03c2acd2a333ad5832927672dc809cfaf704aa6a4f71fff40c27bf`.
- Toolchain: Rust and Cargo 1.98.1; native SDK 26.5; Mach-O deployment target 11.0.
- Dynamic libraries: system libiconv and libSystem only. This does not lower the
  declared runtime baseline.

Archive inventory inspection and extracted smoke passed help, version,
initialization, detached daemon, real PTY, and orderly shutdown with a constrained
runtime PATH. A separate versioned user-local installation preserved the executable
hash. Its exact executable passed `session_presentation`, `tui_gate` (six tests,
one fixture ignored), and `backup_restore` using `RELAYTERM_TEST_RT` and
`cargo test -p relayterm-cli --test session_presentation --test tui_gate --test backup_restore --locked -- --test-threads=1`.
The archive, manifest, checksums, and build record were retained outside Git.

Before switching candidates, a live backup from the original binary captured
schema 3, revision 29, event watermark 32. The original daemon, home, and binary
were retained during preparation. Assistant-driven installation checks verified
that a repeat install refused the existing binary without changing its hash,
and that an unrelated synthetic `rt` resolved in an isolated zsh PATH and was
not overwritten by the installer. These are automated local observations.

## Corrected candidate manual retest

The operator quit both original clients and explicitly stopped the original
daemon with session termination. The assistant restored the live backup into a
fresh private home using the corrected executable and verified successful
workspace and collection snapshots. The operator reopened the restored workspace
and observed CURRENT, the saved handover, unclaimed handover-ready status, and
all three original sessions marked lost. The original backup and home were retained.

The operator launched two new task-neutral shell sessions, Session 4 and
Session 5. Session 4 claimed the restored task, submitted a new handover with
Ctrl-S, and immediately saw handover_ready and unclaimed with the new handover
visible. CURRENT was confirmed before the next claim, without a repair refresh.
Session 5 then claimed the task: active status, a different owner, CURRENT, and
retained progress and handover were visible. Pressing d completed the task,
cleared its owner, and closed the new claim for completion. Quitting with q and
reopening the same corrected executable preserved done, CURRENT, and history.
This is client reopen evidence, not an additional daemon-restart test.

Independent read-only `task list` and `task history` readback at revision 38,
event watermark 48, confirmed done with no owner, three closed claims (two
handovers and one completion), the original progress record, and both handovers.
The affected installed handover, continuation, completion, backup recovery, and
client-reopen retest passed. Earlier observations remain attributed to their
original executable; the correction changes persistence projection filtering,
and exact-candidate automated TUI and session-presentation gates provide the
additional regression evidence. Cross-platform acceptance remains open.

The operator also verified that `command -v rt` produced no resolution while
the corrected absolute executable returned `rt 0.1.0`. The installation uses
explicit paths and has not changed shell profiles or global PATH.

## Scoped removal observation

A disposable installation from the corrected extracted archive was created and
its executable checksum verified. No daemon was launched from that copy. The
operator removed only its executable and empty install directory with the
documented POSIX commands, then successfully queried the retained candidate's
version. Independent verification confirmed the disposable directory was absent,
the retained candidate checksum was unchanged, the backup directory remained,
and the restored task was still done and unclaimed. No shell profile or PATH
entry was changed. Collision refusal remains attributed to the assistant-driven
checks described above, rather than to an operator keyboard observation.

## Remaining observations

- Perform the focused SSH observation on one authorized supported host.
- Complete Windows and Linux observations and reconcile the final artifact and
  acceptance matrices before closing M12.

## Focused correction candidate preparation, 2026-09-21

Candidate 691a8fb follows the Linux handoff 7f18686 and adds typed informational
form feedback. The [correction candidate report](m12-macos-correction-candidate.md)
records exact source, native environment, identical double-build/package hashes,
local checks and eleven passing exact-installed gates. These are assistant
observations. A separate synthetic workspace is ready, while prior resources
remain intact. Header/minimum size, writer transfer and inherited Linux
correction physical checks are pending. The earlier manual journey is not
reopened wholesale; its unchanged evidence retains its original source mapping.

## Initial focused operator checkpoint

On candidate 691a8fb the operator supplied a launcher view reporting 24 rows
and 80 columns before startup, and an Overview view with the complete CURRENT
label and Relayterm footer. The operator enlarged the window after seeing a
message. The dimensions of the displayed Overview were not independently
measured, so this does not yet pass the exact-80-by-24 observation or establish
which message prompted enlargement. Subsequent process inspection found only
the detached candidate daemon, with no candidate TUI attached to a terminal.
The separate shell screenshot shows the prompt again; no exit status was captured.
Screenshots and raw terminal material remain outside Git.

## Launcher resize clarification

The next operator screenshot identifies the message as the launcher's Press
Enter prompt, before Relayterm starts. Automatic shrinking came from the private
launcher's explicit window-resize escape sequence, not from the product. An
immediate stty measurement could also precede the asynchronous window resize,
as illustrated by the earlier 35-by-119 value. The assistant removed automatic
resize requests from both private launchers, retaining backups. Each modified
launcher passed zsh syntax validation and an isolated PTY check reaching the
prompt without emitting a resize request. The test stopped only its own waiting
launcher, before product startup. No candidate executable or daemon was changed.
The latest physical screenshot reports 24 rows and 80 columns at the launch
prompt. Exact-size TUI observation remains pending after Enter without resizing.

## Tasks and Sessions at exactly 80 by 24

On candidate 691a8fb the operator confirmed CURRENT in Tasks and Sessions.
Both supplied screenshots show the complete freshness label, Relayterm footer
and applicable shortcut text. Independent read-only stty inspection of the live
candidate TUI terminal returned 24 rows and 80 columns. These two minimum-size
screen observations pass. The remaining screen checks and below-minimum
recovery are still pending. No screenshots or runtime identifiers were committed.

## Remaining minimum-size screen observations

The operator supplied the requested Overview, Agents, Events and Help views
for the same 80-by-24 checkpoint. All four screenshots show the complete CURRENT
label and Relayterm footer. Together with the independently measured Tasks and
Sessions checkpoint, header/footer presentation passes on all six screens.
This verifies the Help header/footer, not access to all Help content. No
below-minimum warning or subsequent recovery observation has yet been supplied.

## Below-minimum warning and recovery passed

The operator supplied a view explicitly reporting current size 79x24 and the
80x24 minimum, followed by the recovered Help view with complete CURRENT and
Relayterm footer after the requested return to 80x24. The native minimum-size
header/footer and shrink/recovery block passes on candidate 691a8fb. This does
not expand the observation to full Help content accessibility. Writer-size
transfers and the other correction observations remain pending.

## Two-client input baseline

The operator supplied both attached views showing NAVIGATION and READ ONLY on
candidate 691a8fb, identifying the larger green window as B. Independent stty
reads confirmed outer terminal sizes of 24 rows by 80 columns and 34 rows by
120 columns. An administrative session attachment read the sole live session's
snapshot as 24 rows by 80 columns before input acquisition. This snapshot is
live terminal state, not the durable creation-size field. First acquisition and
transfers remain pending; no input was sent by the assistant.

## First input acquisition at the narrow writer passed

The operator acquired input in A without resizing and executed the supplied
long synthetic echo command. The screenshots show A in INPUT/WRITER, fresh
command and output wrapping at its narrow viewport, and B remaining in
NAVIGATION/READ ONLY with the same output. The operator confirmed correct
behavior. Independent live snapshot readback returned 17 rows by 78 columns;
outer terminal measurements remained A 24 by 80 and B 34 by 120. This proves
first acquisition synchronized the shared PTY to A's inner viewport rather
than retaining its initial 24-by-80 creation dimensions. Observer resize
isolation, competing acquisition and both transfers remain pending.

## Competing input and observer resize isolation passed

The operator screenshots show B rejecting acquisition with the inline competing
owner guidance while remaining NAVIGATION/READ ONLY, and A retaining
INPUT/WRITER. Independent outer terminal measurements after B's resize returned
A 24 rows by 80 columns and B 34 rows by 113 columns. The actual B size differs
from the suggested 30-by-100 target but establishes a resize from its previous
34-by-120 size. Independent live snapshot readback remained 17 rows by 78 columns,
matching A. Competing acquisition and observer resizing therefore preserved
A's ownership and shared terminal dimensions. No assistant input was sent.

## Release chord and narrow-to-wide transfer passed

The operator confirmed that releasing input in A and repeating Ctrl-] while
read-only kept Sessions selected. Screenshots show A in NAVIGATION/READ ONLY
and B in INPUT/WRITER without the competing-input notice. Independent live
snapshot readback measured 27 rows by 111 columns, matching B's inner viewport,
while outer measurements remained A 24-by-80 and B 34-by-113. The explicit
A-to-B ownership and size transfer passes without resizing after acquisition.
Fresh output at B's width and the reverse transfer remain pending. The operator
confirmed the requested chord; no alternative physical key mapping is inferred.

## Wide input and reverse writer transfer passed

The operator confirmed the supplied long echo command and output each occupied
one row in B while B held input. Subsequent screenshots show B read-only and A
writing after explicit release/acquisition. Independent live snapshot readback
returned to 17 rows by 78 columns, with unchanged outer dimensions A 24-by-80
and B 34-by-113. Both transfer directions now have physical ownership and
independent live-size evidence on 691a8fb. The historical B lines are clipped
in the narrower A view, consistent with the documented fixed-grid limitation;
this is not evidence of fresh-input loss. Fresh long-line input after the reverse
transfer remains the last check in this width block.

## Fresh narrow input after reverse transfer passed

The operator screenshot shows the final synthetic long echo command and its
output continuing on the next row after the B-to-A transfer, with A still in
INPUT/WRITER. The complete new marker is retained across the wrapped rows.
This closes the macOS writer-size block on 691a8fb: first acquisition, competing
rejection, observer resize isolation, both transfers with independent live
snapshot dimensions, and fresh input at each writer width. Historical wide
rows remain subject to the previously documented fixed-grid clipping limit.
Connected release and repeated release without an Events transition also passed.
Disconnected feedback/exit and rename review remain pending.

## Stale rename rejection and draft preservation passed

The operator kept Draft A open in A, saved Saved B from the second client, and
then submitted A's older draft. The screenshots show Saved B in B and the
explicit workspace-changed Error in A with Draft A intact and the physical
cursor at its end. Independent ordered-session readback confirmed Saved B
remained authoritative at revision 5. This verifies rejection and preservation;
two-step review, informational adoption feedback and discard remain pending.

## Two-step review and informational adoption feedback passed

The first Ctrl-R screenshot shows Saved B in the authoritative session list and
Reviewing latest state in the footer, with a separate Ctrl-R adoption action.
After the second Ctrl-R, the screenshot shows Current label: Saved B, the
unchanged Draft A, the physical cursor at its end, and Info-prefixed guidance
that Ctrl-S performs a new explicit submission. The informational text wraps
within the form and is not labelled Error. The preceding stale rejection
retained its Error prefix. Independent ordered-session readback after adoption
matched the entire saved winner baseline, including revision 5. No automatic
submission occurred. Safe discard and the separate validation-error observation
remain pending.

## Reviewed draft discard passed

After the requested Esc and confirmation, the operator screenshot shows the
closed form and Saved B still selected. Independent ordered-session readback
matches the entire pre-review winner baseline, including revision 5. This closes
the reviewed rename discard checkpoint without overwriting the winner. Stale
rejection, two-step review, refreshed context, retained draft/cursor and Info
adoption guidance are now physically verified. Separate validation-error
presentation and disconnected-client observations remain pending.

## Real validation-error presentation passed

The operator submitted an empty task form in B. The screenshot shows Error:
Title is required., the retained open form and the visible cursor in the empty
title field. Independent task-list readback remained empty and the session
winner remained Saved B at revision 5. Together with the Info adoption and
stale Error observations, this closes the focused macOS form-feedback and
rename block on 691a8fb. For the next disconnect observation, read-only checks
identified one running synthetic shell and zero tasks in the dedicated workspace.
Its daemon has not been stopped; explicit operator authorization is still required.

## Authorized disconnected-client checkpoint started

The operator authorized stopping and starting processes needed for Relayterm
validation. The assistant targeted the dedicated correction workspace only,
verified its sole Saved B session and live 17-by-78 snapshot, then invoked the
exact candidate with daemon stop --terminate-sessions. The command exited
successfully and returned lifecycle stopped. The candidate daemon has not been
restarted so the open clients can be inspected while disconnected. Physical
non-writer feedback, release navigation, diagnostic bounds and exit observations
remain pending. This action does not establish physical input focus at the exact
stop instant; that requires the operator's observation.

## First disconnect observation boundary and focused retry setup

The operator supplied both clients showing DISCONNECTED. A shows the session
list and B retains the empty validation form. This proves the disconnect label,
but does not establish loss of writer focus on an attached terminal. Cached
running metadata in A is not evidence that the stopped child survived. The
writer-specific checkpoint remains pending rather than being marked passed.

Under the existing process-lifecycle authorization, the assistant reopened the
dedicated workspace and created one new shell from the same neutral definition.
Independent ordered readback shows the original Saved B session terminated and
the new unnamed Session 2 running. The next stop will wait for physical proof
that A is attached and writing. Previously passed tests are not reopened.

## Daemon stopped after physical writer confirmation

The operator supplied the green client showing CURRENT and INPUT/WRITER on the
new shell. Although the procedure named A, the confirmed writer is B, which is
sufficient for this focused native disconnect observation. After verifying the
old session terminated and the new session running, the assistant stopped the
dedicated daemon with explicit session termination under the existing operator
authorization. Exit zero and lifecycle stopped were verified. The post-stop
writer-label transition, chord behavior, diagnostics and exit remain pending.

## Disconnected writer presentation passed

After the physically confirmed green-client INPUT/WRITER state and authorized
daemon stop, the operator screenshot shows DISCONNECTED, NAVIGATION, READ ONLY
and RECONCILE WITH R. Neither WRITER nor Attaching is shown. Sessions remains
selected in the supplied post-instruction view. This passes the writer-to-
disconnected presentation checkpoint on 691a8fb. Retained shell output is not
interpreted as a surviving child. Explicit confirmation of repeated release
chord behavior, diagnostic stability over time and clean exit remains pending.

## Disconnected release chord passed and Events baseline observed

The operator explicitly confirmed that pressing Ctrl-] more than twice while
disconnected did not leave Sessions. The subsequent Events screenshot shows
two Connection lost diagnostics separated by workspace events, consistent with
the two authorized daemon stops in this client history. It does not show a
consecutive transport-diagnostic flood. The screenshot is a point-in-time
observation; stability over the requested interval and native exit restoration
still require operator confirmation.

## Diagnostic stability and native exit passed

The operator confirmed no new Events notices after the requested observation
interval. The second Events screenshot retains the same two transport-loss
entries separated by workspace events, consistent with the two distinct daemon
stops. The diagnostics did not flood the view. Both subsequent screenshots show
the outer shell prompt restored and the requested synthetic echo command
executing normally. Read-only process inspection found zero remaining processes
from this installed candidate. The dedicated daemon and its test shells remain
stopped; private state, backups, packages and unrelated installations are retained.

All required focused native macOS physical observations in this round pass on
source 691a8fb and executable hash
4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060:
minimum layout and recovery, writer acquisition and both size transfers,
observer isolation, connected/disconnected release navigation, disconnected
indicators, diagnostic stability, native exit, stale rename review, retained
context/draft/cursor, informational feedback, validation errors and safe discard.
This does not close the separate SSH, clean-runtime, other-platform, final CI or
global AC reconciliation gates. Historical failures and source mappings remain.
