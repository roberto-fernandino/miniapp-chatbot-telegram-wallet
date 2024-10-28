-- Add migration script here


-- Alter user_settings table to add gas column
ALTER TABLE user_settings ADD COLUMN gas_lamports INTEGER;