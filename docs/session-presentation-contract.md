# Session presentation contract

Status: Frozen for M12 implementation.
Task: M12.00a.
Scope: Durable session labels, creation order, bounded reads, form cursor placement, stale-form guidance, and competing-input guidance.

## Source assessment

The M11 model stores process identity and lifecycle in `AgentInstance`. It has no mutable presentation metadata or creation ordinal. SQLite schema version 2 stores instances by stable IDs and timestamps. The legacy `session.list` protocol operation pages instances by ID, while the TUI collects every page and sorts the complete history by session ID. Form editing already stores a UTF-8 byte offset, but rendering does not place the physical terminal cursor. Application conflicts map to one protocol error code, so the TUI cannot safely distinguish a stale draft or a competing input owner from other conflicts.

M12 keeps process identity, launch snapshots, task context, worktree context, terminal leases, and lifecycle records unchanged. Presentation metadata is a separate durable entity. No name becomes an identity or command argument.

## Domain mapping

Add a private `SessionPresentation` entity with these fields:

- `workspace_id: WorkspaceId`
- `session_id: TerminalSessionId`
- `creation_ordinal: SessionCreationOrdinal`
- `display_name: Option<String>`

`SessionCreationOrdinal` is a nonzero positive integer whose largest persisted value is `i64::MAX`. A workspace stores `next_session_ordinal`, initially one. Registering an instance and its session consumes that value atomically and advances the counter with checked arithmetic. Every complete workspace has exactly one presentation row per instance session. Session IDs and creation ordinals are unique within the workspace. Transaction projections may contain a subset, but every loaded presentation must reference a loaded instance or be validated by adapter constraints.

A display name is normalized only at the application request boundary by removing leading and trailing Unicode whitespace. An empty normalized value means `None`. A present name contains at most 128 UTF-8 bytes. It rejects NUL, C0 and C1 terminal controls, line breaks, ESC, and Unicode bidirectional formatting controls U+061C, U+200E, U+200F, U+202A through U+202E, and U+2066 through U+2069. Other Unicode, combining characters, internal spaces, and duplicate names are valid. Domain reconstruction accepts only an already normalized canonical value and validates the same limits.

Add a `RenameSession` user command. It requires `LocalUser`, the stable session ID, and a canonical optional name. An equal value is a successful no-op that does not update the workspace revision, timestamp, or event sequence. A real change updates only the presentation entity and emits a version 1 `session_renamed` payload containing the session ID and changed field name. The event excludes the name. Historical, running, lost, exited, failed, and terminated sessions remain renameable.

The default label is derived for display as `Session N`, where `N` is the creation ordinal. It is never persisted as a user name. Lists show the chosen label and a collision-safe abbreviated session ID. Details show the full session and instance IDs.

## Persistence mapping

Add migration `0003_session_presentation.sql`; never edit migrations 0001 or 0002. The migration adds a bounded `next_session_ordinal` to `workspace_meta` and creates `session_presentations` with foreign-key, unique ordinal, byte-length, and positive-range constraints. It backfills existing sessions per workspace with `ROW_NUMBER()` ordered by `(started_seconds, started_nanoseconds, instance_id)`, then sets the next counter to one more than the maximum.

The migration is one SQLite transaction. An interruption leaves schema version 2 unchanged. Reopening a migrated database does not repeat or duplicate the backfill. Fresh databases apply all migrations. Schema 1 and schema 2 populated copies must reach the same current representation. Equal timestamps use instance ID bytes as the deterministic tie-breaker. Counter exhaustion rejects registration without partial instance, presentation, event, or revision changes.

The adapter loads the counter for mutations that can register a session. It persists the instance, presentation, counter, workspace revision, and pending event in the same commit. Rename persists metadata and event atomically under the expected workspace revision. Full integrity validation, backup, restore, and the manifest's workspace schema value include the new table and counter. A pre-M12 binary refuses schema version 3 before mutation.

## Application and paging mapping

Add an ordered session summary read port separate from the legacy instance page. A summary contains the immutable instance projection required by the Sessions view plus display name and creation ordinal. The order is ascending creation ordinal, followed by session ID as a defensive tie-breaker.

The request contains a limit, an optional keyset cursor `(creation_ordinal, session_id)`, and an optional expected workspace revision. The first page captures and returns the authoritative revision. Every continuation supplies that revision. A changed revision returns the typed stale-page result and no mixed page. Limits default to 50, reject zero, and cannot exceed 200. SQL uses a set-based keyset predicate and fetches at most `limit + 1` rows. The returned next cursor identifies the last emitted row. No normal TUI refresh collects the complete session history.

The TUI stores page cursors and selection by session ID. Moving between pages performs one bounded read. A background session creation does not move focus on an installed page. A rename refresh preserves the selected ID. If a selected session is absent after an explicit page refresh, selection falls to the first visible ID. Names near 128 bytes remain subject to the existing response-size budget.

## Protocol compatibility

Protocol version 1 remains unchanged. `session.list` and `AgentInstanceDto` retain their exact existing request, response, and ordering behavior for older peers. Add advertised operations `session.list_ordered` and `session.rename`; an older server omits them and rejects them before mutation. A new client against an older server uses the legacy list display, marks ordering as legacy, and disables rename with bounded guidance. An older client against a new server continues using `session.list`.

The hello operation list is the capability source. The client retains negotiated operations across the connection and refreshes them after reconnect. New request and result DTOs are strict and versioned by their operation contract. Decimal-string encoding remains in use for ordinals, revisions, and counters.

Add an allowlisted optional conflict reason to the error contract, negotiated through the new operation capability where needed. The initial reasons are `stale_revision` and `input_owned`. Unknown reasons degrade to the existing safe generic conflict. Raw messages are never parsed. Older peers can continue to use the generic `conflict` code. Error effects and recovery guidance remain authoritative, and an uncertain result is never converted into a rejection.

## TUI interaction and cursor layout

In Sessions navigation, `n` opens the rename form for the selected session. The binding is inactive while child input owns keys. The form shows the current effective label, stable abbreviated identity, and that Ctrl-U followed by save clears a custom name. Ctrl-S dispatches once. Esc follows the existing discard flow. Validation and stale-revision rejection preserve the draft and byte cursor. An uncertain result preserves unresolved state and requires authoritative readback before another explicit mutation.

All form families use one editor layout helper for both text rendering and physical cursor placement. `FormField.cursor` remains a UTF-8 byte offset. The helper transforms source text through the same safe-text function used for display, maps the source boundary to a rendered grapheme boundary, computes terminal cell width, wraps inside the field viewport, and applies vertical or horizontal clipping. A combining mark advances zero cells. A wide character advances its complete width, and the cursor never targets its continuation cell.

Empty text uses column zero. End-of-text follows the final grapheme. A trailing newline places the cursor at column zero of the following visual line. An insertion point at an exact wrap boundary appears at column zero of the next visual line. The active insertion point is scrolled into view. Resize clamps view offsets without changing text. At dimensions below the safe form layout, the existing minimum-size view owns the screen and no form cursor is emitted. A discard dialog hides the form cursor. Otherwise an active form cursor takes precedence over the child terminal cursor.

A typed stale revision displays: `The workspace changed while you were editing. Your draft is kept. Review the latest state before submitting again, or press Esc to discard.` Review refreshes the authoritative view without overwriting the draft or changing its submission revision. A separate explicit reconcile action adopts the latest revision after inspection.

A typed competing input owner displays inline: `Another client controls input. This view remains read-only. Try i after that client releases input.` The view remains READ ONLY, output continues, and the rejected client sends no input and receives no lease. The message clears after successful acquisition, detach, target change, or successful reconciliation. Other conflicts and uncertain outcomes retain their distinct existing guidance.

## Verification design

The implementation must include these focused checks before M12.00 is complete:

- Domain: cleared, duplicate, Unicode, combining, 128-byte, oversized, control and bidi names; reconstruction; actor authorization; no-op rename; private event payload; ordinal uniqueness and overflow rollback.
- Persistence: fresh schema 3; populated schema 1 and 2 upgrades; equal timestamps; deterministic and idempotent backfill; interrupted migration rollback; more than 200 rows; keyset boundaries; concurrent revision rejection; restart; backup and restore; newer-schema refusal.
- Application: normalized input; stable IDs; rename of every lifecycle state; two-client conflict; unknown delivery with no replay; page limits 0, 50, 200 and 201; revision change between pages; selection inputs independent from labels.
- Protocol and client: strict DTO round trips; unknown operation; advertised capability; old-client/new-server and new-client/old-server paths; decimal bounds; typed and unknown conflict reasons; response-size limits; no name in events or diagnostics.
- TUI: duplicate and cleared labels; abbreviation collision; background creation; next and previous pages; selection preservation; rename success, validation, discard, conflict and uncertainty; no `n` interception in INPUT mode.
- Cursor: every form family at normal and minimum dimensions; empty and end positions; trailing newline; exact wrap; horizontal and vertical clipping; wide and combining text; arrows, Home, End, Backspace, Delete, Enter, Tab, Shift-Tab, paste, Ctrl-U, rejection, resize and cancellation; physical cursor visibility on native terminals.
- Integration: one synthetic two-client journey covers rename, ordering, more than 200 sessions, stale draft, competing input ownership, release, acquisition, reconnect, restart and backup preservation.

The final usability candidate needs one consolidated affected observation on Linux, macOS, and Windows. It covers names and order, a task-form cursor, a stale form, and competing input with two clients. Because acquisition and rendering paths change, perform one focused SSH observation on an available native platform. Reuse unaffected M11 rows only after a source-impact assessment identifies why the changed files cannot affect them.
