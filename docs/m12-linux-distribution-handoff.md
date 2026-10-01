# M12 Linux distribution continuation

## Scope and stopping rule

Complete the affected native Linux regression after the ordinary Windows ConPTY
integration. Automate preparation, tests, packaging, installation, evidence and
local fixes. Do not repeat the completed Linux physical journey or ask for another
full screenshot battery. This is the remaining native platform regression, not
proof that Linux is the only outstanding global M12 requirement.

The current queue is at the top of the [closure audit](m12-closure-audit.md).
The old Linux continuation, reports and dated pending paragraphs are historical
checkpoints. Their earlier SSH/Windows next actions do not reopen later completed
work. Follow the latest mapped evidence and assess source impact first.

After the Linux checks pass, publish public-safe results once on a dedicated Linux
work branch as explicitly authorized by the copy-paste prompt below. Report its
commit and finite remaining gates. Do not send another generic Mac/Windows test
request. No main merge, PR, tag, release, binary upload or corporate-host access.
Do not claim M12 complete while the current closure table contains missing proof.

## Source and evidence mapping

- Repository: https://github.com/fpinero/relayterm.
- Shared delivery branch: `fix/m12-final-validation`.
- Integrated product/test/tooling checkpoint:
  `e712603cc753686675319358f0413946461249e2`.
- New Linux result branch: `fix/m12-linux-distribution-validation`, or a suitable
  existing branch if it already contains related Linux work. Preserve unrelated work.
- [Mac regression](m12-macos-distribution-regression.md) and its
  [numeric evidence](evidence/m12-macos-distribution-20261001/README.md) certify
  Mac at e712603, not Linux. Mac executable SHA-256:
  `ef20a01284008741b03229f6aa8610403dc60205e3cd534df51cbef09e6b14a4`.
- [Ordinary Windows validation](m12-windows-distribution-validation.md) certifies
  product f2b3f6f and executable SHA-256
  `ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa`.
- [Linux final report](m12-linux-final-report.md) already certifies physical
  9cf91f7/15794ad/691a8fb observations and independent Ubuntu 24.04 runtime within
  their original scope. Do not relabel their artifacts as e712603.

Fetch origin, inspect the active branch/status and verify ancestry. If the original
checkout is dirty, use a separate clean clone without discarding, hiding or
reapplying old patches. Read the handoff from the fetched delivery. Verify that its
descendants of e712603 change only documentation/evidence before reusing that
product checkpoint. Investigate any unexpected product delta before adopting it.

Read AGENTS.md and applicable local instructions, PROJECT_VISION.md,
MVP_TECHNICAL_SPEC.md, README.md, TODO.md, the latest avances.md, this guide,
the closure audit, the Mac/Windows reports above, the final Linux report,
[release builds](release-builds.md), [installation](install.md),
[quick start](quick-start.md), [acceptance matrix](acceptance-matrix.md),
and the Quality/Security workflows.

## Host and evidence preparation

Record distribution/version, kernel, architecture, glibc, shell, compiler/linker,
Rust/Cargo version, source/lock hashes and relevant code-generation settings
privately. Do not export hostnames, usernames, private paths or an environment dump.
Ubuntu 24.04 x86_64 GNU is the declared Linux baseline. Verify the actual host;
another distribution/version only establishes that observed environment. The
release wrapper currently supports `x86_64-unknown-linux-gnu`, not Linux ARM64 or
musl. Do not force a non-native target or silently extend the support contract.
Complete available checks and identify the exact missing native prerequisite if
the host differs materially.

Inventory existing artifacts/results first. Reuse valid exact-source proof rather
than rerunning a completed gate. Old artifacts need their original identities.
Use M12.LINUX-DISTRIBUTION-REGRESSION in TODO; add it before implementation only
if it is missing and this source has no completed verification. Store raw
logs, drivers, temporary projects, runtime homes and packages outside Git. Keep
all attempts with command, source/product/harness identity, exit, timeout, elapsed
time and log SHA-256. Use short owned roots for Unix socket paths.

Use Rust 1.98.1 and Cargo.lock. Try locked offline checks first. If required cache
inputs are absent, fetch the pinned Cargo dependencies explicitly and record that
preparation; do not loosen the lock or upgrade tools to obtain a pass. An unavailable
audit tool/cache is a concrete missing check, not a successful audit.

Remove CI, RELAYTERM_TEST_RT, experimental ConPTY selection and reader
metrics/delay variables from ordinary child environments. Preserve the user's
persistent environment. Do not change PATH profiles, SSH, security settings or
accounts. Windows runtime staging/download is not needed on Linux.

## Native automated checks

Run each command independently and check its actual exit status. Logging pipelines
must preserve failure. Use bounded private orchestration: 1,200 seconds for the
complete workspace, 360 for focused sustained/resource gates, 600 for exact-package
TUI and up to 900 for each clean release build. Record an exhausted limit, inspect
only owned fixture processes and resolve the cause before a retained new attempt.
Never kill by executable name, drop failed logs or raise acceptance thresholds.

```sh
cargo fmt --all -- --check
cargo fmt --manifest-path third_party/portable-pty-psmux/Cargo.toml --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo clippy --workspace --all-features --all-targets --locked --offline -- -D warnings
cargo test --manifest-path third_party/portable-pty-psmux/Cargo.toml --locked --offline --lib
python3 -m unittest scripts/test_build_release.py scripts/test_package_release.py scripts/test_install_release.py scripts/test_smoke_release.py scripts/test_verify_sustained_evidence.py scripts/test_windows_runtime_staging.py
python3 scripts/check_repository.py
python3 scripts/check_secrets.py
cargo deny --locked --offline check advisories licenses bans sources
python3 scripts/check_audit_controls.py
cargo test --workspace --locked --offline --no-run
cargo test --workspace --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test resource_runtime_gate --locked --offline sustained_output_memory_and_reconnect_resources_are_bounded -- --ignored --exact --nocapture --test-threads=1
cargo test -p relayterm-daemon --features test-hooks --test sqlite_kill_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-daemon --features test-hooks --test worktree_cancellation_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-git --locked --offline tests::completed_git -- --ignored --nocapture --test-threads=1
```

Run available Git-history Gitleaks with redaction. Distinguish a cached advisory
check from freshly fetched security/hosted evidence. Record platform skips.
Native Mac counts are reference observations, not a required Linux test count.
Confirm intended tests actually ran instead of accepting a zero-test selector.

The complete workspace already includes debug sustained TUI and hardening gates.
Do not add unnecessary repeats if both qualified. Prebuild before measuring, run
serially without other builds or verification workloads, retain all raw observations,
and independently recompute with scripts/verify_sustained_evidence.py. Its summary
alone does not assert success; inspect every section's qualified flag and original
command outcome. Require 2 MiB/s, at least 120 seconds, 100 samples per latency kind,
maximum three-second sampling gaps, navigation p95 100 ms and echo p95 250 ms.
Keep 512 MiB memory ceilings, 32 MiB plateau allowance, minimum 40 resource samples
and original reconnect/descriptor tolerance. Unix reader stays 64 KiB.

If a check fails, diagnose and correct the smallest evidence-supported source,
harness or environment problem autonomously. Preserve failed attempts. A product
fix must be committed on the work branch before clean production builds and needs
its own source mapping plus affected tests. Do not change workloads or thresholds.
A genuine access/credential/destructive-action blocker is the point to stop, not
an ordinary test failure.

## Reproducible package and exact-installed checks

Use two separate clean copies of e712603 with fresh output directories. If a real
Linux correction is required, replace this source with its committed corrected
revision and document the exact delta. Do not include test hooks in production.
Follow release-builds.md and the tools' --help. For each clean source copy:

```sh
python3 scripts/build_release.py build --target x86_64-unknown-linux-gnu --output /tmp/rt-m12-build-a --offline
python3 scripts/package_release.py create --build /tmp/rt-m12-build-a --output /tmp/rt-m12-package-a
```

These are synthetic paths. Choose fresh owned equivalents for A and B; never
reuse or overwrite a nonempty directory. Run scripts from the matching clean source
copy so notices, documentation and build identities agree. Compare build records
and the two external package manifests with the existing compare commands. Verify
SHA256SUMS, inspect/extract the nine-file archive and compare exact binary/archive
bytes. Record version, source, target, profile/features, size/hash and unsigned state.

Inspect ELF with file, readelf and ldd: architecture, interpreter, all dynamic
libraries, resolved paths and highest required GLIBC symbol version. Distinguish
build-host glibc from the executable's actual symbol floor. A filtered PATH is
not an independent clean machine. Run scripts/smoke_release.py on the archive.

Install with the packaged POSIX helper into a new owned directory. Verify the
installed hash, absolute-path help/version and actual Bash/sh command discovery
without persistent PATH changes. Test byte-preserving unrelated-command collision,
reinstallation rejection and removal of only a disposable hash-verified owned
copy, preserving a foreign marker, private state and the retained main candidate.
Run deterministic setup/readback with synthetic agents, never provider credentials.

Set RELAYTERM_TEST_RT only in the children running these exact-installed tests:

```sh
cargo test -p relayterm-cli --test tui_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test session_presentation --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test worktree_gate --locked --offline -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test backup_restore --locked --offline -- --nocapture --test-threads=1
```

Verify the product hash before/after. Harness executables are not package contents.
Recompute the release TUI sustained section independently. Keep installation probes
and other checks outside its measurement interval. Use populated backup/worktree
fixtures; an empty restore does not cover populated recovery.

## Existing runtime and manual evidence

Reuse the completed Linux names/order/Unicode/cursor/conflict/writer-size,
coordination/handover/reopen, disconnect feedback, normal exit and Info/Error
observations. The accepted fixed-grid and interrupted-SSH restoration limitations
are not tasks to rediscover. Reuse the later Mac SSH delta for the one-platform
SSH presentation requirement; the old Linux report's SSH next action is historical.
Do not restart SSH, terminate another SSH client or change service configuration.

Assess whether the vendored Unix dependency/runtime mapping changes the independent
Ubuntu 24.04 proof. The previous container workflow is already complete at its
mapped ancestor. If the existing disposable Linux runtime and driver are available,
reuse that fixture and adapt only the exact candidate input for an automated
installed/quick-start smoke, with no network, no host developer mounts, no build
tools and an unprivileged runtime. Preserve its old state. Do not delete containers,
volumes or backups, or run privileged containers. Runtime-driver orchestration can
remain outside the isolated environment. If no affected boundary needs rerunning,
write a source/dependency impact explanation supporting reuse. If the proof is
missing and isolation is unavailable, name that single prerequisite and continue
all independent work; do not ask for a complete screenshot journey.

Before any manual request, write which new behavior is affected and why existing
proof plus automated PTY/admin readback cannot establish it. Prepare the complete
synthetic fixture first. Request only the minimal cropped observation. If all
relevant boundaries are already covered, require no operator screenshots.
Windows account/console, macOS floor, hosted coverage and the private real-agent
trial are separate owners/gates. Do not claim them from Linux execution.

## Results to preserve and deliver

Create docs/m12-linux-distribution-regression.md with exact source/environment,
commands/outcomes, package hashes/inventory, sustained/resource results, runtime
impact/reuse assessment and failures/limitations. Add only public-safe numeric
observations and sanitized ledgers when useful. Keep raw logs, usernames, hostnames,
private paths, terminal captures, databases, binaries and provider data outside Git.
Remove only verified tasks from TODO and append their exact verification to avances.
Update the current closure table and acceptance matrix without rewriting history.

Validate resulting docs/secrets/whitespace, append-only history and production impact.
Commit focused Linux results in English. Fetch before pushing and reconcile divergence
without discarding work or force-pushing. Push only the Linux work branch, verify
remote/local SHA equality, and report the commit plus a finite list of genuinely
missing non-Linux proof. Do not publish binaries or advance to a release.
A docs-only result commit does not invalidate the tested product artifact.

## Copy-paste request for Linux Codex

```text
Continue Relayterm M12 on this Linux laptop. Fetch origin and read
fix/m12-final-validation:docs/m12-linux-distribution-handoff.md, including
its linked current evidence. Integrated product checkpoint is exactly
e712603cc753686675319358f0413946461249e2; later Mac handoff commits are
public documentation/evidence, not a new product to rebuild blindly.

Preserve existing changes, packages, logs, backups and trial state. Use a
separate clean copy if the checkout is dirty, and a Linux work branch.
Execute the bounded Linux continuation autonomously: native dependency/tooling,
workspace/resource/fault checks, two clean production builds/packages,
exact-installed gates and POSIX installer checks. Record actual distribution,
architecture, runtime, source and artifact hashes. Preserve all failures and
unchanged budgets; diagnose and correct ordinary failures without asking me.

Reuse completed Linux physical and independent-runtime evidence within its
source scope. Do not repeat full manual/SSH journeys. Automate every deterministic
binary check. Ask me only for genuinely missing access/critical information,
destructive actions, or a minimal physical observation whose new behavior you
have identified and cannot verify automatically. Prepare any fixture first.

Write public-safe results in English and communicate with me in Spanish.
Keep TODO pending-only and avances append-only. Commit and push the verified
public Linux results to a dedicated Linux work branch, fetch/reconcile safely,
verify its remote SHA, and give me that commit and the finite remaining gates.
This explicitly authorizes the Linux result branch push. Do not merge main,
create a PR, tag, release, upload binaries, access the corporate host, change
security settings or start another generic Mac/Windows testing cycle.
```
