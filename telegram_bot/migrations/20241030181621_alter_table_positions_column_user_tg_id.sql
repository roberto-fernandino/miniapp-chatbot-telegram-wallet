-- Add migration script here

-- Add column user_tg_id to positions table
ALTER TABLE positions ALTER COLUMN user_tg_id TYPE VARCHAR(255);
