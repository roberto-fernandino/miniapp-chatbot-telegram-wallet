-- Add migration script here

-- Add chat_id to positions table
ALTER TABLE positions ADD COLUMN chat_id VARCHAR(255);