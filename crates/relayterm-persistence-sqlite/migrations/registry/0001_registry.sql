CREATE TABLE registry_meta (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    schema_version INTEGER NOT NULL CHECK (schema_version = 1)
) STRICT;

INSERT INTO registry_meta(singleton, schema_version) VALUES (1, 1);

CREATE TABLE workspace_registrations (
    workspace_id BLOB PRIMARY KEY CHECK (length(workspace_id) = 16),
    root_codec TEXT NOT NULL,
    canonical_root BLOB NOT NULL CHECK (length(canonical_root) <= 8192),
    root_lookup_key BLOB NOT NULL UNIQUE CHECK (length(root_lookup_key) = 32),
    filesystem_guard BLOB,
    initialization_state TEXT NOT NULL CHECK (initialization_state IN ('initializing', 'ready')),
    created_seconds INTEGER NOT NULL,
    created_nanoseconds INTEGER NOT NULL CHECK (created_nanoseconds BETWEEN 0 AND 999999999),
    updated_seconds INTEGER NOT NULL,
    updated_nanoseconds INTEGER NOT NULL CHECK (updated_nanoseconds BETWEEN 0 AND 999999999)
) STRICT;
