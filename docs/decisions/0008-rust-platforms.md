# 0008: Rust, platforms, and dependency policy

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.08.
Requirements: Sections 3.2, 9.6, 10, 17; NFR-1, NFR-7; AC-11, AC-12.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Pin Rust 1.98.1, edition 2024, resolver 3, and MSRV 1.98.1. The official stable manifest dated 2026-09-03 and release announcement identify this release. Test both the pinned compiler and current stable. Commit Cargo.lock; initial package version is 0.1.0, Apache-2.0, with publish=false.

## Invariants and behavior

Use six crates with inward dependency boundaries. Bootstrap direct production dependencies are uuid (MIT OR Apache-2.0, std only) and clap (MIT OR Apache-2.0, derive enabled). serde_json (MIT OR Apache-2.0) is test-only for graph inspection. Exact resolved versions are in Cargo.lock. Do not add Tokio, SQLx, portable-pty, Ratatui, or adapter scaffolding until needed.

Native CI targets are x86_64-unknown-linux-gnu on ubuntu-24.04, aarch64-apple-darwin on macos-14, and x86_64-pc-windows-msvc on windows-2022. Record actual rustc host and runner OS, not inferred coverage. Pinned-toolchain CI also runs on Linux. OS/architecture support beyond these targets requires evidence before release claims.

Use minimal rustup profile with rustfmt/clippy. Deny unsafe code in first-party bootstrap crates. Audit with cargo-deny and scan with Gitleaks. Allowed licenses are Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, and MPL-2.0. MPL-2.0 was added after reviewing the `option-ext` dependency used by the cross-platform directory resolver. Its file-level copyleft terms permit use in this Apache-2.0 project without changing the license of Relayterm source files. Unknown licenses and known vulnerabilities fail. Review narrowly scoped exceptions explicitly; do not hide findings to pass CI.

## Alternatives

An older MSRV increases dependency constraints without a stated user requirement. Floating-only compilers reduce reproducibility. Adding all proposed crates immediately creates unused coupling.

## Consequences

Compiler/dependency upgrades require lockfile and compatibility review. Current stable CI may reveal changes independently of the pinned build. Native bootstrap compilation does not demonstrate terminal support.

## Verification and ownership

M01.09 tests the graph and behavior. M01.13 verifies locked native builds, tests, lint, formatting, audit, and scanning. Core tests run without UI selection. M07/M08/M11 own real shell/terminal evidence.

Sources: [Rust 1.98.1 announcement](https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/), [stable distribution manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml), [Cargo resolver](https://doc.rust-lang.org/stable/edition-guide/rust-2024/cargo-resolver.html), [uuid](https://docs.rs/uuid/latest/uuid/), [clap](https://docs.rs/clap/latest/clap/).
