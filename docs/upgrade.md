# Upgrade, rollback, and recovery

## Compatibility boundary

Relayterm 0.1.0 uses protocol major version 1 and workspace schema version 3. Additive protocol capabilities are negotiated. A client disables an unavailable operation rather than asking an older daemon to ignore it. A major-version mismatch fails before mutation. Workspace migrations run transactionally in the daemon or offline restore path. An older binary refuses a newer schema and must leave it unchanged.

The binary, daemon process, coordination database, live PTY state, project source, and Git worktrees are separate assets. Replacing a binary does not migrate a running daemon. A database backup does not preserve live processes, terminal cells, project files, Git objects, provider credentials, or environment values.

## Safe upgrade sequence

1. Resolve the selected `rt` by absolute path and record `rt --version`.
2. Run `workspace status` for every workspace being upgraded. Do not connect an incompatible client to an unknown daemon.
3. Create a private backup with the current compatible binary. Keep the original home stopped during a later restore.
4. Stop the exact workspace daemon with `daemon stop`. If live sessions exist, review them and use `--terminate-sessions` only when their termination is intended.
5. Verify the new archive, target, manifest, checksum, and exact inventory.
6. Install it into a new versioned directory. Do not overwrite the old binary.
7. Run the new binary by absolute path with `--help` and `--version` before changing `PATH` precedence.
8. Open a copied or backed-up test workspace first. Check schema migration, session names and order, tasks, claims, progress, handovers, events, and worktree records.
9. Select the new directory deliberately. Keep the old binary and backup until the installed quick start and real workspaces have been verified.

Windows can lock a running executable. Stop its selected daemon and clients, then retry the versioned installation. Do not force replacement or kill arbitrary processes by name.

## Rollback

Do not point an older binary at a home already migrated by a newer version. Stop the newer daemon, choose a fresh private home, restore a backup whose schema is compatible with the older binary, and invoke that binary by absolute path. Never run original and restored homes for the same workspace identity concurrently. Never copy a live SQLite file without its WAL.

Restore validates the exact backup inventory, BLAKE3 checksum, schema, workspace identity, event watermark, foreign keys, and domain invariants before publishing a fresh home. A corrupt or incompatible backup preserves the source, original home, and requested destination. See [private backup and restore](backup-restore.md).

## Failure guidance

| Failure | Safe action | Recovery boundary |
| --- | --- | --- |
| Protocol or capability mismatch | Stop, identify both absolute binaries and the live daemon generation, then use a compatible pair. | No mutation should be inferred from a rejected negotiation. |
| Newer workspace schema | Preserve the home and use the matching newer binary. Roll back through a compatible backup in a fresh home. | An older binary cannot downgrade the database. |
| Migration or storage failure | Preserve the original home and backup. Check private permissions and free space before inspecting diagnostics. | Transactional migration must not be treated as successful. |
| Corrupt or mismatched backup | Keep source and original home. Create or obtain another validated backup. | Restore never repairs or overwrites the source. |
| Stale endpoint or ownership conflict | Use `workspace status`, wait for the selected generation, and inspect bounded diagnostics. | Do not delete sockets, locks, or databases as a first action. |
| `daemon_unavailable` with an explicit long home on Unix | Retry only after selecting a shorter private home whose socket path fits the platform limit. Preserve the original state. | Default private locations are compact; shortening a path does not migrate an existing home. |
| Unknown mutation result | Read authoritative state, event history, or the durable receipt before deciding on a retry. | Never replay a possibly accepted mutation automatically. |
| Detached or lost session | Reconnect while the same daemon lives. After daemon or host loss, accept the honest `lost` state and restart work explicitly. | PTY contents and processes cannot be recovered after daemon loss. |
| Missing shell, agent, or Git | Correct the neutral definition or install the prerequisite deliberately, then run availability checks again. | Relayterm does not install or authenticate providers. Git is optional outside worktree commands. |
| Wrong permissions | Restore current-user private ownership and restrictive access without making state public. | Do not bypass private-state checks. |
| Partial installation or wrong architecture | Preserve existing installation, remove only the failed staging file, and install a verified matching archive into a new directory. | Never report success from another `rt` found on `PATH`. |

Diagnostics are private, bounded, and intentionally omit command arguments, terminal output, environment values, task prose, names, and raw paths. Preserve relevant data before investigation. Relayterm cannot recover lost terminal history or serve as a security sandbox.
