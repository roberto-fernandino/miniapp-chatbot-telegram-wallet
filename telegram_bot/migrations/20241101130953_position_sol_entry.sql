-- Add migration script here


-- Add sol_entry to positions table
ALTER TABLE positions ADD COLUMN sol_entry DECIMAL(18, 9);
