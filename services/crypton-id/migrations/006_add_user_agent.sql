-- Add user_agent field to track device OS/browser
ALTER TABLE credentials ADD COLUMN user_agent TEXT DEFAULT 'unknown';
