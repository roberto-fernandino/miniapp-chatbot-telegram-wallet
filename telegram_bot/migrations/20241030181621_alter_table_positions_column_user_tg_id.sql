-- Add migration script here

-- Add column user_tg_id to positions table
ALTER TABLE positions ALTER COLUMN tg_user_id TYPE VARCHAR(255);
