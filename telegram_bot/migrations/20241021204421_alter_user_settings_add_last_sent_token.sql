-- Add migration script here


ALTER TABLE user_settings ADD COLUMN last_sent_token VARCHAR(255);