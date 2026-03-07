-- Backfill claude_session_id from the stored attributes JSON for rows that were saved
-- with an empty claude_session_id due to the extract_string_attr dot-key bug.
-- The quoted path syntax '$."session.id"' handles the dot in the key name.
UPDATE otel_metrics
SET
    claude_session_id = json_extract(attributes, '$."session.id"'),
    session_id = (
        SELECT id FROM sessions
        WHERE claude_session_id = json_extract(otel_metrics.attributes, '$."session.id"')
        LIMIT 1
    ),
    task_id = (
        SELECT task_id FROM sessions
        WHERE claude_session_id = json_extract(otel_metrics.attributes, '$."session.id"')
        LIMIT 1
    )
WHERE otel_metrics.claude_session_id = ''
  AND json_extract(attributes, '$."session.id"') IS NOT NULL;
