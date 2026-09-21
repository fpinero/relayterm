# M12 Linux native observations

## Scope and candidate

Status: Required Linux observation attempts completed, with failed checks and
open findings. See the [final Linux report](m12-linux-final-report.md). Earlier
pending statements below describe their checkpoint date, not the final status.

The evidence branch was created from the fetched Windows continuation branch
with an initially clean working tree. Production source is exactly
`9cf91f7164235d35beda81d9c8c2d4200b703aad`. The evidence descendant changes
only documentation. Two separate clean detached source clones and fresh build
outputs are used outside the repository. Both production clones remained clean after building and packaging.

## Environment

Preparation date: 2026-09-21. Ubuntu 24.04.5 LTS, x86_64; this is within the
declared Ubuntu 24.04 baseline. GCC 13.3.0, glibc 2.39 (Ubuntu package
2.39-0ubuntu8.9), Bash 5.2.21, Dash 0.5.12-6ubuntu5, GNOME Terminal 3.52.0,
and VTE 0.76.0. The operator selected GNOME Terminal and a Spanish keyboard.
Rust/Cargo 1.98.1 was prepared in an isolated temporary toolchain location;
no shell profile was changed. Public locked dependencies were fetched before
starting the offline release builds.

## Built and installed artifacts

Both native release builds passed from separate clean clones and fresh output
directories using `scripts/build_release.py build --target
x86_64-unknown-linux-gnu --output <fresh-output> --offline`. Build-record
comparison and package-manifest comparison passed with byte-identical copies.
Rust 1.98.1 uses LLVM 22.1.8; Cargo is 1.98.1. Native linker: GNU ld 2.42.
Version 0.1.0, empty default production features, unsigned.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 13,030,408 | `b2bbc872bf4b7c8175f07dfd66b64d2cc8a02bf66ae6c409ee33542c0a9fcfd4` |
| Archive | 4,826,579 | `0f7add246e4406eb7a30a955844a9a04d51026014ca69301232d3d536cd3450b` |

External archive and manifest checksums passed before the extraction used for
installation. Inventory inspection confirmed exactly nine expected regular
members. `file`, `readelf`, and `ldd` confirmed x86-64 PIE, interpreter
`/lib64/ld-linux-x86-64.so.2`, and direct dependencies on libgcc_s, libm, libc,
and the loader. All resolved locally; the maximum observed glibc symbol version
is GLIBC_2.39. This does not establish support below Ubuntu 24.04.

The extracted smoke passed help, version, initialization, detached daemon,
real synthetic PTY, and orderly shutdown with constrained runtime PATH.
The packaged helper installed a fresh dedicated user-local copy. Its checksum,
help, and version passed. Both archives, manifests, checksums, build records,
and native inspection are retained in a dedicated user artifact directory
outside Git. No global PATH or shell profile was changed.

Assistant-driven installation checks verified an existing synthetic unrelated
`rt` was refused and preserved byte-for-byte, and independently resolved that
sentinel in an isolated Bash PATH. A separate space/Unicode install retained the
candidate checksum. That owned copy ran only version, never a daemon, and its
executable and empty directory were removed explicitly; the sentinel remained.
These are automated observations, not operator keyboard confirmations.

## Installed gate investigation

With RELAYTERM_TEST_RT pointing to the exact installed executable, the command
`cargo test -p relayterm-cli --test session_presentation --test tui_gate
--test backup_restore --test worktree_gate --locked --offline -- --nocapture
--test-threads=1` passed backup/restore and session presentation, but failed the
TUI input-ownership scenario. Five other TUI tests passed and one fixture helper
was ignored. The failed assertion did not observe `Another client controls input`
within 45 seconds after the observer acquired unsuccessfully and switched to
Events at 80 by 24. This remains a failure under investigation, not a passing
writer-size gate. Cargo did not reach worktree_gate in that invocation. Its previously compiled
harness was then invoked separately with the same RELAYTERM_TEST_RT; both
worktree tests passed, including two isolated worktrees, three sessions, two
claims, restart recovery, and the non-Git boundary with Git 2.43.0.

The diagnostic reproduction also failed waiting for READ ONLY or WRITER after
an immediate resize/input sequence. The original test sent resize, attachment,
acquisition, and screen navigation without waiting for the new frame. A passing
diagnostic variant alone was not accepted as a fix. The retained test correction
sizes its parser before notifying the child, waits for the footer at its new
row, waits for discard confirmation to close, and observes read-only attachment
before acquisition. It retains both inline and Events notice checks and all
live snapshot dimension assertions. Production code and installed binary remain
unchanged; this evidence branch changes the test harness only. The subsequent serial complete gate
passed, and the initial failed results remain recorded.

The first complete rerun with the corrected harness passed input ownership but
failed the separate echo reference budget: p95 was 299 ms against 250 ms, with
navigation p95 27 ms and startup p95 400 ms. Assistant-started Clippy compilation
was concurrent on the same host. This loaded result is retained as failed;
a serial rerun with no build workload is required. No budget was relaxed.

After Clippy finished, the complete four-gate command passed serially through
the unchanged installed binary: ten tests passed and one fixture helper was
ignored. Startup p95 was 82 ms, navigation p95 24 ms, input echo p95 179 ms, and
flood throughput 23,500,094 bytes/s. The original 250 ms local echo budget passed.
Input ownership verified initial live dimensions 21 by 88, transfer to 17 by 78,
read-only resize isolation, rejected-acquisition isolation, and return to 28 by
118. Both worktree tests passed in the same invocation. This validates the
installed candidate with the corrected test harness, not a rebuilt executable.

`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
passed on the isolated source with the same harness correction. Formatting
passed in the evidence working tree. The first full workspace run without RELAYTERM_TEST_RT failed the debug-build
hardening echo reference check at p95 470 ms against 250 ms; navigation p95 was
23 ms and the other eight hardening tests passed (one helper ignored). This
unoptimized-binary performance failure is not relabelled as passing. A subsequent serial
`cargo test --workspace --locked --offline -- --test-threads=1` completed with
exit 0 and RELAYTERM_TEST_RT selecting the installed production candidate in
supported gates. Original budgets are unchanged. Ignored helpers and isolated
opt-in gates remain ignored, not newly passing evidence. The corrected test
file was byte-compared with the evidence working tree after validation.

The passing integrated TUI scenario reported startup p95 148 ms, navigation
p95 25 ms, input echo p95 172 ms, and flood throughput 10,982,501 bytes/s.
A separate diagnostic checkout is used to inspect the failure without modifying
the installed candidate or the clean production checkouts.

## Initial operator checkpoint

The operator reported an outer terminal of exactly 24 rows by 80 columns and
TERM=xterm-256color. The initial discovery command accidentally checked `rt.`
with a trailing period. The corrected `type -a rt` subsequently reported no
command resolution. This is terminal preparation, not TUI layout evidence.

The operator supplied three local screenshots showing the installed candidate
in Overview, Tasks, and Sessions. The assistant inspected them without copying
them into Git. At the reported 80 by 24 size, all six tab names, the complete
CURRENT label, and the Relayterm footer with keyboard hints are visible.
Tasks and Sessions are empty. Independent installed workspace readback confirms
revision 1, ready daemon, and zero definitions, instances, and tasks. Platform
warning confirmation and the remaining screens/resize observations are pending.

The next operator screenshot showed one running Session 1 with CURRENT and
full, distinct session/instance identities. Independent ordered readback matched
both identities, creation ordinal 1, the enabled Shell A definition with
`/bin/sh`, empty arguments/environment allowlist, terminal capability, and no
task assignment at revision 5 and watermark 5. The remaining two intended definition launches are
pending operator observation. A subsequent screenshot showed Shell B disabled
while Shell A remained selected and was also disabled. Readback at revision 10
and watermark 10 showed Session 1 still running and a second Shell A session
terminated with exit code 1. This deviation is retained; it does not establish
a Shell B launch or the intended three-session journey. Guidance was narrowed
to one explicit selection and launch at a time, without deleting history.

A subsequent Sessions screenshot and ordered readback at revision 15 and
watermark 15 confirmed four stable rows: Session 1 running from Shell A,
Session 2 terminated from Shell A, and Sessions 3 and 4 running from Shell B.
Thus three children are live, but the planned third distinct definition has
not yet launched. Both Shell B launches are retained with distinct session and
instance IDs. The operator is being guided through Shell C creation separately
from selection, enablement, and launch.

## SSH availability

The M11 record identifies a temporary loopback server that was stopped after
observation. Read-only inspection confirmed that the global SSH service and
socket are inactive and disabled, with no initial listener on ports 22 or 22222.
OpenSSH client/server 9.6p1 Ubuntu-3ubuntu13.19 is installed.

The operator explicitly authorized a new temporary loopback SSH server.
A user-owned server bound to loopback port 22222 was prepared with isolated
synthetic Ed25519 host/client keys, pinned host verification, public-key-only
authentication, and forwarding disabled. StrictModes is disabled only in this
temporary configuration because its synthetic authorized-key file is below the
sticky temporary directory, matching the M11 fixture boundary. No global SSH
configuration or service setting was modified. A fresh batch connection returned
the expected synthetic marker. This proves connection availability only;
interactive SSH cursor, conflict, writer-size, disconnect/reconnect, identity,
and terminal recovery observations remain pending.

## Preparation checks

Assistant checks passed:

- `python3 -m unittest discover -s scripts -p 'test_*release.py'`: 22 cases,
  20 passed and two Windows-only cases skipped.
- `cargo fmt --all -- --check` on the exact clean candidate.
- `python3 scripts/check_repository.py`: style, links, queue, and boundaries.
- `python3 scripts/check_secrets.py`: candidate scan and synthetic negative
  control, using the workflow-pinned Gitleaks 8.30.1 after checksum validation.
- `python3 scripts/check_audit_controls.py`: license and dependency-ban negative
  controls with workflow-pinned cargo-deny 0.20.2 after checksum validation.

The initial scanner and audit-control invocations could not start because their
required tools were absent from PATH. The isolated tools and explicit toolchain
PATH resolved those preparation failures; they were not product failures or
passing scans. The full dependency advisory audit is not claimed by these
negative-control checks.

## Observation boundary and pending checkpoints

Assistant automation and operator observations will be recorded separately.
No screenshots, raw terminal captures, databases, private paths, credentials,
or personal host/account identifiers belong in this record.

- Resolve the installed TUI gate failure and finish exact-installed integration
  gate evidence; retain the original failed result.
- Observe the three-session quick start, IDs/order/names, Unicode form cursor,
  exact 80 by 24 layout, below-minimum recovery, and two-client stale review.
- Observe first writer acquisition, transfers in both directions, read-only
  resize isolation, detach/reattach, and client quit/reopen.
- Observe task-neutral claim, conflict, progress, immediate CURRENT and
  unclaimed handover_ready, successor claim, completion, and retained history.
- Verify live backup and fresh-home restore after explicit operator agreement
  to terminate the disposable sessions; retain original home and backup.
- Complete focused SSH physical observations.
- Independent clean-machine evidence, affected macOS retests, and global M12
  reconciliation remain separate open requirements.

## Shell C preparation checkpoint

The operator supplied successive screenshots showing Shell C selected, enabled,
and available, followed by Sessions with the same four historical rows.
Independent ordered readback at revision 20 and watermark 20 confirmed no
Shell C instance and no Session 5. Sessions 1, 3, and 4 remain running; Session 2
remains terminated. Creation, enablement, and availability are observed, but a
successful Shell C launch is not established. No cause is inferred from the
screenshots and no existing session was changed by the assistant.

## Shell C launch confirmation

The operator clarified that the earlier checkpoint had not included pressing
`a`; the absent Session 5 was not a launch failure. After the explicit launch,
the supplied screenshot shows Session 5 running with CURRENT. Independent
ordered readback at revision 22 and watermark 22 matches its full session and
instance IDs and confirms creation ordinal 5, Shell C launch context, and no
task assignment. All five historical rows retain their order and prior IDs.
Sessions 1, 3, 4, and 5 are running; Session 2 remains terminated. Running
sessions from each of the three neutral definitions are now independently
established, with the extra Shell B launch and terminated Shell A row preserved.
The three-definition launch checkpoint is complete; naming and subsequent
manual checks remain pending.

## First session rename checkpoint

The operator screenshot shows the first session with the instructed synthetic
custom name and CURRENT. Independent ordered readback at revision 23 and
watermark 23 confirms the saved name, unchanged session and instance IDs,
creation ordinal 1, and running status. All five rows retain their prior order,
identities, and statuses. Session 3 still has no custom name. The first rename
is verified; duplicate-name and cleared-name observations remain pending.

## Duplicate session names

The operator screenshot shows the first and third session rows with the same
instructed synthetic name, distinct abbreviated IDs, unchanged positions, and
CURRENT. Independent installed ordered readback at revision 24 and watermark 24
confirms both names are equal and their full session and instance identities
are unchanged. Creation ordinals remain 1 through 5; four sessions are running
and the second remains terminated. Duplicate-name observation passed. Clearing
a name remains a separate pending checkpoint.

## Cleared session name

After the instructed Ctrl-U and Ctrl-S sequence, the operator screenshot shows
Session 3 restored in its original third row with the same abbreviated ID,
running status, and CURRENT. Independent installed ordered readback at revision
25 and watermark 25 confirms a null display name, unchanged full session and
instance IDs, and creation ordinal 3. The first session retains its custom name.
All five creation ordinals remain in their prior order. Cleared-name observation
passed; form cursor editing and resize remain pending.

## Form cursor editing

Following the prescribed ASCII, wide-character, and combining-accent title
sample, the operator confirmed that editing works and the cursor is visible.
The checkpoint requested Left/Right, Home/End, insertion, and Backspace. The
supplied screenshot shows the edited title in an open Create task form with
CURRENT. Cursor behavior is attributed to the operator's dynamic observation;
the screenshot alone does not prove movement or the underlying Unicode code
points. The draft has not been submitted. Field navigation, multiline editing,
resize preservation, and explicit discard remain pending.

## Multiline draft and below-minimum resize

The operator supplied a task-form screenshot with a two-line description,
a minimum-size notice explicitly reporting 57 columns by 23 rows, and the
restored form after enlargement. The title and both description lines remain
visible, with CURRENT restored. The active-field marker is on Title in both
full-form images, establishing preservation of that selected field across the
observed resize. These images do not establish the requested return to
Description or dynamic Tab/Shift-Tab cursor behavior. Multiline input and draft
preservation through below-minimum resize passed; explicit field-navigation
confirmation and draft discard remain pending. Images and private window-title
information were not copied into the repository.

## Form field navigation confirmation

The operator explicitly confirmed that all requested Tab and Shift-Tab movements
work correctly after the Title-to-Description, Description-to-Title, and return
to Description checkpoint. This closes the pending active-field and cursor
navigation observation. The confirmation is attributed to the operator rather
than inferred from the earlier still images. Explicit discard of the unsaved
task draft and independent empty-task readback remain pending.

## Unsaved draft closure

Following the instructed Escape/confirmation sequence, the operator supplied a
screenshot showing the form closed, an empty Tasks view, and CURRENT.
Independent installed task-list readback confirms zero tasks at unchanged
revision 25 and watermark 25. The draft was not persisted. The final screenshot
establishes the closure outcome; appearance of the intermediate discard dialog
has not been separately confirmed. Two-client stale rename review is next.

## Discard confirmation and two-client preparation

The operator explicitly confirmed that Escape displayed the discard confirmation
and that answering y was required. This closes the earlier intermediate-dialog
observation gap. A supplied screenshot shows two native clients on Sessions,
both CURRENT, with matching five-row identities, labels, creation order, and
statuses. The operator identifies the upper-left window as client A and the
lower-right window as client B. A currently selects the third session and B
selects the first. Stale rename rejection and two-step revision review remain
pending; merely opening two clients is not conflict evidence.

## Concurrent rename rejection

Operator screenshots show client A holding the unsaved synthetic label
`Borrador SA` for the first session while client B saves `Guardado B` for
the same session identity. After the instructed single Ctrl-S in A, the
form displays the workspace-changed error and retains the complete draft.
This verifies visible stale-submit rejection and draft preservation.
Two-step Ctrl-R review and independent persisted-label readback remain
pending. No screenshot or private window-title information is committed.

## Two-step stale revision review

The first Ctrl-R screenshot shows the authoritative `Guardado B` label,
the same selected session, CURRENT, and the latest-state review footer.
The second Ctrl-R restores the unchanged `Borrador SA` draft and explicitly
states that the reviewed revision is selected and Ctrl-S would perform a
new submission. Independent installed CLI readback after both steps confirms
`Guardado B` remains persisted at revision 26 and watermark 26, with all five
session identities, creation ordinals, and statuses preserved. Neither review
step submitted the draft. The restored form still describes the original
label as `Prueba compartida`; this visible context text did not refresh even
though the review screen showed the authoritative value. Final draft discard
and subsequent readback remain pending.

## Reconciled draft discard

After the instructed Escape and confirmation checkpoint, the operator screenshot
shows the rename form closed, `Guardado B` selected, and CURRENT. Independent
installed CLI readback confirms revision 26 and watermark 26 remain unchanged,
with all five session identities, ordinals, and statuses preserved. This closes
the stale rename rejection, two-step review, and final discard sequence without
submitting the retained draft. Physical input ownership checks remain pending.

## Narrow-client initial input acquisition

The operator measured outer client A at 131 columns by 33 rows and client B
at 80 columns by 24 rows after exiting each TUI. Reopened client A shows the
same five sessions, the first selected, and CURRENT. Client B attaches to that
session with NAVIGATION and READ ONLY, then a single instructed i produces
INPUT and WRITER with a visible cursor. No intervening resize was requested.
An independent installed session-attach snapshot read after acquisition reports
78 columns by 17 rows (terminal snapshot revision 3), matching the narrow
client viewport. This uses live snapshot dimensions, not durable creation size.
Long-line wrapping, transfer in both directions, read-only resize isolation,
and explicit shell cursor recovery confirmation remain pending.

## Narrow writer wrapping and competing acquisition

Operator screenshots show the synthetic long command wrapping within the narrow
writer viewport before submission, with the cursor visible. After Enter, both
clients show its output and a new prompt; B remains INPUT/WRITER and A attaches
as NAVIGATION/READ ONLY. A subsequent instructed i in A leaves it read-only and
displays the complete competing-input warning and release guidance. Independent
installed session-attach readback after this rejected acquisition reports live
78x17 dimensions at terminal snapshot revision 7. The wide observer attachment
and rejected acquisition therefore did not change the narrow writer dimensions.
Explicit read-only window resize isolation and both transfer directions remain
pending.

## Read-only resize isolation

The operator reduced client A while it remained NAVIGATION/READ ONLY. The
screenshot shows CURRENT and the competing-input warning wrapping to the new
width. Client B is mostly obscured, so its current ownership label cannot be
independently read from this image. Installed session-attach readback after the
resize still reports 78 columns by 17 rows and terminal snapshot revision 7.
This verifies that the observed read-only resize did not resize the shared
terminal. Explicit release and transfer in both directions remain pending.

## Native Spanish keyboard input release

The first release-attempt screenshot still showed client B in INPUT/WRITER;
the physical combination used for that attempt was not established. The
operator subsequently reported Ctrl+AltGr+] as the working combination on
their Spanish keyboard in GNOME Terminal. The accompanying screenshot shows
B in NAVIGATION/READ ONLY, CURRENT, with the existing output and prompt
preserved. This verifies the native release transition for this configuration.
Acquisition by the wide client and transfer back remain pending.

## Wide-client acquisition after explicit release

After B visibly released input, the operator followed the instruction to press
i in maximized A. The screenshot shows A in INPUT/WRITER and CURRENT.
Independent installed session-attach readback reports live 144x28 dimensions
at terminal snapshot revision 14, replacing the previous narrow 78x17 size.
The latest maximized window differs from the earlier operator-measured 131x33
outer size; that earlier measurement is not used to assert an exact current
viewport match. New long-command wrapping in A and transfer back to B remain
pending. The older narrow-client output alone is not a wrapping test for A.

## Wide wrapping and return acquisition by the narrow client

The operator screenshot shows a newly entered synthetic long command using
the wide viewport and wrapping inside its right border, with a visible cursor.
Subsequent screenshots show output and a new prompt in A, followed by both
clients in NAVIGATION/READ ONLY and CURRENT after explicit release. After the
instructed single i in B, its screenshot shows INPUT/WRITER and CURRENT.
Independent installed session-attach readback now reports 78x17 at terminal
snapshot revision 17, compared with 144x28 under A. No post-acquisition resize
was instructed. Acquisition and live size changes have been observed in both
directions; a fresh long command after the return to B remains pending to
complete the wrapping check for that transfer.

## Fresh wrapping after return to the narrow writer

The operator screenshot shows the newly entered `M12-RETURN` synthetic command
wrapping inside the narrow panel after B reacquired input. B remains INPUT/WRITER
and CURRENT. Together with the previous live 78x17 snapshot readback, this
completes the fresh-command wrapping checkpoint for the return transfer. Initial
acquisition, transfers in both directions, competing acquisition rejection, and
read-only resize isolation have now been observed locally. This does not close
the separate physical SSH checks or the remaining native manual journey.

## Return command execution confirmation

The initial post-release screenshot showed READ ONLY but no output or new
prompt for the return command, so execution was not inferred. After the
operator was instructed to reacquire input and press Enter once, the next
screenshot shows the synthetic return output, a new shell prompt, and a
visible cursor in INPUT/WRITER. Command execution is now visually confirmed.
Detach and reattach remain pending.

## Local detach and reattach

After the release and Escape instructions, the operator screenshot shows B
back on the session list with the first session selected, running, and CURRENT.
Its session and instance identities match the earlier observations; all five
rows retain their order and statuses. Following Enter, the next screenshot
shows NAVIGATION/READ ONLY and CURRENT with the synthetic return-command
output and shell prompt retained. Local detach and reattach passed. Explicit
outer-shell cursor/input recovery confirmation after client exit remains open.

## Normal client exit terminal recovery

After the instructed q in read-only client B, the operator explicitly confirms
that the outer shell cursor is normal and typing works. The screenshot shows
the synthetic `M12-RECOVERY-OK` command output and a fresh shell prompt with
a visible cursor, establishing normal client-exit screen and input recovery.
Client A remained open. This observation does not establish recovery after
a failure or SSH connection loss; those separate checks remain pending.

## Reopen after normal client exit

Following the normal-exit recovery checkpoint, the operator reopened B using
the installed launcher and followed the selection and attach instructions for
the first session. The screenshot shows the retained synthetic return-command
output and shell prompt, NAVIGATION/READ ONLY, and CURRENT. This verifies
retained terminal output across client quit and reopen while A and the daemon
remained alive. It does not imply survival of a daemon restart.

## Task-neutral session claim

Operator screenshots show creation of the synthetic handover task in backlog
with normal priority and no owner, transition to ready, and then active after
selecting the first running session and claiming. CURRENT remains visible.
Independent installed task-list readback confirms one active task at revision
29 and watermark 30, owned by the first session instance (not its session ID).
That instance was previously launched without a task association. Competing
claim rejection, progress, handover, successor claim, and completion remain
pending.

## Competing task claim rejection

After instructions to select the third running session and attempt a competing
claim, the operator screenshot shows the Events rejection diagnostic and
CURRENT. The message is generic and does not itself identify a typed rejection
reason. Independent installed task-list readback confirms the task remains
active under its original instance owner, with revision 29 and watermark 30
unchanged. This verifies the observed competing attempt did not replace the
owner or mutate the task. Progress and handover remain pending.

## Progress persistence before handover

After the progress submission instruction, the operator screenshot shows the
form closed, task active under the original owner, and CURRENT. Independent
installed task-history readback at revision 30 confirms a progress entry with
the exact synthetic summary and explicit verification requested in this
checkpoint, at sequence 31, alongside the open original claim. The progress
entry agent-instance field is null; no instance attribution is inferred from
the selected session. The first history invocation lacked the required expected
revision and returned invalid_usage; the subsequent revision-bound read
succeeded. Handover and successor ownership remain pending.

## Immediate handover state before successor claim

The operator supplied the populated handover form and the resulting Tasks
view after the instructed single Ctrl-S without refresh, tab change, or a new
claim. The resulting view shows CURRENT, handover_ready, and Owner: unclaimed.
Independent installed task-list and revision-bound history reads, performed
before requesting any successor claim, confirm revision 31 and watermark 34,
null task ownership, the original claim closed with reason handover, preserved
progress, and all submitted synthetic handover fields saved exactly, including
empty changed paths. The handover source-instance field is null for this local
user submission; claim closure separately identifies the original owner.
The handover history is not visible in the supplied top-of-detail screenshot;
physical history navigation, successor claim, and completion remain pending.

## Physical handover history review before successor claim

Following PageDown instructions, the operator supplied successive Tasks detail
views showing the original claim closed for handover, the saved progress summary
and verification, and all structured handover fields, including the empty
changed-paths section and recommended next action. The task remains
handover_ready and CURRENT throughout these views. This establishes physical
forward history navigation and readable saved handover content before a new
claim. Reverse navigation and successor ownership/completion remain pending.

## Successor claim with preserved history

Following the instructions to return to the top using PageUp, select the third
running session, and claim, the operator screenshot shows active and CURRENT
with the third session instance as owner, distinct from the original instance.
Independent installed task-list and revision-bound history reads confirm
revision 32 and watermark 36, a new open successor claim, the original claim
closed for handover, and the unchanged progress and structured handover.
Physical history review after the successor claim and task completion remain
pending. The screenshot shows the resulting top view; it does not independently
isolate the PageUp key action from the intervening tab changes.

## Successor history review and completion

Operator screenshots under successor ownership show the original closed claim,
the successor open claim, retained progress and verification, and all handover
fields, with active and CURRENT visible. Following the instructed d, the next
screenshot shows done, unclaimed, and CURRENT while the saved handover remains
visible in the scrolled detail. Independent installed task-list and
revision-bound history reads confirm revision 33, watermark 38, null ownership,
the successor claim closed with reason completion, and unchanged prior progress
and handover content. Post-completion client quit/reopen verification remains
pending.

## Completed task after client reopen

The operator supplied the outer-shell view after exiting B and the Tasks view
after the instructed relaunch. The reopened view shows the synthetic task
still done, Owner: unclaimed, and CURRENT. This closes the post-completion
state check across client quit/reopen. History retention was independently
verified before exit; the reopened top-of-detail screenshot alone does not
show that history. Backup/restore and focused physical SSH checks remain open.

## Startup warning confirmation and Help at minimum size

The operator explicitly reports no warning appeared during any observed
Relayterm startup. The supplied Help screenshot at the retained 80x24 window
shows all six tab labels, CURRENT, and the Relayterm footer and shortcuts.
The lower portion of the help text is outside the visible panel; this image
does not establish access to all help content. Together with earlier screen
observations, all six tabs have now been visited physically.

Before the physical SSH sequence, a new batch-mode loopback connection using
the existing synthetic identity and pinned host key returned the synthetic
readiness marker successfully. The temporary service remains available under
the earlier authorization. This connectivity check is not physical SSH TUI
acceptance.

## SSH Unicode draft and stale revision review

Following the SSH launcher instructions, client B displays the same five
sessions and CURRENT. The operator supplied Unicode rename-form images before
and after removal of an inserted character, with wide and accented text
retained and a visible cursor at the end. Dynamic cursor correctness has not
yet been explicitly confirmed, and screenshots do not establish code points.
While B retained its unsaved draft, local A saved `Guardado local` for the same
first session. B then displayed the workspace-changed rejection with its draft
preserved. The first Ctrl-R showed the authoritative label and review footer;
the second restored the Unicode draft and explicit resubmission guidance.
The restored form still displays the old `Guardado B` context label.
Independent installed readback after both review steps confirms the persisted
label remains `Guardado local`, revision 34 and watermark 39, with session
identities, ordinals, and statuses preserved. Final discard, dynamic Unicode
cursor confirmation, SSH ownership transfers, and connection-loss recovery
remain pending.

## SSH cursor confirmation and reconciled draft discard

The operator explicitly confirms that the cursor behaved normally and remained
visible throughout the requested SSH Unicode editing checkpoint. This closes
the earlier dynamic cursor observation gap without inferring code points from
images. After the instructed Escape and discard confirmation, the supplied
screenshot shows the form closed, `Guardado local` selected, CURRENT, and
unchanged visible session and instance identities and statuses. SSH Unicode
cursor behavior and the stale rename review/discard sequence are now observed.
SSH input ownership, size transfer, and connection-loss recovery remain pending.

## SSH initial input acquisition

The operator supplied consecutive screenshots of B attaching in NAVIGATION/
READ ONLY and then acquiring INPUT/WRITER, both with CURRENT and retained
terminal output. A visible cursor appears at the prompt in input mode.
Independent installed session-attach readback after acquisition reports live
78x17 dimensions at terminal snapshot revision 21. Fresh command wrapping,
competing input, transfers, and connection-loss recovery over SSH remain
pending; the existing output is not a new SSH input test.

## SSH writer wrapping and competing local acquisition

The narrow SSH client screenshot shows a newly entered synthetic SSH command
wrapping inside its viewport with a visible cursor. Subsequent local A
screenshots show that command output and a fresh prompt, first in READ ONLY
and then with the competing-input warning after the instructed i. CURRENT
remains visible. Independent installed live snapshot readback after rejection
reports 78x17 at terminal revision 25. Attaching the wide local observer and
rejecting its acquisition did not change the SSH writer dimensions. SSH
read-only resize isolation, ownership transfer, and connection loss remain
pending.

## Read-only resize while SSH owns input

The operator reduced local observer A while SSH client B was instructed to
retain input ownership. The screenshot shows A still NAVIGATION/READ ONLY
and CURRENT, with the competing-input warning reflowed to the reduced width.
Independent installed live snapshot readback remains 78x17 at terminal
revision 25. The observed local read-only resize did not change the shared
terminal dimensions during the SSH writer sequence. Transfers involving SSH
and connection-loss recovery remain pending.

## SSH-to-local writer transfer

The operator screenshot after the SSH release instruction shows B in
NAVIGATION/READ ONLY with CURRENT and retained output. Following the instructed
i in maximized local A, its screenshot shows INPUT/WRITER with a visible
prompt cursor and CURRENT. Independent installed live snapshot readback
reports 144x28 at terminal revision 33, compared with 78x17 under SSH B.
Fresh wrapping after this transfer, the return transfer to SSH, and
connection-loss recovery remain pending.

## Fresh wide wrapping and return transfer to SSH

The operator screenshot shows a fresh synthetic command wrapping at the wide
local viewport after SSH-to-local acquisition. The next screenshot shows its
output, a new prompt, and NAVIGATION/READ ONLY after release. Following i in
SSH B, its screenshot shows INPUT/WRITER and CURRENT. Independent installed
live snapshot readback confirms 78x17 at terminal revision 39, versus 144x28
under local A. The narrow screenshot clips older content and places the cursor
over an older displayed command near the bottom; this is recorded without
inferring fresh input correctness. Fresh wrapping after return to SSH and
connection-loss recovery remain pending.

## Fresh input after return to SSH

The operator screenshot shows the fresh synthetic SSH return command wrapping
inside the narrow viewport with a visible cursor at its end. After the
instructed Enter, the next screenshot shows the command output and a new prompt
while B remains INPUT/WRITER and CURRENT. Together with live snapshot checks,
fresh input wrapping has been observed after transfers in both directions
between local A and SSH B. This does not resolve the separately recorded
older-content clipping/cursor observation during shrink. Connection loss,
server-side lease release, reconnection, and terminal recovery remain pending.

## SSH disconnect attempt did not close the connection

The operator reported arrow keys displaying escape characters. The supplied
screenshot still shows INPUT/WRITER and CURRENT inside Relayterm, with a
spaced tilde/dot sequence and a visible left-arrow escape sequence at the child
shell prompt. The connection-loss checkpoint has not occurred and outer-shell
recovery is not established. The session was launched with /bin/sh; host
inspection resolves that path to Dash. Child-shell line editing must not be
confused with navigation keys in the Relayterm forms or outer Bash.

## Scoped SSH connection interruption

The SSH escape attempt also remained literal when entered without a visible
space; no successful escape disconnect is claimed. After the operator cleared
the line while B remained WRITER, the assistant identified the single SSH
client by its exact temporary key, loopback destination, port, and launcher.
SIGTERM was sent only to that verified client, without Relayterm quit or input
release. A subsequent process check shows the SSH client and its server-side
connection processes gone, with the temporary listener retained. Installed
session-list readback confirms unchanged session/instance identities and
statuses at revision 34: four running, one previously terminated. This is a
client-process interruption, not a network outage simulation. Physical outer
terminal recovery, lease reacquisition, and SSH reconnection remain pending.

## Failed visual recovery after SSH client interruption

After the scoped SSH client SIGTERM, the operator ran the synthetic recovery
command. The screenshot shows its output and the local shell prompt, but
Relayterm borders, header, prior terminal content, and input-mode footer remain
on screen. The local prompt overlaps the stale TUI. Command execution recovered;
clean visual/alternate-screen restoration did not pass. The stale WRITER label
is residual screen content, not evidence of a live input lease. The cause is
not assigned to Relayterm or OpenSSH without further diagnosis. Manual terminal
reset is a recovery workaround and must not retroactively turn this result
into a pass. Lease reacquisition and reconnection remain pending.

## Manual screen reset and lease reacquisition after SSH loss

The operator screenshot after the instructed reset shows a clean local shell
view with the stale TUI removed. This establishes manual visual recovery only,
not automatic recovery or a fresh dynamic cursor confirmation. Following the
instruction to press i in surviving local client A, its screenshot shows
INPUT/WRITER and CURRENT with prior SSH command output retained. The input
lease became available after the scoped SSH client interruption, without
explicit release in the interrupted Relayterm client. SSH reconnection and
post-reconnection identity checks remain pending.

## SSH reconnection preserves identity and terminal context

After the scoped interruption and manual terminal reset, the operator reopened
B through the SSH launcher. The session-list screenshot shows all five rows
with unchanged order and statuses, CURRENT, and the same full session and
instance identities for the selected first running session. Following Enter,
the next screenshot shows NAVIGATION/READ ONLY, CURRENT, retained synthetic
SSH return-command output, and shell prompts while A retains input ownership.
This verifies reconnection and session context survival for the tested SSH
client interruption. Automatic visual recovery after that interruption remains
failed and is not superseded by reconnection success. Normal SSH exit recovery
and diagnosis of the interruption recovery issue remain pending.

## Normal SSH exit and live backup preparation

The operator screenshot following q shows the SSH connection closed, the
synthetic normal-exit marker executed in the local shell, and a clean prompt
without stale TUI content. Visual restoration and command input pass for
normal SSH exit; fresh explicit dynamic cursor confirmation was not supplied.
This does not supersede the interrupted-client visual recovery failure.

The assistant created a live backup through the exact installed candidate in
a new private destination while the original daemon remained active. The
command succeeded with format 1, schema 3, revision 34, and event watermark 39.
Read-only SQLite integrity_check returned ok. Database SHA-256:
`9eb62023bebe1168116cc1ac8ea96a2c8ba8c7569f0c0ed13df40244c9d1d3ed`.
The backup and original home are retained outside version control. No daemon
stop or session termination has been performed for restore; explicit operator
agreement is required before that next step. Restore remains pending.

## Authorized stop and fresh-home restore

The operator explicitly authorized daemon stop and the operations required
for remaining validation. The installed candidate stopped the original daemon
with terminate-sessions successfully. The verified live backup was restored
into a new private home and the workspace explicitly opened there. Original
home and backup remain retained. Installed readback reports revision 35 and
watermark 43: the four formerly running instances are lost, the previously
terminated instance remains terminated, and all five session/instance IDs,
ordinals, and names match. The task remains done with no owner.

Read-only database comparison against the backup establishes exact row equality
for tasks, both claims, progress, handover, presentation metadata, definitions,
and launch definition/argument/environment/capability tables. All 39 original
events remain with four recovery events added. The worktrees table is empty
in both, so this fixture does not establish restoration of populated worktree
metadata. Restored SQLite integrity_check is ok and the backup database hash
is unchanged. Physical restored TUI observation remains pending.

## Physical restored workspace verification

The restored Sessions screenshot shows four lost instances, the previously
terminated instance unchanged, stable selected session/instance identities,
creation order, preserved label, and CURRENT. Subsequent Tasks screenshots
show done and no owner, both claims closed with their original handover and
completion reasons, and retained progress and structured handover fields.
The operator explicitly confirms forward and backward page navigation works;
the final screenshot returns to the top of task detail. Together with the
prior exact database comparisons, this completes the restored fixture check.

Source inspection of the candidate TUI shows explicit SIGINT/SIGTERM handling
and terminal restoration on normal return, but no explicit SIGHUP handler in
the inspected TUI sources. This is diagnostic context, not proof of the cause
of the interrupted SSH client screen residue: after transport loss, remote
cleanup output may not reach the local terminal. No production change or
passing interruption-recovery retest is claimed.

## Original client display after authorized daemon stop

The operator supplied untouched local A after the original daemon was stopped.
Its header correctly shows DISCONNECTED, but the terminal title still shows
INPUT/WRITER, the body says Attaching..., and the footer retains the input-mode
escape hint. These stale ownership/attachment indicators are recorded as a
UI consistency issue, not proof of a live lease. The original daemon stop was
confirmed independently; the restored daemon uses a separate home. Exit and
outer-terminal recovery from this disconnected state remain pending.

## Disconnected exit and unexpected Events navigation

The operator explicitly reports that Ctrl+AltGr+] moved disconnected A to
Events without pressing 5. The screenshot shows DISCONNECTED and many repeated
transport-loss diagnostics. The operator then pressed q and supplied a clean
outer-shell screenshot plus successful synthetic exit-marker output. Exit
from the disconnected client and outer command execution passed. The unexpected
tab switch and repeated diagnostics remain separate unresolved observations;
no unobserved keypress is attributed to the operator. Source inspection shows
input release can report transport errors, but does not establish the cause
of this tab change without native input-event evidence.

## Empty task title validation

The operator supplied before-and-after screenshots of submitting the restored
workspace Create task form with an empty title. The second view displays
`Error: Title is required.` while retaining the open form, empty fields, and
CURRENT header. Physical empty-title rejection and form retention passed.
This checkpoint does not independently establish dynamic cursor behavior after
the error; forward Delete editing remains to be observed. No screenshot or
private terminal title is included in repository evidence.

## Editing confirmation after validation failure

The operator explicitly reports normal cursor movement, deletion, insertion,
and successful Ctrl-U clearing in the title field after the validation error.
The supplied images show AXBC with a four-character count and then an empty
title with the cursor at its start. Clearing and continued editing are
confirmed. The requested forward-Delete sequence would have produced AXC,
so the exact forward-Delete checkpoint remains open for a focused observation;
the discrepancy alone is not classified as a product defect.

## Forward Delete confirmed

The operator clarified that the earlier AXBC image followed free insertion
and deletion rather than the prescribed sequence. For the focused retest,
the supplied images show ABC with count three followed by BC with count two
after Home and forward Delete. Forward deletion passed. Together with the
prior explicit cursor, insertion, and Ctrl-U confirmation, this closes the
remaining title-editing checkpoint. The earlier discrepancy is not a defect.

## Documented Ctrl-Space detach check failed

After launching a fresh shell in the restored workspace, the operator reports
that Ctrl-Space has no visible effect. The screenshot retains CURRENT and
INPUT/WRITER. The installed quick start explicitly prescribes Ctrl-Space for
detach, so that documented checkpoint failed. Inspection of the exact 9cf91f7
TUI key handler confirms that input focus intercepts Ctrl-] or Ctrl-5 for
release and forwards other encodable keys to the child; navigation uses Esc
for detach. There is no Ctrl-Space detach binding in that handler. This is
a documentation/implementation mismatch, not an operator mistake. The exact
bytes delivered by the physical terminal were not captured. Production code
and the quick start remain unchanged pending the final findings review.

## Release and detach confirmed on the restored workspace

The operator explicitly confirms Ctrl+AltGr+] as the working release chord
on the observed Spanish keyboard. Sequential screenshots show NAVIGATION /
READ ONLY followed by the Sessions list after Esc, with Session 6 still
running and CURRENT. The implemented release-and-detach sequence passed.
This does not supersede the failed documented Ctrl-Space shortcut or the
unexpected Events transition previously observed while disconnected.
