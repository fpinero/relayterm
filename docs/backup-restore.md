# Private backup and restore

## Scope

Relayterm can create a consistent private backup of one workspace database and restore it into a new private Relayterm home. This is disaster-recovery tooling, not a project export, archive format or multi-machine migration promise.

The backup contains tasks, definitions, claims, progress, handovers, events, session metadata, launch snapshots and worktree metadata. It does not contain project files, Git objects or checkouts, provider credentials, environment values, live processes, terminal state or scrollback. Back up the project and Git repository separately.

Backup directories and databases contain sensitive coordination data. Store them in a current-user private directory. Relayterm does not encrypt backups.

## Create a backup

Stop the workspace daemon first. This avoids a cloned live workspace identity and gives the operation exclusive workspace ownership:

```text
rt --workspace /path/to/project --home /private/relayterm-home daemon stop --terminate-sessions
rt --workspace /path/to/project --home /private/relayterm-home backup create --destination /private/backups/workspace-001
```

The destination must not exist and its parent must already satisfy Relayterm's private access policy. Relayterm reserves the SQLite output with private permissions before `VACUUM INTO`, validates the completed snapshot, computes a BLAKE3 integrity value and writes `manifest.json` last. The manifest records its own format, the producing application version, the workspace schema version, identity, revision and event watermark. Relayterm never overwrites an existing destination.

An interrupted operation may leave a private incomplete directory without a manifest. Treat it as unusable evidence and choose a new destination. Relayterm does not automatically delete it. Do not copy only a live `workspace.sqlite3` file because WAL content may be omitted.

## Restore a backup

Keep the original workspace daemon stopped. Choose a new Relayterm home that does not exist:

```text
rt --workspace /path/to/project backup restore \
  --source /private/backups/workspace-001 \
  --destination /private/restored-relayterm-home
```

Relayterm validates the private directory, exact member inventory, bounded versioned manifest, checksum, SQLite schema, workspace identity, revision and event watermark before creating the destination. It copies into a fresh private home, opens the copy using embedded migrations and creates the minimal registry entry. Source backup and original home are never moved or overwritten.

Start the restored workspace explicitly:

```text
rt --workspace /path/to/project --home /private/restored-relayterm-home workspace open
```

Only one home for the restored workspace identity may be active. Relayterm cannot discover arbitrary copies stored in unknown homes. Starting original and restored homes concurrently against the same project or Git common directory is unsupported. The operator must keep the original stopped.

On first start, ordinary recovery marks former nonfinal sessions lost, closes active claims and blocks affected tasks once. It does not adopt old processes or recreate terminal history. Missing or changed project/worktree paths remain unavailable; restore never runs `git worktree add`, approves a new root, launches a provider or repairs Git.

## Failure guidance

| Result | Meaning and action |
| --- | --- |
| `workspace_busy` | The daemon or another owner is active. Stop it and retry with a new destination. |
| `invalid_location` | A destination exists, is relative, or violates the private-location contract. Preserve it and choose a new private path. |
| `access_denied` | Source, destination parent or a member fails ownership/mode/ACL policy. Do not relax public access. |
| `recovery_required` | Manifest, checksum, member inventory, schema, identity or database integrity is invalid. Preserve original and backup for private inspection. |
| `storage_error` | Storage is locked, unavailable or could not be synchronized. Preserve all originals and check free space and permissions. |

Newer backup formats or database schemas fail closed. A corrupt backup never triggers automatic recreation. Backup success does not claim protection against failing hardware, storage that ignores synchronization, or a power loss before the command reports completion.
