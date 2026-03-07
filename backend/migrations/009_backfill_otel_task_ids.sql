-- Backfill task_id and session_id for otel_metrics that have a known claude_session_id
-- but were stored before the correlate() call was wired up (historical race condition).
UPDATE otel_metrics
SET
    session_id = (
        SELECT id FROM sessions
        WHERE sessions.claude_session_id = otel_metrics.claude_session_id
        LIMIT 1
    ),
    task_id = (
        SELECT task_id FROM sessions
        WHERE sessions.claude_session_id = otel_metrics.claude_session_id
        LIMIT 1
    )
WHERE otel_metrics.task_id IS NULL
  AND otel_metrics.claude_session_id != '';
