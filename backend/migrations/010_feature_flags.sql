-- Fix LiteLLM session summary comments incorrectly stored with author='claude'
UPDATE comments
SET author = 'litellm'
WHERE author = 'claude'
  AND content LIKE '**Session Summary%';

-- Feature flags for LiteLLM optimizations (toggleable at runtime)
CREATE TABLE IF NOT EXISTS feature_flags (
    key TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO feature_flags (key, enabled) VALUES
    ('litellm_session_summary', 1),
    ('litellm_context_compression', 0),
    ('litellm_pre_session_briefing', 0),
    ('litellm_task_enrichment', 0);

-- Compressed context storage for tasks (populated when token usage exceeds threshold)
ALTER TABLE tasks ADD COLUMN compressed_context TEXT;
