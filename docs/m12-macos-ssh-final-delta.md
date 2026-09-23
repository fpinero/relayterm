# M12 macOS SSH presentation delta

## Result

The bounded SSH presentation delta passed on 2026-09-23. Reviewed editing,
explicit adoption without automatic writing, safe discard, normal exit, fresh
connection identity, disconnected read-only navigation, repeated release and
stable diagnostics were physically observed. The sections below preserve the
checkpoint sequence; their earlier pending statements are superseded by this
result. Global M12 acceptance still requires native Windows runtime and debug
latency disposition, followed by final artifact and acceptance reconciliation.

## Candidate and scope

The operator enabled the existing macOS Remote Login service temporarily.
The bounded test uses a fresh loopback SSH connection in Terminal.app and the
retained installed product from 691a8fb, executable SHA-256
4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060.
No alternative SSH listener was started. System host identity was verified
against the local public host key using a private known-hosts file. The operator
entered the account password in Terminal; no credentials were collected.

The first interactive attempt ended with Broken pipe and exit 255 before a
remote-start marker existed. Its cause is not established. The corrected launcher
waits for operator readiness before connecting, retains private verbose client
diagnostics and records remote command entry. The subsequent connection reached
Relayterm. No service, firewall, account authorization or product change was made.

## Reviewed rename and safe discard

The operator opened an unsaved SSH draft while the current label was SSH baseline.
A separate administrative client saved SSH saved on the same session identity.
The operator then observed the following through the actual SSH terminal:

- Ctrl-S rejected the stale form with Error guidance and retained the draft/cursor.
- The first Ctrl-R hid the form and showed SSH saved with separate review/adopt
  guidance and unchanged session identity.
- The second Ctrl-R restored SSH draft with visible cursor, refreshed the current
  context to SSH saved and displayed Info requiring an explicit Ctrl-S submission.
- Esc opened the discard confirmation; y closed the form and retained SSH saved.

Independent complete ordered snapshots after stale rejection, revision adoption
and final discard each matched the saved winner at revision 6. Neither adoption
nor discard wrote automatically or overwrote the winner. Screenshots and raw
readbacks remain private; only sanitized findings are recorded here.

## Remaining bounded observations

Normal SSH exit, a fresh connection with retained session/instance identity,
and changed disconnected writer/navigation/repeated-release feedback with stable
diagnostics remain pending. Reuse prior width and child-survival evidence. Do not
repeat forced SSH-client interruption or any complete native physical round.

## Normal SSH exit and continuity readback

The operator pressed q from Sessions. Terminal.app showed the SSH connection
closed normally, client exit 0 and the local shell marker with an input cursor.
Independent ordered readback after connection closure still matched the complete
revision-6 winner; the same session and instance remained running. The daemon
was not stopped. A fresh-connection physical observation remains next.

## Fresh connection and disconnected-writer preparation

A new SSH connection reached the existing session. The operator image shows
INPUT/WRITER and the native shell prompt. Independent full ordered readback
still matched the revision-6 winner with the same running session/instance.
After that physical writer confirmation, the exact candidate stopped only the
dedicated synthetic workspace daemon using daemon stop --terminate-sessions.
The command returned exit zero and lifecycle stopped. The SSH transport itself
was not killed, and the daemon has not been restarted. Disconnected presentation,
repeated release, diagnostic stability and final normal exit remain pending.

## Disconnected writer presentation

The operator screenshot after the dedicated daemon stop shows DISCONNECTED
while Sessions remains selected. The terminal retains the last shell prompt and
shows NAVIGATION, READ ONLY and RECONCILE WITH R. INPUT and WRITER are absent,
and the footer exposes normal client navigation. This verifies the physical
transition out of writer mode. Repeated release, diagnostic stability and the
final normal exit remain pending.

## Repeated release while disconnected

The operator reports pressing Ctrl-] three times with no visible change. The
resulting screenshot still shows Sessions selected, DISCONNECTED, NAVIGATION,
READ ONLY and RECONCILE WITH R. No unsolicited switch to Events occurred.
Diagnostic stability and final normal exit remain pending.

## Diagnostic stability while disconnected

The operator explicitly navigated to Events with 5. The initial screenshot
shows exactly two diagnostics: one transport connection-loss notice and one
workspace-event refresh notice for sequence 7. After the requested idle
observation interval, the operator reported no change and supplied a second
screenshot showing the same two entries, without duplicates or additional
diagnostics. Events remains selected and DISCONNECTED remains visible.
This verifies bounded diagnostic stability through physical SSH observation;
the final normal exit from the disconnected client remains pending.

## Final normal exit and process verification

The operator pressed q while disconnected. The final screenshot shows SSH
client exit 0, the local shell marker and a visible input cursor. A subsequent
process inventory found no remaining process using the dedicated installed
candidate executable. Its full SHA-256 still matches the candidate above.
The fixture state and private evidence are retained outside Git. No forced
SSH-client interruption, daemon restart or additional build was needed.
The operator controls disabling the temporarily enabled system Remote Login.
