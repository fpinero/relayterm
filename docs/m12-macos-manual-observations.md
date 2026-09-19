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
repository checks, and whitespace checks. Candidate replacement and manual
retesting remain tracked in TODO. The installed original binary and the operator's workspace have
not been replaced or modified by the diagnostic probe. This report does not
claim that the correction has passed the installed manual journey.

## Remaining observations

- Repeat the affected handover refresh, second-session resume, completion, and
  persistence checks with a separately identified corrected candidate.
- Finish installation collision, discovery, recovery, and scoped removal checks.
- Perform the focused SSH observation on one authorized supported host.
- Complete Windows and Linux observations and reconcile the final artifact and
  acceptance matrices before closing M12.
