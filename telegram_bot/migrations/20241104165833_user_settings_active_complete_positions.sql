-- Add migration script here


-- Add the active_complete_positions column to the user_settings table
ALTER TABLE user_settings ADD COLUMN active_complete_positions VARCHAR(255); 