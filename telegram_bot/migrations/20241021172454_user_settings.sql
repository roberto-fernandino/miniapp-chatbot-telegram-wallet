-- Add migration script here


CREATE TABLE IF NOT EXISTS user_settings (
    id SERIAL PRIMARY KEY,
    tg_id VARCHAR(255) NOT NULL,
    slippage_tolerance VARCHAR(255) NOT NULL,
    buy_amount VARCHAR(255) NOT NULL,
    swap_or_limit VARCHAR(255) NOT NULL,
    FOREIGN KEY (tg_id) REFERENCES users(tg_id)
)