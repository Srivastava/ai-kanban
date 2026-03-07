-- Fix LiteLLM session summary comments incorrectly stored with author='claude'
UPDATE task_comments
SET author = 'litellm'
WHERE author = 'claude'
  AND content LIKE '**Session Summary%';

-- Feature flags for LiteLLM optimizations (toggleable at runtime)
CREATE TABLE IF NOT EXISTS feature_flags (
    key TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO feature_flags (key, enabled, updated_at) VALUES
    ('litellm_session_summary', 1, datetime('now')),
    ('litellm_context_compression', 0, datetime('now')),
    ('litellm_pre_session_briefing', 0, datetime('now')),
    ('litellm_task_enrichment', 0, datetime('now'));

-- Compressed context storage for tasks (populated when token usage exceeds threshold)
ALTER TABLE tasks ADD COLUMN compressed_context TEXT;
