-- Add migration script here

-- Add amount column to positions table
ALTER TABLE positions ADD COLUMN ui_amount VARCHAR(255);
