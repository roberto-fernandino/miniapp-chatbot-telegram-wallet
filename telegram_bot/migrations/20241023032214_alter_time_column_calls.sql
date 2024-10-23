-- Add migration script here


-- Alter time column to be TIMESTAMPTZ
ALTER TABLE calls
   ALTER COLUMN time TYPE TIMESTAMPTZ
   USING time::timestamptz;
