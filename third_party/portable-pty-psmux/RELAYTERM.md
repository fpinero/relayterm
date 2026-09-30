# Vendored adapter provenance

Source: portable-pty-psmux 0.9.7 from the hash-pinned crates.io archive identified
in the Windows runtime decision. Upstream VCS identity from .cargo_vcs_info.json
is 66cf61354c473b35d4f0c06c57384fc46d61ffdb, path crates/portable-pty-psmux.

Keep the upstream MIT license. Local Windows changes select only executable-local,
verified Microsoft runtime files, use WINAPI exports with zero flags, preserve the
input endpoint and answer one bounded DA1 query. Runtime errors retain validation
stages and native numeric codes. Unix production source behavior is unchanged.

The manifest omits unused upstream examples and dev-dependency declarations.
The published crate references two tests-rs files that are absent from its archive;
those test-only declarations are removed. Existing inline tests and added loader/
DA1 regressions remain runnable through the separate vendored workspace and lock.
Documentation/comment punctuation and trailing whitespace are normalized to project
policy. The root Cargo patch is relative and committed, never a private override.

Run cargo test --manifest-path third_party/portable-pty-psmux/Cargo.toml --lib
--locked --offline -- --test-threads=1 after staging the runtime beside harnesses.
The application workspace gates exercise the adapter through actual daemon/TUI
sessions. No runtime binary belongs in this directory or in Git.
