# M09: Editable agent templates and generic integration execution contract

## 1. Delivery, authority and scope

Implement M09.01-M09.05 in [TODO](../TODO.md). This document is the execution contract for the implementation agent. Creating and merging it completes only M09-DOC. It does not complete any implementation or establish new runtime evidence.

Read [AGENTS](../AGENTS.md), [vision](../PROJECT_VISION.md), [specification](../MVP_TECHNICAL_SPEC.md), [README](../README.md), the queue and recent [logbook](../avances.md) entries before work. Read ADRs [0002](decisions/0002-local-ipc.md), [0005](decisions/0005-environment-privacy.md), [0006](decisions/0006-agent-configuration.md) and [0008](decisions/0008-rust-platforms.md), plus [daemon commands](daemon_cli.md), [PTY supervision](pty-supervision.md), [TUI workflow](tui-workflow.md), [supported platforms](supported-platforms.md) and [M08](M08_details.md).

The outcome is an account-free Relayterm workflow in which Claude Code, Codex, OpenCode and a previously unknown interactive executable use the same editable definition, IPC, launch, task, terminal and handover mechanisms. Provider names belong in template data and documentation, never in domain state machines or launch branches. Coverage is FR-2, FR-8, AC-14 and AC-15, while retaining existing acceptance guarantees.

Include editable templates, definition creation/editing/enabling in the TUI, executable availability, arbitrary CLI documentation, concurrency regressions and native acceptance. Preserve administrative commands and explicit TOML import. Exclude M10 worktrees, provider APIs, installation, authentication automation, credential forms, automatic task extraction from output, transcript ingestion, hosted catalogs, update checks, telemetry, durable terminal recordings and releases. Do not add production synthetic processes. The unknown test executable is a real child process built only for tests.

## 2. Baseline and prerequisite investigation

Inspected baseline: M08 PR #15 merged as `a68f3a2291bb621acaebc9603b92199876289f25`. Its post-merge Quality run `34158217388` and Security run `34158217472` succeeded. Recheck main, open PRs and current CI when implementation begins. Continue suitable existing work without overwriting changes.

| Inspected location | Observation | Required investigation and acceptance |
| --- | --- | --- |
| `relayterm-config/src/lib.rs` | Version-1 TOML candidates already validate generic definitions | Reuse this parser and ADR 0006; do not create another active store |
| `relayterm-tui/src/model.rs` and form submission | Drafts do not capture an original entity/revision pair; submission uses refreshed application state | Reproduce a refresh during editing; pin definition ID and base revision at form creation, reject stale writes |
| TUI submission error path | Pending state is cleared after errors and the form can be submitted again | Distinguish known rejection from unknown delivery; require explicit reconciliation before another potentially duplicate write |
| `relayterm-daemon/src/lib.rs`, `session_create` | Definition is read before registration, while registration captures its own snapshot; instance discovery compares collections | Barrier-test edit/disable/concurrent registration between those points; the launched arguments must match the exact registered instance snapshot and returned IDs |
| Daemon definition update | Update currently uses the import upsert path | Prove nonexistent update cannot silently create a definition; preserve explicit import upsert semantics |
| TUI agent launch selection | An absent selected definition can flow to the generic shell launch path | Empty/missing agent selection must not launch a shell or a different definition |
| Agents list and forms | List navigation and generic field representation were designed before definition editing | Prove offscreen selection, duplicate labels, empty arguments and lossless array editing at 80x24 |
| PTY launch | Explicit environment and argument arrays exist; no shared first-party availability resolver is present | Inspect actual native library resolution and ensure checking and launching use one documented resolution policy |
| `.github/workflows/ci.yml` | Native gates now have independent repetition steps | Preserve independent failure propagation, including PowerShell; do not turn a failed repetition into a later success |

These are source-inspected risks, not claims of reproduced defects. Add focused failing tests before corrective changes; record a dismissed suspicion with the test and reason. Fix shared task-form or launch infrastructure only where the same reproduced defect affects M09. Do not broadly rewrite the TUI.

M08's macOS hosted latency observations exceeded its reference targets while passing its documented hosted-runner guardrails. Do not claim that M09 fixes this, relabel those results, or edit historical log entries. Retain existing performance gates and report new measurements separately. Native CI is not evidence of manual GUI terminal or SSH behavior.

## 3. Architecture, storage and compatibility

### 3.1 Ownership

- Domain and application retain provider-neutral `AgentDefinition`, instance snapshots, claims and coordination services. No provider enum, special capability semantics, credentials or installation logic.
- Configuration owns checked-in version-1 template documents and parsing. Prefer `templates/agents/*.toml` embedded using `include_str!` through the configuration crate so installed `rt` needs no checkout or network.
- Daemon exposes a small authenticated read-only template catalog, composes executable resolution, owns active definitions and supervises launch. Template creation still calls the existing definition mutation.
- Platform owns native filesystem/executable lookup helpers. Keep PTY independent; daemon passes the resolved native executable into the existing neutral spawn request.
- Protocol carries bounded neutral DTOs. Client owns typed request helpers and delivery outcomes. TUI depends only on client/protocol, never config, daemon, SQLx, PTY handles or domain repositories.
- CLI keeps existing commands and adds a bounded `agent check` command for reproducible diagnostics. No second required executable is shipped.

Preserve the graph asserted by `crates/relayterm-cli/tests/architecture.rs`. New modules within existing crates are sufficient. Update legitimate graph assertions precisely if an edge is necessary, without adding an adapter exception to core. Follow the pinned toolchain, lockfile and licensing policy; this milestone does not authorize a compiler upgrade.

### 3.2 Active state and identity

SQLite remains authoritative. TOML is an explicit import candidate, not a live backing file. Editing a file, opening the catalog, restarting the daemon or upgrading `rt` must not create, replace, enable or remove definitions. TUI edits never rewrite the source TOML.

Keep the existing fields: ID, workspace ID, display name, command, arguments, environment allowlist, capabilities and enabled. No schema migration is expected. Availability and template origin are not durable facts; do not add them to the database or instance lifecycle. If implementation proves a migration necessary, document the exact missing invariant and migration/restart tests before changing this decision.

Generic registration allocates a fresh definition ID. Generic update requires an existing ID in the current workspace. Explicit file import remains an atomic upsert by supplied stable ID; omitted definitions remain intact. An identical import is a no-op only at a valid revision. A stale identical import still conflicts. Display names need not be globally unique; distinguish entities by IDs.

Edits and disabling affect future launch admissions. They do not kill or alter existing children, mutate captured launch snapshots, release claims, replace history or reopen tasks. No definition deletion is added. Disabling leaves historical queries intact.

### 3.3 Protocol evolution

Keep IPC version 1 if additions remain capability-negotiated and wire-compatible. Add capability strings for the template catalog and availability operation; define them once and test handshake negotiation. An older daemon must leave existing list/edit/launch functionality usable and show these added actions as unsupported, without reconnect loops. Existing clients may ignore extra advertised capabilities. Reject unknown fields/invalid versions according to current DTO policy.

Recommended new operations and typed results:

| Operation | Input | Result and effects |
| --- | --- | --- |
| `agent.list_templates` | Empty object | Catalog version 1, at most three template descriptors and their validated neutral candidates; no durable writes |
| `agent.check_definition` | Definition ID and expected workspace revision | ID, observed revision, bounded status and guidance code; stale revision conflicts; no child process or durable events |
| CLI `agent check` | Existing definition ID | Same daemon result in established text/JSON formats, with a documented exit-code mapping |

Catalog descriptors contain a stable catalog key, display label and candidate editable fields. Catalog keys are only lookup/presentation metadata, not launch identities. No arbitrary URL fetch or catalog mutation endpoint. Document exact DTO names, capability names, error mapping and JSON examples in the protocol reference during implementation. Fit each response under current transport limits; do not enlarge global frames for three small templates.

Reuse `agent.register_definition`, `agent.update_definition`, `agent.import_definitions`, existing revision preconditions and mutation receipts. Template selection prefills registration; it is not an additional privileged write operation. Retain LocalUser attribution. No new public instance/System actor selector.

## 4. Template contract

### 4.1 Initial content and sources

The minimal interactive defaults were checked against official documentation on 2026-09-08. Recheck executable names, supported operating systems and any added arguments during implementation, record date and sources, and prefer no arguments over guessed flags.

| Template | Command | Arguments | Official reference |
| --- | --- | --- | --- |
| Claude Code | `claude` | `[]` | [CLI reference](https://code.claude.com/docs/en/cli-reference) |
| Codex | `codex` | `[]` | [Codex CLI](https://developers.openai.com/codex/cli/) |
| OpenCode | `opencode` | `[]` | [CLI reference](https://opencode.ai/docs/cli/) |

Each template starts with `environment_allowlist = []`, `capabilities = ["terminal"]`, and `enabled = false`. The capability is an informational neutral name, not evidence of a verified provider feature or a dispatch key. No model, account, prompt, automatic approval, remote-control flag, API endpoint or credential name is required by default. Provider authentication and network use belong to the user's independently installed tool.

Each checked-in TOML file uses the existing `format_version = 1` and definition schema, with one stable synthetic UUID per template for explicit file imports. Generate these fixture-safe IDs once, commit them, and preserve them thereafter. Test all three through the real parser. No template-specific schema fields may be added to `AgentDefinition`.

TUI selection deliberately copies editable values into a new registration draft, excluding the file's import ID. Saving allocates a fresh ID. Repeating that explicit action creates another definition, with the proposed name and intent visible; do not deduplicate by name or silently update a previous copy. In contrast, reimporting the same TOML ID targets that ID under the existing upsert contract. Explain both paths in help and documentation.

Users can edit every template value through the same form as a custom command. Checked-in files are editable examples, and embedded defaults are versioned product data. Existing saved definitions never track template changes automatically. Catalog display, cancel, availability check and template preview produce no mutation, event, installation or launch.

### 4.2 Validation and bounded representation

Reuse domain validation server-side and mirror only useful field feedback client-side. Limits are UTF-8 bytes, not displayed columns. Preserve current stricter checks if the source differs; reconcile changes explicitly.

| Field | Required contract |
| --- | --- |
| Display name | Nonempty, at most 256 bytes, single line |
| Command | Nonempty, at most 4 KiB, single line, one executable path/name without shell splitting |
| Arguments | Ordered array, at most 128 items, 4 KiB/item and 32 KiB total; preserve duplicates and empty strings |
| Environment allowlist | At most 128 valid variable names, no assignments or values; no duplicates, including Windows case variants |
| Capabilities | At most 128 validated names, no duplicates, no provider interpretation |
| Enabled | Explicit boolean; disabled definitions can be stored even when unavailable |
| Imported document | Existing 1 MiB input bound and 128-definition bound; whole import atomic |
| Form memory | Existing 256 KiB aggregate draft budget; account list editing copies and previews |

Commands, paths and arguments must not be trimmed, expanded, normalized or joined into a shell expression silently. Narrative help does not become an argument. Reject prohibited controls, NUL and recognized credential patterns with field-only errors. Detector patterns do not recognize every arbitrary secret, and documentation must say so.

Use an item editor for argument arrays: add, edit, remove and reorder entries, with a visible empty-item representation. Never split on spaces or commas. Environment/capability set editors may use one item per row, but must preserve boundaries, reject duplicates and explain constraints. Byte limits apply before growing buffers, including paste. Preview only the selected definition in the private UI, safely escaped, with no raw terminal control interpretation.

## 5. TUI and administrative behavior

### 5.1 Agents screen and actions

Expose discoverable keyboard actions: create custom, create from template, details, edit, enable/disable, check availability and launch. Preserve session navigation, task coordination, Ctrl-] focus escape and administrative CLI commands. No agent action with an empty or stale selection may fall back to launching a shell.

At 80x24 every field, list item and action must remain reachable. Use stable definition IDs for selection, stateful scrolling and paged existing list APIs. Duplicate display names show an ID suffix. A refresh must not retarget an open form to a different row. Below the supported minimum preserve drafts and existing resize/help/exit behavior.

Show enabled and availability independently: an enabled command may be missing, and a disabled definition may have a resolvable executable. Initial status is unchecked. A saved definition may be enabled while missing; saving configuration must remain usable offline and across installations. Launch still validates availability and enabled status authoritatively. Do not label a command authenticated, healthy or trusted based on filesystem presence.

### 5.2 Form transactions

Capture workspace ID, definition ID for edits, base revision, original values and connection generation when opening the form. Registration captures its workspace revision too. New events may refresh background lists but must not advance a draft's precondition. Save validates, shows pending state and submits once with the captured revision. Disable/enable also uses the captured selected ID and revision.

On a known validation error preserve draft and field focus. On conflict show safe changed-field names and options to reload/discard or review changes against a newly fetched version. Only an explicit reviewed action may establish a new base revision. Do not silently retry, merge fields or overwrite another edit. Changing workspace invalidates pending completions from the old workspace.

On unknown delivery preserve the draft and mark outcome uncertain. Read authoritative definitions/receipts using the existing delivery model before presenting a resubmit action. For registration, matching names or values is not proof of request identity; if success cannot be established, explain that another registration may duplicate the first and require explicit user intent. Reconnect never resends writes automatically. Cancel during an admitted mutation does not imply rollback; consume the outcome safely.

Use one pending write per form and existing bounded effects. Keyboard repeats, double submit and late replies cannot create duplicate local submissions. Confirm discarding unsaved changes, including list edits. Successful save refreshes by returned ID and revision. Saving or enabling never launches automatically. Launch remains a separate deliberate action.

### 5.3 Privacy and guidance

Full definition values are available to the authenticated local user through authorized detail/query paths. They must not appear in events, diagnostic logs, panic/debug output, raw OS error formatting or public test artifacts. Avoid deriving content-revealing Debug for drafts or resolver requests. Sanitize labels and errors before rendering; terminal payload stays within the actual terminal view.

Missing-command guidance should explain manual installation outside Relayterm, daemon PATH versus the current shell, explicit executable paths, definition editing and rechecking. Do not print PATH contents or resolved private roots. Do not read login files or execute `--version`, `auth status`, `which`, `where`, a login shell or a provider process as an availability probe. Installation/authentication links are static help, never automatically opened.

## 6. Availability and launch consistency

### 6.1 Shared resolver

Implement one neutral resolver used by the read-only check and real launch. Resolution uses the daemon's launch environment and validated workspace working directory, not client PATH or an interactive shell's aliases/functions. Preserve paths with spaces and native path types internally. The public command remains the existing validated string; do not introduce lossy conversion for internal native roots.

First inspect the pinned PTY/process library's exact Unix and Windows behavior and native-test a resolver prototype before consolidating its API. Record the chosen rules in ADR 0005 or a narrowly scoped new ADR. The following intended policy is explicit so differences cannot be hidden:

- A command without path separators searches ordered, absolute, nonempty directories from the daemon PATH. Empty or relative search entries do not implicitly execute a workspace-local file. This deliberate lookup restriction must be documented and tested; explicitly configured relative executable paths remain possible within the validated workspace root.
- Explicit absolute paths are allowed by the existing launch policy. Explicit workspace-relative paths are resolved against the authorized launch directory, canonicalized using native operations, and checked against the applicable workspace boundary. No tilde, variable, glob, quote or shell expansion. Do not pretend this makes the child a sandbox.
- On Unix require an existing regular executable target with appropriate executable permissions, follow symlinks under the explicit path policy, and handle missing/broken links, directories and permission denial. Metadata is advisory; the OS launch remains authoritative.
- On Windows establish case-insensitive native lookup and executable suffix handling with native tests, including PATH directories with spaces. Do not silently run `.cmd`, `.bat` or `.ps1` via interpolated shell strings. Report unsupported launchers with guidance to select a native executable or an explicitly configured interpreter plus argument array. Native package shims must be investigated; inability to launch a required native provider installation cannot be hidden behind an unavailable badge.
- Generic interpreter configurations, such as a native runtime executable plus a script-path argument, use the same fields and work without provider branches. Document installation-specific paths as user choices, never guessed hardcoded provider locations.

If existing supported launch semantics differ, reproduce the difference, assess compatibility and document the smallest safe resolution. Do not weaken argument separation to accommodate a shim. Availability means the configured executable is resolvable under this policy, not that the OS loader, script argument, provider dependencies, authentication or account will succeed.

### 6.2 Results, bounds and invalidation

Use stable status/guidance enums, for example `available`, `not_found`, `not_executable`, `unsupported_launcher`, `invalid_command`, and `unavailable`. Enabled is a separate field. Return definition ID and observed revision, without private resolved path, environment values, arguments or underlying OS message text. A status result is not a workspace mutation or terminal lifecycle observation. Authentication failures remain child behavior, never an availability state.

Check only a selected saved definition, explicitly or once after selection settles. Do not scan all definitions on every frame or event. Allow one UI check in flight, at most four daemon resolver jobs, and a bounded waiting queue of 16. Use a two-second response budget and bounded PATH input (64 KiB, at most 256 entries). Reject or report resource/unavailable status when limits are exceeded, with no silent partial positive claim. Do not persist a cache. A UI result expires after 30 seconds or any definition/revision/generation change; launch always resolves anew.

Filesystem calls can stall on network paths. A timeout must not create unbounded detached workers: retain the permit until a blocked operation actually completes and reject excess admission. Bound serialized response to 4 KiB. Test timeout/admission accounting with controlled resolver doubles and ordinary native lookup separately. Do not claim a filesystem timeout cancels a kernel call or proves bounded daemon shutdown unless measured.

A stale completion cannot overwrite a newer definition badge. A stale expected revision is a conflict, not a result for a different candidate. Report disconnected/unsupported separately from missing. CLI JSON contains only the typed safe result; choose documented success/nonzero statuses for available, unavailable and operation failure without changing other commands.

### 6.3 Launch admission, snapshot and failure

Availability is a preflight, not a reservation. Revalidate within the actual launch sequence. Resolve the definition candidate, then register only if the same definition/revision is still admissible. Registration must return the exact created instance/session IDs and immutable launch snapshot through the service boundary. Do not discover the created instance by set difference when concurrent requests exist.

The executable, ordered arguments and allowlist used for spawn must correspond to that admitted snapshot. If a definition changes or is disabled before admission commits, return conflict/disabled without launching. If disabling commits after successful admission, the already admitted launch may finish; document this linearization point. It cannot retroactively change that instance. Resolve a changed candidate only after a new explicit launch request, without silent retries.

Retain M07 ordering, durable `starting`, admission limits, single supervisor ownership and spawn/persistence compensation. If resolution fails before registration, create no instance or lifecycle event. If the OS fails after a valid starting record was committed, record honest `failed` without an invented exit code. An executable removed between check and spawn must not orphan an instance or child. Post-spawn persistence failures use existing termination/reconciliation safeguards. Concurrent launches return their own IDs and snapshots.

Read environment values only during actual launch from the daemon environment, using the baseline plus allowlisted names and platform case rules. Validate the final combined environment against PTY bounds (including the baseline), reject overflow explicitly, and never silently drop requested variables. Values are neither stored nor logged. Missing optional names stay absent. Existing instances retain their original launch snapshot after edits and daemon restart.

## 7. Verification design

### 7.1 Focused tests

| Area | Minimum proof |
| --- | --- |
| Templates | All three real TOML documents parse; stable unique import IDs; no credential/default bypass flags; catalog and files share values; no automatic import on startup/upgrade |
| Configuration | Explicit reimport by ID, omitted records preserved, stale identical conflict, invalid batch rollback, file unchanged after TUI edit |
| Definitions | Create/edit/disable/enable through public mutations; missing/cross-workspace ID rejected; disabled/missing commands remain editable; events and revision match changes |
| Form fidelity | Ordered duplicate/empty/space/comma/quote/Unicode arguments survive editing and real child argv; limits at boundary and one byte over; duplicate environment names on Windows |
| Form concurrency | Open at R, second client edits to R+1, event refresh, submit still uses R and conflicts; entity reorder cannot retarget; unknown outcome never auto-resends |
| Availability | Absolute/relative/bare name, missing PATH, multiple directories, spaces, Unicode, directory/broken symlink/permission denial; Windows native suffix/case/script launcher tests; no execution side effects |
| Resolver bounds | PATH byte/item bounds, job/queue admission, timeout permits, stale result invalidation, no unbounded per-frame checks |
| Launch races | Barrier-controlled edit/disable before and after admission, concurrent registrations return correct IDs, remove executable between check/spawn, exact argv/snapshot correspondence |
| Environment | Baseline plus explicitly named additions only, no values in durable snapshots, case collision and final count bounds, no credential file access by checker |
| Privacy | Synthetic secret/path/control markers absent from events/errors/diagnostics/debug and public artifacts; authorized private definition queries remain lossless |
| Compatibility | Old capability set degrades added features; existing CLI output and import contract preserved; architecture test passes |
| Running sessions | Edit/disable leaves child, lease, claim and prior snapshot unchanged; later launch uses new definition; restart preserves edits and honest lifecycle recovery |

Prefer deterministic transaction barriers and channels over sleeps for concurrency tests. Keep test-only hooks outside production control surfaces. For availability, use a real fixture that would create a synthetic side-effect marker if executed, then prove checking never creates it. Do not use private provider directories as fixtures. Tests must assert public durable outcomes as well as screen state, not merely a green render or successful keystroke injection.

### 7.2 Native unknown-CLI gate

Add `crates/relayterm-cli/tests/agent_templates_gate.rs` or an equivalently named dedicated integration target, with test-only helpers in `tests/support`. Use real `rt`, real SQLite, authenticated native IPC, real PTYs/ConPTY and a separately built synthetic interactive executable. Its arbitrary name and behavior must not be recognized anywhere in domain, protocol or TUI code. It emits bounded synthetic ready/argv markers and accepts bounded input; it performs no provider authentication or network request.

The gate must execute this sequence through the real TUI for M09 actions. Administrative IPC may inspect state, introduce a concurrent edit or coordinate test barriers, but must not replace registration/editing/enabling with fixture setup:

1. Start an isolated workspace with no definitions, templates unapplied, and no credentials. Open the Agents screen and prove launch with no selection creates no shell or instance.
2. Open each template preview, check its editable defaults, cancel one and verify no durable mutation. Save a selected template disabled without requiring its provider executable. Compare fields with a custom definition form.
3. Register the unknown CLI from the TUI, including separate arguments with spaces, an empty argument and repeated arguments. Check unavailable before a fixture path is supplied, correct it, check availability and enable explicitly.
4. Launch through the normal agent action. Verify actual argv using synthetic markers and the durable instance snapshot, exact definition ID, working directory and environment-name policy.
5. Create and ready a task, claim it for the running unknown instance through LocalUser, append progress and explicit verification. Child prose resembling a completion must not change task state.
6. Edit the definition for a future invocation while the first child runs. Verify the first snapshot/argv and claim stay unchanged. Disable it, reject a new launch, then enable explicitly.
7. Leave input focus, detach and exit the real client while keeping daemon and child alive. Start another client, reconnect and inspect the same instance, task and progress.
8. Prepare a structured handover and verify the handover, closed claim and `handover_ready` task are atomic. Launch a second instance from the edited definition, claim the task, read the first instance's context and complete explicitly.
9. Assert both immutable snapshots, ordered events/history, correct claim closures and absence of output-driven mutation. The replacement has new arguments, the first retains old arguments.
10. Close clients, stop/restart the production daemon according to the existing lifecycle contract, reopen and verify definitions and enabled state persisted. Do not claim live PTY adoption across daemon restart.
11. In an independent case, hold an edit draft while another client updates that definition; refresh and reject stale submission without losing the draft. Exercise known failure and unknown delivery separately.
12. Restore the outer native terminal and clean up only test-owned processes/resources, even on assertion failure. Keep bounded sanitized metrics instead of raw captures in public evidence.

Two consecutive complete invocations of this gate must pass on native Linux, macOS and Windows for the same code candidate. Add separate CI steps, each checking its own exit code. Cross-compilation, an in-memory terminal or provider mock does not establish native acceptance. Do not increase deadlines or rerun failures into a pass without diagnosing the failure and changing justified code/tests.

Run existing durable-slice, PTY and TUI gates unchanged in coverage. Their real terminal-close survival checks continue to protect daemon/child lifetime; M09's ordinary client exit scenario alone does not replace that evidence. Optional manual smoke checks may use already installed, already authenticated provider CLIs with user-owned credentials outside Relayterm. Do not install tools, log in or submit paid prompts just to satisfy the gate. Missing optional providers are recorded as not run and do not block account-free acceptance.

### 7.3 Required commands and evidence

Run relevant subsets during iteration, then all of the following before delivery. Replace a proposed integration-target name only after documenting the actual equivalent. Execute native gates serially and keep every repetition independently failing.

```text
cargo test -p relayterm-config -p relayterm-domain -p relayterm-application --locked
cargo test -p relayterm-protocol -p relayterm-client -p relayterm-platform -p relayterm-tui --locked
cargo test -p relayterm-cli --test agent_templates_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test agent_templates_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test durable_slice --locked -- --nocapture --test-threads=1
cargo test -p relayterm-daemon --test pty_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test tui_gate --locked -- --nocapture --test-threads=1
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny --locked check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner .
git diff --check
```

Retain the existing CI matrix (`ubuntu-24.04` stable and MSRV, `macos-14` stable, `windows-2022` stable) unless an evidenced infrastructure update is separately justified. Preserve pinned actions and Quality/Security checks. Where a command group uses PowerShell, explicitly propagate each failing native command or put commands in separate steps. A group ending successfully does not prove all commands passed.

Publish revision, OS/architecture/toolchain, exact command, two gate outcomes, run/job URLs, test counts and sanitized observations. Measure selected-definition check latency, resolver concurrency/queue high-water marks, draft bound behavior and existing TUI latency/flood gates. No new cross-machine latency claim is inferred from the two-second timeout. Distinguish local results, hosted native CI, manual emulator tests and optional provider/SSH checks. Missing mandatory native evidence keeps the gate open.

## 8. Documentation deliverables

Create `docs/agent-templates.md` and link it from README, the TUI guide and daemon guide. Include keyboard paths, copying versus importing templates, the complete generic schema, stable-ID import behavior, editing and enabling, availability statuses, launch-admission semantics and running snapshot immutability. Add tested text/JSON `agent check` examples and static official installation/authentication links without embedding credentials.

Include a worked unknown CLI example using an executable already supplied by the reader or a test-only fixture clearly labeled as such. Use synthetic relative paths and IDs. Explain argument arrays, empty/duplicate arguments, environment names without values, neutral informational capabilities, Windows launcher limitations, daemon PATH capture and explicit manual environment changes/restart consequences. Do not prescribe restarting a daemon with live children merely to refresh PATH without explaining the existing shutdown contract.

Explain that the provider owns its authentication, may read its own files and contact external services, and that Relayterm neither requires a provider account for core operation nor acts as a process sandbox. Relayterm never interprets agent prose as task mutations; users explicitly claim, record progress, hand over and complete. Terminal content stays in bounded private memory under existing policy.

Update protocol reference, ADR 0006 for catalog/copy semantics and ADR 0005 for shared resolution/environment behavior where needed. Update supported-platform evidence with actual results only. Preserve all previous `avances.md` bytes. Correct a historical statement only by appending a specifically identified correction with evidence.

## 9. Atomic implementation queue

Before coding, add these 30 child tasks under the five pending parents in TODO, retaining the link to this contract. Each row is an independently reviewable outcome; close it only with its verification and same-change logbook entry. Parent tasks stay pending until their children and acceptance pass.

| ID | Implementation outcome | Required verification |
| --- | --- | --- |
| M09.01a | Recheck baseline, sources, architecture and all section 2 risks; record resolutions | Source references, focused reproduction tests for suspected defects, no unsupported completion claims |
| M09.01b | Add three minimal version-1 TOML templates with stable import IDs | Real parser tests, defaults/limits/privacy, official command references rechecked |
| M09.01c | Embed templates and expose bounded authenticated catalog with capability negotiation | Installed executable without checkout/network, no auto-registration, old-daemon fallback |
| M09.01d | Define copy versus import identity, explicit enable defaults and catalog version policy | Repeated selection/import and upgrade tests, ADR 0006 update |
| M09.02a | Add Agents actions, stable selection and scrolling | Empty selection no-op/error, duplicate names, more rows than viewport, 80x24 navigation |
| M09.02b | Implement lossless custom/template definition forms and array item editor | Empty/duplicate/quoted/Unicode arguments, reorder/delete/paste, exact limits |
| M09.02c | Capture edit target, base revision and original values at opening | Event refresh and concurrent edit deterministically conflict without retargeting |
| M09.02d | Implement single-submit effects, conflict review and delivery uncertainty | Double key, late completion, reconnect and unknown-write tests, no automatic resubmission |
| M09.02e | Wire generic create/update/enable/disable, enforce update-existing semantics | Atomic public mutations, missing/cross-workspace rejection, no accidental launch |
| M09.02f | Verify explicit import parity and persistence | Stable-ID upsert, stale no-op conflict, rollback, source unchanged, restart |
| M09.02g | Fix admitted launch snapshot and exact result-ID consistency | Barrier races with edit/disable/concurrent registration, immutable running snapshots |
| M09.03a | Prototype native executable resolution and document compatibility decisions | Unix permissions/path tests, native Windows suffix/shim/interpreter evidence |
| M09.03b | Implement shared neutral resolver for check and spawn | Same candidate resolution, native spaces/Unicode, no shell interpolation |
| M09.03c | Implement bounded check operation, statuses and capability | Revision checks, timeout permit retention, PATH limits, queue saturation, no events |
| M09.03d | Add TUI check and administrative `agent check` | Safe text/JSON and exit codes, stale badge invalidation, disabled versus missing |
| M09.03e | Preserve launch failure and environment guarantees | Removed executable race, failed starting record, final environment bounds, no orphan |
| M09.03f | Prove credential-free check and diagnostic privacy | Side-effect fixture never runs, no login-file inspection, synthetic marker negative tests |
| M09.04a | Write template/custom-CLI setup and editing guide | Execute documented account-free steps, verify exact CLI help and keyboard actions |
| M09.04b | Document environment, auth ownership, resolver and Windows limitations | Official sources, no secrets/private paths, no unsupported universal-launch claim |
| M09.04c | Update README/TUI/daemon/protocol/ADR references | Local-link and schema consistency, architecture test, no M10 scope |
| M09.04d | Publish native test method and sanitized evidence tables | Commands/candidate/run links, required versus optional coverage explicit |
| M09.05a | Build real unknown interactive test executable and native gate harness | Test-only build exposure, separate real processes, deterministic cleanup and bounded output |
| M09.05b | Gate real TUI template/custom registration, editing and availability | No administrative setup replacing M09 UI actions, actual argv fidelity |
| M09.05c | Gate claim/progress/detach/edited replacement/handover/completion | Real SQLite and IPC, atomic claims/history, no output-driven task state |
| M09.05d | Gate conflicts, uncertain delivery and durable restart | Preserved drafts, no duplicate automatic write, stable definitions/snapshots |
| M09.05e | Integrate two independently failing native repetitions | Two passes on each stable Linux/macOS/Windows job, exit propagation verified |
| M09.05f | Run all existing regression and quality/security controls | Section 7.3 and retained native durable/PTY/TUI gates, no skipped regressions |
| M09.05g | Record optional real-provider/manual observations honestly | Installed/authenticated-only checks or explicit not-run reasons, no mandatory account |
| M09.05h | Audit acceptance, close verified queue and append evidence | All five parents/30 children mapped to proof, logbook append-only, no missing required platform |
| M09.05i | Deliver PR, required reviews, merge and post-merge CI | Passing candidate and main checks, local/origin main equality, no force push or release |

Order: investigate risks; templates/catalog; safe form foundations and mutations; resolver prototype; availability and admitted launch consistency; guides; native gate; full verification and delivery. Documentation may proceed beside implementation, but API choices must follow the native prototype and concurrency evidence. Keep M09.05h/i open until acceptance/delivery is actually verified, and record final delivery evidence after it exists rather than anticipating a merge.

## 10. Delivery and stop condition

The implementation agent must verify this document's PR is merged, synchronize main without losing work, then create `feature/m09-agent-templates` or continue an appropriate implementation branch. Add the atomic queue before code changes. Preserve existing user changes and historical log entries.

Use coherent English commits, publish the feature branch and open a PR with behavior, compatibility decisions, test evidence and limitations. Monitor Quality and Security, investigate failures, fix related defects and rerun affected controls. Do not merge until all mandatory native tests and repository-required reviews pass. A missing credential for GitHub, required reviewer or unavailable mandatory platform is a concrete blocker, not grounds to weaken acceptance. Optional provider accounts are never a blocker.

After authorized merge, fetch/synchronize local main, verify it equals origin/main and monitor post-merge Quality and Security. If a post-merge failure appears, report it and fix an in-scope regression through the normal reviewed branch workflow. Do not delete branches, force-push, discard existing work, bypass checks, tag or publish releases.

M09 is complete only when all five parent outcomes hold: editable minimal templates, equivalent generic TUI management, safe useful availability, reproducible arbitrary-CLI documentation, and the native unknown-CLI continuity gate. Return PR URL, merge commit, exact checks, native results, optional omissions and CI state. Stop before M10.
