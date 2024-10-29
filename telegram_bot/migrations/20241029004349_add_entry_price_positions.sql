-- Add migration script here

-- Add entry_price to positions table
ALTER TABLE positions ADD COLUMN entry_price FLOAT;