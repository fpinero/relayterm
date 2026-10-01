# M12 Linux handoff to the Mac orchestrator

## Delivery and source boundaries

Linux has completed the requested native and distribution continuation. No
additional Linux execution is pending within that scope. Global M12 acceptance
remains open. This guide is documentation only and does not replace a tested
binary with a rebuild.

Fetch both work branches before integration, preserving all existing changes,
packages, private logs and backups. Use a separate clean checkout if necessary.

| Revision | Meaning |
| --- | --- |
| `e712603cc753686675319358f0413946461249e2` | Requested integrated product checkpoint |
| `3845420e88bb8f6b3b40f95f4b042f8afb603b87` | Shared handover used as the Linux branch base |
| `6486da6c6005f366586f7ded036708bf3f6d3ccd` | Verified Linux correction and exact Linux artifact source |
| `0210d35887bf76d0a3326d4592c70016a377deff` | Linux report, public evidence and acceptance reconciliation |
| `b5f886503865a67d6620de47fba3f4d2ed0ec357` | Shared branch observed on 2026-10-01, with the newer Windows standard-account handoff |

The Linux branch is `origin/fix/m12-linux-distribution-validation`. The shared
branch is `origin/fix/m12-final-validation`. Fetch again because either branch
may advance after this document. The Linux branch tip includes this guide in
addition to the source and evidence commits above.

The Mac agent leads integration. Reconcile both branches on a work branch,
reviewing overlaps in `TODO.md`, `avances.md` and the closure audit. Preserve the
Windows standard-account handoff and every appended log entry from both histories.
Keep TODO limited to remaining tasks, remove the completed Linux prerequisite,
and retain the bounded Windows observation task. Do not replace the shared queue
or log wholesale with the Linux versions. The newer shared delta observed here
changes documentation only.

## Material to read and retain

- [Linux distribution report](m12-linux-distribution-regression.md), including
  the original failed attempt, diagnosis, correction and exact test counts.
- [Public Linux evidence](evidence/m12-linux-distribution-20261001/README.md),
  including hashes, candidate inventory, bounded command ledger, numeric
  observations and retained-attempt records.
- `Cargo.toml` and `CONTRIBUTING.md` from `6486da6`: the `vt100` development
  package profile uses `opt-level = 1`. Debug checks, release profile, dependency
  versions, workload and acceptance thresholds remain unchanged.
- [Closure audit](m12-closure-audit.md), [acceptance matrix](acceptance-matrix.md),
  `TODO.md` and the appended Linux entries in `avances.md`.
- `docs/m12-windows-standard-account-handoff.md` from the shared branch. Its
  Linux-pending wording predates this completion and should be reconciled during
  integration, while preserving its Windows candidate and account boundaries.

Everything required for source review and public evidence reconstruction is in
Git. Raw logs, runtime state, binaries, packages and private build copies remain
preserved locally and are intentionally not uploaded. No release, package upload,
PR, tag or merge into main was performed or authorized by this handoff.

## What Linux actually verified

Native Ubuntu 24.04.5 LTS, x86_64, glibc 2.39, Rust 1.98.1,
`x86_64-unknown-linux-gnu`. The required symbol floor observed in the executable
is GLIBC_2.39, so older Linux distributions are not certified by these runs.

Dependencies and pinned tooling, formatting, linting, workspace tests, resources,
controlled failures, reproducible release builds, exact-installed tests, POSIX
installation/discovery/collision/removal and isolated installed runtime/recovery
all passed after the retained development-parser correction. Existing physical
and SSH evidence was reused with explicit source boundaries.

The full corrected workspace passed 205 tests with 25 intentional ignores. The
resource gate retained its 180-second window and passed with 60 samples and 100
reconnects. Exact-installed tests passed 14 tests with five intentional ignores.
Both debug sustained sections and the installed release section qualified under
the original budgets. The original failed debug section remains unqualified.

Two independent clean native release builds from `6486da6` produced identical
unsigned 0.1.0 artifacts with empty production features:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `rt` | 13,053,304 | `248c36e33d25601cb2dcee5d8dc2760b881dded44c79845fd79d85f5ef027236` |
| Normalized Linux tar.gz | 4,840,159 | `e2e716d3549681b129fc8813c2e4ba7d62f97cadaef8764e96bf75b4b6e9797b` |

The independent runtime used a retained isolated Ubuntu 24.04 container with no
network, host mounts, build tools or source tree, running an unprivileged user.
This proves that runtime scope, not another distribution or an older glibc floor.
The source report records the exact image, inventory and preserved-state checks.

## Evidence checks after integration

From the evidence directory, run `sha256sum -c SHA256SUMS`. From the repository
root, reparse the four `*-observations.txt` files using
`scripts/verify_sustained_evidence.py`. Require exactly four sustained sections:
three qualified and the original failed section unqualified. Inspect the section
counts and qualification values, not merely the verifier exit status. Keep
`numerical-evidence.json` and the raw numeric exports consistent.

Run repository/documentation and public-secret checks for the reconciled tree.
Check that `avances.md` retains the existing entries from each branch and that
TODO contains pending work only. A documentation-only merge does not invalidate
the mapped Linux artifact. Reopen native Linux checks only for a concrete source,
dependency, release profile or workload change affecting the verified behavior.

The development-profile correction needs inclusion in the current hosted
quality/security coverage. Existing mapped Mac and Windows production artifacts
retain their original identities; do not attribute a new execution to them or
repeat completed physical/SSH journeys merely because documents were integrated.

## Remaining coordination

Use the reconciled closure audit and TODO as the authoritative queue. The finite
remaining categories are:

- Complete the mapped Windows package's actual standard-account and real-console
  observation using its prepared synthetic fixture. Account creation and password
  entry remain operator actions; no corporate-host access or protection changes.
- Resolve genuinely missing Mac/Windows runtime-floor and independent-environment
  evidence or record a justified disposition. A standard account on the same
  Windows developer machine does not prove an independent clean host.
- Obtain current hosted quality/security evidence including the committed parser
  development profile and unchanged sustained budgets. Work-branch pushes do not
  trigger the present workflows automatically.
- Complete the private operator-guided real-agent practical trial with an
  explicitly identified executable, without uploading private transcripts/state.
- Reconcile all 16 acceptance criteria, phase gates, source/artifact inventory,
  installation/quick-start coverage, limitations and any remaining policy choices.
  Prepare the final handoff for a separate maintainer publication decision.
- Keep the deferred corporate ACL case separate. Preserve its diagnostic and
  correction tasks without probing the corporate host or treating status 1355 as
  successful access.

Prepare fixtures before requesting any necessary new physical observation. Reuse
completed observations unless a concrete new behavior cannot be verified
automatically. Keep failed attempts, exact source/hash mappings and thresholds.
