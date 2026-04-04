-- Track the peak context size (input + cache_read + cache_creation tokens) seen
-- in any single turn during a session. Used to decide zone-based handover strategy.
ALTER TABLE sessions ADD COLUMN peak_context_tokens INTEGER;
