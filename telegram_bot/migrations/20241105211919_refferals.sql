-- Add migration script here


-- Add a table for refferals
CREATE TABLE IF NOT EXISTS refferals (
    id SERIAL PRIMARY KEY,
    user_tg_id VARCHAR(255) NOT NULL UNIQUE,
    uuid VARCHAR(255) NOT NULL UNIQUE,
    users_referred INTEGER DEFAULT 0,
    referral_rebates INTEGER DEFAULT 0,
    total_rewards VARCHAR(255) DEFAULT '0',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE users ADD COLUMN refferal_id INTEGER REFERENCES refferals(id);