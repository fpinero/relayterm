# M12 Windows native observations

## Scope and source

Validation started on 2026-09-20. Corrected production source:
`d7605541bfa829cfab6d8a3c53b87ad6e0f7b4fa`. The fetched evidence branch ended at
`a541832`; its diff from the corrected source contains only TODO, logbook, and
evidence documentation. The original `491d5f1` handover failure and the macOS
impact mapping remain preserved in their existing records.

Work proceeds on a dedicated Windows evidence branch. Two clean detached clones,
fresh build directories, packages, and private runtime state are outside Git.
Physical terminal observations must be supplied by the operator and are recorded
separately from assistant-driven checks. The local Windows journey and affected
retests are complete as recorded below; global M12 acceptance remains open.

## Environment inspected by the assistant

- Windows 10 Pro 22H2, x64, build 19045.7725.
- PowerShell 7.6.6; Windows PowerShell 5.1.19041.7725 is also installed.
- Windows Terminal package 1.24.11911.0 is installed. The operator confirmed
  Windows Terminal through WT_SESSION, PowerShell 7.6.6, a physical US keyboard,
  and ENG INTL layout.
- cmd.exe file version 10.0.19041.1, OS version output 10.0.19045.7725.
- Rust 1.98.1 (`48a229cea`), Cargo 1.98.1 (`797e8a9bc`), LLVM 22.1.8,
  native target `x86_64-pc-windows-msvc`.
- Visual Studio 2019 Build Tools, MSVC tools directory 14.29.30133,
  compiler 19.29.30158, linker 14.29.30158.0, Windows SDK 10.0.19041.0.
- Visual C++ x64 runtime registry reports installed version v14.42.34433.00;
  the system VCRUNTIME140.dll file reports 14.44.35211.0. PE imports and
  extracted execution are verified below.
- Python 3.12.10 is used for build and packaging tooling, not product runtime.
- Existing PowerShell execution policies are RemoteSigned at user and machine
  scope. No execution policy or reputation protection has been changed.

## Assistant-driven artifact and installation checks

Both clean clones built successfully with the checked-in release wrapper,
`--target x86_64-pc-windows-msvc`, separate fresh output directories, and
`--offline`, after `cargo fetch --locked --target x86_64-pc-windows-msvc`.
Version remains 0.1.0 with default empty production features.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Executable | 11,880,448 | `df4f0b7285432a57855aee862aad71879af416bfb4e301aac532fca05298b9f8` |
| ZIP archive | 4,511,930 | `b4030f4a7dc6c97ad1d69a33265635d437bfa5aa6b92dd5bc2f61271640ec652` |

`build_release.py compare` reported identical binary records.
`package_release.py compare` reported identical archive records. The package
inspector confirmed exactly nine expected regular entries. PowerShell
`Get-FileHash -Algorithm SHA256` matched both archive and external manifest
against SHA256SUMS before retained extraction. Both build records, packages,
manifests, checksums, and native PE inspection are retained outside Git.

Native `dumpbin /headers /imports` confirmed x64 PE, linker version 14.29,
VCRUNTIME140.dll, Universal CRT API sets, and Windows system DLLs. SQLite does
not appear as a separate imported DLL. Authenticode status is NotSigned.
`smoke_release.py` passed extracted help, version, workspace initialization,
detached daemon startup, a real synthetic ConPTY shell with clean exit, and
orderly shutdown with only Windows system directories on the product PATH.
This is an audited developer-host runtime check, not an independent clean VM.

The packaged PowerShell helper installed into a new dedicated user-local
directory without changing PATH or shell profiles. Installed SHA-256 matches
the reviewed binary. Absolute invocation passed help and version in PowerShell
and version in cmd.exe. Assistant-shell discovery found no existing rt command;
cmd.exe `where rt` returned exit 1. Operator-shell discovery remains separate.

A repeated install refused the existing candidate and preserved its hash.
In a temporary process-only PATH, a synthetic unrelated rt.exe resolved first;
the helper refused that destination and preserved its checksum. A separate
installation into a path containing spaces and a Greek letter also preserved
the candidate hash and passed absolute version invocation. No daemon has been
launched from that disposable copy. Scoped removal later verified its exact
inventory and candidate hash, rejected reparse directories, and checked that no
process was running from that executable. Removing only that executable and its
empty directory succeeded. The retained candidate hash was unchanged and an
independent restored-task read still returned done with no owner.

The operator reported no Windows security or permission warnings throughout
installation and execution, and described the experience as smooth. This is an
observation on this developer host, not a signed-artifact or clean-machine claim.

Release-tooling validation passed `python -m unittest discover -s scripts
-p 'test_*release.py' -v`: 18 tests passed and four POSIX-only tests were skipped.
Formatting, repository checks, audit negative controls, candidate secret checks,
and `git diff --check` passed during preparation.

The exact installed executable passed
`RELAYTERM_TEST_RT=<installed-candidate> cargo test -p relayterm-cli
--test session_presentation --test tui_gate --test backup_restore --locked
--offline -- --nocapture --test-threads=1`: one backup/restore test, one session
presentation test, and six TUI tests passed; one fixture helper was ignored.
The harness was compiled from the clean corrected-source checkout with the
pinned toolchain. Startup p95 was 158 ms, navigation p95 21 ms, input-to-rendered
echo p95 126 ms, and flood throughput 2,642,888 bytes/s. These are automated
native ConPTY results, not operator observations.

Corrected-source regressions passed offline with the pinned lockfile:
`cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol
-p relayterm-persistence-sqlite -p relayterm-tui --locked --offline`, followed
by `cargo test -p relayterm-daemon --test protocol_gate --locked --offline
-- --nocapture` (three passed, one process helper ignored). These include the
task-neutral handover projection regression and two-client snapshot refresh
before continuation. Both artifact source clones remain clean.

The exact installed executable also passed both tests in
`cargo test -p relayterm-cli --test worktree_gate --locked --offline
-- --nocapture --test-threads=1` with RELAYTERM_TEST_RT set. Git version was
2.47.1.windows.2. The TUI scenario verified two tasks and worktrees, three
sessions, two claims, and daemon restart; the administrative scenario verified
isolation, retained metadata, and the independent non-Git workflow. This is
automated evidence for the optional quick-start worktree path.

## Operator observations

The operator confirmed PowerShell 7.6.6 and Windows Terminal. PowerShell command
discovery returned no rt resolution before interactive startup. Physical keyboard
and active layout were reported as US and ENG INTL. The optional language-list
query failed with a marshaling error; layout evidence comes from the operator,
not from that failed query. No system configuration was changed.

The operator successfully initialized and opened the installed candidate in the
prepared disposable workspace. Independent administrative readback confirmed a
ready daemon, revision 1, and zero definitions, instances, and tasks. An
operator-supplied image confirmed a readable overview, CURRENT, revision 1,
and the same empty collections. The image was inspected only in conversation
and was not copied into the repository. Platform warning observations remain
to be confirmed.

The operator observed a visible cursor in the agent form, entered a neutral
cmd.exe definition with /Q and terminal capability, saved it disabled, then
enabled and launched it. The Sessions view showed Session 1, running, and
CURRENT. Full session and instance IDs were distinct and matched independent
installed `session list` readback at revision 5 and event watermark 5. Readback
also confirmed a running task-neutral instance with the intended launch command
and arguments. No image or raw administrative output was added to Git.

During the next operator checkpoint, six running sessions were present rather
than the intended three. Ordered administrative readback at revision 28 and
watermark 28 confirmed unique creation ordinals 1 through 6. The first three
had custom session names and retained their identities; the last three had no
custom names and correctly displayed Session 4, Session 5, and Session 6.
The first three launch snapshots referred to the same original definition.
Three definitions now existed, with the original definition renamed separately.
This does not establish three distinct-definition launches. The guidance had
omitted explicit selection of the intended definition after saving. No session
was terminated or deleted; subsequent checks continue on identified rows.

The operator cleared the second session's custom name with Ctrl-U and Ctrl-S.
The displayed label returned to Session 2 with the same abbreviated identity,
running status, and second-row position. The operator then renamed the first
two sessions to the same synthetic name. Both remained distinguishable by their
IDs and retained their positions. Independent ordered readback at revision 31
and watermark 31 confirmed duplicate names, unchanged full session and instance
identities, creation ordinals 1 and 2, and all six sessions still running.

In an unsaved task form, the operator confirmed visible cursor alignment during
ASCII, wide-character, and combining-accent editing, arrow navigation, Home,
End, insertion, and Backspace. Multiline description input and Tab/Shift-Tab
field navigation worked. Resizing below the minimum showed the documented
guidance at an observed 73 by 43 cells. Enlarging restored the form with its
draft and active field preserved. Exact 80 by 24 operation has not yet been
independently established. The operator then pressed Escape, observed the
explicit discard confirmation, and pressed y. The form closed and Tasks
remained empty with CURRENT. Independent installed task-list readback confirmed
zero tasks at revision 31 and watermark 31.

Two operator-controlled clients displayed the same six sessions. Client A kept
a rename draft open while client B saved a different name on the first session.
A's submission was rejected with the expected stale-workspace guidance and
preserved the draft. The first reported Ctrl-R hid the form and exposed B's
saved value. After the second reported Ctrl-R, the draft returned, but the
message was `State refreshed. The draft revision was not changed.`, rather
than the explicit reviewed-revision adoption message. Source inspection places
this message in the refresh path for a form that is no longer stale or uncertain;
the precise input sequence remains unverified. This checkpoint is not counted
as passing two-step adoption. The controlled repetition follows below. No draft was
resubmitted and no product correction has been made on this evidence alone.

After discarding the first draft, the operator repeated the conflict with a new
draft and a new saved value. Submission was rejected and preserved the draft.
One brief Ctrl-R hid the form and exposed the saved value; a second brief Ctrl-R
restored the unchanged draft with `The reviewed revision is now selected.
Ctrl-S performs a new explicit submission.` The controlled two-step adoption
passed. Independent installed readback at revision 33 and watermark 33 still
showed B's saved value and the original first-session identity, confirming that
review and adoption did not submit the draft. The earlier unexpected message
was not reproduced and its cause remains unconfirmed. The operator then
discarded the draft and confirmed the saved value remained unchanged.

Client A attached to the first session, acquired INPUT / WRITER, and executed
a synthetic echo marker successfully. The operator reported that the command
line was clipped on the right instead of visibly wrapping in the narrower
window. Administrative instance readback reported the original
148-column by 38-row size. Source inspection found that successful input
acquisition does not call the existing lease-owned resize path. A size mismatch
is the working explanation. After resizing while holding the writer lease, the
operator observed the next command wrap visibly onto a second line. The image
shows the command still at the input cursor, not its execution result. Instance
readback still reported the original dimensions after this resize; that stored
field is not independent proof of current live PTY dimensions.
The usability issue is tracked as M12.WIN-SIZE, without treating successful echo
output as proof of correct command-line rendering.

The operator then executed the resize marker and attached client B to the same
session. B's input acquisition was rejected with the expected inline competing
owner guidance; B remained NAVIGATION / READ ONLY while A retained INPUT /
WRITER. Both received the marker output. The physical Ctrl-] chord released A's
lease on the US ENG INTL keyboard. Explicit i in B acquired INPUT / WRITER and
cleared the warning; A remained read-only. A new marker entered in B appeared
in both clients. Command-line clipping recurred in the narrower B window after
writer transfer while A displayed the complete command. This adds a second
native observation to M12.WIN-SIZE; input exclusion and explicit transfer passed.

The operator released B's input and detached both clients. Both session lists
still showed all six sessions running. Quitting each client with q restored
normal PowerShell input and output; a synthetic shell marker succeeded in both
windows. Independent installed status and ordered-session reads after both
clients exited confirmed the same daemon generation, revision 33, all six
running sessions, and unchanged session identities, names, and creation order.
The operator reopened client A, observed the same six running sessions with
CURRENT, and reattached to the first session in READ ONLY. Earlier markers,
including the marker entered by client B, remained visible.

The operator created a synthetic task, observed backlog and unclaimed state,
made it ready, selected the first task-neutral session, and claimed it. The
task became active with the corresponding instance as owner and CURRENT.
Selecting the second running session and attempting another claim preserved
the original owner. Events showed the rejected-operation diagnostic. Independent
installed task readback confirmed active state and the first instance owner at
revision 36 and watermark 37.

The operator saved progress with a summary and explicit verification, then
submitted a structured handover from the original task-neutral session. Before
any successor claim, the view immediately displayed handover_ready, unclaimed,
the claim closed for handover, retained progress, and the saved handover with
CURRENT, without a repair refresh or client restart. Independent installed task
readback confirmed handover_ready and a null owner at revision 38 and watermark
41. Revision-pinned task history readback succeeded. The handover verification
field contained an operator-entered local path instead of the suggested test
text; that content remains private and is not reproduced here. The visible
corrected projection boundary passed.

The operator selected the second running session and claimed the handed-over
task. It immediately became active with a different instance owner, retained
progress and handover history, and CURRENT. Pressing d completed the task,
cleared the owner, and closed the second claim for completion. Independent
installed task and revision-pinned history readback at revision 40 and watermark
45 confirmed done, null owner, two distinct instance claims closed for handover
and completion, one progress entry, and one handover. The operator quit and
reopened the client, then confirmed done, unclaimed, retained progress and both
closed claims, retained handover, and CURRENT.

A live backup attempt under the home root was rejected. Inspection found that
the parent ACL was inherited rather than protected. Retrying under the existing
private data directory succeeded without changing permissions. The published
backup contains only manifest.json and workspace.sqlite3. Its complete manifest
records schema 3, revision 40, and event watermark 45. An independent session
read after backup still reported all six sessions running at the same revision
and watermark.

The operator explicitly authorized this and subsequent necessary daemon stops
for the validation. The installed candidate stopped the original daemon with
session termination; an independent daemon status read confirmed stopped.
Restore into a previously absent home under the private data directory succeeded
at backup revision 40 and watermark 45. Opening only the restored home recovered
all six formerly running sessions as lost, at revision 41 and watermark 51.
Installed task and revision-pinned history reads retained done, no owner, both
claims closed for handover and completion, one progress entry, and one handover.
SHA-256 comparisons of both backup members before and after restore were equal.
The original home and backup were retained. The operator opened the restored
client and supplied visual confirmation of done, unclaimed, CURRENT, both closed
claims, saved progress, and the retained handover. The operator then confirmed
all six sessions as lost, with retained names, stable session IDs, creation
order, and CURRENT. The live-backup and fresh-home restore journey passed.

The operator resized the restored Sessions view through several widths and
heights. The list and detail panels remained readable, and reducing the terminal
to 56 by 34 cells displayed the minimum 80 by 24 guidance. Enlarging restored
the session list, selection, names, and lost states. The rightmost CURRENT label
was partially clipped at narrower supported widths. These observations verify
resize recovery and below-minimum guidance, but do not establish an exact
80 by 24 physical observation or resolve live command-line clipping on writer
acquisition.

The operator subsequently measured exactly 80 by 24 cells in PowerShell and
opened Tasks and Sessions at that size. Both screens rendered, but CURRENT was
clipped in the header. This confirms a minimum-size header defect in the retained
d760554 candidate. A subsequent source correction moves the product name to the
footer and reserves a separate header region for freshness. Render regression
coverage exercises all six screens and eight freshness labels at 80 by 24.
The installed candidate has not been replaced; corrected-artifact native retest
remains pending.

## Prepared header candidate

After the operator requested candidate preparation and branch publication, two
new clean detached clones built source
`0bc4b6384adf37dbdd9939e840d9c3e14e86d9e1` offline for
`x86_64-pc-windows-msvc`. The same inspected Windows, MSVC, SDK, and Rust/Cargo
environment above was used. Build comparison confirmed equal executables:
11,882,496 bytes, SHA-256
`8a6e231f253c83321aaabbd97318a613e13c68d56eb3859c1f204aa0436b1fd4`.
Package comparison confirmed equal ZIP archives: 4,511,842 bytes, SHA-256
`e3008b927f077b768c261a2fe0568e8a0a73db96d8492293640d03d5b86a9128`.
Version is 0.1.0, default empty production features, unsigned.

External manifest and archive checksums passed before extraction. The inspector
confirmed exactly nine expected members. Native PE inspection confirmed x64,
linker 14.29, VCRUNTIME140.dll, and UCRT dependencies; Authenticode reported
NotSigned. Extracted smoke passed help, version, initialization, detached daemon,
real ConPTY child, and orderly shutdown. The packaged helper installed a separate
copy; installed SHA-256, help, and version passed. The previously tested install,
original runtime home, restored home, and backup were preserved. Build records,
archives, manifests, logs, and native inspection remain outside version control.

The header change passed all 24 TUI tests and Clippy with denied warnings.
With RELAYTERM_TEST_RT set to the new installed executable, the clean source
passed `cargo test -p relayterm-cli --test session_presentation --test tui_gate
--test backup_restore --test worktree_gate --locked --offline -- --nocapture
--test-threads=1`: ten tests passed and one fixture helper was ignored. Startup
p95 was 206 ms, navigation p95 21 ms, input echo p95 126 ms, and flood throughput
2,431,049 bytes/s. The worktree scenarios exercised two isolated worktrees,
three sessions, two claims, restart recovery, and the non-Git boundary.
Its affected physical 80 by 24 observation remains open. The prior operator
report of no platform warnings applies to the prior installation; it is not
relabelled as an observation of this new install.

The operator then opened the installed 0bc4b63 candidate at the requested
80 by 24 size and confirmed the full CURRENT label, all six tabs, and Relayterm
with keyboard hints in the footer in Overview, Tasks, and Sessions. This closes
the affected Windows header observation for that candidate.

The operator reported OpenSSH installed on both Windows and the Linux host from
prior M11 work. Read-only local inspection found OpenSSH for Windows 9.5p2,
LibreSSL 3.8.2, with sshd stopped and configured for manual startup; no local
port 22 listener was observed. No service configuration was changed. Installed
OpenSSH alone does not establish M12 SSH acceptance. The Linux service and
authorized connection still need verification on that host.

## Writer-size source correction

The input-ownership integration scenario now reads live display snapshot
dimensions while transferring input between differently sized native clients.
It expects 21 by 88 on initial acquisition, 17 by 78 after transfer, and 28 by
118 after transfer back, while rejected and read-only clients preserve the
writer's dimensions. Against the installed 0bc4b63 executable, the regression
failed waiting for the first resize. Adding the existing lease-owned resize
operation immediately after successful acquisition made the scenario pass.

The complete TUI integration gate then passed six tests with one helper ignored;
all 24 TUI unit tests and Clippy for TUI and CLI with all targets and denied
warnings passed. This is source-level automated evidence. Release rebuilding
and physical observation of newly entered commands remain separate gates.

## Prepared writer-size candidate

Two clean detached clones at `9cf91f7164235d35beda81d9c8c2d4200b703aad`
built offline with the same native toolchain and four Cargo jobs per copy.
Both executables are 11,803,648 bytes, SHA-256
`65da79d61ba031cfef02c3971ae65a35d706e88c743d0f68fc8139f334325045`.
Both normalized ZIP archives are 4,497,240 bytes, SHA-256
`0409cc32358acd4cd2526309ddcc5a3b8246d8abce13263df704c2e2120c3d4c`.
Build and package comparisons passed. Version is 0.1.0 with empty default
production features and no signature.

External manifest and archive hashes passed before extraction. The package
inspector confirmed the exact nine-member inventory. PE inspection confirmed
x64, linker 14.29, and VCRUNTIME140.dll; extracted ConPTY smoke passed help,
version, initialization, detached daemon, real child, and orderly shutdown.
A separate packaged-helper installation passed help, version, executable hash,
and unsigned Authenticode checks. Earlier candidates and runtime homes remain
untouched. The remaining physical writer-size observation must use this candidate.

The exact installed binary passed `cargo test -p relayterm-cli --test
session_presentation --test tui_gate --test backup_restore --test worktree_gate
--locked --offline -- --nocapture --test-threads=1` through RELAYTERM_TEST_RT:
ten tests passed and one fixture helper was ignored. This includes the new
live-dimension input-transfer regression. Startup p95 was 186 ms, navigation
p95 21 ms, input echo p95 146 ms, and flood throughput 2,495,711 bytes/s.
Both clean source clones remained unchanged. A new disposable writer-test
workspace was initialized with this installed candidate, separately from the
retained restored workspace, for the operator's next checkpoint.

The operator opened a wide client A and a measured 80 by 24 client B on the
same new session. Without corrective window resizing, initial acquisition in B
wrapped a newly entered long command completely and produced the expected
synthetic output. After explicit release, A acquired input and displayed its
complete new command while B stayed read-only. After another explicit release,
B reacquired input and again wrapped and executed a complete long command while
A stayed read-only. Both views retained CURRENT. Independent installed live
snapshot readback after the return to B reported 17 rows by 78 columns, matching
the inner viewport of the 80 by 24 client. First acquisition and both transfer
directions passed physical Windows observation. The operator then resized only
read-only client A while B retained input. B's newly entered long command remained
complete and wrapped correctly, its expected output appeared in both clients,
and both retained CURRENT. A remained read-only. Independent installed snapshot
readback still reported 17 rows by 78 columns. Read-only resize isolation passed.

## Pending evidence

No further physical Windows checkpoint is pending for the changes tested here.
The native journey uses d760554 evidence for unchanged behavior, 0bc4b63 for the
header observation, and 9cf91f7 for writer-size acceptance and installed gates.
Linux native acceptance, affected macOS header/writer-size retests, focused SSH,
independent clean-machine evidence where required, and final global M12
reconciliation remain open. Any further behavioral change needs impact analysis
and affected retesting; this record does not freeze or publish a release.
