# M12: Usability, installable release candidate, and final handoff

## 1. Status, authority, and execution boundary

This is an implementation plan, not evidence of completed M12 work. Planning baseline: `81b9b8c536669e4e12ac2c3657a01c9518f00455`. M11 was merged through PR #22 at `dc5b2e6`; its final source, automated runs, manual observations, limitations, and maintainer approval are recorded in [the final review](m11-final-review.md) and [acceptance matrix](acceptance-matrix.md).

The owner explicitly schedules all five former usability proposals at the beginning of M12. They become M12 requirements, without reopening or retroactively changing M11 acceptance. Complete M12.00 before freezing release artifacts. Preserve M12.01 through M12.07 identifiers from [TODO](../TODO.md); the atomic tasks below refine them.

Read `AGENTS.md`, `CLAUDE.md`, [vision](../PROJECT_VISION.md), [specification](../MVP_TECHNICAL_SPEC.md), the eight [ADRs](architecture/README.md), relevant code, and `avances.md` before implementation. Source maps below describe the baseline and must be checked against the actual checkout. Proposed operation names, fields, files, and tests are design requirements or suggestions, not claims that they already exist.

Work on a feature or fix branch. Preserve unrelated local files and edits. Communicate in Spanish; write documentation, comments, test names, and public UI text in English. Do not use Unicode U+2014. Do not add personal paths, account data, raw screenshots, transcripts, or credentials to public evidence.

This planning delivery does not authorize the next implementation agent to push, create PRs, merge, tag, publish packages, or publish a release. It may implement and verify locally under the next user's implementation request, making coherent local commits. Obtain the owner's explicit authorization for external delivery. The owner can approve their personal MVP after assistant-assisted technical review; another human or account is not mandatory. Never impersonate independent review. Critical/high security exceptions still require an explicit maintainer decision with rationale and mitigation.

## 2. Outcome and non-goals

A new user on each supported target can obtain a checked release candidate, install it without overwriting another command, complete the documented workflow, and understand upgrades and recovery. Sessions are easier to identify and edit forms and rejected actions provide usable feedback.

Keep one required product executable, `rt` (`rt.exe` on Windows). Preserve per-workspace daemon ownership, current-user local IPC, bounded in-memory terminal state, neutral agent definitions, explicit task claims, and non-destructive Git worktrees. Product runtime must remain independent of network services and hosted accounts.

Do not add an auto-updater, package manager integrations, system services, GUI, website, cloud synchronization, telemetry, provider APIs, transcript persistence, automatic Git cleanup, remote listeners, or scheduling. Portable archives and user-local installation are sufficient for the first candidate. Code signing, notarization, package registries, and public release publication are separate explicit decisions, not prerequisites silently added by the agent.

Do not upgrade Rust, dependencies, or CI action versions merely because M12 begins. Changes needed for a demonstrated defect require rationale, lockfile review, and applicable verification. Do not turn minor UI fixes into a TUI framework rewrite.

## 3. Baseline source map and implementation sequence

| Area | Inspect before editing | Baseline observation and consequence |
| --- | --- | --- |
| Session identity | `crates/relayterm-domain/src/models.rs`, `state.rs`, domain commands/events | AgentInstanceRecord owns session ID, instance ID, status, started_at, and immutable launch context. Display metadata must not redefine identity or launch snapshots. |
| Persistence | `crates/relayterm-persistence-sqlite/src/store.rs`, workspace migrations, continuity tests | Workspace migrations 0001 and 0002 exist. Add a new migration, never edit an applied migration. Bounded SQL paging and mutation projections must survive additions. |
| Protocol and routing | `crates/relayterm-protocol/src/message.rs`, daemon routing/runtime, client, CLI | Typed operations, decimal-string revisions, capabilities, error categories, and uncertain results already exist. Extend through all layers rather than writing SQLite from TUI. |
| Session list | `crates/relayterm-tui/src/model.rs`, `render.rs` | Snapshot installation sorts instances by session_id, and rows show UUIDs. Preserve selection by identity while changing ordering and labels. |
| Forms | TUI `model.rs`, `lib.rs`, `render.rs`, `safe_text` | FormField.cursor is a UTF-8 byte offset. draw_form renders text but does not set a form cursor. Child-terminal cursor rendering is a separate existing path. |
| Error UX | TUI submit_form, acquire_input, record_client_error; protocol ErrorCode/ErrorEffect/Recovery | Generic rejection text exists. Definitive conflict and uncertain delivery must remain distinct. |
| Release | root Cargo.toml, Cargo.lock, rust-toolchain.toml, CLI Cargo.toml | Version 0.1.0, Rust 1.98.1, publish=false, bundled SQLite. Verify actual runtime-linked libraries rather than assuming a standalone executable. |
| Verification | `.github/workflows/ci.yml`, `security.yml`, native CLI gates, repository scripts | Required native matrix and separately failing repetitions already exist. Preserve workloads, deadlines, and failure propagation. |

Sequence: M12.00a contracts, M12.00b metadata, M12.00c ordered reads, M12.00d rename UI, M12.00e cursor, M12.00f conflict guidance, M12.00g input guidance, M12.00h integrated verification. Then execute M12.01 through M12.07. Read-only release planning may happen earlier, but do not freeze or certify artifacts before the usability source is stable.

## 4. M12.00: Scheduled usability work

### 4.1 Session names: authoritative model

Use optional display-name metadata associated with the stable session ID in the existing workspace database. Prefer a separate session presentation record/table so historical launch snapshots and event payloads do not acquire an unrelated mutable field. The exact Rust type is an implementation choice; the following contract is mandatory:

- The daemon/application owns mutations. TUI and administrative clients use authenticated typed requests. There is one authoritative name shared by every client, not a per-client alias.
- A name is optional. Empty or whitespace-only input explicitly clears the custom name. Trim leading/trailing whitespace before validation; preserve internal spacing and Unicode. Limit normalized input to 128 UTF-8 bytes. Reject oversized input rather than silently truncating storage.
- Reject newlines, terminal control characters, escape sequences, and bidi formatting controls using the project's safe-text/validation policy. Allow ordinary non-ASCII names and combining characters. Store valid text, then apply display clipping separately.
- Duplicate names are allowed. IDs, never names or list indices, address operations. Show an abbreviated ID next to the name and full session/instance IDs in a details area. Resolve short-ID collisions by extending the abbreviation or showing the full ID.
- The default label is deterministic and synthetic, such as `Session 1`, based on the stable creation ordinal below. Do not derive it from cwd, command arguments, child output, provider credentials, or OS window titles.
- Names can be changed for running or historical sessions. A rename changes no process, task claim, worktree, terminal size, input lease, status, or launch snapshot. An unchanged normalized name is a successful no-op without duplicate durable events.
- A successful non-no-op rename updates metadata, advances the workspace revision, and emits one event atomically. Event diagnostics contain opaque identity and event category, never the name. User-supplied names are private coordination data and must not enter logs or repository evidence.
- Capture expected workspace revision before editing. Two clients submitting against the same revision get one winner and one definitive conflict, with no last-writer-wins overwrite. Unknown session, wrong workspace, invalid text, read-only state, busy storage, and transport uncertainty leave prior metadata intact.
- Persist through client restart, daemon restart, and supported backup/restore. A lost or terminated session keeps its name; persistence does not imply resurrection of its process.

### 4.2 Session creation order and paging

Display sessions oldest first with new sessions appended. Rename, status updates, reattach, resize, and snapshot refresh must not reorder existing rows. Do not sort by UUID, label, mutable timestamps, or current list position.

Persist a per-workspace creation ordinal, allocated transactionally when an instance/session is created. It must be stable, unique, monotonic, and independent of wall-clock changes. Use a bounded integer representation consistent with existing sequence conventions and explicit overflow rejection. Never compute it by loading all historical sessions into the TUI. Keep allocation independent from presentation row count; clearing a name must not remove the ordinal.

For pre-M12 rows, backfill once in deterministic ascending `(started_seconds, started_nanoseconds, instance_id)` order, documenting that ties in historical timestamps cannot reconstruct a previously unrecorded real-world order. New ordinals start after the migrated maximum. Use indexed/set-based or bounded batch migration, not an unbounded application vector. Test an empty database and equal timestamps. Do not rewrite historical events or old launch snapshots to pretend the ordinal always existed.

Return session presentation with coherent revision-checked reads. If a dedicated session list page is required, use a capability-advertised additive operation or explicitly version an incompatible contract. Its keyset cursor must include the stable ordinal/tie-break identity and revision. Do not reuse an ID-ordered cursor for ordinal ordering or sort only the first 200 UUID-selected rows and claim global creation order.

Keep the 200-item page and existing byte limits. Do not automatically load all historical pages. Provide next/previous page navigation or an equivalent bounded, documented way to reach later sessions. An extra query anchored on the selected identity is acceptable; SQL must remain indexed and bounded. Keep the active attachment visible even if its list page changes. With more than one page, show page/navigation state and allow reaching new sessions without hiding them behind an arbitrary fixed page.

Preserve selection by session ID across rename, refresh, page replacement, status transition, and creation. Explicit creation may select the returned new ID and its page; a background creation must not steal focus. If the selected row disappears, choose a documented adjacent surviving row or empty state, never operate on a different row using a stale index.

### 4.3 Session metadata migration and protocol compatibility

Record schema and capability decisions in the persistence/IPC ADRs and compatibility guide. Do not change the major protocol version solely for additive capabilities if the existing negotiation can represent them safely. A capability-absent server keeps old display behavior and disables rename with safe guidance. Unsupported peers must reject before mutation, not silently accept and drop metadata.

Audit schema-version constants, migrations, row decoding, entity projections, collection queries, validators, foreign keys, integrity checks, backup manifests, and restore logic. Separate metadata is still part of durable workspace state and must be backed up and validated. Preserve existing revision and event-watermark semantics.

Test populated schema 1 and schema 2 upgrades on copies, fresh current-schema initialization, interruption/rollback, repeated open without duplicate backfill, and backup round-trip. Keep backup manifest format unchanged if its semantics did not change; record the new workspace schema version accurately. A pre-M12 binary opening a newer schema must fail safely without mutating it. Old event payloads must still decode. Do not infer compatibility from serde defaults alone.

### 4.4 Rename interaction

From Sessions navigation, `n` opens a rename form for the selected ID. Check existing key collisions before binding and update help/footer in the same change. The key must never intercept child input in INPUT mode. Show current name, clear-name behavior, and stable identity. Use existing form submission, cancel/discard, revision, and uncertainty mechanisms; do not introduce a second mutation engine.

Ctrl-S submits once; disable repeated submission while pending. Success refreshes authoritative metadata and preserves selection. Definitive validation errors retain the draft and editing position. A conflict retains the draft and offers explicit review/reconciliation. Unknown delivery retains the unresolved state and requires authoritative readback before an explicit further action, with no automatic replay.

### 4.5 Visible form cursor

Fix the form rendering path for every form kind, including the new rename form, rather than only task creation. Preserve the existing byte-offset editing contract unless a separately justified change is needed. Never treat byte offsets as display columns.

Compute a mapping from the stored UTF-8 boundary to the rendered cell position after safe-text transformation, wrapping, horizontal/vertical scrolling, borders, labels, and error lines. Use terminal cell widths for wide characters and combining marks. Combining characters must not advance the cursor as full-width cells. Do not place the cursor inside a double-width continuation cell. Exact line-wrap boundaries, empty fields, end-of-text, trailing newline, and a field wider than the viewport need defined behavior.

Keep the active field and insertion point in view. At tiny dimensions, either show a clipped safe editor with a valid cursor or the existing minimum-size guidance; never subtract into underflow or set a cursor outside the frame. Clamp scroll offsets on resize without changing text. Reuse a layout helper for rendering and cursor position so the two cannot drift.

Give the active form cursor precedence over a child cursor underneath. Hide it when no editable form/child owns it or when a blocking discard dialog is active. Restore normal terminal cursor/raw/alternate-screen state on exit and recoverable failure. Do not depend on color or blinking alone. Keep Ctrl-U as clear-field; it must not become an editing-mode toggle.

Verify typing, arrows, Home/End, Backspace/Delete, multiline Enter, Tab/Shift-Tab, paste, clearing, rejection, resize, and cancellation. Preserve the 256 KiB aggregate draft bound and each field's byte limit. Clipboard/control sanitization and rejected paste must not create invalid UTF-8.

### 4.6 Actionable stale-form conflict

Classify the definitive stale-revision rejection using typed protocol meaning. Existing generic Conflict may cover multiple causes: inspect the daemon mapping before assigning a specific explanation. If needed, add an allowlisted structured reason/capability rather than parsing raw error strings or reclassifying every rejection as concurrent editing.

For a confirmed stale form, use wording equivalent to: `The workspace changed while you were editing. Your draft is kept. Review the latest state before submitting again, or press Esc to discard.` A workspace-wide revision can change because of another item; claim that this item changed only if an entity-specific comparison proves it. Identify the actual supported reconciliation key in help. Show the latest state for review without silently overwriting the draft or advancing its submission revision. Explicit reconciliation can adopt the new revision only after the user can inspect the change; the next Ctrl-S remains a deliberate submit.

Preserve ErrorEffect and Recovery semantics. ResultUnknown or a disconnect after dispatch must say that the outcome is unknown, not that the server definitely rejected it. Validation, missing reference, unavailable operation, and storage errors need their existing truthful category or a safe generic fallback. Never expose raw server errors, SQL, paths, environment, or draft text in diagnostics.

Acceptance journey: client A opens a draft; B saves a different value; A submits and gets a typed conflict; B's value remains authoritative; A's draft remains editable; discard reveals B's value; a separate explicit reconcile-and-submit journey commits exactly once. Repeat with response loss to prove no mutation replay and no false definitive message.

### 4.7 Actionable input-acquisition rejection

On a confirmed competing-owner rejection, show inline in the terminal navigation area: `Another client controls input. This view remains read-only. Try i after that client releases input.` Keep the existing redacted Events entry and READ ONLY label. Use a bounded status message, not a modal that blocks navigation.

Show this message only when the typed reason proves competing ownership. If the current protocol conflates conflicts, add a bounded reason or retain a neutral message until it can be distinguished. A missing/ended session, stale generation, resource limit, and unknown result are different cases.

The rejected client sends no input, receives no lease, and does not revoke the owner. Continue output refresh. Clear stale feedback on successful acquisition, detach, target change, or successful reconciliation. For unknown acquisition/release/input, preserve the existing unresolved lease and explicit reconnect behavior; do not encourage repeated `i` to bypass it.

Two-client tests must prove A writes, B is rejected and stays read-only, A retains ownership, and B can acquire only after release or confirmed disconnection cleanup. Keep identity stable and distinguish the message from a claim conflict.

### 4.8 Usability verification and manual scope

Create focused domain/persistence/protocol/TUI tests plus one integrated synthetic journey for the five improvements. Required cases: duplicate/cleared/Unicode/oversized names, concurrent rename, unknown delivery, restart/backup preservation, deterministic migration, equal timestamps, more than 200 sessions, selection stability, and bounded ordered pages. Exercise name length near the limit in large pages and include metadata in response/resource limits.

Use Ratatui TestBackend or equivalent render assertions for cursor geometry and clipping, then native terminal observation for actual visibility. A buffer-only assertion does not prove the physical terminal cursor appeared. Test every supported form family at representative normal and minimum sizes, with wide and combining text.

Perform one consolidated affected-behavior observation per OS on the final usability candidate, covering names/order, task-form cursor, stale form, and input rejection with two clients. Cover PowerShell and cmd.exe where changed behavior differs; use the inherited Windows launch/SSH tests for unchanged lifecycle behavior. Add a focused SSH observation when input dispatch, rendering transport, detachment, or acquisition code changes, as these features are likely to touch those paths. Record exactly which local shells and SSH scenarios were observed. Do not mandate rerunning unrelated M11 backup, crash, or three-day full manual procedures just because documentation changed.

Reuse each unaffected M11 row only with a source-change impact explanation. Mark affected evidence pending until rerun; keep historical M11 success intact. If manual access is unavailable, continue automated and documentary work and batch the remaining short operator steps rather than repeatedly asking for the same evidence.

## 5. M12.01: Release targets and build inventory

### 5.1 Supported targets and honest support claims

Use the existing three first-class targets initially:

| Artifact target | Native build/test baseline | Required runtime check |
| --- | --- | --- |
| x86_64-unknown-linux-gnu | Ubuntu 24.04 x86_64 | Verify ELF dependencies and glibc requirement on the declared minimum runtime. Do not claim all Linux distributions or musl support. |
| aarch64-apple-darwin | macOS arm64 | Inspect Mach-O deployment target and dependencies; verify native execution on the declared supported macOS baseline. Do not claim Intel or universal support. |
| x86_64-pc-windows-msvc | Windows x86_64 with ConPTY | Inspect PE imports and any redistributable requirement; verify native execution on the declared Windows baseline. Do not infer desktop support from Server CI alone. |

Declare tested OS versions separately from minimum supported versions. Use M11 desktop evidence as input, not proof of release linkage compatibility. If an older baseline is desired, test it before claiming it. Additional architectures are optional future work and must not delay these three without an owner decision.

### 5.2 Repeatable build contract

Pin the source SHA, checked-in lockfile, Rust toolchain, target, profile, and native build environment. Use an isolated clean checkout and target directory. Do not package a developer's dirty build tree or a binary compiled with test-hooks. Keep production features explicit and test-only binaries out of the archive.

The expected build shape is `cargo build --locked --release -p relayterm-cli --bin rt --target <verified-target>`. Before treating it as definitive, inspect package features and confirm it selects only the intended product. After an explicit dependency fetch, an offline or frozen build verifies that no undeclared fetch is required. Build dependencies can require a toolchain; installed runtime must not require Cargo, rustup, Python, or a source checkout merely to launch Relayterm. User-selected child commands and Git for worktree features remain explicit runtime prerequisites.

Keep package version 0.1.0 unless the owner chooses a candidate version. Tie every artifact to a SHA even when versions match. Do not claim a tag exists, embed a fabricated revision, or silently generate dirty-source metadata. If revision reporting is added, make it deterministic and safe without Git at runtime.

Repeat the release build twice in clean output directories with the same declared inputs. Record binary and archive SHA-256 values. Normalize archive entry order, timestamps, permissions, and root names where feasible. Distinguish a repeatable documented build from demonstrated byte-for-byte reproducibility. If outputs differ, inspect build paths, metadata, debug sections, signing, and toolchain environment; document the exact remaining non-determinism. Do not falsely declare identical artifacts or require cross-OS byte identity. At minimum the recipe must be repeatable and differences explained before sign-off.

### 5.3 Packaging and artifact evidence

Proposed inventory per target: one archive containing `rt` or `rt.exe`, LICENSE, applicable NOTICE material, third-party license texts/attribution, and concise installation/recovery pointers. Prefer tar.gz on Unix and zip on Windows. Test extraction paths and executable permissions. No absolute entries, parent traversal, symlink surprises, debug dumps, databases, logs, source checkout, or test fixtures.

Produce SHA256SUMS and a machine-readable manifest containing version, source SHA, target, OS baseline, toolchain, features, build command, archive size/hash, binary size/hash, and signing status. Keep native build hostnames and paths out of it. Keep release inventory hashes distinct from backup's BLAKE3 checksums. Explain that a checksum detects corruption but is not an independent authenticity guarantee.

Inspect actual linked runtime requirements using platform tooling. Bundled SQLite does not prove absence of every other dynamic dependency. Run help, version, initialization, detached daemon startup, a synthetic PTY, and shutdown from the extracted archive outside the repository with build tools absent from PATH. A hosted runner containing libraries still does not prove a clean runtime; use an identified clean runtime/user environment with an audited dependency inventory for final installation claims.

No silent installation of compilers, system runtimes, SSH servers, or firewall changes on the owner's machines. Prepare artifacts and exact procedures before asking for missing environment access. Do not install an unrelated package manager to solve a packaging problem.

### 5.4 CI and license controls

Add build/package verification using native targets and least-privilege permissions. Keep existing Quality and Security gates. Archive upload for review, if authorized, is not public release publication. Pin any added actions; verify current official documentation during implementation. Do not run untrusted PR artifacts in a privileged publishing context or add write tokens to test jobs.

Generate third-party notices from the locked production dependency graph and actual distributed components, including bundled native code. Do not treat a cargo-deny pass as the complete attribution bundle or include only direct dependencies. Record license-choice rationale where alternatives exist. Unknown obligations require investigation and, if necessary, a focused maintainer decision before publication.

## 6. M12.02: Installation, naming conflicts, and removal

Default to explicit user-local extraction and PATH guidance, with no administrator privileges and no automatic shell-profile edits. Do not build a complex installer when documented extraction suffices.

Before installation, enumerate every existing `rt` resolution in the intended shell, including aliases/functions and executable paths. Provide shell-appropriate examples verified on the target: POSIX shell discovery, PowerShell Get-Command with all matches, and cmd.exe where. Check destination files separately because PATH discovery can miss an existing file. Resolve the expected target by absolute path after install; do not claim success because a different `rt --version` returned output.

If an unrelated command exists, stop before overwriting and explain explicit alternatives: invoke Relayterm by absolute path or select a dedicated directory and deliberate PATH precedence. Do not rename another tool, delete aliases, or replace files automatically. For upgrading a confirmed Relayterm install, follow section 8 rather than treating it as an unrelated collision.

Test spaces/non-ASCII paths, a fresh shell after PATH change, existing unrelated command/file, alias or function where applicable, read-only destination, partial extraction, wrong architecture, and checksum mismatch. Negative cases preserve sentinel contents. In PowerShell and batch examples, check native exit codes explicitly. Quote paths correctly and do not reuse HOME as a scratch variable.

Removal instructions delete only the identified installed archive files after stopping the selected application. Keep private workspace state, backups, project files, Git checkouts, and other PATH entries. No recursive removal of user data or profiles. A state-purge feature is out of scope.

For unsigned macOS or Windows artifacts, document the observed operating-system warning and signature status honestly. Do not prescribe disabling global security protections. Signing/notarization is an owner publication decision and must not be falsely represented as completed.

## 7. M12.03: Executable quick start

Write the user-facing guide only from the actual CLI and help. The guide should use synthetic temporary projects and a dedicated private home outside the project. Separate POSIX, PowerShell, and cmd.exe syntax where necessary. Do not copy a Mac-specific launch script or loopback setup into every platform.

Required journey from the extracted artifact:

1. Verify checksum, target and actual executable resolution; display help/version without creating private state.
2. Initialize an existing synthetic project with private state outside its source tree; explain the confirmation shown on interactive first use.
3. Open TUI and launch one shell plus two synthetic interactive commands through neutral definitions. Use commands available in the documented environment; a test helper is not a second required product binary. Do not require commercial provider accounts.
4. Name the three sessions, verify order and selection, attach/input/resize/detach, and observe the new input-conflict message with two clients.
5. Create a task with a visible cursor; make it ready, claim it from a running instance, prove a competing claim fails, append progress, hand over, and resume from another instance. Use the exact implemented state transitions, not guessed keys.
6. Close/reopen the TUI and confirm the same live sessions while the daemon lives. Demonstrate a stale form without overwriting the other client's work.
7. Explain daemon shutdown/restart limits and verify durable metadata plus honest lost/ended session handling. Do not promise a child survives a daemon stop or host reboot.
8. In a separate disposable Git repository with a synthetic commit and local identity, explicitly create a task worktree and launch in it. Record Git as a prerequisite for this optional workflow, not a hosted remote requirement.
9. Perform backup and fresh-home restore using the published commands, keeping original ownership stopped and source files intact. Explain that source/Git/credentials/live terminals are excluded.
10. Shut down only test-owned processes and keep cleanup explicit and scoped. Do not delete real workspaces or alter the global Git/shell configuration.

Record command sequence, exit codes, candidate/archive hashes, target environment, expected/actual outcomes, and sanitized operator confirmation. A build-tree test is supplementary, not the installed-artifact journey. Automate deterministic steps and consolidate human checks into one short script per OS; do not request screenshots for assertions available through safe administrative readback.

## 8. M12.04: Upgrade and recovery contract

Before replacing a binary, discover the selected install, version, workspace daemon and compatibility. Never silently connect an incompatible new client to an old live daemon or kill arbitrary processes by name. Document current failure messages and the orderly stop/start sequence, including the existing explicit session-termination confirmation when required. On Windows, handle a running executable file lock with stop-and-retry guidance, not forceful replacement.

Create a private backup before upgrading persistent state. Install the candidate into a new versioned directory first, verify it, then change the selected executable deliberately. Preserve the old binary and backup until verification. Do not run the old and restored identity concurrently. On rollback, restore the backup into a fresh home and use a compatible binary; never ask an older binary to mutate a newer schema or copy a live SQLite file without WAL.

Cover: populated pre-M12 schema upgrade, names/order persistence, old client/new daemon and new client/old daemon negotiation, unsupported schema, corrupt backup, wrong permissions, missing shell/agent/Git, stale socket/ownership, detached client, unknown mutation, ended/lost sessions, and partial installation. Map each to truthful action and state whether data or sessions can be recovered. Never claim recovery of lost PTY contents.

Link [backup and restore](backup-restore.md), [daemon CLI](daemon_cli.md), [TUI](tui-workflow.md), and [worktrees](worktrees.md). Test examples against copies and synthetic failures. Keep clear boundaries between changing a binary, migrating coordination state, and backing up source files.

## 9. M12.05: Documentation and publication decisions

Reconcile README's initial-design/planned wording with implemented behavior without claiming an already published release. Update supported targets, installation, quick start, compatibility, help/key map, session naming/paging, security limitations, and candidate status. Preserve vision and exclusions. Explain newly persisted user names in the privacy and backup inventories.

Review CONTRIBUTING, LICENSE/NOTICE, SECURITY, ADRs, specification, and acceptance links for consistency. Verify that the documented private reporting route exists using read-only inspection; do not send a test report or email without authorization. An unavailable private route is a publication decision to resolve, not permission to expose sensitive findings publicly.

Known governance decision: sole maintainer approval with assistant-assisted review is sufficient. Do not ask again for external approval. Batch any still-missing publication choices into one concise request: candidate version, whether/where to publish, signing/notarization if desired, and code-of-conduct/release policy. Supply concrete prepared options and effects. Continue all independent preparation while waiting. Do not let an unanswered optional publication preference block local candidate preparation.

## 10. M12.06: Validation and evidence contract

### 10.1 Automated checks

During iteration run narrow tests that demonstrate changed behavior, then run the required full checks for the coherent final source:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo build --workspace --locked
cargo deny --locked check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner --exit-code 1
git diff --check
```

Verify installed tool syntax before execution if versions differ. Run the existing native Quality workload on all three targets, including both separate repetitions of hardening, protocol fault, SQLite interruption, Git cancellation/descendants, durable scale, offline and sustained resources. Read the checked-in workflow for feature flags and ignored gates; cargo test alone does not run every isolated gate. Retain pinned-compiler coverage and Security negative controls. Do not combine separately failing PowerShell commands or turn a first failure into a passing retry.

Add focused M12 tests for metadata/upgrade/paging, cursor geometry, typed feedback, release archive contents, installed smoke, PATH conflict, and quick start. Do not require two full new manual repetitions solely because CI has two repetitions. Avoid duplicating implementation logic in tests; use independent state readback, preserved sentinels, backend cursor position, actual executable launch, and native observations.

Freeze release source after all usability changes. A later source change requires impact analysis, rebuild/re-hash and rerun affected evidence. Documentation-only descendants may reuse binary behavior evidence with an explicit mapping. Do not start an endless cycle of rebuilding artifacts just to insert their own hash into their own manifest; the inventory identifies the source/binary and excludes self-referential checksums.

If an unrelated untracked personal document fails a repository scan, preserve it and validate an exact candidate-only checkout. Never delete, stash, publish, or silently ignore a tracked failure to obtain a pass.

### 10.2 Required evidence matrix

| Evidence | Linux | macOS | Windows | Completion rule |
| --- | --- | --- | --- | --- |
| Required native Quality and Security | Native target | Native target | Native target | All required jobs pass for final source; pinned Linux job retained. |
| Five usability scenarios | Targeted local observation | Targeted local observation | Targeted local observation | Candidate-mapped cursor, names/order and rejection behavior; affected SSH paths observed if changed. |
| Release archive and dependency inventory | ELF target | Mach-O target | PE target | Extracted artifact, correct target, notices, hashes and actual dependency inspection. |
| Clean installation and command collision | POSIX shell | Zsh/Bash as applicable | PowerShell and cmd.exe | Actual resolution verified and unrelated sentinel preserved. |
| Installed quick start | Native runtime | Native runtime | Native runtime | Complete journey with synthetic commands and no hosted account. |
| Upgrade and backup compatibility | Native tests | Native tests | Native tests | Metadata retained; unsupported/failed operations preserve originals. |
| Unchanged M11 behavior | Reuse with impact note | Reuse with impact note | Reuse with impact note | No affected behavior inferred from old results. |

A clean runtime means an identified environment without reliance on the build checkout/toolchain or undeclared preinstalled dependencies. It can be a suitable disposable user/machine environment; a VM is not mandated and is never a claimed product security boundary. If evidence is unavailable, mark that specific row pending and supply the shortest missing procedure. Do not invent a pass from cross-compilation, old screenshots, or a masked PATH alone.

Preserve M11 budgets: no silent increase of memory, queues, response size, or timeouts. The two historical supplemental Windows throughput misses remain disclosed. New metadata participates in resource accounting. A final candidate regression is investigated on its own evidence; the historical disposition is not a blanket waiver.

### 10.3 Acceptance mapping

| AC | M12 proof |
| --- | --- |
| 1 | Installed initialization preserves private locations; clean quick start. |
| 2 | Extracted binary launches independent daemon with restricted IPC. |
| 3 | Three real synthetic sessions, usable names and stable order. |
| 4 | Changed forms/input feedback tested; inherited full-screen, resize and bounds reconciled. |
| 5 | Exclusive task claim remains separate from session input ownership; no regression. |
| 6 | Progress and structured handover in installed quick start. |
| 7 | Second instance resumes using durable state and full stable identities. |
| 8 | Client closure preserves live sessions; rename and reattach preserve metadata. |
| 9 | Upgrade/restart retains durable records and honestly marks lost sessions. |
| 10 | Installed Git worktree journey preserves explicit identity and no destructive cleanup. |
| 11 | Core-only tests and dependency direction remain intact. |
| 12 | Required native CI plus release-target build evidence. |
| 13 | New names, errors, manifests, archives and public evidence pass privacy controls. |
| 14 | Installed workflow requires no hosted service/account; child and build dependencies are explicit. |
| 15 | Synthetic unknown commands use the existing neutral configuration. |
| 16 | One product binary with tested collision-safe installation guidance. |

Review all phase exit criteria from the specification, not just the packaging rows. Distinguish prepared release candidate, verified MVP acceptance, and published release. None is inferred from merging this plan.

## 11. M12.07: Candidate handoff and stop point

Prepare a local release handoff with version/source SHA, target inventory and hashes, reproducibility findings, native CI links, manual/installation matrix, compatibility and backup boundaries, licenses, signing status, known limitations, and exact unfulfilled publication decisions. Prepare release notes from actual changes, not planned features. Do not include personal paths or raw terminal artifacts.

Reconcile every atomic task below. Move completed items out of TODO only after verification and append exact results to avances in the same change. Leave blocked rows with concrete next actions. M12 is not complete while a required runtime/installation row is absent. An optional signing or public-publication decision can remain distinct from verified local candidate readiness.

Stop with the prepared candidate and ask the maintainer for the publishing decision. No tags, GitHub release, package upload, domain deployment, or automatic merge is implied. If implementation delivery is later authorized, fetch, verify final head and required checks, use ordinary merge/synchronization, monitor post-merge CI, and record facts only after they exist. Do not reintroduce mandatory external human review.

## 12. Atomic implementation queue

Each row is one pending outcome. Child tasks may be split further without changing parent identity. This table is a contract; TODO is the mutable pending queue and avances is the append-only completion record.

| ID | Work | Required proof before completion |
| --- | --- | --- |
| M12.00a | Freeze session metadata, ordinal, paging, typed-error and cursor layout contracts; inspect actual source. | Written mappings, compatibility strategy and test design address all section 4 cases. |
| M12.00b | Implement private metadata, creation ordinal, migration and atomic rename through domain/application/persistence. | Validation, competing revision, rollback, old-schema backfill, restart and backup round-trip pass. |
| M12.00c | Expose typed rename and bounded ordered reads through daemon/protocol/client/CLI. | Capability fallback, revision/cursor contract, more than 200 rows and uncertainty tests pass without full-history loads. |
| M12.00d | Add session labels, rename form, details and stable list/page selection. | Duplicate/clear names, background creation, rename and page transitions preserve identity and lease. |
| M12.00e | Implement visible cursor and active-field viewport for all forms. | UTF-8/cell-width/resize/clipping tests and actual native visibility pass. |
| M12.00f | Add precise stale-form feedback and explicit reconciliation. | Two-client conflict preserves winner/draft; unknown outcome remains distinct and never replays. |
| M12.00g | Add inline competing-input feedback with safe fallback. | Owner retains lease, rejected reader stays read-only, subsequent explicit acquisition works. |
| M12.00h | Verify consolidated usability changes and update help/privacy/compatibility. | Targeted native/manual matrix and affected SSH coverage recorded; unaffected M11 evidence mapped. |
| M12.01a | Freeze release targets, runtime baselines, version and production feature set. | Evidence-backed target table distinguishes tested and minimum OS; no unsupported architecture claims. |
| M12.01b | Implement repeatable native release build recipe. | Clean locked builds twice per target; hashes compared and differences explained. |
| M12.01c | Assemble bounded portable archives, manifests and notices. | Exact safe inventory, executable bits, hashes and transitive/native license obligations verified. |
| M12.01d | Inspect runtime dependencies and execute extracted smoke tests. | No hidden build-tree/toolchain dependency; native daemon/PTY/shutdown proof per target. |
| M12.01e | Integrate release verification without publication privileges. | Native jobs and required checks propagate failures; no test-hooks or test executables shipped. |
| M12.02a | Write explicit user-local installation and discovery procedures. | Commands validated per shell and actual installed resolution identified. |
| M12.02b | Implement/verify collision handling and safe extraction behavior. | Existing files/aliases/functions and invalid artifacts never overwritten or reported as successful install. |
| M12.02c | Verify clean installations on three targets. | Runtime inventory, checksum, target, help/version and TUI startup recorded. |
| M12.02d | Write and test removal and platform-warning guidance. | Only owned install files removed; state and unrelated PATH entries preserved; signing claims accurate. |
| M12.03a | Write synthetic quick start from implemented commands. | Steps cover section 7 with shell-correct syntax and no provider prerequisite. |
| M12.03b | Execute installed initialization and three-session UI journey. | All targets; new names/order/cursor and stable session identity observed. |
| M12.03c | Execute claim/progress/handover/resume and explicit worktree journey. | Durable readback and isolated cwd prove outcomes without production projects. |
| M12.03d | Execute closure/restart/backup recovery portions and reconcile guide. | Same children while daemon lives, honest loss after restart, validated fresh-home restore. |
| M12.04a | Define binary/protocol/schema compatibility and upgrade sequence. | Old/new pair behavior and backup requirement documented from tests. |
| M12.04b | Test populated pre-M12 upgrade and metadata preservation. | Original/backup retained, migration atomic, no duplicate ordinals, names preserved. |
| M12.04c | Verify rollback and failed-upgrade preservation. | Older binary cannot damage newer state; fresh-home rollback uses compatible backup. |
| M12.04d | Publish and test troubleshooting actions. | Every listed failure maps to truthful action with no destructive default. |
| M12.05a | Reconcile README/specification/ADRs/platform/help documentation. | Current behavior, privacy and exclusions consistent; no premature release claim. |
| M12.05b | Reconcile license and security reporting material. | Archive obligations accounted for and reporting route inspected without unsolicited messages. |
| M12.05c | Obtain only missing publication policy choices. | Existing sole-maintainer decision respected; optional preferences distinguished from technical gates. |
| M12.05d | Review public artifact and evidence privacy. | Source/history scans and synthetic negative controls pass; names/drafts/transcripts absent from logs. |
| M12.06a | Freeze final source and run local required verification. | Formatting, checks, lint, full/core tests, scans and focused M12 tests pass. |
| M12.06b | Obtain final native CI and release-build results. | Every required target and independent inherited repetition passes without hidden retries. |
| M12.06c | Complete final installation/usability evidence matrix. | Candidate/hash mapping, affected manual observations and clean-runtime results complete. |
| M12.06d | Reconcile AC-1 through AC-16, phases, budgets and limitations. | No unresolved acceptance blocker or unreviewed critical/high security issue. |
| M12.06e | Audit final artifact inventory against the tested candidate. | Hashes/version/features/notices/signing state match; no untested rebuild substituted. |
| M12.07a | Prepare local release notes and handoff. | Complete evidence links, inventory, upgrade instructions and known limitations. |
| M12.07b | Reconcile pending queue and append verified milestone results. | Only actual completed tasks removed; blocked tasks retain precise next step. |
| M12.07c | Present candidate and stop for publication decision. | Owner receives concrete candidate; no unauthorized external publication or release claim. |

There are 37 atomic outcomes. Completion requires evidence, not the presence of code or this plan. Implementation starts at M12.00a and proceeds in order unless a documented dependency permits useful independent work.
