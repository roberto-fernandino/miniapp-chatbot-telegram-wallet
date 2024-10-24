-- Add migration script here


-- Alter time column to be TIMESTAMPTZ
ALTER TABLE user_settings
ALTER COLUMN sell_percentage TYPE VARCHAR(255);