-- Add migration script here

-- Create 'users' table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    tg_id TEXT NOT NULL UNIQUE,
    username TEXT
);

-- Create 'calls' table
CREATE TABLE IF NOT EXISTS calls (
    id SERIAL PRIMARY KEY,
    time TEXT,
    mkt_cap TEXT,
    token_address TEXT,
    token_mint TEXT,
    token_symbol TEXT,
    price TEXT,
    user_tg_id TEXT REFERENCES users(tg_id),
    chat_id TEXT,
    message_id TEXT,
    chain TEXT
);