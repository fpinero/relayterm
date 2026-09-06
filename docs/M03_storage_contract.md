# M03 private storage contract

## Location policy

Relayterm resolves private locations once and passes native paths to adapters. Resolution is side-effect free. Initialization creates only the required managed directories and rejects unsafe existing file types, links, ownership, permissions, or ACLs.

| Platform | Configuration | Durable data | Runtime | Cache |
| --- | --- | --- | --- | --- |
| Linux | XDG configuration home plus `relayterm` | XDG local data home plus `relayterm` | XDG runtime directory plus `relayterm`, or a private data child | XDG cache home plus `relayterm` |
| macOS | Application Support bundle directory plus `config` | Application Support bundle directory plus `data` | Private durable-data child | Library Caches bundle directory |
| Windows | Roaming application configuration directory | Local application data directory | Private local-data child | Local cache directory |

`RELAYTERM_HOME` is the documented test and portable override. It must be absolute and has `config`, `data`, `runtime`, and `cache` children. A supplied `LocationOptions` value wins over the environment variable. An override may deliberately point inside a project, but default resolution never does and never falls back to the current directory.

Unix managed directories use mode `0700` and files use `0600`. Windows applies a protected DACL for the current owner, Local System, and administrators, with inheritable directory entries for SQLite sidecars. Broad Everyone or built-in Users access is rejected. Registry, workspace, WAL, SHM, lock, migration, and snapshot files are private.

Canonical workspace roots use a hash of the reversible native path encoding. Unix also persists device and inode as a physical-directory guard. Rust 1.98.1 does not expose stable Windows volume and file IDs through safe standard APIs, so that optional guard is absent on Windows; canonical path lookup, registry uniqueness, and native alias tests remain mandatory there.

## Configuration contract

TOML candidates use `format_version = 1` and are limited to 1 MiB and 128 definitions. Definition IDs are mandatory and stable. Storage settings allow a busy timeout from 1 through 30,000 milliseconds and 1 through 16 connections. Color mode is `auto`, `always`, or `never`.

Parsing produces validated candidates without changing active state. Unknown non-security fields produce at most 32 structural warnings plus a truncation warning. Security-shaped unknown fields and recognized credential patterns fail closed. Diagnostics contain stable codes and field names, never source excerpts, raw parser errors, configuration values, or private paths. Pattern recognition cannot detect every arbitrary secret.

Applying a candidate compares its captured workspace revision inside the transaction. A stale revision conflicts. A current identical import changes no rows, events, revision, or notification. Supplied definitions are created or updated atomically, omitted definitions remain unchanged, and no import rewrites its source.

## Database layout and versions

The private data root contains `registry.sqlite3` and one `workspaces/<workspace-id>/workspace.sqlite3` database per workspace. Registry schema, SQL migration ledger, domain model schema, configuration format, event payload, and IPC protocol versions are independent. Their initial M03 values are all 1 by coincidence, except that SQL migration versions are maintained by their own ordered checksummed ledger.

SQLite uses a bundled engine at least 3.51.3, WAL, `synchronous = FULL`, foreign keys on every pooled connection, a bounded busy timeout, and a pool of at most 16 connections. M03 pins SQLx without PostgreSQL, MySQL, TLS, or runtime extension loading. SQL statements are not logged.

The directory resolver introduces `option-ext` under MPL-2.0. The project license policy explicitly accepts that reviewed file-level copyleft license; Relayterm source remains Apache-2.0.

Typed UUIDs use 16-byte blobs. Unsigned counters use eight-byte big-endian blobs and preserve all `u64` values. Timestamps use signed Unix seconds plus nanoseconds from 0 through 999,999,999. Native paths use tagged Unix bytes or Windows UTF-16LE with an 8 KiB encoded bound. Narrative content remains relational text and passes domain reconstruction.

The schema uses composite workspace references, unique session IDs, ordered child rows, and partial unique indexes for one open claim per task and per instance. Progress, handovers, events, launch snapshots, and their immutable children have defensive update and delete triggers. Claims permit one closure update and no deletion.

## Transactions, queries, and recovery

`Store::begin` loads a validated snapshot and releases the read transaction. Commit acquires `BEGIN IMMEDIATE`, reloads the current revision, validates the opaque application batch, computes relational deltas, assigns consecutive event sequences, and commits entity rows, history, revision, and events together. Stale revisions conflict. Rolled-back transactions leave no partial rows or events. Notification occurs after commit and cannot reverse durable success.

Snapshot queries return revision, last event sequence, and retention floor from one read transaction. Event pages read their watermark and rows in one transaction and use a keyset cursor, default application limit 50, and maximum 200. A cursor ahead of the watermark is invalid. A cursor older than retained coverage requires resnapshot. M03 retains every event and keeps the normal floor at 1.

Durable task history merges claims, progress, and handovers by their append event sequence. Requests carry an exclusive `after_sequence` cursor, a limit from 1 through 200, and the snapshot revision against which the caller is paging. A revision change returns a conflict so clients do not combine pages from different states. Supporting indexes bound every source query, and returned records still pass domain reconstruction.

Opening an existing database first validates its file type, private access, migration ledger, checksums, and expected registry or workspace identity through a read-only connection. Unknown, newer, altered, corrupt, missing, or identity-mismatched files are preserved and rejected. Only an initializing registry reservation may resume a recognizable empty database. A ready registration with missing storage requires recovery and never creates a replacement.

`VACUUM INTO` creates a consistent snapshot at a fresh private destination. Relayterm validates the copied schema and permissions before accepting it. Restore commands and user-facing backup workflow remain assigned to M11.

## Downstream synchronization handoff

M04 must fetch a consistent snapshot and watermark, then subscribe after that watermark. Gaps, expired cursors, unknown payloads, and uncertain mutation results require explicit recovery. M03 does not prune events, retry ambiguous commits, or claim that timestamp order replaces workspace sequence order. M05 obtains exclusive daemon ownership before reconciling persisted live instances. M07 resolves executables and supervises real processes from immutable launch snapshots.
