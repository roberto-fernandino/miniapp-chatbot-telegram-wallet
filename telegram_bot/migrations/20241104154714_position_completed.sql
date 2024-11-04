-- Add migration script here

-- Add column to positions table
ALTER TABLE positions ADD COLUMN completed BOOLEAN NOT NULL DEFAULT FALSE;