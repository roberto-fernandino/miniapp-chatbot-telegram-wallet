-- Add migration script here

-- add jito_tip_amount column to user_settings table
ALTER TABLE user_settings ADD COLUMN jito_tip_amount INTEGER;