-- Add migration script here

-- Alter sol_entry to double precision
ALTER TABLE positions ALTER COLUMN sol_entry TYPE DOUBLE PRECISION USING sol_entry::DOUBLE PRECISION;
