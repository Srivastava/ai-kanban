-- Remove 'result' event type rows from token_events.
-- Result events contain cumulative session totals (not per-API-call data) and
-- were incorrectly stored by old code before the message_id dedup fix.
-- Including them in SUM queries inflates token/cost counts by ~55%.
-- The new manager.rs code never stores result events — this migration cleans legacy data.
DELETE FROM token_events WHERE event_type = 'result';

-- Also remove the small number of streaming-start assistant events (output_tokens = 0)
-- that have a matching final event with the same input/cache tokens in the same session.
-- These are the streaming-start half of duplicated pairs from pre-dedup code.
DELETE FROM token_events
WHERE event_type = 'assistant'
  AND output_tokens = 0
  AND EXISTS (
      SELECT 1 FROM token_events b
      WHERE b.session_id = token_events.session_id
        AND b.event_type = 'assistant'
        AND b.output_tokens > 0
        AND b.input_tokens = token_events.input_tokens
        AND b.cache_read_tokens = token_events.cache_read_tokens
        AND b.cache_creation_tokens = token_events.cache_creation_tokens
  );
