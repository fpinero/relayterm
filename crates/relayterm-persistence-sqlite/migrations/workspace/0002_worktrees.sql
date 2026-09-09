CREATE TABLE approved_worktree_roots (
    workspace_id BLOB NOT NULL CHECK(length(workspace_id)=16),
    root_id BLOB NOT NULL CHECK(length(root_id)=16),
    path_codec TEXT NOT NULL,
    canonical_parent BLOB NOT NULL CHECK(length(canonical_parent)<=8192),
    filesystem_identity BLOB CHECK(filesystem_identity IS NULL OR length(filesystem_identity)<=1024),
    private_default INTEGER NOT NULL CHECK(private_default IN(0,1)),
    created_seconds INTEGER NOT NULL,
    created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999),
    PRIMARY KEY(workspace_id, root_id),
    UNIQUE(workspace_id, canonical_parent),
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
) STRICT;

CREATE TABLE worktree_intents (
    workspace_id BLOB NOT NULL CHECK(length(workspace_id)=16),
    operation_id BLOB NOT NULL CHECK(length(operation_id)=16),
    task_id BLOB NOT NULL CHECK(length(task_id)=16),
    worktree_id BLOB NOT NULL CHECK(length(worktree_id)=16),
    root_id BLOB NOT NULL CHECK(length(root_id)=16),
    schema_version INTEGER NOT NULL CHECK(schema_version=1),
    expected_revision BLOB NOT NULL CHECK(length(expected_revision)=8),
    repository_codec TEXT NOT NULL, repository_identity BLOB NOT NULL CHECK(length(repository_identity)<=8192),
    common_codec TEXT NOT NULL, common_directory_identity BLOB NOT NULL CHECK(length(common_directory_identity)<=8192),
    destination_codec TEXT NOT NULL, destination BLOB NOT NULL CHECK(length(destination)<=8192),
    branch TEXT NOT NULL CHECK(length(CAST(branch AS BLOB)) BETWEEN 1 AND 256),
    base_expression TEXT NOT NULL CHECK(length(CAST(base_expression AS BLOB)) BETWEEN 1 AND 256),
    resolved_commit TEXT NOT NULL CHECK(length(resolved_commit) BETWEEN 4 AND 128),
    request_fingerprint BLOB NOT NULL CHECK(length(request_fingerprint)=32),
    phase TEXT NOT NULL CHECK(phase IN('prepared','applying','ready','failed','needs_attention')),
    reason TEXT CHECK(reason IN('git_missing','git_unsupported','not_repository','unsupported_root','invalid_reference','branch_conflict','destination_conflict','path_rejected','busy','unsupported_checkout_filter','storage_unavailable','outcome_uncertain','cancelled_task')),
    created_seconds INTEGER NOT NULL, created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999),
    updated_seconds INTEGER NOT NULL, updated_nanoseconds INTEGER NOT NULL CHECK(updated_nanoseconds BETWEEN 0 AND 999999999),
    PRIMARY KEY(workspace_id, operation_id),
    UNIQUE(workspace_id, worktree_id),
    FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id),
    FOREIGN KEY(workspace_id, root_id) REFERENCES approved_worktree_roots(workspace_id, root_id),
    CHECK((phase IN('prepared','applying','ready') AND reason IS NULL) OR (phase IN('failed','needs_attention') AND reason IS NOT NULL))
) STRICT;
CREATE UNIQUE INDEX reserve_worktree_branch ON worktree_intents(workspace_id, branch) WHERE phase IN('prepared','applying','needs_attention');
CREATE UNIQUE INDEX reserve_worktree_destination ON worktree_intents(workspace_id, destination) WHERE phase IN('prepared','applying','needs_attention');
CREATE UNIQUE INDEX reserve_worktree_task ON worktree_intents(workspace_id, task_id) WHERE phase IN('prepared','applying','needs_attention');
CREATE INDEX unresolved_worktree_intents ON worktree_intents(workspace_id, phase, operation_id);

CREATE TABLE worktrees (
    workspace_id BLOB NOT NULL CHECK(length(workspace_id)=16),
    worktree_id BLOB NOT NULL CHECK(length(worktree_id)=16),
    task_id BLOB NOT NULL CHECK(length(task_id)=16),
    operation_id BLOB NOT NULL CHECK(length(operation_id)=16),
    root_id BLOB NOT NULL CHECK(length(root_id)=16),
    checkout_codec TEXT NOT NULL, checkout_path BLOB NOT NULL CHECK(length(checkout_path)<=8192),
    common_codec TEXT NOT NULL, common_directory_identity BLOB NOT NULL CHECK(length(common_directory_identity)<=8192),
    branch_ref TEXT NOT NULL CHECK(length(CAST(branch_ref AS BLOB)) BETWEEN 1 AND 256),
    initial_base_commit TEXT NOT NULL CHECK(length(initial_base_commit) BETWEEN 4 AND 128),
    health TEXT NOT NULL CHECK(health IN('ready','missing','mismatch','unavailable')),
    created_seconds INTEGER NOT NULL, created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999),
    updated_seconds INTEGER NOT NULL, updated_nanoseconds INTEGER NOT NULL CHECK(updated_nanoseconds BETWEEN 0 AND 999999999),
    PRIMARY KEY(workspace_id, worktree_id),
    UNIQUE(workspace_id, operation_id),
    FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id),
    FOREIGN KEY(workspace_id, operation_id) REFERENCES worktree_intents(workspace_id, operation_id),
    FOREIGN KEY(workspace_id, root_id) REFERENCES approved_worktree_roots(workspace_id, root_id)
) STRICT;
CREATE INDEX worktrees_by_task ON worktrees(workspace_id, task_id, worktree_id);

ALTER TABLE agent_instances ADD COLUMN worktree_id BLOB CHECK(worktree_id IS NULL OR length(worktree_id)=16);
