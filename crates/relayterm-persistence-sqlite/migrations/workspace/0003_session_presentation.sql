ALTER TABLE workspace_meta ADD COLUMN next_session_ordinal INTEGER NOT NULL DEFAULT 1
    CHECK(next_session_ordinal BETWEEN 1 AND 9223372036854775807);

CREATE TABLE session_presentations (
    workspace_id BLOB NOT NULL CHECK(length(workspace_id)=16),
    session_id BLOB NOT NULL CHECK(length(session_id)=16),
    creation_ordinal INTEGER NOT NULL CHECK(creation_ordinal BETWEEN 1 AND 9223372036854775807),
    display_name TEXT CHECK(display_name IS NULL OR length(CAST(display_name AS BLOB)) BETWEEN 1 AND 128),
    PRIMARY KEY(workspace_id, session_id),
    UNIQUE(workspace_id, creation_ordinal),
    FOREIGN KEY(workspace_id, session_id) REFERENCES agent_instances(workspace_id, session_id)
) STRICT;

INSERT INTO session_presentations(workspace_id, session_id, creation_ordinal, display_name)
SELECT workspace_id, session_id,
       ROW_NUMBER() OVER (
           PARTITION BY workspace_id
           ORDER BY started_seconds, started_nanoseconds, instance_id
       ),
       NULL
FROM agent_instances;

UPDATE workspace_meta
SET next_session_ordinal = COALESCE(
    (SELECT MAX(creation_ordinal) + 1
     FROM session_presentations
     WHERE session_presentations.workspace_id = workspace_meta.workspace_id),
    1
);

CREATE INDEX session_presentations_by_order
    ON session_presentations(workspace_id, creation_ordinal, session_id);

CREATE TRIGGER session_presentation_identity_immutable
BEFORE UPDATE ON session_presentations
WHEN NEW.workspace_id <> OLD.workspace_id
  OR NEW.session_id <> OLD.session_id
  OR NEW.creation_ordinal <> OLD.creation_ordinal
BEGIN
    SELECT RAISE(ABORT, 'immutable_session_presentation_identity');
END;
