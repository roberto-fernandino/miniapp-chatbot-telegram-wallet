-- Add migration script here

-- add anti-mev column to user_settings table
ALTER TABLE user_settings ADD COLUMN anti_mev BOOLEAN DEFAULT FALSE;