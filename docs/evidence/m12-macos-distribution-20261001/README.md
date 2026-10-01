# Mac distribution numerical evidence

This public-safe export preserves the native Mac checkpoint at
`e712603cc753686675319358f0413946461249e2`. It adds no product execution,
Linux/Windows coverage, fresh hosted audit or physical screenshot. See the
[Mac report](../../m12-macos-distribution-regression.md) and
[Linux continuation](../../m12-linux-distribution-handoff.md).

- [Numerical evidence](numerical-evidence.json) contains three independently
  qualified sustained sections and Unix resource observations/recomputed medians.
- [Command ledger](command-ledger.json) preserves the 16 native and 16 package
  commands, limits, elapsed times, exit/timeout status, source and raw-log hashes.
  Private command operands use `<private-evidence>`. The separate installer probe
  and final document checks are not fabricated as extra ledger entries.
- [Candidate inventory](candidate-inventory.json) maps executable/archive bytes,
  before/after exact-product hashes and the actual installer outcomes.
- [Input identities](input-identities.json) retains captured source/lock/native
  debug executable and harness hashes. Git identifies the complete source tree;
  this file is a captured subset, not an exhaustive source manifest.
- The three `*-observations.txt` files contain only allowlisted numeric test lines.
- [Retained attempts](retained-attempts.md) preserves preparation failures,
  denied GUI access and the brief installer/release-measurement overlap.
- [SHA-256 manifest](SHA256SUMS) identifies eight evidence data files.

From the repository root, verify with an output outside source:

```sh
python3 scripts/verify_sustained_evidence.py docs/evidence/m12-macos-distribution-20261001/*-observations.txt --output /tmp/rt-m12-mac-recomputed.json
(cd docs/evidence/m12-macos-distribution-20261001 && shasum -a 256 -c SHA256SUMS)
```

Expected: three logs, three sustained sections and three qualified sections.
Windows helper aggregate fields in the generic parser are not applicable to
Unix. Use the raw Unix process samples and `unix_memory_recomputed`, not those
Windows-only booleans, for Mac resource accounting. Printed microseconds remain
separate from native nanosecond assertions. Original 2 MiB/s, 120 seconds,
100 samples, three-second gap, 100/250 ms latency, 512 MiB memory, 32 MiB plateau
and minimum 40 resource samples are unchanged. Raw logs and artifacts stay private.
