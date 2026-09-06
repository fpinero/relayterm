CREATE TABLE workspace_meta (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    workspace_id BLOB NOT NULL UNIQUE CHECK (length(workspace_id) = 16),
    revision BLOB NOT NULL CHECK (length(revision) = 8),
    last_event_sequence BLOB NOT NULL CHECK (length(last_event_sequence) = 8),
    retained_from_sequence BLOB NOT NULL CHECK (length(retained_from_sequence) = 8),
    model_schema_version INTEGER NOT NULL CHECK (model_schema_version = 1)
) STRICT;

CREATE TABLE workspaces (
    workspace_id BLOB PRIMARY KEY CHECK (length(workspace_id) = 16),
    display_name TEXT NOT NULL CHECK (length(CAST(display_name AS BLOB)) BETWEEN 1 AND 256),
    root_codec TEXT NOT NULL,
    project_root BLOB NOT NULL CHECK (length(project_root) <= 8192),
    created_seconds INTEGER NOT NULL,
    created_nanoseconds INTEGER NOT NULL CHECK (created_nanoseconds BETWEEN 0 AND 999999999),
    updated_seconds INTEGER NOT NULL,
    updated_nanoseconds INTEGER NOT NULL CHECK (updated_nanoseconds BETWEEN 0 AND 999999999),
    schema_version INTEGER NOT NULL CHECK (schema_version = 1)
) STRICT;

CREATE TABLE agent_definitions (
    workspace_id BLOB NOT NULL CHECK (length(workspace_id) = 16),
    definition_id BLOB NOT NULL CHECK (length(definition_id) = 16),
    display_name TEXT NOT NULL,
    command TEXT NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    PRIMARY KEY (workspace_id, definition_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
) STRICT;
CREATE TABLE definition_arguments (workspace_id BLOB NOT NULL, definition_id BLOB NOT NULL, ordinal INTEGER NOT NULL CHECK (ordinal >= 0), value TEXT NOT NULL, PRIMARY KEY(workspace_id, definition_id, ordinal), FOREIGN KEY(workspace_id, definition_id) REFERENCES agent_definitions(workspace_id, definition_id)) STRICT;
CREATE TABLE definition_environment (workspace_id BLOB NOT NULL, definition_id BLOB NOT NULL, ordinal INTEGER NOT NULL CHECK (ordinal >= 0), value TEXT NOT NULL, PRIMARY KEY(workspace_id, definition_id, ordinal), UNIQUE(workspace_id, definition_id, value), FOREIGN KEY(workspace_id, definition_id) REFERENCES agent_definitions(workspace_id, definition_id)) STRICT;
CREATE TABLE definition_capabilities (workspace_id BLOB NOT NULL, definition_id BLOB NOT NULL, ordinal INTEGER NOT NULL CHECK (ordinal >= 0), value TEXT NOT NULL, PRIMARY KEY(workspace_id, definition_id, ordinal), UNIQUE(workspace_id, definition_id, value), FOREIGN KEY(workspace_id, definition_id) REFERENCES agent_definitions(workspace_id, definition_id)) STRICT;

CREATE TABLE tasks (
    workspace_id BLOB NOT NULL, task_id BLOB NOT NULL, title TEXT NOT NULL, description TEXT NOT NULL,
    priority TEXT NOT NULL CHECK(priority IN ('low','normal','high','urgent')),
    status TEXT NOT NULL CHECK(status IN ('backlog','ready','active','blocked','handover_ready','done','cancelled')),
    acceptance_notes TEXT NOT NULL, worktree_id BLOB CHECK(worktree_id IS NULL OR length(worktree_id) = 16),
    created_seconds INTEGER NOT NULL, created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999),
    updated_seconds INTEGER NOT NULL, updated_nanoseconds INTEGER NOT NULL CHECK(updated_nanoseconds BETWEEN 0 AND 999999999),
    PRIMARY KEY(workspace_id, task_id), FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
) STRICT;
CREATE TABLE task_scope_paths (workspace_id BLOB NOT NULL, task_id BLOB NOT NULL, ordinal INTEGER NOT NULL CHECK(ordinal >= 0), value TEXT NOT NULL, PRIMARY KEY(workspace_id, task_id, ordinal), UNIQUE(workspace_id, task_id, value), FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id)) STRICT;
CREATE TABLE task_dependencies (workspace_id BLOB NOT NULL, task_id BLOB NOT NULL, ordinal INTEGER NOT NULL CHECK(ordinal >= 0), dependency_id BLOB NOT NULL, CHECK(task_id <> dependency_id), PRIMARY KEY(workspace_id, task_id, ordinal), UNIQUE(workspace_id, task_id, dependency_id), FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id), FOREIGN KEY(workspace_id, dependency_id) REFERENCES tasks(workspace_id, task_id)) STRICT;

CREATE TABLE agent_instances (
    workspace_id BLOB NOT NULL, instance_id BLOB NOT NULL, session_id BLOB NOT NULL, definition_id BLOB, task_id BLOB,
    working_directory_codec TEXT NOT NULL, working_directory BLOB NOT NULL CHECK(length(working_directory) <= 8192),
    status TEXT NOT NULL CHECK(status IN ('starting','running','exited','failed','terminated','lost')),
    started_seconds INTEGER NOT NULL, started_nanoseconds INTEGER NOT NULL CHECK(started_nanoseconds BETWEEN 0 AND 999999999),
    observed_seconds INTEGER NOT NULL, observed_nanoseconds INTEGER NOT NULL CHECK(observed_nanoseconds BETWEEN 0 AND 999999999),
    ended_seconds INTEGER, ended_nanoseconds INTEGER CHECK(ended_nanoseconds BETWEEN 0 AND 999999999), exit_code INTEGER,
    terminal_rows INTEGER NOT NULL CHECK(terminal_rows BETWEEN 1 AND 1000), terminal_columns INTEGER NOT NULL CHECK(terminal_columns BETWEEN 1 AND 1000),
    PRIMARY KEY(workspace_id, instance_id), UNIQUE(workspace_id, session_id),
    CHECK((ended_seconds IS NULL) = (ended_nanoseconds IS NULL)),
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id),
    FOREIGN KEY(workspace_id, definition_id) REFERENCES agent_definitions(workspace_id, definition_id),
    FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id)
) STRICT;
CREATE TABLE instance_launch_definitions (workspace_id BLOB NOT NULL, instance_id BLOB NOT NULL, definition_id BLOB NOT NULL, display_name TEXT NOT NULL, command TEXT NOT NULL, enabled INTEGER NOT NULL CHECK(enabled IN(0,1)), PRIMARY KEY(workspace_id, instance_id), FOREIGN KEY(workspace_id, instance_id) REFERENCES agent_instances(workspace_id, instance_id)) STRICT;
CREATE TABLE instance_launch_arguments (workspace_id BLOB NOT NULL, instance_id BLOB NOT NULL, ordinal INTEGER NOT NULL, value TEXT NOT NULL, PRIMARY KEY(workspace_id, instance_id, ordinal), FOREIGN KEY(workspace_id, instance_id) REFERENCES instance_launch_definitions(workspace_id, instance_id)) STRICT;
CREATE TABLE instance_launch_environment (workspace_id BLOB NOT NULL, instance_id BLOB NOT NULL, ordinal INTEGER NOT NULL, value TEXT NOT NULL, PRIMARY KEY(workspace_id, instance_id, ordinal), UNIQUE(workspace_id, instance_id, value), FOREIGN KEY(workspace_id, instance_id) REFERENCES instance_launch_definitions(workspace_id, instance_id)) STRICT;
CREATE TABLE instance_launch_capabilities (workspace_id BLOB NOT NULL, instance_id BLOB NOT NULL, ordinal INTEGER NOT NULL, value TEXT NOT NULL, PRIMARY KEY(workspace_id, instance_id, ordinal), UNIQUE(workspace_id, instance_id, value), FOREIGN KEY(workspace_id, instance_id) REFERENCES instance_launch_definitions(workspace_id, instance_id)) STRICT;

CREATE TABLE claims (
    workspace_id BLOB NOT NULL, claim_id BLOB NOT NULL, task_id BLOB NOT NULL, instance_id BLOB NOT NULL,
    requested_by_kind TEXT NOT NULL CHECK(requested_by_kind IN('local_user','instance')), requested_by_instance_id BLOB,
    opened_seconds INTEGER NOT NULL, opened_nanoseconds INTEGER NOT NULL CHECK(opened_nanoseconds BETWEEN 0 AND 999999999),
    closed_seconds INTEGER, closed_nanoseconds INTEGER CHECK(closed_nanoseconds BETWEEN 0 AND 999999999),
    close_reason TEXT CHECK(close_reason IN('explicit_release','blocking','handover','completion','cancellation','instance_end')),
    closed_by_kind TEXT CHECK(closed_by_kind IN('local_user','instance','system')), closed_by_instance_id BLOB,
    opening_event_sequence BLOB NOT NULL CHECK(length(opening_event_sequence)=8),
    PRIMARY KEY(workspace_id, claim_id), FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id), FOREIGN KEY(workspace_id, instance_id) REFERENCES agent_instances(workspace_id, instance_id),
    CHECK((closed_seconds IS NULL)=(closed_nanoseconds IS NULL) AND (closed_seconds IS NULL)=(close_reason IS NULL) AND (closed_seconds IS NULL)=(closed_by_kind IS NULL)),
    CHECK((requested_by_kind='instance')=(requested_by_instance_id IS NOT NULL)),
    CHECK(closed_by_kind IS NULL OR (closed_by_kind='instance')=(closed_by_instance_id IS NOT NULL))
) STRICT;
CREATE UNIQUE INDEX one_open_claim_per_task ON claims(workspace_id, task_id) WHERE closed_seconds IS NULL;
CREATE UNIQUE INDEX one_open_claim_per_instance ON claims(workspace_id, instance_id) WHERE closed_seconds IS NULL;
CREATE INDEX claims_by_task_order ON claims(workspace_id, task_id, opening_event_sequence);

CREATE TABLE progress_entries (workspace_id BLOB NOT NULL, progress_id BLOB NOT NULL, task_id BLOB NOT NULL, instance_id BLOB, summary TEXT NOT NULL, verification TEXT NOT NULL, created_seconds INTEGER NOT NULL, created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999), creation_event_sequence BLOB NOT NULL CHECK(length(creation_event_sequence)=8), PRIMARY KEY(workspace_id, progress_id), FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id), FOREIGN KEY(workspace_id, instance_id) REFERENCES agent_instances(workspace_id, instance_id)) STRICT;
CREATE INDEX progress_by_task_order ON progress_entries(workspace_id, task_id, creation_event_sequence);
CREATE TABLE handovers (workspace_id BLOB NOT NULL, handover_id BLOB NOT NULL, task_id BLOB NOT NULL, from_instance_id BLOB, summary TEXT NOT NULL, decisions TEXT NOT NULL, verification_performed TEXT NOT NULL, open_questions TEXT NOT NULL, recommended_next_action TEXT NOT NULL, created_seconds INTEGER NOT NULL, created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999), creation_event_sequence BLOB NOT NULL CHECK(length(creation_event_sequence)=8), PRIMARY KEY(workspace_id, handover_id), FOREIGN KEY(workspace_id, task_id) REFERENCES tasks(workspace_id, task_id), FOREIGN KEY(workspace_id, from_instance_id) REFERENCES agent_instances(workspace_id, instance_id)) STRICT;
CREATE TABLE handover_changed_paths (workspace_id BLOB NOT NULL, handover_id BLOB NOT NULL, ordinal INTEGER NOT NULL, value TEXT NOT NULL, PRIMARY KEY(workspace_id, handover_id, ordinal), UNIQUE(workspace_id, handover_id, value), FOREIGN KEY(workspace_id, handover_id) REFERENCES handovers(workspace_id, handover_id)) STRICT;
CREATE INDEX handovers_by_task_order ON handovers(workspace_id, task_id, creation_event_sequence);

CREATE TABLE workspace_events (
    workspace_id BLOB NOT NULL, sequence BLOB NOT NULL CHECK(length(sequence)=8), event_id BLOB NOT NULL CHECK(length(event_id)=16),
    event_type TEXT NOT NULL, entity_kind TEXT NOT NULL, entity_id BLOB NOT NULL CHECK(length(entity_id)=16),
    timestamp_seconds INTEGER NOT NULL, timestamp_nanoseconds INTEGER NOT NULL CHECK(timestamp_nanoseconds BETWEEN 0 AND 999999999),
    actor_kind TEXT NOT NULL CHECK(actor_kind IN('local_user','instance','system')), actor_instance_id BLOB,
    payload_version INTEGER NOT NULL, payload_json BLOB NOT NULL CHECK(length(payload_json) <= 16384),
    PRIMARY KEY(workspace_id, sequence), UNIQUE(workspace_id, event_id), FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id),
    CHECK((actor_kind='instance')=(actor_instance_id IS NOT NULL))
) STRICT;
CREATE INDEX events_after_sequence ON workspace_events(workspace_id, sequence);

CREATE TRIGGER progress_no_update BEFORE UPDATE ON progress_entries BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER progress_no_delete BEFORE DELETE ON progress_entries BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER handover_no_update BEFORE UPDATE ON handovers BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER handover_no_delete BEFORE DELETE ON handovers BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER event_no_update BEFORE UPDATE ON workspace_events BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER event_no_delete BEFORE DELETE ON workspace_events BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_no_update BEFORE UPDATE ON instance_launch_definitions BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_no_delete BEFORE DELETE ON instance_launch_definitions BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_arguments_no_update BEFORE UPDATE ON instance_launch_arguments BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_arguments_no_delete BEFORE DELETE ON instance_launch_arguments BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_environment_no_update BEFORE UPDATE ON instance_launch_environment BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_environment_no_delete BEFORE DELETE ON instance_launch_environment BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_capabilities_no_update BEFORE UPDATE ON instance_launch_capabilities BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER launch_capabilities_no_delete BEFORE DELETE ON instance_launch_capabilities BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER handover_paths_no_update BEFORE UPDATE ON handover_changed_paths BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER handover_paths_no_delete BEFORE DELETE ON handover_changed_paths BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER claim_no_delete BEFORE DELETE ON claims BEGIN SELECT RAISE(ABORT, 'append_only'); END;
CREATE TRIGGER claim_close_once BEFORE UPDATE ON claims WHEN OLD.closed_seconds IS NOT NULL OR NEW.workspace_id <> OLD.workspace_id OR NEW.claim_id <> OLD.claim_id OR NEW.task_id <> OLD.task_id OR NEW.instance_id <> OLD.instance_id OR NEW.requested_by_kind <> OLD.requested_by_kind OR NEW.requested_by_instance_id IS NOT OLD.requested_by_instance_id OR NEW.opened_seconds <> OLD.opened_seconds OR NEW.opened_nanoseconds <> OLD.opened_nanoseconds OR NEW.opening_event_sequence <> OLD.opening_event_sequence BEGIN SELECT RAISE(ABORT, 'immutable_claim'); END;
CREATE TRIGGER claim_update_must_close BEFORE UPDATE ON claims WHEN NEW.closed_seconds IS NULL BEGIN SELECT RAISE(ABORT, 'claim_update_must_close'); END;
