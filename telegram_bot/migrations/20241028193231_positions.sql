-- Add migration script here

-- Creating positions table
CREATE TABLE IF NOT EXISTS positions (
    id SERIAL PRIMARY KEY,
    tg_user_id INTEGER NOT NULL,
    token_address TEXT NOT NULL,
    take_profits JSONB NOT NULL DEFAULT '[]',
    stop_losses JSONB NOT NULL DEFAULT '[]',
    amount FLOAT NOT NULL,
    mc_entry FLOAT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);