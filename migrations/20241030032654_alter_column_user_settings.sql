-- Add migration script here

-- Set default values for take_profits and stop_losses
ALTER TABLE user_settings 
    ALTER COLUMN take_profits SET DEFAULT '[]'::jsonb,
    ALTER COLUMN stop_losses SET DEFAULT '[]'::jsonb;

-- Update existing NULL values to empty arrays
UPDATE user_settings 
    SET take_profits = '[]'::jsonb 
    WHERE take_profits IS NULL;

UPDATE user_settings 
    SET stop_losses = '[]'::jsonb 
    WHERE stop_losses IS NULL;
