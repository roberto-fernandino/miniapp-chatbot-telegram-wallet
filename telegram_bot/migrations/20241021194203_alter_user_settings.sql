-- Add migration script here
ALTER TABLE user_settings ALTER COLUMN tg_id SET NOT NULL;
ALTER TABLE user_settings ADD CONSTRAINT user_settings_tg_id_key UNIQUE (tg_id);
