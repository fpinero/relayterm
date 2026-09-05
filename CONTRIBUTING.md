# Contributing

Relayterm is implementing its MVP. Read the [vision](PROJECT_VISION.md), [specification](MVP_TECHNICAL_SPEC.md), [agent instructions](AGENTS.md), and [architecture](docs/architecture/README.md) before substantial changes.

## Development setup

Install Rust with the official [rustup installer](https://rustup.rs/) and the platform's native linker/build tools. The checked-in toolchain selects Rust 1.98.1, Rustfmt, and Clippy. Linux requires a C linker toolchain; macOS requires Command Line Tools; Windows MSVC requires Visual Studio C++ build tools and the Windows SDK.

```sh
cargo fetch --locked
cargo build --workspace --locked
cargo run -p relayterm-cli --bin rt -- --help
```

Fetch first: the architecture tests inspect the locked graph for all targets, including dependencies not compiled on the current host. After fetching, the Rust tests can run offline. Initial dependency/toolchain installation needs network access; the `rt` bootstrap does not.

## Required checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
python3 scripts/check_repository.py
git diff --check
```

On Windows, use `python` if `python3` is unavailable. CI additionally runs current stable and the pinned compiler on the documented native targets. See [platform evidence](docs/supported-platforms.md).

Install cargo-deny 0.20.2 and Gitleaks 8.30.1 from their official releases. The [security workflow](.github/workflows/security.yml) pins and verifies Linux archives. Run:

```sh
cargo deny check advisories licenses bans sources
python3 scripts/check_audit_controls.py
gitleaks git --redact --no-banner .
python3 scripts/check_secrets.py
```

The source scan copies only tracked and non-ignored candidate files into a disposable directory, excluding build caches. It includes untracked candidate source changes and checks a synthetic negative control. No whole-fixture exclusion or secret suppression is configured.

## Change workflow

Work on a feature or fix branch. Add planned tasks to `TODO.md` before implementation. After a task is verified, remove it from that queue and append exact checks and results to `avances.md`. Keep unverified work pending. Public documentation and comments are English, with sentence-case headings and no em dash or emojis.

Use small coherent changes and synthetic fixtures. Preserve domain/application independence from storage, Git, PTY, and UI. Add ports only when use cases need them. Run proportionate behavior tests and review version/migration implications. The bootstrap dependency checker rejects unreviewed internal edges, including target-specific declarations.

Do not commit runtime databases, environment values, credentials, personal paths, or terminal transcripts. See [privacy](docs/privacy.md) and [security reporting](SECURITY.md). Publishing, merging, tagging, and releases require explicit maintainer authorization.
