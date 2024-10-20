-- Add migration script here

-- Add turnkey info to users table
ALTER TABLE users ADD COLUMN api_public_key TEXT UNIQUE, ADD COLUMN api_private_key TEXT UNIQUE, ADD COLUMN suborg_id TEXT UNIQUE, ADD COLUMN wallet_id TEXT UNIQUE, ADD COLUMN solana_address TEXT UNIQUE, ADD COLUMN eth_address TEXT UNIQUE;