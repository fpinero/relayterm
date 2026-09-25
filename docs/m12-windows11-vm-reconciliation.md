# M12 Windows 11 VM evidence reconciliation

## Decision on 2026-09-25

The third Windows VM run passes the exercised installed runtime, standard-account
coordination, real-console TUI, reconnect and populated recovery gates. This is
imported operator evidence, not a Windows execution performed on Mac. M12 remains
open. The corporate initialization failure is not superseded by this success.

Read both supplied final reports completely:
`m12-windows11-vm-runtime-report-20260925-185735.md` and
`m12-windows11-vm-continuation-for-mac-20260925-185735.md`.
Originals remain outside the repository. Private scripts and raw records were not
supplied or independently rerun here. The continuation is evidence context, not
new authorization to change systems, publish or transfer data.

## Separate runs

| Run | Decision and scope |
| --- | --- |
| Corporate Windows 11 laptop | Six init attempts failed with exit 1 and invalid_location; downstream checks blocked; external ACL probes returned 1355. See the [ACL review](m12-windows11-acl-review.md). |
| VM run 1, original non-timestamped reports | BLOCKED before product execution: installer policy and runtime prerequisite. |
| VM run 2, 20260925-135052 | Installer, collision/reinstallation refusal and owned executable removal passed using process-only RemoteSigned. Runtime prerequisite remained blocked; no product execution. Administrator environment account and separate sandbox principal were identified. |
| VM run 3, 20260925-185735 | Authorized runtime and dedicated standard account prepared; product ran in the normal standard-account session outside the sandbox. Exercised functional gates passed, with original init response capture unavailable. |

Do not combine the first two VM runs into failed product executions or erase
their blockers. Do not label the third run a tool-free machine: existing Git/Node
and private DLL copies were observed. No Relayterm checkout, rebuild, compiler
installation or binary replacement occurred.

## Candidate and environment

| Item | Identity |
| --- | --- |
| Product source | `691a8fbb45980658b98d647a85ea8305b2325938` |
| Product | 0.1.0, release, x86_64-pc-windows-msvc, empty production features, unsigned |
| Executable | 11804160 bytes; SHA-256 `2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708` |
| Archive | 4497901 bytes; SHA-256 `875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94` |
| External manifest | 541 bytes; SHA-256 `c480ec0df02791d06877dbd4be5f4b680051ee709e22c74aa3d99bcc83e589b1` |
| SHA256SUMS | 230 bytes; SHA-256 `9d6561ea52eb02ce6fac36d9d8c048a845f44a0073870750d2595d1788f28cb5` |
| VM | VirtualBox, Windows 11 Core 25H2, build 26200.8037, x64 |
| Execution shell | Windows PowerShell 5.1.26100.7920, normal standard-account session |
| System runtime | Microsoft v14 x64 14.51.36247.0; UCRT 10.0.26100.7623 |

Source/profile/features are manifest assertions, not a new reproducible-build
proof. The signed Microsoft runtime installer was 18731856 bytes, SHA-256
843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c.
User-run elevated preparation returned exit 0 without a restart. The new account
was Users-only, with no Administrators membership or token SID. Elevated
preparation and non-elevated product execution are separate facts. The private
fixture stayed inaccessible to the Codex sandbox.

The operator ran reviewed scripts and TUI exit keys; Codex prepared scripts and
inspected sanitized handoffs. A daemon sample observed system UCRT/VCRUNTIME,
corroborated by exact process identity and shutdown tracking. Short-lived CLI
modules and every DLL search path were not covered. Final user-run checks covered
originals, fixture copies and installed executable; independent Codex rechecks
covered accessible originals, not the protected extraction at closure.

## Accepted evidence and limits

- Installation, exact PowerShell/cmd discovery, help/version and installed hash
  passed. Reuse run 2 collision/reinstallation/removal coverage for the unchanged
  installer; the run 3 installation remains in place.
- Initialization's postcondition passed: fresh status clients found a ready
  workspace and stable original daemon. The first init exit code and output
  remain unknown. They must not be recorded as captured exit 0.
- Three simultaneous shell sessions had distinct stable session/instance IDs,
  names and ordinals 1/2/3. Each executed an input-created synthetic marker.
- Coordination passed at revisions 11 through 17: backlog, ready, claim A,
  unchanged competing rejection at revision 13, progress, handover, successor B,
  done. Claims closed for handover and completion. Four history entries persisted;
  LocalUser author-instance fields being null is expected.
- Two TUI visits used real inherited console handles and normal exit 0. Reconnect
  and administrative reads retained the same live sessions and daemon generation.
- Backup captured revision 17, event watermark 22 and storage schema 3. After
  original shutdown, fresh-home restore preserved workspace identity, task,
  history and claims exactly. Session IDs/names/order persisted; all three
  sessions were honestly lost. Recovery advanced the restored revision to 18.
- Both daemons stopped through product commands, recorded process identities
  exited and no process using the installed image remained. Shell termination
  was acknowledged by the product, not a complete independent descendant audit.

Original init capture, full event-journal equality, exhaustive dependency
sampling, cross-version migration, corrupt-backup behavior and an instrumented
corporate ACL trace are not newly proven here. Existing applicable evidence must
be mapped during final acceptance; these limits do not automatically require
repeating completed visual or SSH journeys. Account, runtime, installation,
backup and state remain preserved; no cleanup or further mutations are requested.

## Contract correction and harness requirements

The earlier Mac response incorrectly predicted `draft` after task creation.
At the exact product commit, `Command::CreateTask` in domain `state.rs` sets
`TaskStatus::Backlog`; `models.rs` serializes enum variants with snake_case.
There is no Draft variant. Runtime `backlog` agrees with source. This was a Mac
instruction error, not a product regression. The continuation correctly reused
the created task instead of issuing a duplicate create.

The initial runner bounded WaitForExit but subsequently waited without a bound
on ReadToEndAsync EOF. Synthetic reproduction and restored-daemon startup with
incomplete EOF support inherited-pipe retention; they do not establish an
instrumented trace of the original stall. Future harnesses must independently
bound process completion and stdout/stderr draining. Complete success JSON plus
exit 0 and authoritative readback can establish an acknowledged mutation despite
incomplete EOF; an unknown response requires reconciliation before retrying.

Windows PowerShell 5.1 may unwrap a single object. Wrap inventories with `@()`
before testing `.Count`. Preserve the successful backup when a later harness
assertion fails. Retain original failed attempts, prompt-prefix and regex errors,
timeouts and uncertainty separately from product results. No Relayterm patch is
justified solely by these harness errors.

## Remaining M12 work and next executable step

The VM fulfills the exercised prepared-Windows runtime journey. Keep M12.NATIVE
as a narrower evidence reconciliation task for the init-capture gap and declared
runtime-floor/independence scope. Do not request another complete VM journey.
Corporate ACL diagnosis remains independently open: preserve numeric native
errors in a development-only handle-based diagnostic and distinguish actual
1355 reproduction from synthetic injection. VM success proves this binary works
in another account/runtime/environment, not that FortiClient caused the failure.

The next performance work is to correct the measurement contract before taking
new acceptance measurements. Local source inspection confirms:

- `tui_gate.rs` emits 80 chunks of (2048 * 16 + 2) bytes, 2621600 bytes total.
- It starts that finite burst before 100 navigation samples, followed by 100 echo
  samples; it does not prove that output remained active throughout either set.
- Its throughput assertion is at least 1 MiB/s. The declared M11 contract requires
  at least 2 MiB/s for 120 seconds, a concurrent full-screen producer and echo
  samples under that load. A passing burst average is not this proof.

Prepare a separately identified test-harness change on a development checkout:
keep output running for the declared interval, acknowledge producer readiness,
record output deltas over the actual sample windows, include the full-screen
producer, and bound stop/cleanup. Reject sample sets outside the active verified
load window. Keep 100 navigation/100 echo samples, 100/250 ms reference budgets
and separately identified hosted guardrails unchanged. Map debug and release to
the same source/harness; retain historical seven-of-eight debug failures and
eight passing release attempts. Do not use VM timing as their disposition.

That harness correction and its measurements are pending, not implemented here.
The original Windows debug environment or another explicitly comparable native
reference environment is needed for the debug investigation; Mac runs cannot
establish its root cause. Final AC/phase reconciliation and three-platform
artifact freeze remain after these gates. No publication is authorized.
