-- Add migration script here

-- Add take_profits and stop_losses to user_settings
ALTER TABLE user_settings ADD COLUMN take_profits JSONB;
ALTER TABLE user_settings ADD COLUMN stop_losses JSONB;