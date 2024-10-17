use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use anyhow::Result;
use serde::Serialize;
use chrono::{NaiveDateTime, Utc};
use std::env;
use dotenv::dotenv;

/// Represents a user in the system.
#[derive(Debug, Serialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub tg_id: String,
}

/// Represents a call in the system.
#[derive(Debug, Serialize)]
pub struct Call {
    pub id: i64,
    pub time: NaiveDateTime,
    pub mkt_cap: String,
    pub token_address: String,
    pub token_mint: String,
    pub token_symbol: String,
    pub price: String,
    pub user_tg_id: String,
    pub chat_id: String,
    pub message_id: String,
    pub chain: String,
}

/// Represents a call history with additional ATH data.
#[derive(Debug, Serialize)]
pub struct CallHistoryUser {
    pub call: Call,
    pub multiplier: f64,
    pub ath: f64,
}

/// Type alias for the PostgreSQL connection pool.
pub type PgPoolType = Pool<Postgres>;

/// Initializes and returns a PostgreSQL connection pool.
pub async fn get_pool() -> Result<PgPoolType> {
    // Load environment variables from `.env` file.
    let database_url = env::var("DATABASE_URL")?;
    
    // Configure the connection pool.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    Ok(pool)
}

/// Configures the database by creating necessary tables if they don't exist.
pub async fn configure_db(pool: &PgPoolType) -> Result<()> {
    // Create 'users' table.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            tg_id TEXT NOT NULL UNIQUE,
            username TEXT
        );
        "#
    )
    .execute(pool)
    .await?;
    
    // Create 'calls' table.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS calls (
            id SERIAL PRIMARY KEY,
            time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
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
        "#
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Retrieves a user by their Telegram ID.
pub async fn get_user(pool: &PgPoolType, tg_id: &str) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, tg_id, username
        FROM users
        WHERE tg_id = $1
        "#,
        tg_id
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

/// Adds a new user to the database.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `tg_id` - The user's Telegram ID.
/// * `username` - The user's username.
///
/// # Returns
///
/// An empty result indicating success or an error.
pub async fn add_user(pool: &PgPoolType, tg_id: &str, username: Option<&str>) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO users (tg_id, username)
        VALUES ($1, $2)
        ON CONFLICT (tg_id) DO NOTHING
        "#,
        tg_id,
        username
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Adds a new call to the database and returns its ID.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `tg_id` - The user's Telegram ID.
/// * `mkt_cap` - The market capitalization of the token.
/// * `token_address` - The token's address.
/// * `token_mint` - The token's mint.
/// * `token_symbol` - The token's symbol.
/// * `price` - The price of the token.
/// * `chat_id` - The chat ID.
/// * `message_id` - The message ID.
/// * `chain` - The blockchain chain.
///
/// # Returns
///
/// The ID of the newly created call.
pub async fn add_call(
    pool: &PgPoolType, 
    tg_id: &str, 
    mkt_cap: &str, 
    token_address: &str, 
    token_mint: &str,
    token_symbol: &str,
    price: &str, 
    chat_id: &str,
    message_id: &str,
    chain: &str
) -> Result<i64> {
    let record = sqlx::query!(
        r#"
        INSERT INTO calls (user_tg_id, mkt_cap, token_address, token_mint, token_symbol, price, chat_id, message_id, chain)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
        tg_id,
        mkt_cap,
        token_address,
        token_mint,
        token_symbol,
        price,
        chat_id,
        message_id,
        chain
    )
    .fetch_one(pool)
    .await?;
    
    Ok(record.id)
}

/// Retrieves the first call of a token in a specific chat.
pub async fn get_first_call_by_token_address(pool: &PgPoolType, token_address: &str, chat_id: &str) -> Result<Option<Call>> {
    let call = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE token_address = $1 AND chat_id = $2
        ORDER BY time ASC
        LIMIT 1
        "#,
        token_address,
        chat_id
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(call)
}

/// Retrieves a call by its ID.
pub async fn get_call_by_id(pool: &PgPoolType, id: i64) -> Result<Option<Call>> {
    let call = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(call)
}

/// Retrieves all calls made in a specific chat.
pub async fn get_all_calls_chat_id(pool: &PgPoolType, chat_id: &str) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE chat_id = $1
        ORDER BY time ASC
        "#,
        chat_id
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves all calls made in a channel in the last `x` days.
pub async fn get_channel_calls_last_x_days(pool: &PgPoolType, chat_id: &str, days: u32) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE time >= NOW() - INTERVAL '$1 days' AND chat_id = $2
        ORDER BY time ASC
        "#,
        days,
        chat_id
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves all calls made in a channel in the last `x` hours.
pub async fn get_channel_calls_last_x_hours(pool: &PgPoolType, chat_id: &str, hours: u32) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE time >= NOW() - INTERVAL '$1 hours' AND chat_id = $2
        ORDER BY time ASC
        "#,
        hours,
        chat_id
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves all calls made in a channel in the last `x` months.
pub async fn get_channel_calls_last_x_months(pool: &PgPoolType, chat_id: &str, months: u32) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE time >= NOW() - INTERVAL '$1 months' AND chat_id = $2
        ORDER BY time ASC
        "#,
        months,
        chat_id
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves all calls made by a specific user in the last `x` days.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `tg_id` - The user's Telegram ID.
/// * `days` - The number of days.
///
/// # Returns
///
/// A vector of calls.
pub async fn get_user_calls_last_x_days(pool: &PgPoolType, tg_id: &str, days: u32) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '$2 days'
        ORDER BY time ASC
        "#,
        tg_id,
        days
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves all calls made by a specific user in the last `x` hours.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `tg_id` - The user's Telegram ID.
/// * `hours` - The number of hours.
///
/// # Returns
///
/// A vector of calls.
pub async fn get_user_calls_last_x_hours(pool: &PgPoolType, tg_id: &str, hours: u32) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '$2 hours'
        ORDER BY time ASC
        "#,
        tg_id,
        hours
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves all calls made by a specific user in the last `x` months.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `tg_id` - The user's Telegram ID.
/// * `months` - The number of months.
///
/// # Returns
///
/// A vector of calls.
pub async fn get_user_calls_last_x_months(pool: &PgPoolType, tg_id: &str, months: u32) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '$2 months'
        ORDER BY time ASC
        "#,
        tg_id,
        months
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Retrieves the first call for each token addressed by a user.
pub async fn get_all_user_firsts_calls_by_user_tg_id(pool: &PgPoolType, user_id: &str) -> Result<Vec<Call>> {
    let calls = sqlx::query_as!(
        Call,
        r#"
        SELECT DISTINCT ON (token_address)
            id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1
        ORDER BY token_address, time ASC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    
    Ok(calls)
}

/// Deletes a call by its ID.
pub async fn delete_call(pool: &PgPoolType, call_id: i64) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM calls
        WHERE id = $1
        "#,
        call_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Clears all calls made by a user in a specific chat.
pub async fn clear_calls(pool: &PgPoolType, tg_id: &str, chat_id: &str) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM calls
        WHERE user_tg_id = $1 AND chat_id = $2
        "#,
        tg_id,
        chat_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Retrieves the number of distinct tokens a user has called within a specific period.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `user_tg_id` - The user's Telegram ID.
/// * `chat_id` - The chat ID.
/// * `period` - The period string (e.g., "7 days").
///
/// # Returns
///
/// The count of distinct tokens.
pub async fn get_distinct_token_count(pool: &PgPoolType, user_tg_id: &str, chat_id: &str, period: &str) -> Result<i64> {
    let count = sqlx::query!(
        r#"
        SELECT COUNT(DISTINCT token_symbol) as count
        FROM calls
        WHERE user_tg_id = $1
          AND chat_id = $2
          AND time >= NOW() - INTERVAL $3
        "#,
        user_tg_id,
        chat_id,
        period
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);
    
    Ok(count)
}

/// Retrieves the total number of calls in a chat within a specific period.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `chat_id` - The chat ID.
/// * `period` - The period string (e.g., "7 days").
///
/// # Returns
///
/// The total number of calls.
pub async fn get_total_calls_in_chat(pool: &PgPoolType, chat_id: &str, period: &str) -> Result<i64> {
    let count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM calls
        WHERE chat_id = $1
          AND time >= NOW() - INTERVAL $2
        "#,
        chat_id,
        period
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);
    
    Ok(count)
}

/// Retrieves the number of calls a user has made in the last 24 hours.
pub async fn get_qtd_calls_user_made_in_24hrs(pool: &PgPoolType, user_tg_id: &str) -> Result<i64> {
    let count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '24 HOURS'
        "#,
        user_tg_id
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);
    
    Ok(count)
}

/// Checks if a call is the first one in a chat for a given token.
pub async fn is_first_call(pool: &PgPoolType, token_address: &str, chat_id: &str) -> Result<bool> {
    let count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM calls
        WHERE token_address = $1 AND chat_id = $2
        "#,
        token_address,
        chat_id
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);
    
    Ok(count == 0)
}

/// Retrieves the user associated with a specific call.
pub async fn get_user_from_call(pool: &PgPoolType, call_id: i64) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT u.id, u.tg_id, u.username
        FROM users u
        JOIN calls c ON c.user_tg_id = u.tg_id
        WHERE c.id = $1
        "#,
        call_id
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}