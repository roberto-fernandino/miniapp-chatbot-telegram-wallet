-- Add migration script here


ALTER TABLE user_settings
ADD COLUMN sell_percentage VARCHAR(255) NOT NULL;