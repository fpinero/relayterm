# Private backup and restore

## Scope

Relayterm can create a consistent private backup of one workspace database and restore it into a new private Relayterm home. This is disaster-recovery tooling, not a project export, archive format or multi-machine migration promise.

The backup contains tasks, definitions, claims, progress, handovers, events, session metadata, launch snapshots and worktree metadata. It does not contain project files, Git objects or checkouts, provider credentials, environment values, live processes, terminal state or scrollback. Back up the project and Git repository separately.

Backup directories and databases contain sensitive coordination data. Store them in a current-user private directory. Relayterm does not encrypt backups.

## Create a backup

When the selected workspace daemon is active, the command requests an authenticated live snapshot. Existing sessions and unrelated requests remain available while SQLite creates one committed point-in-time copy. Relayterm admits only one backup at a time for a workspace. When the daemon is stopped, the same command acquires exclusive workspace ownership for the duration of the snapshot and does not leave a daemon running:

```text
rt --workspace /path/to/project --home /private/relayterm-home backup create --destination /private/backups/workspace-001
```

The destination must not exist and its parent must already satisfy Relayterm's private access policy. The native path is carried as a typed IPC value and never interpreted as SQL. Relayterm builds the SQLite output and manifest in a uniquely owned private sibling staging directory. It reserves the SQLite file with private permissions before `VACUUM INTO`, validates the completed snapshot, computes a BLAKE3 integrity value and writes `manifest.json` last. The manifest records its own format, the producing application version, the workspace schema version, identity, revision and event watermark from the captured database. Relayterm publishes the validated directory with a native no-replace rename and never overwrites an existing path, dangling symbolic link or Windows reparse point.

A failed operation removes only its unpublished private staging directory, so the requested final path never names a partial backup. A process or machine failure can still leave a hidden private staging directory. Treat such staging as unusable and choose a new destination. Do not copy only a live `workspace.sqlite3` file because WAL content may be omitted.

## Restore a backup

Keep the original workspace daemon stopped. Choose a new Relayterm home that does not exist:

```text
rt --workspace /path/to/project backup restore \
  --source /private/backups/workspace-001 \
  --destination /private/restored-relayterm-home
```

Relayterm validates the private directory without following symbolic links or Windows reparse points, then validates the exact member inventory, bounded versioned manifest, checksum, SQLite schema, workspace identity, revision and event watermark before publishing the destination. It builds and validates the database and minimal registry in a uniquely owned private staging directory beside the requested home, closes their SQLite pools, then publishes the directory with a native no-replace rename. A failed validation removes only that unpublished staging directory. The source backup, original home and any path that appears at the requested destination are never moved or overwritten.

Supported older workspace schemas are migrated only in the private restored copy. The source backup remains unchanged. Native acceptance constructs a populated version 1 workspace backup, restores it with a copied `rt` executable, reopens it at the current schema, and verifies that its retained task is still queryable.

Start the restored workspace explicitly:

```text
rt --workspace /path/to/project --home /private/restored-relayterm-home workspace open
```

Only one home for the restored workspace identity may be active. Relayterm cannot discover arbitrary copies stored in unknown homes. Starting original and restored homes concurrently against the same project or Git common directory is unsupported. The operator must keep the original stopped.

On first start, ordinary recovery marks former nonfinal sessions lost, closes active claims and blocks affected tasks once. It does not adopt old processes or recreate terminal history. Missing or changed project/worktree paths remain unavailable; restore never runs `git worktree add`, approves a new root, launches a provider or repairs Git.

## Failure guidance

| Result | Meaning and action |
| --- | --- |
| `operation_unavailable` or `storage_busy` | Another backup is active or storage cannot admit the operation. Let that operation finish, inspect its destination, then choose a new destination deliberately. |
| `invalid_location` | A destination exists, is relative, or violates the private-location contract. Preserve it and choose a new private path. |
| `access_denied` | Source, destination parent or a member fails ownership/mode/ACL policy. Do not relax public access. |
| `recovery_required` | Manifest, checksum, member inventory, schema, identity or database integrity is invalid. Preserve original and backup for private inspection. |
| `storage_error` | Storage is locked, unavailable or could not be synchronized. Preserve all originals and check free space and permissions. |

Newer backup formats or database schemas fail closed. A corrupt backup never triggers automatic recreation. Backup success does not claim protection against failing hardware, storage that ignores synchronization, or a power loss before the command reports completion.
