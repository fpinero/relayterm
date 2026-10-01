# M12 Windows to Mac/Linux handoff

M12 remains open. Continue on `fix/m12-final-validation` after fetching origin.
Preserve dirty checkouts and historical evidence in place; use a separate clean
checkout when necessary. Do not automatically reapply old patches.

## Source and artifact mapping

The delivered baseline is `5ca6ab9fbacbaf6035ffae2f387610d325791659`.
The Windows product source is `f2b3f6f446466f5b3657c3cd91746735cab59d78`.
The verified source/report checkpoint is `df241836f667bcd84873b971f1a1ad5986c1f358`.
The sharing commit that adds this guide changes only documentation and evidence.
It does not certify different product bytes.

Read [Windows validation](m12-windows-distribution-validation.md),
[runtime contract](windows-runtime-distribution.md),
[runtime decision](decisions/0009-windows-packaged-conpty.md),
[original handoff](m12-windows-distribution-handoff.md), `AGENTS.md`,
`PROJECT_VISION.md`, `MVP_TECHNICAL_SPEC.md`, `README.md`, `TODO.md` and `avances.md`.
The Windows package's executable SHA-256 is
`ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa`;
archive SHA-256 is
`039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50`.

## Shared evidence

[Evidence index](evidence/m12-windows-distribution-20260930/README.md) contains
the numeric-only observations, independently parsed results, all 118 driver
command outcomes (including 19 failures), preparation failures, candidate
identities and PE inspection. Raw logs, private machine/token inventories,
databases, old patches and binaries remain outside Git. Numeric process IDs are
ephemeral fixture observations. They do not identify a host or account.

The earlier private Mac ZIP and Git bundle stop at `df241836`; use the fetched
branch for this additional shared evidence. All earlier attempts remain preserved.
Source sharing is authorized, but no merge, tag or release is part of this handoff.

## Native continuation

Windows uses the pinned executable-local ConPTY runtime. Unix source behavior is
preserved, but the relative Cargo patch and release tooling require native checks.
Windows runtime staging flags are not needed on Unix. Preserve original thresholds,
failures, native reference environment and exact source/binary hashes for each run.

```sh
cargo fmt --all -- --check
cargo fmt --manifest-path third_party/portable-pty-psmux/Cargo.toml --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo test --workspace --locked -- --nocapture --test-threads=1
cargo test --manifest-path third_party/portable-pty-psmux/Cargo.toml --locked --lib
python3 -m unittest scripts/test_build_release.py scripts/test_package_release.py scripts/test_install_release.py scripts/test_smoke_release.py scripts/test_verify_sustained_evidence.py scripts/test_windows_runtime_staging.py
python3 scripts/check_repository.py
python3 scripts/check_secrets.py
cargo deny --locked --offline check advisories licenses bans sources
python3 scripts/check_audit_controls.py
```

Run two clean native production builds and archives using the
[release build procedure](release-builds.md). Compare exact bytes, notices and
inventory, then run constrained-PATH smoke and extracted TUI, worktree and populated
backup/restore gates. Run the POSIX installer checks skipped on Windows.
Reconcile sustained/resource gates with the original load qualification and budgets;
keep serial performance runs free of competing builds. Windows reader size stays
16 KiB and Unix stays 64 KiB. An unavailable native host remains an open gate.

Reuse unaffected physical, SSH and Windows 11 VM evidence. Repeat a completed
journey only after identifying the new behavior requiring verification. Do not
access the corporate host or change persistent security settings. No ordinary
Windows defect is being transferred back unresolved. Real console restoration and
an existing actual standard-account journey remain Windows operator observations.
Clean-machine/runtime-floor independence, practical trial, fresh hosted checks
and global acceptance remain separate open gates.

Keep public documentation and code comments in English without Unicode U+2014;
communicate with the operator in Spanish. Use the pending-only TODO and append-only
logbook workflow. Do not close M12 based solely on this Windows checkpoint.
