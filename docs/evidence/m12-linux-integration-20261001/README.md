# Mac verification after Linux integration

This sanitized export maps seven serial native Mac commands to
`4e138d28155060d3ed616d3f5f6aec058cf43b7d`. See the
[integration report](../../m12-linux-integration.md) for boundaries and retained
production identities. It also maps the retained e712603 executable to a fresh bounded restricted
runtime delta. It contains no new package, physical or Linux execution.

- `command-ledger.json` records exact commands, source, bounds, durations, exits
  and hashes of raw logs retained privately.
- `source-identity.json` identifies both merge parents and build inputs.
- `workspace-observations.txt` contains exactly two sustained sections;
  `scripts/verify_sustained_evidence.py` independently qualifies both.
- `resource-observations.txt` contains the 60 Unix resource samples and native
  median/lifecycle summaries. Warm-up is 20 samples; medians use the upper middle
  element of each ten-sample window, matching the unchanged native gate.
- `numerical-evidence.json` records independently recomputed measurements and
  workspace totals. Its Unix resource record omits inapplicable ConPTY fields.
- `isolation-result.json` and `isolation-command-ledger.json` map the passing
  exact-product runtime and positive/negative controls, with raw-log hashes.
- `isolation-profile.sb` is the public profile with a RUNTIME_ROOT parameter.
  Invoke sandbox-exec with `-D RUNTIME_ROOT=<owned-private-runtime> -f` this file.
  The original literal-path profile has its separately recorded hash. Supply a
  synthetic HOME/TMPDIR and system-only PATH plus the copied installation.
- `isolation-retained-attempts.json` preserves all nine driver outcomes, actual
  profile/driver hashes, bounded command outcomes and raw-log hashes. Earlier
  failures are not relabeled as qualified.
- `hosted-candidate-inventory.json` identifies the three distinct hosted products
  from their actual source-filtered build/package records. Synthetic tooling
  control outputs are excluded. Retained native packages are not replaced.
- `hosted-verification.json` records completed Quality and Security runs and all
  native/release job outcomes at the exact integration source.
- `SHA256SUMS` identifies these eleven data files.

Verify hashes from this directory with `shasum -a 256 -c SHA256SUMS`. Reparse the
numeric exports and require the section counts and qualified values, rather than
using verifier exit status alone. Imported Linux exports remain unchanged in their
own evidence directory, including the original unqualified failure. An initial
incorrect resource summary was preserved privately before correction. Raw logs,
private paths, state, drivers and binaries remain outside the public repository.
