-- Add context field to tasks table for storing markdown content
ALTER TABLE tasks ADD COLUMN context TEXT;
