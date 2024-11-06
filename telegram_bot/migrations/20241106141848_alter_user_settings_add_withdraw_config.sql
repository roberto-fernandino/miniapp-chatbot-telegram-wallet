-- Add migration script here

-- Add the withdraw config to the user settings table
ALTER TABLE user_settings ADD COLUMN withdraw_sol_amount VARCHAR(255), ADD COLUMN withdraw_sol_address VARCHAR(255);