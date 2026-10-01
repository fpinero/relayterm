# Windows numerical evidence

This public-safe export preserves the Windows validation checkpoint. It adds no
new product run or acceptance claim. See the
[validation report](../../m12-windows-distribution-validation.md) and
[Mac/Linux continuation](../../m12-windows-to-unix-handoff.md).

- [Numerical evidence](numerical-evidence.json): all 15 retained measurement records,
  original command exit/timeout status, source/binary/log hashes, unchanged budgets
  and independent arithmetic.
- [Command ledger](command-ledger.json) and [outcomes](command-outcomes.md): all 118
  bounded driver commands, including 19 failures. Paths use synthetic placeholders.
- [Preparation failures](pre-driver-attempts.md): earlier invocation failures and
  numeric-export corrections. Private files mentioned there remain outside Git.
- [Candidate inventory](candidate-inventory.json) and [PE inspection](pe-inspection.json):
  exact final package identities, signatures, architecture and runtime prerequisite.
- `observations/`: numeric-only measurement lines from 15 timestamped attempts.
- [SHA-256 manifest](SHA256SUMS): identities of the exported evidence files.

Directory-local Git attributes preserve evidence bytes across checkout line-ending
settings so the manifest remains valid on Windows, Mac and Linux.

From the repository root, recompute using an output outside the repository:

```sh
python3 scripts/verify_sustained_evidence.py docs/evidence/m12-windows-distribution-20260930/observations/*/observations.txt --output /tmp/relayterm-m12-recomputed.json
python3 -m unittest scripts/test_verify_sustained_evidence.py
(cd docs/evidence/m12-windows-distribution-20260930 && shasum -a 256 -c SHA256SUMS)
```

Expected: 15 logs, 11 sustained sections, 11 qualified sections. A section that
qualified before a later command failure does not convert that command to success.
Resource records include failed/incomplete lifecycle attempts and final passes.
The exported observations were parsed again and compared with every original
structured record, including resource and lifecycle fields. Raw-log hashes refer
to private retained logs, not to these derived numeric-only files.

Preserve 2 MiB/s, at least 120 seconds, 100 navigation/echo samples, a maximum
three-second sampling gap, navigation p95 100 ms, echo p95 250 ms, original
512 MiB memory limits, 32 MiB plateau allowance and minimum 40 resource samples.
Printed rates use microseconds while native assertions used nanoseconds. Maximum
latencies and failed outcomes remain present even when the p95 checks passed.
