# M10: Explicit Git worktree isolation

## 1. Execution contract and scope

This is the implementation contract for the six M10 parents in [TODO.md](../TODO.md), FR-7, FR-8, AC-10, and sections 6.7 and 9.5 of the [specification](../MVP_TECHNICAL_SPEC.md). Read [project vision](../PROJECT_VISION.md), [README](../README.md), repository instructions, [ADR 0007](decisions/0007-worktree-ownership.md), [ADR 0002](decisions/0002-local-ipc.md), [ADR 0004](decisions/0004-sqlite-recovery.md), [ADR 0005](decisions/0005-environment-privacy.md), and the recent logbook before coding. Resolve stale filenames through the [ADR index](architecture/README.md).

M10 delivers explicit creation of a new local branch and linked worktree, durable ownership and recovery records, task association, authenticated bounded queries, TUI and administrative controls, and actual task-session launch in the associated checkout. Two tasks must run in different directories without changing the source checkout or each other's files. Git remains optional for every non-worktree workflow.

This document is planning only. Its publication does not close M10.01 or establish runtime evidence. The implementation agent must add the 36 atomic children in section 9 before coding and keep unverified items pending. Implement on `feature/m10-worktrees` from synchronized main, preserving existing work. Verify the documentation PR has merged before beginning.

Excluded: automatic commits, merges, rebases, resets, branch deletion, worktree removal, pruning, moving, repair, stashing, fetching, pulling, pushing, cloning, submodule initialization, provider APIs, release work, and production simulation. Existing external worktrees may be inspected but must not be silently adopted. No filesystem isolation is represented as a security sandbox. M11's wider manual reliability matrix remains M11 work.

## 2. Baseline and investigations required before implementation

Planning baseline: M09 merge `77fda5b99da0b1d18b669a14df4627472018e47e`. Recheck actual main and CI, rather than assuming this commit remains current. Source inspection identifies the following integration risks; these observations are not proof that regression scenarios have been executed.

| Location | Existing behavior or risk | Required reproduction and resolution |
| --- | --- | --- |
| `relayterm-domain/src/models.rs`, Task validation | Any non-null worktree ID is rejected | Round-trip a valid associated task and reject a dangling or cross-workspace reference at snapshot validation; replace the bootstrap prohibition with real invariants |
| `relayterm-domain/src/state.rs` and application ports | No worktree entities, intents, batch changes or transitions | Add deterministic pure state and transactional operations; do not implement side effects inside a database transaction |
| `relayterm-persistence-sqlite` | Nullable task column exists without a complete worktree model; migration discovery uses a source-directory path | Test a database upgraded from M09 and an installed executable after the build checkout is unavailable; embed migrations if the source dependency reproduces |
| `relayterm-protocol/src/message.rs` | Reserved create DTO has task, base, branch and relative destination only | Version the new durable receipt/root/revision contract explicitly; old reserved requests must not gain unsafe implied defaults |
| `relayterm-daemon/src/lib.rs`, session create | CWD must be under original root; resolver also confines relative executables to that root | Prove rejection before adaptation; add a validated task-specific root, never a global removal of containment checks |
| `relayterm-platform/src/process.rs` | Relative executable containment depends on the root parameter | Test `./tool` in a linked checkout, symlink escape, and context-sensitive availability with the same root used at spawn |
| TUI forms and launch selection | M09 claims captured revisions and stable IDs | Barrier tests must prove worktree creation/association cannot retarget after event refresh, selection movement, or reconnect |
| Runtime ownership and cleanup | M09 added a lifetime lock and bounded response reads | Preserve ownership before recovery; prove admitted Git operations survive client loss, shutdown waits or records uncertainty, and inherited pipe EOF cannot hang a test |
| Whole-snapshot reads and event codecs | New collections can exceed frames or be omitted from validation | Count and page worktrees/intents; preserve revision/watermark coherence and reject unsupported payloads |
| Common repository across separate workspaces | Workspace locks do not serialize common Git metadata | Start two production daemons for separate linked roots sharing one common Git directory and race branch creation |

Inspect all referenced code again. Add focused regression tests before fixing each reproduced defect. Document dismissed risks with exact evidence. Historical claims in `avances.md` are not substitutes for missing tests; append corrections if necessary, never edit earlier entries.

## 3. Product behavior and domain invariants

### 3.1 Repository identity and supported repositories

The workspace's canonical project root remains its stable identity. Discover Git only on explicit worktree inspection or creation, not as a prerequisite to opening a normal workspace. Support ordinary non-bare repositories and workspaces rooted at linked worktree tops, including `.git` files. Determine the canonical Git common directory and the canonical checkout top through Git, not by assuming `.git` is a directory. Record both privately. Use common-directory identity for local coordination, not remote URL, branch name, or personal identity.

For M10, creation requires the workspace root to equal the discovered checkout top. A workspace rooted in a subdirectory remains usable and receives explicit guidance to open the checkout top for worktree operations. Bare, unborn, missing and untrusted-ownership repositories get distinct bounded statuses. Do not run `git init`, create an initial commit, or add `safe.directory` automatically. Detached HEAD is supported if it resolves to a commit. SHA-1 and SHA-256 object IDs must remain opaque validated object IDs, not UUIDs or fixed 40-character assumptions.

Repository metadata is shared across linked worktrees. Dirty source files are preserved and never copied to the new checkout. The chosen base must exist locally. A remote-tracking ref already present locally is acceptable; network lookup is not. No automatic recursive submodule checkout. Document submodule and partial-clone behavior, including refusal when required objects are absent and lazy fetch is disabled.

### 3.2 Creation, selection and task rules

Only `LocalUser` requests creation, selection, clearing and explicit recovery. The system may inspect a recorded intent on startup; that observation must retain the original user actor without impersonating the user for unrelated mutations.

Default branch proposal: `rt/task-<full-task-uuid>`. Default destination: `<private-data>/worktrees/<workspace-uuid>/<worktree-uuid>`. Titles never become command text or filesystem names. The user can edit the proposed branch, choose the base (default `HEAD`), and explicitly select an alternative parent root. Show the exact branch and destination before submission. All three are captured in the draft with task ID and base workspace revision.

A task may own multiple historical worktree records, but selects at most one. Each worktree belongs permanently to exactly one task and workspace. Selection can choose only a ready worktree owned by that task, or explicitly clear the selection. Creation normally selects its new worktree atomically with final registration. Existing records for other tasks and external checkouts are not selectable. Clearing a selection changes no files and does not remove the worktree record.

Create/select/clear require a nonfinal, non-active task without an open claim and without starting/running task-context sessions. This avoids changing the intended checkout below existing work. Final tasks retain their historical association read-only. An in-flight creation reserves association changes for that task, but does not lock ordinary unrelated tasks. Task cancellation may proceed; if cancellation wins before finalization, keep a verified worktree ready but unselected. Never reopen a final task to complete an association.

Worktree state does not activate, claim, block, finish, or cancel a task automatically. Claims and handovers retain M02 semantics. A handover's changed paths refer to the selected checkout context; no diff extraction or automatic progress generation is added. A process with a task context uses the selection captured when launched, even after later user changes that are allowed once it has exited.

### 3.3 Durable models

Introduce validated records with private mutable fields and safe diagnostics. Reuse `WorktreeId`; add distinct operation and root identifiers if needed. Native paths use existing lossless native-path codecs. Never deserialize unchecked domain entities.

| Record | Required persisted data |
| --- | --- |
| Approved root | ID, workspace ID, native canonical parent path, observed filesystem identity where available, whether private default or explicitly selected, created timestamp |
| Creation intent | Stable operation UUID, workspace/task/worktree IDs, original actor, schema version, original expected revision, canonical repository/common-directory identity, approved root ID, native destination, branch, original base expression, resolved commit ID, request fingerprint, phase, safe reason, created/updated timestamps |
| Worktree | ID, workspace/task IDs, operation ID, root ID, canonical native checkout path, common-directory identity, branch ref, initial base commit, health, timestamps |
| Task association | Existing nullable `worktree_id`, validated against same workspace/task ownership; exposed path and branch are a joined projection, not independently editable duplicates |
| Instance launch context | Immutable nullable worktree ID plus actual canonical working directory; preserve existing definition snapshot, task, instance and session IDs |

`ready` health authorizes selection/launch after fresh validation. `missing`, `mismatch`, and `unavailable` health refuse new launches but retain history and selection for inspection. Health is an observation, not permission to modify Git. Deleting an external directory never causes Relayterm to recreate it automatically. Do not overwrite recorded branch when Git reports a different current branch; expose a mismatch requiring manual review. Normal commits on the expected branch are allowed after creation, so future launch does not require HEAD to equal the initial base commit.

Enforce foreign references and uniqueness in domain snapshot validation and SQL. Reserve destination and branch across nonterminal intents within one workspace. Enforce global/common-repository collisions through filesystem locks and Git itself. Reject time reversal and invalid reconstruction. Retain intents and records rather than deleting history to permit ID reuse.

### 3.4 Creation state machine and durable receipts

Use these semantic states, with typed reasons rather than raw Git stderr:

| Phase | Meaning | Permitted next states |
| --- | --- | --- |
| `prepared` | Request validated and durable intent committed; no add dispatched yet | `applying`, `failed`, `needs_attention` |
| `applying` | Durable dispatch marker committed before the Git child may run | `ready`, `needs_attention`, `failed` only with proof of no effects |
| `ready` | Verified worktree record committed, with selected or explicitly unselected outcome | None; health observations are separate |
| `failed` | Definitive pre-effect rejection or verified absence of all side effects | None; a deliberate new operation uses a new ID |
| `needs_attention` | Partial or uncertain effects, conflicting identity, or unprovable ownership | `ready` through read-only verification and finalization, or remain unresolved |

The operation UUID is a durable idempotency key independent of request IDs and daemon generations. Identical duplicate submissions return the recorded status/result without invoking `worktree add` again, including after restart. A different payload with the same ID conflicts. Compute identity from canonical typed request fields, not unstable JSON ordering; compare full fields as well as a digest if used. Check a known receipt before rejecting its original stale expected revision. New requests still require the captured revision. Two simultaneous duplicates must yield one intent and at most one dispatch.

An accepted `prepared` intent may be executed by its owning live worker. On restart, do not automatically dispatch an interrupted operation or assume that `prepared` authorizes a new attempt after ambiguous persistence. Expose status and explicit review. An `applying` intent is never blindly rerun, even when no directory is visible. An externally surviving Git child may still act after the daemon dies. Startup must account for that possibility and must not dispatch a competing add for unresolved reservations.

An unresolved creation blocks further task-context launch and worktree reassignment for its task until safely resolved, but leaves unrelated tasks operational. Explicit reconciliation may mark `prepared` or `needs_attention` as `failed` only when durable dispatch evidence, confirmed process quiescence and a fresh branch/destination inventory prove there are no effects; this is the sole additional state-machine edge. Merely observing absent files is insufficient. This releases reservations without deleting any Git object. Otherwise provide manual inspection guidance and retain the reservation. Test this path so a pre-dispatch interruption does not permanently strand a task after proven safe resolution.

### 3.5 Side-effect sequence

1. Authenticate, validate bounded request, check receipt, capture task/revision and verify eligibility.
2. Resolve installed Git once through the neutral resolver. Resolve repository identity and canonical root, branch and local base commit. Resolve symbolic base exactly once and persist that object ID.
3. Acquire a private common-repository creation lock shared by cooperating local Relayterm daemons. Revalidate identity, collisions and task state under it. Never lock solely by workspace UUID. Lock naming uses an opaque digest of canonical native common-directory identity. Do not change Git's own lock files.
4. Commit the intent and reservations with a revision comparison and ordered metadata-only event. Release the SQL transaction before invoking Git.
5. Persist the `applying` marker. Spawn exactly one bounded Git process using argument arrays and the pinned commit. Own its process and pipe readers independently from client connection lifetime.
6. Wait for confirmed child exit. Timeout, cancellation or nonzero exit may leave side effects; terminate/reap owned children where possible and inspect state. Never equate a process error with rollback.
7. Verify destination, linked-checkout registration, branch, common directory, HEAD and approved-root containment. A zero exit alone is insufficient. The just-created checkout must match the pinned base; later recovery with an advanced HEAD cannot assume pristine success and needs review.
8. In a fresh transaction, compare the intent phase and task association expectations, insert the validated record, complete the receipt, optionally select the worktree, and emit events atomically. Unrelated workspace revisions need not destroy successful Git work: re-read and finalize using intent identity and task-specific guards. If only task eligibility changed, finalize unselected and report that outcome.
9. Notify after commit. Return the durable operation status and worktree ID. A notifier failure cannot repeat the side effect. Release common-repository worker ownership after cleanup.

Do not keep a SQLite write transaction or the main IPC control loop blocked while Git runs. At every failure point preserve existing content. No compensation by removing a directory, branch, index lock or Git administrative record.

Document the scope of that coordination lock when explicit private-home overrides differ: a normal per-user runtime namespace must converge for the same common directory, while isolated test namespaces may deliberately differ. Git's own collision checks remain authoritative across noncooperating processes or namespaces. Do not claim that a private lock excludes arbitrary external Git commands. Require proven dispatch ownership or explicit unresolved status rather than using a branch/path match alone as attribution.

Recovery inspects the same receipt, destination and Git inventory. Match the exact canonical path, expected common directory, expected branch and pinned initial commit, plus the durable intent. If evidence is inconsistent or potentially external, retain `needs_attention`; do not adopt a coincidentally matching unrelated checkout. Before automatically finalizing on restart, establish no owned/external in-flight child can still modify the operation, using platform process ownership or conservative unresolved status. An explicitly requested reconciliation can finalize a positively verified result without creating Git state. No PID-only kill after restart. Surviving children and stale Git locks require manual guidance if safe ownership cannot be established.

## 4. Native Git adapter and security contract

### 4.1 Boundaries and command inventory

Add `relayterm-git` only when the adapter is implemented. It may depend inward on application/domain and platform primitives, but never TUI, daemon, client or SQLite. The daemon composes it. The application exposes async ports with `Send` futures for finite discovery/verification/create operations and deterministic test doubles. Domain/application/protocol remain free of Tokio and concrete Git dependencies. Extend the architecture allowlist only for these deliberate edges.

Prototype commands against the actual native Git versions before freezing adapter APIs. Use an absolute resolved executable and separate native argv items, null stdin, bounded concurrent stdout/stderr drain, explicit CWD and explicit environment. No shell command string or PowerShell wrapper. Git must be installed by the user; no installer or download action.

Allowed command purposes: version/capability check, `rev-parse` repository/commit discovery, `check-ref-format` validation, `show-ref` exact collision inspection, `worktree list --porcelain -z`, and `worktree add -b <branch> -- <absolute-destination> <pinned-commit>`. Validate actual support and option ordering in the prototype. Never use `-B`, `--force`, orphan creation, guessing, tracking defaults, or branch reuse. Prefer `--no-track` where native support is verified.

Use `rev-parse --verify --end-of-options <base>^{commit}` or an empirically equivalent option-safe command. Reject leading-option bases before Git. Ref-format validation alone is insufficient: `--branch` may expand checkout shorthand, so reject shorthand such as `@{-1}` and validate the literal `refs/heads/<branch>` form. Full object IDs must be passed as a single final argument, with SHA-256 supported. Git worktree listing uses NUL-delimited porcelain records, preserving native path bytes and spaces without splitting on whitespace or assuming quoted paths.

Official references checked during planning on 2026-09-09: [worktree operations and porcelain format](https://git-scm.com/docs/git-worktree), [repository and commit parsing](https://git-scm.com/docs/git-rev-parse), [reference validation](https://git-scm.com/docs/git-check-ref-format), and [Git environment and configuration controls](https://git-scm.com/docs/git). Revalidate supported flags during implementation and record exact tested Git versions. A missing required capability returns a safe unsupported-version result; do not silently fall back to human-readable parsing.

### 4.2 Environment, hooks and network behavior

The Git adapter has its own minimal environment, independent of provider environment allowlists. Clear inherited `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_INDEX_FILE`, object/alternate-directory variables, `GIT_CONFIG_*` injection, trace variables and credential/askpass controls. Retain only platform essentials and bounded executable search as needed. Disable prompts, pagers, optional locks for read-only operations, automatic maintenance and lazy fetching. Do not bypass dubious-ownership checks or alter global/local Git configuration.

A checkout can invoke post-checkout hooks, filters and external helpers. Prototype and document this explicitly. Disable hooks per invocation using an application-owned verified empty hooks directory, never a project-provided path. Do not run credential helpers or fetch missing objects. Inspect configured checkout filters without executing them: if active external smudge/process filters cannot be safely neutralized with a tested per-command policy, reject creation before dispatch with `unsupported_checkout_filter`; retain ordinary non-Git operations. No silent data-content substitutions. Test a canary hook, process filter, askpass, pager and lazy-fetch configuration. Native evidence must prove the chosen contract; argv separation alone is not sufficient.

The MVP trusts the local user, but treats repository strings as untrusted input to diagnostics and command parsing. Avoid claiming protection against an adversarial same-user process replacing files between validation and Git's path lookup. Reject detected swaps and document residual TOCTOU limitations. Use native handle/file identity where feasible without pretending a canonicalization call is an atomic filesystem capability.

### 4.3 Root, destination and reference validation

Default parents are created only on explicit create, under private application data with existing owner-only permissions. An additional root must already be an absolute native directory explicitly selected for this workspace and validated before registration. Do not chmod a user-owned existing tree, take ownership, create arbitrary missing ancestors, or persist consent from an unsubmitted form. Display the final destination before user submission.

Use a single portable destination leaf (default worktree UUID) under the approved parent. Reject empty, dot, dot-dot, NUL/control characters, path separators, drive-relative paths, UNC/device prefixes, colon/alternate data streams, Windows reserved names, trailing dot/space, and absolute destinations. Parent paths may contain spaces and Unicode and use the native DTO. Reject known escape components lexically and canonicalize the existing parent physically. Verify it is outside the source checkout, Git administrative directory, runtime socket/log/database directories, and any other tracked checkout discovered for this repository. The default application's dedicated worktree subtree is permitted.

The leaf must not exist, even as an empty directory, dangling symlink or reparse point. Do not precreate the leaf before Git. Recheck parent identity and destination absence immediately before dispatch. After Git, canonicalize the resulting directory and prove containment by path components/native identity, not string prefix. On Windows test junctions and case aliases; on Unix test symlinks and non-UTF-8 parent paths. Treat case-folding collisions according to the actual filesystem. Same Git branch or common-directory path reserved by a different operation must fail predictably.

Branch input is a literal short local branch name, at most 256 UTF-8 bytes, nonempty and accepted by Git ref rules plus portable restrictions. Reject leading `-`, controls, special revision expressions, ambiguous shorthand and existing refs. Branch slashes are allowed, unlike destination leaf separators. Base expression is at most 1024 bytes, nonempty, no controls or leading `-`; only locally resolved commit-ish values are accepted. Do not expose ambiguous ref warnings or raw filenames in diagnostic errors.

### 4.4 Resource limits and cancellation

| Resource | Initial bound |
| --- | --- |
| Worktree records and retained creation intents | 1024 each per workspace, reject new admissions at cap |
| Approved roots | 16 per workspace including default |
| Destination leaf | 128 UTF-8 bytes and portable validation |
| Native path encoding | Existing 8192-byte DTO ceiling plus native OS validation |
| Concurrent Git children | 2 per daemon, at most 1 creating per common repository |
| Pending Git requests | 16 per daemon; reject excess without spawning |
| Read-only Git command | 5 seconds |
| Create worker | 60 seconds, followed by bounded terminate/reap and uncertain-state recording |
| Captured Git stdout / stderr | 4 MiB / 64 KiB, concurrently drained, overflow explicit |
| Parsed external inventory | 4096 records, fail explicitly if exceeded |
| Returned page | 50 default, 200 maximum; reject zero; enforce encoded frame bound |
| Startup reconciliation | Bounded pages, at most 5 seconds before deferring unresolved work; normal workspace readiness must remain possible |

Elapsed limits use monotonic time. Output readers and subprocess teardown must also be bounded; no unconditional join or EOF wait defeats the deadline. A timeout must retain worker admission until the process is confirmed gone or conservatively marked unresolved. Do not stack unbounded blocking tasks after timing out their async wrappers. Graceful shutdown stops new admissions, drains admitted workers within their existing deadline and commits known outcomes before releasing lifetime ownership. Abrupt shutdown leaves durable intent for inspection. Tests must include shared metadata contention without sleeps to select a winner.

## 5. Persistence, protocol and launch integration

### 5.1 Migration and storage

Add the next workspace migration without editing migration 0001 or altering its checksum. Preserve registry schema unless a concrete field requires change. Upgrade from a real M09 database containing tasks, claims, progress, handovers, definitions and instance snapshots. Old nullable associations remain null. Add constraints, foreign keys, unique operation IDs, and indexes for workspace/task listing, unresolved phases, branch/destination reservations and pagination. Rebuild tables safely if SQLite requires it for a foreign key.

All new records, receipt transitions, association changes and events must flow through typed batches and atomic commit. Update insertion ordering, read reconstruction, in-memory doubles, full snapshot validation and corruption tests. SQL-only tests do not replace pure invariants. Store paths and branch data only in private entity tables and authorized detail responses. Event payloads contain IDs, phases, reason enums and changed-field names, not paths, base expressions, command output or branch labels. Version new payloads explicitly and fail clearly on unsupported versions. A known receipt with a missing referenced record is integrity failure, never permission to re-create.

Read-only recovery never migrates or repairs an unknown database. Failed migrations must preserve existing data. Test old-binary rejection of newer schema, migration interruption, rollback, backups and fresh install away from the source tree. Never claim SQLite provides atomicity with Git.

### 5.2 Public operations and DTOs

Activate reserved operations through a new advertised `worktrees_v1` capability while retaining protocol version 1 only if the existing envelope compatibility contract holds. Reserved M09 requests returned unavailable, so replace their unimplemented DTOs deliberately and document the change. Clients without the capability retain all existing worktree-free flows.

| Operation | Request | Result and semantics |
| --- | --- | --- |
| `worktree.inspect_repository` | Empty or expected revision | Typed Git/repository availability and root eligibility, safe capabilities; no automatic mutation |
| `worktree.create` | Payload version 1, operation UUID, task ID, expected revision, base, branch, destination leaf, optional explicitly approved native parent | Quickly accepted durable intent or definitive rejection; Git runs under daemon ownership; result includes stable operation/worktree IDs and phase |
| `worktree.get_operation` | Operation UUID | Durable phase, safe reason, selected/unselected outcome, worktree ID; read-only reconciliation view |
| `worktree.list` | Optional task filter, after ID, limit, expected revision for continuation | Owned records, health and authorized path/branch details, revision and event watermark; external records never become owned by listing |
| `worktree.select` | Task ID, nullable worktree ID, captured expected revision | Atomic association or explicit conflict; no launch, claim or Git side effect |
| `worktree.reconcile` | Operation UUID and expected revision | Explicit bounded verification/finalization only; never another add |

Choosing an alternate parent through create durably registers that root in the same intent transaction after validation; reopening a persisted root still requires filesystem revalidation. Public clients cannot directly register a fabricated ready worktree or submit canonicalization proof.

Define typed statuses such as `git_missing`, `git_unsupported`, `not_repository`, `unsupported_root`, `invalid_reference`, `branch_conflict`, `destination_conflict`, `path_rejected`, `busy`, `unsupported_checkout_filter`, `needs_attention`, and `storage_unavailable`. Fit them into existing error/recovery conventions without echoing raw errors. Distinguish rejected/not-applied from accepted/partial and unknown delivery. A create receipt is an operation result, not a mutation replay license. The client may poll get_operation automatically, but must not automatically resend create after an uncertain write. Preserve the operation UUID in the draft and status display.

Snapshots and event invalidation must include or coherently reference new collections. Paginate at a single captured revision; restart staged reads on conflict within existing bounds. Ordinary worktree.list reads durable records and cached typed health; explicit inspect/reconcile and launch trigger bounded live Git verification. Do not run an expensive Git inventory for every render or every workspace snapshot.

### 5.3 Launch root and snapshot consistency

For a task-context launch, read task selection and definition from one revision. No selection means original workspace root. A selection requires ready ownership in the same task/workspace and fresh Git/path verification. Default CWD is that checkout root. An explicit requested subdirectory is allowed only within that selected root. A missing or mismatched worktree fails the launch, with no fallback to source root. A taskless launch stays confined to the original workspace root and cannot supply an arbitrary owned worktree path to bypass association rules.

Pass the selected validated root to both CWD validation and relative-executable resolution. Availability checks need optional task context and the same revision/root semantics so an available badge for `./tool` in one checkout is not reused for another. Absolute configured executables retain existing generic behavior. Environment and ordered/empty arguments remain unchanged. The effective worktree ID, working directory, task, definition snapshot and returned instance/session IDs are captured together at registration.

Prevent selection/create/clear racing launch admission with task-scoped guards through starting registration and spawn handoff. Recheck domain revision before registration and release reservations on every rejected path. After registration, any failure including environment assembly must record failed lifecycle and clear the launch receipt/reservation without fabricating a running session. Test edits, cancellation, selection and another launch using deterministic barriers. Existing live sessions' recorded directories never change. Git verification checks common directory and branch, not HEAD equality with initial base after users have committed locally.

## 6. TUI, administrative CLI and user documentation

Add discoverable task-context actions for inspect, create, list, select, clear and review recovery. Prefer contextual forms or a worktree panel over overloading existing single-letter bindings without updated help. Show task ID, selected branch/path, status and recovery action with text, not color alone. Fit the normal layout at 80x24, support scrolling and the existing smaller-terminal behavior. Escape controls in paths/refs outside a real terminal pane, preserving lossless underlying native values.

Create form captures task ID, revision, base, proposed branch, approved parent and leaf on opening. Require explicit submission of the visible destination; duplicate keys dispatch once. A pending operation remains visible after navigation or reconnect. Event refresh never silently changes a draft target or replaces a chosen root. On stale revision, preserve the draft for explicit review. On unknown result, show the operation ID and refresh/poll status before any deliberate new action. Close/discard before submission creates nothing. Clearing selection must clearly say files remain; it is not a deletion flow.

Session launch from Tasks must provide task context. Agent and shell launches must use the same validated root policy. Show selected worktree context before launch and actual immutable context on session details. Selection of a missing ID, empty list, duplicate branch label or worktree belonging to another task must never target another row or the source checkout.

Add `rt worktree inspect`, `create`, `list`, `operation`, `select`, `clear`, and `reconcile` through IPC only. Exact argument spelling must match tested help. Create supports an explicit `--operation-id`, required captured revision, task ID, optional base/branch/parent/leaf, and reports accepted versus complete honestly. No script needs to parse free-form stderr or use a local database path. Reuse versioned text/JSON output and bounded input. Define exit codes for completed, pending/needs-attention and rejected states without pretending pending is fully completed. A CLI wait option may poll reads with a deadline; expiry does not cancel or repeat creation.

Publish `docs/worktrees.md`: Git prerequisite and tested versions, working-tree-root restriction, new-branch-only behavior, local base resolution, dirty files, detached HEAD, missing Git, approved parents, native path limits, filter restrictions, manual reconciliation, uncommitted-file preservation, startup uncertainty, and task-context launches. Explain explicit manual cleanup belongs to the user and should begin by inspecting all uncommitted/untracked work. Do not implement cleanup commands or give destructive one-liners as an automatic workflow. Update README, daemon/TUI guides, specification fields, ADR 0007, IPC/persistence/privacy decisions and platform results to match actual evidence.

## 7. Verification and native acceptance

### 7.1 Focused tests

- Domain/application: every intent transition including identical receipt, conflicting payload, temporal violations, stale revision, task cancellation, final task and cross-workspace references. No effects on rejected batches. Test association and instance snapshots independently of Git/SQLite.
- Adapter: actual Git in disposable repositories; SHA-1 and SHA-256 if installed Git supports both; detached HEAD, unborn/non-Git/bare roots, linked `.git` files, subdirectory policy, invalid refs, dirty source, missing objects, hooks/filters, no network helpers, nonzero status, malformed NUL output, unknown optional porcelain fields, output caps and hung children.
- Paths: spaces, Unicode, Unix non-UTF-8 parent, native Windows encoding, relative drive paths, device paths, ADS, reserved names, case collisions, symlinks/junctions, dangling symlinks, existing empty and nonempty leaves, common-directory replacement and source-root containment. Unsupported encodings fail explicitly without lossy acceptance.
- Races: same receipt twice; same branch/destination under different receipts; separate workspaces sharing Git; association versus launch; definition edit versus launch; cancel versus final commit. Barriers choose interleaving, not sleeps or statistical retries.
- Failure injection: before intent commit, after intent/before dispatch marker, after marker/before spawn, after spawn/before exit, after Git success/before verification, during final SQL commit, after commit/before response, notifier failure, client disconnect and daemon crash. Only tests expose fault controls. Assert directory contents, refs, records and events at each boundary.
- Recovery: identical receipt after restart, branch-only partial effect, directory-only effect, foreign registration, advanced HEAD, missing Git, external move/removal, surviving child, corrupted record and missing receipt. No second add or destructive cleanup. Unresolved operations must not prevent unrelated task/terminal use.
- Transport/TUI: strict DTOs, unsupported capability, older clients, stale drafts, unknown result, safe text and JSON, operation polling, multi-page coherent snapshots, selection by ID, empty selection, keyboard help and bounded rendering.
- Privacy: inject synthetic path/ref/credential markers into Git stdout/stderr and request errors; assert absence from ordinary diagnostics/events. Authorized detail queries may expose selected local paths, never raw Git transcripts or environment values.

### 7.2 Required real process journey

Create `worktree_gate` as a native integration target, using production `rt`, real SQLite, authenticated IPC, installed native Git and actual PTY/ConPTY. Test-only helpers share production composition and cannot be invoked through product simulation flags. Use synthetic committed fixture content and test-only author identity inside disposable test repositories. Real project files and global Git config remain untouched.

1. Initialize a disposable repository with a committed fixture and a separate non-Git directory. Record initial branch, HEAD, source file bytes and dirty/untracked canaries.
2. Open the repository through real TUI, create two tasks and two worktrees through the new TUI flow. Do not replace creation/selection actions with administrative setup.
3. Verify branches differ, destinations differ and both are listed by actual Git and persisted as task-owned records. Source branch, HEAD and dirty/untracked files are unchanged.
4. Launch actual generic interactive processes for each task through TUI. Have each print a bounded synthetic CWD marker and create different contents under the same relative filename in its checkout. Compare disk files and persisted instance snapshots. Source checkout and other task files must be unaffected.
5. Reject cross-task association and changing selection while a task-context session is starting/running. Existing session continues accepting input after rejected operations.
6. Explicitly claim and record progress, detach/reconnect, prepare handover and continue a task in a new instance at the same associated worktree. Output never changes task state automatically.
7. Exercise custom approved root, invalid branch/base/destination and a collision through real controls. No partial SQL association or overwritten material is allowed.
8. Run deterministic injected failure after successful real Git add but before database finalization; close/restart the test host and inspect through production IPC. Prove the directory/branch remains and an explicit reconciliation completes without invoking add again. Cover unprovable cases remaining unresolved.
9. Stop with live Git work or kill the test host at dispatch; verify bounded ownership handling and no automatic duplicate on restart. Keep external uncertain objects intact for test inspection.
10. Restart production daemon and verify durable associations, receipts and honest lost-session recovery. Launch new sessions in both validated directories. Worktrees persist even though live PTYs are not adopted across daemon restart.
11. Run the normal TUI task/session/claim/handover flow in the non-Git workspace and with Git deliberately unavailable. Provider accounts are unnecessary. A missing Git status must not break startup or existing generic sessions.
12. Confirm public project content contains no runtime database, logs, sockets or metadata export. Cleanup only the test harness's known disposable resources after all assertions; product recovery never deletes them.

Run this gate twice consecutively on native stable Linux, macOS and Windows, in separate CI steps with independent failure propagation. Preserve existing M06-M09 gates and their repetitions. Compile-only checks and mocks do not establish native acceptance. Capture OS, Git/rust versions, candidate SHA, command, pass count, stage durations, bounded memory/output observations and sanitized outcomes. Measure create and reconciliation durations against configured deadlines and UI responsiveness while Git is busy; retain existing TUI latency/flood gates. Fail with a typed result at configured resource bounds rather than raising limits to conceal hangs. Manual emulator/SSH observations are useful but distinct from this mandatory matrix.

### 7.3 Required commands

Run narrow crate checks while iterating, then all of the following. Each command must propagate its own exit code on Windows and Unix. Verify the native workflow's multi-line toolchain setup does not mask an earlier command failure.

```text
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
cargo test -p relayterm-git --locked
cargo test -p relayterm-cli --test worktree_gate --locked -- --nocapture --test-threads=1
cargo test -p relayterm-cli --test worktree_gate --locked -- --nocapture --test-threads=1
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner --exit-code 1
git diff --check
```

Also run targeted migration/installed-executable, protocol, runtime, launch race and native path tests. CI must run the complete existing suite and new tests, not just add an unused target. Document warnings, optional omissions and unavailable evidence honestly. No mandatory platform omission closes M10.

## 8. Compatibility and acceptance checklist

- Existing definitions, tasks, claims, handovers, events, configuration imports, generic shell and provider-template flows still operate.
- SQLite upgrade is atomic, historical data is unchanged, all new fields have validated reconstruction, and migration resources travel with the installed executable.
- IPC version 1 remains only with explicit additive capability/version negotiation; old unavailable worktree payloads fail safely. New snapshots/events cannot silently disappear from older-client synchronization.
- Domain/application/protocol retain architecture closure tests. New Git work runs only in adapters composed by the daemon.
- Every accepted creation has a durable stable receipt before side effects. Uncertain outcomes preserve files and remain inspectable after restart. No Git/database distributed transaction is claimed.
- Two tasks launch in their actual distinct Git worktrees with exact immutable context; no arbitrary CWD escape or source-root fallback.
- Git installation, checkout root restrictions and partial recovery are documented with actionable safe errors. Normal non-Git operation stays independent.
- Native stable Linux, macOS and Windows each pass two consecutive gate invocations, plus inherited controls, with real evidence published.
- All six M10 parents and 36 children map to test/evidence locations. Delivery tasks remain open until their actions occur; never claim a merge in advance.

## 9. Atomic implementation queue

Add these 36 children beneath their existing six parents in TODO before code changes. Retain the contract link. Each task includes tests and same-change logbook evidence; remove parents only after all child outcomes and acceptance are verified.

| ID | Reviewable outcome | Required evidence |
| --- | --- | --- |
| M10.01a | Recheck baseline, instructions, official Git contracts and section 2 risks | Focused reproductions or evidence-based dismissal, CI baseline |
| M10.01b | Prototype native discovery, NUL parsing, commands and side-effect policy | Real Git on three platforms, hooks/filter/no-network decisions |
| M10.01c | Refine ADR 0007 with identity, task ownership, receipts and state machine | Cross-review tables, cancellation and partial-failure examples |
| M10.01d | Define domain records, transitions, roots and association invariants | Pure reconstruction, full transition and cross-workspace tests |
| M10.01e | Specify migration, event inventory and protocol compatibility | Field-to-storage/DTO mapping, older-version behavior |
| M10.02a | Add narrow Git adapter and explicit dependency edges | Architecture negative tests and adapter-only invocation |
| M10.02b | Implement bounded native repository/commit discovery | Linked tops, non-Git, unborn/bare, SHA formats, ownership errors |
| M10.02c | Implement streaming NUL inventory parsing and ownership join | Native paths, limits, external entries not adopted |
| M10.02d | Implement explicit Git environment, hooks/filter policy and drains | Canary helpers never run, stdout/stderr bounds, timeout/reap |
| M10.02e | Add bounded inspect/list ports and typed diagnostics | Missing Git non-creating behavior, privacy and pagination |
| M10.03a | Implement portable branch/base/leaf validators | Exact limits, option injection, shorthand and collision tests |
| M10.03b | Implement approved-root and destination validation | Symlink/junction, case/native identity, preexisting path tests |
| M10.03c | Implement common-repository admission and durable reservations | Two daemons sharing one Git directory, same/different receipt races |
| M10.03d | Implement pinned-base add worker with durable dispatch marker | Real add, source unchanged, changed base ref does not retarget |
| M10.03e | Verify actual Git outcome and bounded worker teardown | Nonzero/timeout partial effects, no destructive compensation |
| M10.03f | Integrate daemon client-loss and shutdown ownership | Admitted operation survives disconnect; no premature lock release |
| M10.04a | Add validated worktree/intent/root batches and in-memory support | Atomic associations/events and rejected-batch preservation |
| M10.04b | Add next migration and complete SQL codecs/constraints | M09 upgrade, corruption, rollback and installed migration resources |
| M10.04c | Add durable receipt lookup and duplicate semantics | Same ID after restart, changed payload conflict, at most one add |
| M10.04d | Implement recovery inspection and explicit finalization | Crash boundaries, unprovable ownership remains unresolved |
| M10.04e | Add health checks, coherent queries and metadata-only events | Missing/moved/mismatched checkout, no paths in events |
| M10.04f | Verify finalization against task changes and SQL failures | Unrelated revision, cancellation, unselected result, no data loss |
| M10.05a | Activate versioned authenticated worktree IPC and capabilities | Strict DTOs, reserved legacy rejection, safe older client |
| M10.05b | Add CLI inspect/create/list/operation/select/clear/reconcile | Help, text/JSON, exit codes, no direct SQLite/Git client use |
| M10.05c | Add TUI create and approved-root preview forms | Real submission, captured task/revision, discard creates nothing |
| M10.05d | Add stable list/selection, clearing and recovery status | Empty/missing/cross-task IDs, pagination, unknown outcomes |
| M10.05e | Integrate task launch CWD and context-sensitive resolver/check | Relative tool in checkout, no escape or root fallback |
| M10.05f | Capture immutable worktree launch context and close races | Association/edit/cancel/launch barriers, exact IDs and snapshots |
| M10.05g | Verify client reconnect and busy Git responsiveness | Single-submit, bounded polling, retained draft, inherited latency gate |
| M10.06a | Publish worktree guide and update specification/ADRs/references | Executed examples, manual-safe recovery, no unsupported claims |
| M10.06b | Build real Git/TUI/SQLite/PTY gate with isolated fixtures | Two tasks, real cwd/files, unknown CLI and non-Git journey |
| M10.06c | Add crash, race, migration and security gate scenarios | Real partial add, restart, no repeat/deletion, native path evidence |
| M10.06d | Run two native repetitions on Linux/macOS/Windows | Independent step failures, exact versions and candidate links |
| M10.06e | Run complete quality/privacy/security and inherited gates | Section 7.3, no bypassed check or masked exit code |
| M10.06f | Publish evidence and close verified implementation items | Six parents/36 children mapped, append-only log, omissions explicit |
| M10.06g | Deliver PR, reviews, merge and post-merge verification | Required CI/reviews, synchronized main, final run outcomes |

Order: investigations and native adapter prototype; contracts and records; migration and receipts; validated creation/recovery; protocol and launch; TUI/CLI; fault and native gates; full checks and delivery. Parallel independent work is optional only if the user or applicable instructions authorize delegation. Do not consolidate APIs before the native prototype resolves path parsing and checkout side effects.

## 10. Delivery and stop condition

Create focused English commits and a PR explaining final behavior, compatibility, native tests and partial-recovery limitations. Supervise Quality and Security. Fix related failures, never skip tests, mask exit statuses, repeatedly rerun unchanged failures until green, or extend deadlines without measurements and a justified contract update.

Merge only after mandatory native acceptance and required repository reviews pass. Preserve an open item for missing evidence or access and report the concrete blocker. Explicit authorization to merge does not authorize force-push, branch deletion, discarded changes, releases or destructive cleanup. After merge, synchronize main without losing work, confirm equality with origin/main and monitor post-merge CI. Record delivery only after it exists, using an append-only follow-up if needed.

Return PR URL, merge commit, commands actually executed, native Git/OS evidence, timings, CI state and remaining limitations. Stop before M11 implementation. This planning delivery itself must modify documentation only, record M10-DOC verification, preserve all six M10 implementation parents, and finish with the handoff prompt.
