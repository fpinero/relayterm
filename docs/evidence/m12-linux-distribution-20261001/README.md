# Linux distribution evidence on 2026-10-01

These public-safe records support the [Linux report](../../m12-linux-distribution-regression.md).
Requested checkpoint is e712603; the verified development-profile correction and
production artifact source is 6486da6. Documentation-only delivery commits reuse
that artifact. Original failures and platform/source boundaries are retained.

- `command-ledger.json`: bounded preparation/check/build/install/runtime outcomes,
  raw private log hashes and exact command arguments with synthetic root aliases.
- `candidate-inventory.json`: toolchain, lock/profile, package hashes, nine-file
  inventory, observed Linux environment and isolated-runtime boundary.
- `numerical-evidence.json`: independently recomputed original failed and corrected
  sustained sections, test counts and Unix memory/reconnect observations.
- `*-observations.txt`: numeric observations only, without terminal captures,
  commands containing private paths, databases or provider content.
- `retained-attempts.md`: failure, correction and fixture limitations.

Verify SHA256SUMS from this directory, then reparse the four observation files
with scripts/verify_sustained_evidence.py from the repository root. Expect four
sections, three qualified and the original failed section unqualified. A verifier
summary alone never converts a failed command into acceptance evidence.
The original native source assertions use nanoseconds; exported arithmetic uses
printed microseconds. Resource medians are independently recomputed after the
original 20-sample warm-up, with the first/last ten-sample windows.
No new native Mac/Windows execution or hosted quality/security is claimed.

The public command ledger is frozen after the corrected numeric-export check.
Later final document/staged/publication controls remain in the private ledger.
The package and runtime verification outcomes are included in this export.
