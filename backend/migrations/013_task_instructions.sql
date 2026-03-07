-- Separate user-provided description from LiteLLM-enriched instructions.
-- description: protected, user-written original requirements
-- instructions: LiteLLM can enrich/overwrite; what Claude actually receives
ALTER TABLE tasks ADD COLUMN instructions TEXT;
