-- Add cache token columns to token_events for accurate context-size tracking.
-- Claude's prompt cache means most context is in cache_read_input_tokens (not input_tokens).
-- Without these columns, peak context detection underestimates actual context size.
ALTER TABLE token_events ADD COLUMN cache_read_tokens     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE token_events ADD COLUMN cache_creation_tokens INTEGER NOT NULL DEFAULT 0;
