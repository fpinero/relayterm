# M12 Windows delta continuation for Mac

## Delivery and decision

Windows evidence branch: `fix/m12-windows-final-candidate`.
Verified base: `684582bcc40e3798abb63a39870374935041ca41`.
Product: `691a8fbb45980658b98d647a85ea8305b2325938`.
Baseline Windows harness/delivery: `25c1092849600dc833afefa4cfd58f3a6d1861fb`.
The final delivery commit is supplied in the operator's final message, not embedded
inside its own commit. Fetch the branch and verify that exact identity before use.

The [complete Windows delta report](m12-windows-final-delta.md) is authoritative
for this continuation. Debug performance is **failed**, independent Windows runtime
is **blocked**. Seven of eight new local debug repetitions failed; all eight
retained-release repetitions passed. Startup passed in both profiles. The
Windows developer-host installed CLI journey passed, including three ConPTY
shells and private fresh-home restore, but does not prove an independent runtime.
No product correction or performance waiver was made.

## Copy-ready prompt

```text
Continue Relayterm M12 from the published Windows delta evidence. Communicate
in Spanish; write documentation and comments in English. Read AGENTS.md, the
vision/specification, TODO.md and latest avances.md. Preserve all local changes.
Fetch origin, verify the Windows delivery SHA supplied with this prompt, and
read docs/m12-windows-final-delta.md and this continuation from
fix/m12-windows-final-candidate. Do not reset an existing work branch.

Verified Windows base is 684582bcc40e3798abb63a39870374935041ca41, containing
Windows baseline harness 25c1092849600dc833afefa4cfd58f3a6d1861fb and product
691a8fbb45980658b98d647a85ea8305b2325938. Mac commits f623597 and d8d789e were
local-only at handoff time. Inspect and preserve them and any newer Mac evidence;
reconcile histories without overwriting those reports with older Windows files.

Windows outcomes on 2026-09-24:
- FAILED local debug latency: original 290/294 ms failures remain. Six baseline
  standalone/included runs produced five failures; two diagnostic runs failed
  at 300.060 and 252.350 ms. One passing debug run does not resolve the cause.
- Eight same-workload retained-release runs passed, separately from debug.
  Startup p95 was 312 ms debug and 209 ms installed release, each 20 samples.
- Baseline harness blob d2c9bf0c3cfc75878473e3aac4376d15070220ca; new diagnostic
  blob 1347e0aa1f545442f68f6eb9148938fc6dc30f57. Only microsecond logging and
  post-timing flood readback were added. No product, fixture or budget change.
  CI and binary selection were checked; all local attempts had CI absent.
- Root cause remains unresolved. The harness emits a finite 2,621,600-byte burst
  and asserts 1 MiB/s, whereas acceptance describes sustained concurrent 2 MiB/s.
  Preserve this coverage gap and all measurements; do not silently raise budgets,
  accept hosted guardrails as local proof or label investigation completion a pass.
- BLOCKED independent Windows runtime. The operator chose the current developer
  machine. PATH controls excluded cargo discovery but left absolute checkout/tool
  access available. No independent guest was present and no security/OS feature
  changes were made. WindowsSandbox.exe was unavailable.
- Host-only install/hash/discovery in PowerShell and cmd.exe, collision refusal,
  help/version, detached daemon, three cmd.exe ConPTY shells, ordered names,
  claim conflict, progress/handover/successor/completion and fresh-home restore
  passed. Restored identities/history/claims matched and old sessions were lost.
  Installed TUI automation passed separately. Completed physical observations
  were reused. The private runner's initial where-cargo stdout assertion failure
  was retained and corrected by resuming at that checkpoint.
- Owned host-runtime daemons stopped; removal preserved unrelated files, state
  and backups. One pre-existing debug daemon was preserved. Original package
  and installed binary remain unchanged. Raw evidence stays private on Windows.

Windows retained local release, version 0.1.0, no production features, unsigned,
x86_64-pc-windows-msvc, product 691a8fb:
rt.exe: 11804160 bytes,
2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708.
relayterm-0.1.0-x86_64-pc-windows-msvc.zip: 4497901 bytes,
875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94.
External manifest: 541 bytes,
c480ec0df02791d06877dbd4be5f4b680051ee709e22c74aa3d99bcc83e589b1.
SHA256SUMS: 230 bytes,
9d6561ea52eb02ce6fac36d9d8c048a845f44a0073870750d2595d1788f28cb5.
All three transfer files remain together locally on Windows, not on Mac and not
in Git. Obtain an authorized destination, copy those exact files without rebuild,
and verify them with shasum -a 256 -c SHA256SUMS on Mac. Do not assume access.
VCRUNTIME140.dll 14.44.35211.0 and UCRT 10.0.19041.7725 loaded from System32
on the developer host; clean-image dependency availability remains untested.

Linux native package, installed gates and isolated Ubuntu runtime passed.
Linux binary 26f7f90e27da12239c3130e1163951a4cfe2f08e448fc8b28f6a46cec642d8d7,
archive a9e5bb84b474ab8f37e9778046ec530b3833df67261b294549d1352df63a3285.
The maintainer handoff also supplies completed Mac enforced-isolation runtime
and final bounded SSH presentation, including disconnected exit zero. Read
Mac's own newer reports and artifact identities instead of guessing their content.
Do not repeat Windows physical corrections, Mac/Linux completed visual journeys,
SSH tests or deliberate SSH-client termination. Do not replace newer evidence
with an older TODO. The owner accepted possible local reset after forced SSH
termination and no reflow of fixed-grid historical output; these do not waive
Windows performance or independent-runtime requirements.

Remaining work: preserve failed/blocked Windows tasks; identify a real independent
Windows boundary and profile the debug failure before any product fix; reconcile
the load-coverage discrepancy; transfer all retained platform package sets;
reconcile Mac's newer local evidence, all 16 ACs, phase gates and final inventory.
M12 is not closed. Quality 35637866692 and Security 35637869615 were reconfirmed
successful at e4431f7; they cover unchanged product and the earlier harness, not
the new diagnostic lines. No new full CI or native release rebuild was run here.
Inspect the Windows report and logbook for exact local checks and every attempt.

Keep TODO pending-only and avances append-only. Do not publish private artifacts,
merge main, tag, release or delete branches under this continuation alone.
Windows evidence commit/push authorization was specific to its evidence branch;
obtain the applicable maintainer instruction before later external delivery.
```
