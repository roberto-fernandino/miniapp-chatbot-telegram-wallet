use sqlx::{PgPool, postgres::PgPoolOptions};
use chrono::NaiveDateTime;
use serde_json::json;
use sqlx::{Encode, Pool};
use sqlx::Postgres;
use crate::handlers::PostUserRequest;
use crate::utils::helpers::check_period_for_leaderboard;
use serde::Serialize;
use sqlx::Row;
use anyhow::Result;
use std::env;
use std::sync::Arc;

/// Struct to hold the user settings
#[derive(Debug, Clone, Serialize)]
pub struct UserSettings {
    pub slippage_tolerance: String,
    pub buy_amount: String,
    pub swap_or_limit: String,
    pub tg_id: String,
    pub sell_percentage: String,
    pub gas_lamports: i32,
    pub anti_mev: bool,
    pub take_profits: Vec<(f64, f64)>,
    pub stop_losses: Vec<(f64, f64)>,
    pub jito_tip_amount: i32,
    pub active_complete_positions: String,
}

/// Struct to hold the call with the ATH after the call
/// 
/// 
/// # Fields
/// 
/// * `call` - The call
/// * `ath_after_call` - The ATH after the call
/// * `multiplier` - The multiplier
#[derive(Debug, Clone, Serialize)]
pub struct CallWithAth {
    pub call: Call,
    pub ath_after_call: f64,
    pub multiplier: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Encode)]
pub struct TurnkeyInfo {
    pub api_public_key: Option<String>,
    pub api_private_key: Option<String>,
    pub suborg_id: Option<String>,
    pub wallet_id: Option<String>,
}

/// Represents a user in the system.
#[derive(Debug, Serialize, Clone)]
pub struct User {
    pub id: i32,
    pub username: Option<String>,
    pub tg_id: String,
    pub turnkey_info: TurnkeyInfo,
    pub solana_address: Option<String>,
    pub eth_address: Option<String>,
}

/// Represents a call in the system.
#[derive(Debug, Serialize, Clone)]
pub struct Call {
    pub id: i32,
    pub time: chrono::DateTime<chrono::Utc>,
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

/// Represents a PNL call in the system.
#[derive(Debug, Serialize)]
pub struct PnlCall {
    pub call_id: i64,
    pub token_address: String,
    pub mkt_cap: String,
    pub percent: String,
}

#[derive(Debug, Serialize)]
pub struct Position {
    pub id: i32, // db id
    pub tg_user_id: String, // Telegram user id
    pub token_address: String, // Token address
    pub take_profits: Vec<(f64, f64)>, // Array of arrays with the take profit [ [ +% price limit to sell, % tokens to sell], ... ]
    pub stop_losses: Vec<(f64, f64)>, // Array of arrays with the stop [ [ -% price limit to sell, % tokens to sell], ... ]
    pub amount: f64, // Amount of tokens bought
    pub mc_entry: f64, // Market cap at entry
    pub entry_price: f64, // Price at entry
    pub created_at: NaiveDateTime, // Default value is the current timestamp
    pub chat_id: String, // Chat ID
    pub sol_entry: f64, // SOL at entry
    pub ui_amount: String, // UI amount
    pub completed: bool, // Whether the position is completed
}


#[derive(Debug, Serialize)]
pub struct Refferal {
    pub id: i32,
    pub user_tg_id: String,
    pub uuid: String,
    pub users_referred: i32,
    pub referral_rebates: i32,
    pub total_rewards: String,
}

#[derive(Debug, Serialize)]
pub struct ResponsePaylod {
    pub calls: Vec<CallWithAth>,
    pub username: String,
}

pub type SafePool = Arc<Pool<Postgres>>;



/// Initializes and returns a PostgreSQL connection pool.
pub async fn get_pool() -> Result<PgPool> {
    // Load environment variables from `.env` file.
    let database_url = env::var("DATABASE_URL")?;
    
    // Configure the connection pool.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    Ok(pool)
}



/// Retrieves a user by their Telegram ID.
pub async fn get_user(pool: &PgPool, tg_id: &str) -> Result<User> {
    let q = "SELECT * FROM users WHERE tg_id = $1";
    let row = sqlx::query(q) 
    .bind(tg_id)
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: row.get("id"),
        tg_id: row.get("tg_id"),
        username: row.get("username"),
        turnkey_info: TurnkeyInfo {
            api_public_key: row.get("api_public_key"),
            api_private_key: row.get("api_private_key"),
            suborg_id: row.get("suborg_id"),
            wallet_id: row.get("wallet_id"),
        },
        solana_address: row.get("solana_address"),  
        eth_address: row.get("eth_address"),
    })

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
pub async fn create_user_with_tg_id_and_username(pool: &PgPool, tg_id: &str, username: Option<&str>) -> Result<()> {
    println!("@create_user_with_tg_id_and_username/ tg_id: {}, username: {:?}", tg_id, username);
    let mut q;
    if let Some(_) = username {
        q = "INSERT INTO users (tg_id, username) VALUES ($1, $2) ON CONFLICT (tg_id) DO NOTHING";
    }
    else {
        q = "INSERT INTO users (tg_id, username) VALUES ($1, NULL) ON CONFLICT (tg_id) DO NOTHING";
    }
    let result = sqlx::query(q)
    .bind(tg_id)
    .bind(username)
    .execute(pool)
    .await?;
  
    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("No rows were affected"));
    }
    
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
    pool: &PgPool, 
    time: &str,
    tg_id: &str, 
    mkt_cap: &str, 
    token_address: &str, 
    token_mint: &str,
    token_symbol: &str,
    price: &str, 
    chat_id: &str,
    message_id: &str,
    chain: &str,
    username: Option<&str>
) -> Result<i32> {
    if !user_exists(pool, tg_id).await? {
        create_user_with_tg_id_and_username(pool, tg_id, username).await?;
    }
    let time = time.parse::<chrono::DateTime<chrono::Utc>>()?;
    let q = "INSERT INTO calls (time, user_tg_id, mkt_cap, token_address, token_mint, token_symbol, price, chat_id, message_id, chain) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id";
    let result = sqlx::query_scalar(q)
    .bind(time)
    .bind(tg_id)
    .bind(mkt_cap)
    .bind(token_address)
    .bind(token_mint)
    .bind(token_symbol)
    .bind(price)
    .bind(chat_id)
    .bind(message_id)
    .bind(chain)
    .fetch_one(pool)
    .await?;
    
    Ok(result)
}


/// Retrieves the first call of a token in a specific chat.
pub async fn get_first_call_by_token_address(pool: &PgPool, token_address: &str, chat_id: &str) -> Result<Call> {
    let q = "SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain FROM calls WHERE token_address = $1 AND chat_id = $2 ORDER BY time ASC LIMIT 1";
    let call = sqlx::query(q)
    .bind(token_address)
    .bind(chat_id)
    .fetch_one(pool)
    .await?;
    
    Ok(Call {
        id: call.get("id"),
        time: call.get("time"),
        mkt_cap: call.get("mkt_cap"),
        token_address: call.get("token_address"),
        token_mint: call.get("token_mint"),
        token_symbol: call.get("token_symbol"),
        price: call.get("price"),
        user_tg_id: call.get("user_tg_id"),
        chat_id: call.get("chat_id"),
        message_id: call.get("message_id"),
        chain: call.get("chain"),
    })
}


/// Retrieves a call by its ID.
pub async fn get_call_by_id(pool: &PgPool, id: i64) -> Result<Call> {
   let q = "SELECT * FROM calls WHERE id = $1";
   let call = sqlx::query(q)
   .bind(id)
   .fetch_one(pool)
   .await?;
  
   Ok(Call {
        id: call.get("id"),
        time: call.get("time"),
        mkt_cap: call.get("mkt_cap"),
        token_address: call.get("token_address"),
        token_mint: call.get("token_mint"),
        token_symbol: call.get("token_symbol"),
        price: call.get("price"),
        user_tg_id: call.get("user_tg_id"),
        chat_id: call.get("chat_id"),
        message_id: call.get("message_id"),
        chain: call.get("chain"),
   })
}

/// Retrieves all calls made in a specific chat.
pub async fn get_all_calls_chat_id(pool: &PgPool, chat_id: &str) -> Result<Vec<Call>> {
    let q = "SELECT * FROM calls WHERE chat_id = $1 ORDER BY time DESC";
    let calls = sqlx::query(q) 
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }
    Ok(calls_vec)
}

/// Retrieves all calls made in a channel in the last `x` days.
pub async fn get_channel_calls_last_x_days(pool: &PgPool, chat_id: &str, days: i32) -> Result<Vec<Call>> {
    let q = "SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE time::timestamp >= NOW() - INTERVAL '$1 days' AND chat_id = $2
        ORDER BY time ASC";

    let calls = sqlx::query(q)
    .bind(days)
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }           
    Ok(calls_vec)
}

/// Retrieves all calls made in a channel in the last `x` hours.
pub async fn get_channel_calls_last_x_hours(pool: &PgPool, chat_id: &str, hours: i32) -> Result<Vec<Call>> {
    let q = "SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE time >= NOW() - INTERVAL '$1 hours' AND chat_id = $2
        ORDER BY time ASC";

    let calls = sqlx::query(q)
    .bind(hours)
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),             
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
}

/// Retrieves all calls made in a channel in the last `x` months.
pub async fn get_channel_calls_last_x_months(pool: &PgPool, chat_id: &str, months: i32) -> Result<Vec<Call>> {
   let q = " SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE time >= NOW() - INTERVAL '$1 months' AND chat_id = $2
        ORDER BY time ASC";

    let channels = sqlx::query(q)
    .bind(months)
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in channels {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
}

/// Retrieves all calls made by a specific user in the last `x` years.
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
pub async fn get_user_calls_last_x_years(pool: &PgPool, tg_id: &str, years: i32) -> Result<Vec<Call>> {
    let q = "SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '$2 years'
        ORDER BY time ASC";

    let calls = sqlx::query(q)
    .bind(tg_id)
    .bind(years)
    .fetch_all(pool)
    .await?;
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
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
pub async fn get_user_calls_last_x_hours(pool: &PgPool, tg_id: &str, hours: i32) -> Result<Vec<Call>> {
   let q = " SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '$2 hours'
        ORDER BY time ASC";

    let calls = sqlx::query(q)
    .bind(tg_id)
    .bind(hours)
    .fetch_all(pool)
    .await?;

    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),     
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
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
pub async fn get_user_calls_last_x_months(pool: &PgPool, tg_id: &str, months: i32) -> Result<Vec<Call>> {
    let q = " SELECT id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '$2 months'
        ORDER BY time ASC";

    let calls = sqlx::query(q)
    .bind(tg_id)
    .bind(months)
    .fetch_all(pool)
    .await?;
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
}

/// Retrieves the first call for each token addressed by a user.
pub async fn get_all_user_firsts_calls_by_user_tg_id(pool: &PgPool, user_id: &str) -> Result<Vec<Call>> {
    let q = "SELECT DISTINCT ON (token_address)
            id, time, mkt_cap, token_address, token_mint, token_symbol, price, user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE user_tg_id = $1
        ORDER BY token_address, time ASC";

    let calls = sqlx::query(q)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            price: call.get("price"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
}

/// Deletes a call by its ID.
pub async fn delete_call(pool: &PgPool, call_id: i64) -> Result<()> {
    sqlx::query(
        "
        DELETE FROM calls
        WHERE id = $1
        "
    )
    .bind(call_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Delete all calls made by a user in a specific chat.
pub async fn delete_all_calls(pool: &PgPool, tg_id: &str, chat_id: &str) -> Result<()> {
    sqlx::query(
        "
        DELETE FROM calls
        WHERE user_tg_id = $1 AND chat_id = $2
        "
    )
    .bind(tg_id)
    .bind(chat_id)
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
pub async fn get_distinct_token_count(
    pool: &PgPool,
    user_tg_id: &str,
    chat_id: &str,
    period: &str,
) -> Result<i64> {
    let q = "
        SELECT COUNT(DISTINCT token_symbol) AS count
        FROM calls
        WHERE user_tg_id = $1
          AND chat_id = $2
          AND time >= NOW() - $3::interval
    ";

    // Use `query_scalar` to directly fetch the count as `i64`
    let count: i64 = sqlx::query_scalar(q)
        .bind(user_tg_id)
        .bind(chat_id)
        .bind(period)
        .fetch_one(pool)
        .await?;

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
pub async fn get_total_calls_in_chat(pool: &PgPool, chat_id: &str, period: &str) -> Result<i64> {
    let q = "
        SELECT COUNT(*) AS count
        FROM calls
        WHERE chat_id = $1
          AND time >= NOW() - $2::interval
    ";

    // Use `query_scalar` to directly fetch the count as `i64`
    let count: i64 = sqlx::query_scalar(q)
    .bind(chat_id)
    .bind(period)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Retrieves the number of calls a user has made in the last 24 hours.
pub async fn get_qtd_calls_user_made_in_24hrs(pool: &PgPool, user_tg_id: &str) -> Result<i64> {
    let q = "
        SELECT COUNT(*) as count
        FROM calls
        WHERE user_tg_id = $1 AND time::timestamptz >= NOW() - INTERVAL '24 hours'
        ";

    let count: i64 = sqlx::query_scalar(q)
    .bind(user_tg_id)
    .fetch_one(pool)
    .await?;
   
    Ok(count)
}

/// Checks if a call is the first one in a chat for a given token.
pub async fn is_first_call(pool: &PgPool, token_address: &str, chat_id: &str) -> Result<bool> {
    let q = "
        SELECT COUNT(*) as count
        FROM calls
        WHERE token_address = $1 AND chat_id = $2
        ";

    let count: i64 = sqlx::query_scalar(q)
    .bind(token_address)
    .bind(chat_id)
    .fetch_one(pool)
    .await?;

    Ok(count == 0)
}

/// Retrieves the user associated with a specific call.
pub async fn get_user_from_call(pool: &PgPool, call_id: i64) -> Result<User> {
    let q = "
        SELECT u.id, u.tg_id, u.username
        FROM users u
        JOIN calls c ON c.user_tg_id = u.tg_id
        WHERE c.id = $1
        ";

    let user = sqlx::query(q)
    .bind(call_id)
    .fetch_one(pool)
    .await?;

    Ok(User {
        id: user.get("id"),
        tg_id: user.get("tg_id"),
        username: user.get("username"),
        turnkey_info: TurnkeyInfo {
            api_public_key: user.try_get("api_public_key").ok(),
            api_private_key: user.try_get("api_private_key").ok(),
            suborg_id: user.try_get("suborg_id").ok(),
            wallet_id: user.try_get("wallet_id").ok(),
        },
        solana_address: user.get("solana_address"),
        eth_address: user.get("eth_address"),
    })
}

pub async fn get_first_call_token_chat(
    pool: &PgPool,
    token_address: &str,
    chat_id: &str,
) -> Result<Call> {
    let query = r#"
        SELECT id, time, mkt_cap, price, token_address, token_mint, token_symbol, 
               user_tg_id, chat_id, message_id, chain
        FROM calls
        WHERE token_address = $1 AND chat_id = $2
        ORDER BY time ASC
        LIMIT 1
    "#;

    let call = sqlx::query(query)
        .bind(token_address)
        .bind(chat_id)
        .fetch_one(pool)
        .await?;

    Ok(Call {
        id: call.get("id"),
        time: call.get("time"),
        mkt_cap: call.get("mkt_cap"),
        price: call.get("price"),
        token_address: call.get("token_address"),
        token_mint: call.get("token_mint"),
        token_symbol: call.get("token_symbol"),
        user_tg_id: call.get("user_tg_id"),
        chat_id: call.get("chat_id"),
        message_id: call.get("message_id"),
        chain: call.get("chain"),
    })
}

/// Get the user call count for a user
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `chat_id` - The chat ID
/// * `period` - The period to get the call count
/// 
/// # Returns
/// 
/// The number of calls made by the user in the last period
pub async fn get_user_call_count_for_user_chat_with_period(
    pool: &PgPool,
    user_tg_id: &str,
    chat_id: &str,
    period: &str,
) -> Result<i64> {
    let (number, unit) = match check_period_for_leaderboard(period) {
        Some(p) => p,
        None => return Ok(0), // Invalid period
    };

    let interval = match unit {
        "h" => format!("{} hours", number),
        "d" => format!("{} days", number),
        "w" => format!("{} weeks", number),
        "y" => format!("{} years", number),
        _ => return Ok(0), // Invalid unit
    };

    let count: i64 = sqlx::query_scalar(
        "
        SELECT COUNT(DISTINCT token_symbol) as count
        FROM calls
        WHERE user_tg_id = $1
          AND chat_id = $2
          AND time::timestamp >= NOW() - $3::interval
        "
    )
    .bind(user_tg_id)
    .bind(chat_id)
    .bind(interval)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Get the number of calls in a chat in the last period
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `chat_id` - The chat ID
/// * `period` - The period to get the call count
/// 
/// # Returns
/// 
/// The number of calls made in the last period
pub async fn get_chat_call_count_with_period(
    pool: &PgPool,
    chat_id: &str,
    period: &str,
) -> Result<i64> {
    let (number, unit) = match check_period_for_leaderboard(period) {
        Some(p) => p,
        None => return Ok(0), // Invalid period
    };

    let interval = match unit {
        "h" => format!("{} hours", number),
        "d" => format!("{} days", number),
        "w" => format!("{} weeks", number),
        "y" => format!("{} years", number),
        _ => return Ok(0), // Invalid unit
    };

    let count: i64 = sqlx::query_scalar(
        "
        SELECT COUNT(*) as count
        FROM calls
        WHERE chat_id = $1
          AND time >= NOW() - INTERVAL $2
        ",
    )
    .bind(chat_id)
    .bind(interval)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Gets all calls made by a user
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A vector of Call structs
pub async fn get_all_calls_user_tg_id(pool: &PgPool, user_tg_id: &str) -> Result<Vec<Call>> {
    let query = "SELECT id, time, mkt_cap, price, token_address, token_mint, token_symbol, user_tg_id, chat_id, message_id, chain FROM calls WHERE user_tg_id = $1";
    let calls = sqlx::query(query)
        .bind(user_tg_id)
        .fetch_all(pool)
        .await?;
    
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        calls_vec.push(Call {
            id: call.get("id"),
            time: call.get("time"),
            mkt_cap: call.get("mkt_cap"),
            price: call.get("price"),
            token_address: call.get("token_address"),
            token_mint: call.get("token_mint"),
            token_symbol: call.get("token_symbol"),
            user_tg_id: call.get("user_tg_id"),
            chat_id: call.get("chat_id"),
            message_id: call.get("message_id"),
            chain: call.get("chain"),
        });
    }   
    Ok(calls_vec)
}

// Checks if a user exists in the database by their Telegram ID.
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool.
/// * `user_tg_id` - The user's Telegram ID.
///
/// # Returns
///
/// A result indicating whether the user exists or an error.
pub async fn user_exists(pool: &PgPool, user_tg_id: &str) -> Result<bool, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE tg_id = $1")
        .bind(user_tg_id)
        .fetch_one(pool)
        .await?; // Await the result here
    Ok(count.0 > 0)
}


/// Updates a user in the database
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user` - The user
/// 
/// # Returns
/// 
/// A result indicating whether the user was updated
pub async fn update_user(pool: &PgPool, user: User) -> Result<()> {
    let turnkey_info = serde_json::to_value(user.turnkey_info).unwrap();
    sqlx::query(
        "
        UPDATE users SET username = $1, solana_address = $2, eth_address = $3, api_public_key = $4, api_private_key = $5, suborg_id = $6, wallet_id = $7 WHERE tg_id = $8
        "
    )
    .bind(user.username)
    .bind(user.solana_address)
    .bind(user.eth_address)
    .bind(turnkey_info.get("api_public_key"))
    .bind(turnkey_info.get("api_private_key"))
    .bind(turnkey_info.get("suborg_id"))
    .bind(turnkey_info.get("wallet_id"))
    .bind(user.tg_id)
    .execute(pool)
    .await?;
    Ok(())
}   


/// Gets the user by the user's Telegram ID
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A User struct
pub async fn get_user_by_tg_id(pool: &PgPool, tg_id: &str) -> Result<User> {
    let fetch_response = sqlx::query("SELECT * FROM users WHERE tg_id = $1")
    .bind(tg_id)
    .fetch_one(pool)
    .await?;

    Ok(User{
        id: fetch_response.get("id"),
        tg_id: fetch_response.get("tg_id"),
        username: fetch_response.get("username"),
        turnkey_info: TurnkeyInfo {
            api_public_key: fetch_response.get("api_public_key"),
            api_private_key: fetch_response.get("api_private_key"),
            suborg_id: fetch_response.get("suborg_id"),
            wallet_id: fetch_response.get("wallet_id"),
        },
        solana_address: fetch_response.get("solana_address"),
        eth_address: fetch_response.get("eth_address"),
    })
}


/// Gets the user ID by the user's Telegram ID
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// The user's ID
pub async fn get_user_id_by_tg_id(pool: &PgPool, tg_id: &str) -> Result<i32> {
    let id: i32 = sqlx::query_scalar("SELECT id FROM users WHERE tg_id = $1")
    .bind(tg_id)
    .fetch_one(pool)
    .await?;
    Ok(id)
}   



/// Adds a user to the database from the post user request
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `post_user_request` - The user request
/// 
/// # Returns
/// 
/// A result indicating whether the user was added
pub async fn add_user_post(pool: &PgPool, post_user_request: PostUserRequest) -> Result<()> {
    let turnkey_info = serde_json::to_value(post_user_request.turnkey_info).unwrap();
    sqlx::query(
        "
        INSERT INTO users (tg_id, username, api_private_key, api_public_key, suborg_id, wallet_id, solana_address, eth_address) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "
    )
    .bind(post_user_request.tg_id)
    .bind(post_user_request.username)
    .bind(turnkey_info.get("api_private_key"))
    .bind(turnkey_info.get("api_public_key"))
    .bind(turnkey_info.get("suborg_id"))
    .bind(turnkey_info.get("wallet_id"))
    .bind(post_user_request.solana_address)
    .bind(post_user_request.eth_address)
    .execute(pool)
    .await?;
    Ok(())
}

/// Checks if a user is registered in the mini app
/// 
/// # Description
/// 
/// A user is registered in the mini app if they have a Solana address, an Ethereum address, and a Turnkey API public key and private key
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A boolean indicating whether the user is registered in the mini app
pub async fn is_user_registered_in_mini_app(pool: &PgPool, user_tg_id: &str, username: &str) -> Result<bool> {
    let user_exists = user_exists(pool, &user_tg_id).await?;
    if user_exists {
        let user = get_user_by_tg_id(pool, &user_tg_id).await?;
        Ok(user.turnkey_info.api_public_key != None && user.turnkey_info.api_private_key != None)
    } else {
        create_user_with_tg_id_and_username(pool, &user_tg_id, Some(&username)).await?;
        Ok(false)
    }
}   


/// Sets the user settings for a user
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `slippage_tolerance` - The slippage tolerance
/// * `buy_amount` - The buy amount
/// * `swap_or_limit` - The swap or limit
/// 
/// # Returns
/// 
/// A result indicating whether the user settings were set
pub async fn upsert_user_settings(pool: &PgPool, tg_id: &str, slippage_tolerance: &str, buy_amount: &str, swap_or_limit: &str, last_sent_token: &str, sell_percentage: &str, gas_lamports: i32, anti_mev: bool, take_profits: Vec<(f64, f64)>, stop_losses: Vec<(f64, f64)>, jito_tip_amount: i32, active_complete_positions: &str) -> Result<()> {
    let take_profits_json = serde_json::to_value(take_profits).unwrap();
    let stop_losses_json = serde_json::to_value(stop_losses).unwrap();
    sqlx::query("INSERT INTO user_settings (tg_id, slippage_tolerance, buy_amount, swap_or_limit, last_sent_token, sell_percentage, gas_lamports, anti_mev, take_profits, stop_losses, jito_tip_amount, active_complete_positions) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (tg_id) DO UPDATE SET slippage_tolerance = $2, buy_amount = $3, swap_or_limit = $4, last_sent_token = $5, sell_percentage = $6, gas_lamports = $7, anti_mev = $8, take_profits = $9, stop_losses = $10, jito_tip_amount = $11, active_complete_positions = $12")
    .bind(tg_id)
    .bind(slippage_tolerance)
    .bind(buy_amount)
    .bind(swap_or_limit)
    .bind(last_sent_token)
    .bind(sell_percentage)
    .bind(gas_lamports)
    .bind(anti_mev)
    .bind(take_profits_json)
    .bind(stop_losses_json)
    .bind(jito_tip_amount)
    .bind(active_complete_positions)
    .execute(pool)
    .await?;
    Ok(())
}   


/// Gets the user settings for a user
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_id` - The user's ID
/// 
/// # Returns
/// 
/// A UserSettings struct
pub async fn get_user_settings(pool: &PgPool, user_tg_id: &str) -> Result<UserSettings> {
    let user_settings = sqlx::query("SELECT * FROM user_settings WHERE tg_id = $1")
    .bind(user_tg_id)
    .fetch_one(pool)
    .await?;
    let take_profits: Vec<(f64, f64)> = match user_settings.try_get("take_profits") {
        Ok(Some(take_profits)) => serde_json::from_value(take_profits).unwrap_or_default(),
        _ => vec![],
    };
    let stop_losses: Vec<(f64, f64)> = match user_settings.try_get("stop_losses") {
        Ok(Some(stop_losses)) => serde_json::from_value(stop_losses).unwrap_or_default(),
        _ => vec![],
    };
    Ok(UserSettings {
        tg_id: user_tg_id.to_string(),
        slippage_tolerance: user_settings.get("slippage_tolerance"),
        buy_amount: user_settings.get("buy_amount"),
        swap_or_limit: user_settings.get("swap_or_limit"),
        sell_percentage: user_settings.get("sell_percentage"),
        gas_lamports: user_settings.get("gas_lamports"),
        anti_mev: user_settings.get("anti_mev"),
        take_profits,
        stop_losses,
        jito_tip_amount: user_settings.get("jito_tip_amount"),
        active_complete_positions: user_settings.get("active_complete_positions"),
    })
}

/// Set user sell percentage
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `sell_percentage` - The sell percentage
/// 
/// # Returns
/// 
/// A result indicating whether the user sell percentage was set
pub async fn set_user_sell_percentage(pool: &PgPool, tg_id: &str, sell_percentage: &str) -> Result<()> {
    sqlx::query("UPDATE user_settings SET sell_percentage = $1 WHERE tg_id = $2")
    .bind(sell_percentage)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set user gas lamports
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `gas_lamports` - The gas lamports
/// 
/// # Returns
/// 
/// A result indicating whether the user gas lamports were set
pub async fn set_user_gas_lamports(pool: &PgPool, tg_id: &str, gas_lamports: i32) -> Result<()> {
    sqlx::query("UPDATE user_settings SET gas_lamports = $1 WHERE tg_id = $2")
    .bind(gas_lamports)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set user jito tip amount
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `jito_tip_amount` - The jito tip amount
/// 
/// # Returns
/// 
/// A result indicating whether the user jito tip amount was set
pub async fn set_user_settings_jito_tip_amount(pool: &PgPool, tg_id: &str, jito_tip_amount: i32) -> Result<()> {
    sqlx::query("UPDATE user_settings SET jito_tip_amount = $1 WHERE tg_id = $2")
    .bind(jito_tip_amount)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set user swap or limit
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `swap_or_limit` - The swap or limit
/// 
/// # Returns
/// 
/// A result indicating whether the user swap or limit was set
pub async fn set_user_swap_or_limit(pool: &PgPool, tg_id: &str, swap_or_limit: &str) -> Result<()> {
    sqlx::query("UPDATE user_settings SET swap_or_limit = $1 WHERE tg_id = $2")
    .bind(swap_or_limit)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}   

/// Set user buy amount
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `buy_amount` - The buy amount
/// 
/// # Returns
/// 
/// A result indicating whether the user buy amount was set
pub async fn set_user_buy_amount(pool: &PgPool, tg_id: &str, buy_amount: &str) -> Result<()> {
    sqlx::query("UPDATE user_settings SET buy_amount = $1 WHERE tg_id = $2")
    .bind(buy_amount)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}


/// Set user slippage tolerance
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `slippage_tolerance` - The slippage tolerance
/// 
/// # Returns
/// 
/// A result indicating whether the user slippage tolerance was set
pub async fn set_user_slippage_tolerance(pool: &PgPool, tg_id: &str, slippage_tolerance: &str) -> Result<()> {
    sqlx::query("UPDATE user_settings SET slippage_tolerance = $1 WHERE tg_id = $2")
    .bind(slippage_tolerance)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Checks if a user has settings   
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A boolean indicating whether the user has settings
pub async fn user_has_settings(pool: &PgPool, user_tg_id: &str) -> Result<bool> {
    let user_settings = get_user_settings(pool, user_tg_id).await;
    Ok(user_settings.is_ok())
}


/// Creates the default user settings for a user
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the user settings were created
pub async fn create_user_settings_default(pool: &PgPool, user_tg_id: &str) -> Result<()> {
    upsert_user_settings(pool, user_tg_id, "0.18", "0.2", "swap", "", "100", 5000, false, vec![], vec![], 5000, "active").await.expect("Failed to create user settings");
    Ok(())
}



/// Sets the user last sent token
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// * `token_address` - The token address
/// 
/// # Returns
/// 
/// A result indicating whether the user last sent token was set
pub async fn set_user_last_sent_token(pool: &PgPool, tg_id: &str, token_address: &str) -> Result<()> {
    sqlx::query("UPDATE user_settings SET last_sent_token = $1 WHERE tg_id = $2")
    .bind(token_address)
    .bind(tg_id)
    .execute(pool)
    .await?;
    Ok(())
}   

/// Gets the user last sent token
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A string representing the user's last sent token
pub async fn get_user_last_sent_token(pool: &PgPool, tg_id: &str) -> Result<String> {
    let last_sent_token = sqlx::query_scalar("SELECT last_sent_token FROM user_settings WHERE tg_id = $1")
    .bind(tg_id)
    .fetch_one(pool)
    .await?;
    Ok(last_sent_token)
}


/// Retrieves user settings or creates default settings if none exist.
pub async fn get_or_create_user_settings(pool: &PgPool, user_tg_id: &str) -> Result<UserSettings> {
    match get_user_settings(pool, user_tg_id).await {
        Ok(settings) => Ok(settings),
        Err(_) => {
            create_user_settings_default(pool, user_tg_id).await?;
            get_user_settings(pool, user_tg_id).await
        },
        Err(e) => Err(e.into()),
    }
}

pub async fn insert_position(pool: &PgPool, tg_user_id: &str, token_address: &str, take_profits: Option<Vec<(f64, f64)>>, stop_losses: Option<Vec<(f64, f64)>>, amount: f64, mc_entry: f64, token_price: f64, chat_id: &str, sol_entry: f64, ui_amount: &str) -> Result<()> {
    let take_profits_json = if take_profits.is_some() {
        Some(serde_json::to_value(take_profits).unwrap())
    } else {
        None
    };

    let stop_losses_json = if stop_losses.is_some() {
        Some(serde_json::to_value(stop_losses).unwrap())
    } else {
        None
    };

    sqlx::query("INSERT INTO positions (tg_user_id, token_address, take_profits, stop_losses, amount, mc_entry, entry_price, chat_id, sol_entry, ui_amount, completed) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
    .bind(tg_user_id)
    .bind(token_address)
    .bind(take_profits_json)
    .bind(stop_losses_json)
    .bind(amount)
    .bind(mc_entry)
    .bind(token_price)
    .bind(chat_id)
    .bind(sol_entry)
    .bind(ui_amount)
    .bind(false)
    .execute(pool)
    .await?;
    Ok(())
}


/// Gets the user settings take profits
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<(f64, f64)> representing the take profits
pub async fn get_user_settings_take_profits(pool: &PgPool, user_tg_id: &str) -> Result<Option<Vec<(f64, f64)>>> {
    let take_profits: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT take_profits FROM user_settings WHERE tg_id = $1"
    )
    .bind(user_tg_id)
    .fetch_optional(pool)
    .await?;

    match take_profits {
        Some(json) => Ok(Some(serde_json::from_value(json).unwrap_or_default())),
        None => Ok(None)
    }
}

/// Gets the user settings stop losses
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<(f64, f64)> representing the stop losses
pub async fn get_user_settings_stop_losses(pool: &PgPool, user_tg_id: &str) -> Result<Option<Vec<(f64, f64)>>> {
    let stop_losses: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT stop_losses FROM user_settings WHERE tg_id = $1"
    )
    .bind(user_tg_id)
    .fetch_optional(pool)
    .await?;

    match stop_losses {
        Some(json) => Ok(Some(serde_json::from_value(json).unwrap_or_default())),
        None => Ok(None)
    }
}

/// Sets the user settings take profits
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `take_profits` - The take profits
/// 
/// # Returns
/// 
/// A result indicating whether the user settings take profits were set
pub async fn set_user_settings_take_profits(pool: &PgPool, user_tg_id: &str, take_profits: Option<Vec<(f64, f64)>>) -> Result<()> {
    let take_profits_json = serde_json::to_value(take_profits).unwrap_or(json!(null));
    sqlx::query(
        "UPDATE user_settings SET take_profits = $1 WHERE tg_id = $2"
    )
    .bind(take_profits_json)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Deletes a user settings take profit
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `take_profit` - The take profit
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the user settings take profit was deleted
pub async fn delete_user_settings_take_profit(pool: &PgPool, take_profit: (f64, f64), user_tg_id: &str) -> Result<()> {
    println!("@delete_user_settings_take_profit/ take_profit: {:?}", take_profit);

    if let Some(mut user_take_profits) = get_user_settings_take_profits(pool, user_tg_id).await? {
        println!("@delete_user_settings_take_profit/ user_take_profits: {:?}", user_take_profits);
        
        user_take_profits.retain(|&tp| tp != take_profit);
        println!("@delete_user_settings_take_profit/ user_take_profits after retaining: {:?}", user_take_profits);

        let take_profits = if user_take_profits.is_empty() {
            None
        } else {
            Some(user_take_profits)
        };
        println!("@delete_user_settings_take_profit/ setting user_take_profits to: {:?}", take_profits);

        set_user_settings_take_profits(pool, user_tg_id, take_profits).await?;
        println!("@delete_user_settings_take_profit/ user_take_profits was set.");
    }

    Ok(())
}

/// Deletes a user settings stop loss
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `take_profit` - The take profit
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the user settings stop loss was deleted
pub async fn delete_user_settings_stop_loss(pool: &PgPool, stop_loss: (f64, f64), user_tg_id: &str) -> Result<()> {
    println!("@delete_user_settings_stop_loss/ stop_loss: {:?}", stop_loss);

    if let Some(mut user_stop_losses) = get_user_settings_stop_losses(pool, user_tg_id).await? {
        println!("@delete_user_settings_stop_loss/ user_stop_losses: {:?}", user_stop_losses);
        
        user_stop_losses.retain(|&sl| sl != stop_loss);
        println!("@delete_user_settings_stop_loss/ user_stop_losses after retaining: {:?}", user_stop_losses);

        let stop_losses = if user_stop_losses.is_empty() {
            None
        } else {
            Some(user_stop_losses)
        };
        println!("@delete_user_settings_stop_loss/ setting user_stop_losses to: {:?}", stop_losses);

        set_user_settings_stop_losses(pool, user_tg_id, stop_losses).await?;
        println!("@delete_user_settings_stop_loss/ user_stop_losses was set.");
    }

    Ok(())
}

/// Get all positions
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// 
/// # Returns
/// 
/// A Vec<Position> representing all positions
pub async fn get_all_positions(pool: &PgPool) -> Result<Vec<Position>> {
    let positions = sqlx::query("SELECT * FROM positions")
    .fetch_all(pool)
    .await?;
    let mut positions_vec: Vec<Position> = Vec::new();
    for position in positions {
        let take_profits_value = position.get::<Option<serde_json::Value>, _>("take_profits");
        let stop_losses_value = position.get::<Option<serde_json::Value>, _>("stop_losses");

        let take_profits: Vec<(f64, f64)> = match take_profits_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };

        let stop_losses: Vec<(f64, f64)> = match stop_losses_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };

        positions_vec.push(Position {
            id: position.get("id"),
            tg_user_id: position.get("tg_user_id"),
            token_address: position.get("token_address"),
            amount: position.get("amount"),
            mc_entry: position.get("mc_entry"),
            entry_price: position.get("entry_price"),
            created_at: position.get("created_at"),
            chat_id: position.get("chat_id"),
            sol_entry: position.get("sol_entry"),
            ui_amount: position.get("ui_amount"),
            take_profits,
            stop_losses,
            completed: position.get("completed"),
        });
    }
    Ok(positions_vec)
}

/// Get all open positions
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// 
/// # Returns
/// 
/// A Vec<Position> representing all open positions
pub async fn get_all_open_positions(pool: &PgPool) -> Result<Vec<Position>> {
    let positions = sqlx::query("SELECT * FROM positions WHERE completed = false")
    .fetch_all(pool)
    .await?;
    let mut positions_vec: Vec<Position> = Vec::new();
    for position in positions {
        let take_profits_value = position.get::<Option<serde_json::Value>, _>("take_profits");
        let stop_losses_value = position.get::<Option<serde_json::Value>, _>("stop_losses");

        let take_profits: Vec<(f64, f64)> = match take_profits_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };

        let stop_losses: Vec<(f64, f64)> = match stop_losses_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };

        positions_vec.push(Position {
            id: position.get("id"),
            tg_user_id: position.get("tg_user_id"),
            token_address: position.get("token_address"),
            amount: position.get("amount"),
            mc_entry: position.get("mc_entry"),
            entry_price: position.get("entry_price"),
            created_at: position.get("created_at"),
            chat_id: position.get("chat_id"),
            sol_entry: position.get("sol_entry"),
            ui_amount: position.get("ui_amount"),
            take_profits,
            stop_losses,
            completed: position.get("completed"),
        });
    }
    Ok(positions_vec)
}


/// Gets the position take profits
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<(f64, f64)> representing the take profits
pub async fn get_position_take_profits(pool: &PgPool, token_address: &str, user_tg_id: &str) -> Result<Option<Vec<(f64, f64)>>> {
    let take_profits: Option<serde_json::Value> = sqlx::query_scalar("SELECT take_profits FROM positions WHERE token_address = $1 AND tg_user_id = $2")
    .bind(token_address)
    .bind(user_tg_id)
    .fetch_optional(pool)
    .await?;
    match take_profits {
        Some(json) => Ok(serde_json::from_value(json).unwrap_or_default()),
        None => Ok(None)
    }
}

/// Sets the position take profits
/// 
/// # Description
/// 
/// Sorts the take profits by multiplier (first element of tuple) in ascending order before setting them
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// * `take_profits` - The take profits
/// 
/// # Returns
/// 
/// A result indicating whether the position take profits were set
pub async fn set_position_take_profits(pool: &PgPool, token_address: &str, user_tg_id: &str, take_profits: Vec<(f64, f64)>) -> Result<()> {
    // Sort take_profits by multiplier (first element of tuple) in ascending order
    let mut sorted_take_profits = take_profits;
    sorted_take_profits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let take_profits_json = serde_json::to_value(sorted_take_profits).unwrap();
    sqlx::query("UPDATE positions SET take_profits = $1 WHERE token_address = $2 AND tg_user_id = $3")
    .bind(take_profits_json)
    .bind(token_address)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}


/// Deletes a position take profit
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// * `position` - The position
/// 
/// # Returns
/// 
/// A result indicating whether the position take profit was deleted
pub async fn delete_position_target_reached(pool: &PgPool, token_address: &str, user_tg_id: &str, position: (f64, f64)) -> Result<()> {
    let position_take_profits = get_position_take_profits(pool, token_address, user_tg_id).await?;
    if  position_take_profits.is_some()   {
        if position_take_profits.clone().unwrap().contains(&position) {
            position_take_profits.clone().unwrap().retain(|&tp| tp != position);
            set_position_take_profits(pool, token_address, user_tg_id, position_take_profits.unwrap()).await?;
        }
    }
    Ok(())
}

/// Adds a user take profit if an equal one doesnt exists
/// 
/// # Arguments
/// 
/// * `user_tg_id` - The user's Telegram ID
/// * `take_profit` - The take profit
/// * `pool` - The PostgreSQL connection pool
/// 
/// # Returns
/// 
/// A result indicating whether the user take profit was added
pub async fn add_user_take_profit_user_settings(user_tg_id: &str, take_profit: (f64, f64), pool: &SafePool) -> Result<()> {
    println!("@add_user_take_profit_user_settings/ take_profit: {:?}", take_profit);

    let mut user_take_profits = get_user_settings_take_profits(pool, user_tg_id).await?;
    println!("@add_user_take_profit_user_settings/ user_take_profits: {:?}", user_take_profits);

    if let Some(ref mut profits) = user_take_profits {
        println!("@add_user_take_profit_user_settings/ check if take profit is in user_take_profit");
        if !profits.contains(&take_profit) {
            println!("@add_user_take_profit_user_settings/ take_profit not in user_take_profits, adding");
            profits.push(take_profit);
        }
    } else {
        // If there are no existing take profits, initialize with the new take profit
        user_take_profits = Some(vec![take_profit]);
    }

    println!("@add_user_take_profit_user_settings/ user_take_profits after adding: {:?}", user_take_profits);
    set_user_settings_take_profits(pool, user_tg_id, user_take_profits.clone()).await?;
    println!("@add_user_take_profit_user_settings/ user_take_profits after setting: {:?}", user_take_profits.clone());

    Ok(())
}


/// Adds a user stop loss in user settings
/// 
/// # Arguments
/// 
/// * `user_tg_id` - The user's Telegram ID
/// * `stop_loss` - The stop loss
/// * `pool` - The PostgreSQL connection pool
/// 
/// # Returns
/// 
/// A result indicating whether the user stop loss was added
pub async fn add_user_stop_loss_user_settings(user_tg_id: &str, stop_loss: (f64, f64), pool: &SafePool) -> Result<()> {
    println!("@add_user_stop_loss_user_settings/ stop_loss: {:?}", stop_loss);

    let mut user_stop_losses = get_user_settings_stop_losses(pool, user_tg_id).await?;
    println!("@add_user_stop_loss_user_settings/ user_stop_losses: {:?}", user_stop_losses);
    if let Some(ref mut losses) = user_stop_losses {
        println!("@add_user_stop_loss_user_settings/ check if stop_loss is in user_stop_losses");
        if !losses.contains(&stop_loss) {
            println!("@add_user_stop_loss_user_settings/ stop_loss not in user_stop_losses, adding");
            losses.push(stop_loss);
        }
    } else {
        user_stop_losses = Some(vec![stop_loss]);
    }
    println!("@add_user_stop_loss_user_settings/ user_stop_losses after adding: {:?}", user_stop_losses);
    set_user_settings_stop_losses(pool, user_tg_id, user_stop_losses.clone()).await?;
    println!("@add_user_stop_loss_user_settings/ user_stop_losses after setting: {:?}", user_stop_losses.clone());
    Ok(())
}


/// Sets the user settings stop losses
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `stop_losses` - The stop losses
/// 
/// # Returns
/// 
/// A result indicating whether the user settings stop losses were set
pub async fn set_user_settings_stop_losses(pool: &PgPool, user_tg_id: &str, stop_losses: Option<Vec<(f64, f64)>>) -> Result<()> {
    let stop_losses_json = serde_json::to_value(stop_losses).unwrap_or(json!(null));
    sqlx::query("UPDATE user_settings SET stop_losses = $1 WHERE tg_id = $2")
    .bind(stop_losses_json)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Removes a take profit from a position and sort it by multiplier
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// * `take_profit` - The take profit
/// 
/// # Returns
/// 
/// A result indicating whether the take profit was removed
pub async fn remove_take_profit_from_position(pool: &PgPool, token_address: &str, user_tg_id: &str, take_profit: (f64, f64)) -> Result<()> {
    println!("@remove_take_profit_from_position/ take_profit to remove: {:?}", take_profit);
    let mut position_take_profits = get_position_take_profits(pool, token_address, user_tg_id).await?;
    println!("@remove_take_profit_from_position/ position_take_profits: {:?}", position_take_profits);
    
    if let Some(mut take_profits) = position_take_profits {
        // Remove the take profit
        take_profits.retain(|&tp| tp != take_profit);
        println!("@remove_take_profit_from_position/ position_take_profits after retaining: {:?}", take_profits);
        
        // Sort the take profits
        take_profits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        println!("@remove_take_profit_from_position/ position_take_profits after sorting: {:?}", take_profits);
        
        // Update in database
        println!("@remove_take_profit_from_position/ setting position_take_profits");
        set_position_take_profits(pool, token_address, user_tg_id, take_profits.clone()).await?;
        println!("@remove_take_profit_from_position/ position_take_profits after setting: {:?}", take_profits);
    } 
    
    Ok(())
}

/// Get the postion stop losses
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<(f64, f64)> representing the stop losses
pub async fn get_position_stop_losses(pool: &PgPool, token_address:&str, user_tg_id: &str) ->  Result<Vec<(f64, f64)>> {
    let stop_losses = sqlx::query_scalar(
        "SELECT stop_losses FROM positions WHERE token_address = $1 AND tg_user_id = $2"
    )
    .bind(token_address)
    .bind(user_tg_id)
    .fetch_one(pool)
    .await?;
    match stop_losses {
        Some(json) => Ok(serde_json::from_value(json).unwrap_or_default()),
        None => Ok(Vec::new())
    }
}


/// Sets the position stop losses
/// 
/// # Description
/// 
/// Sorts the stop losses by % loss (first element of tuple) in ascending order before setting them
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// * `take_profits` - The take profits
/// 
/// # Returns
/// 
/// A result indicating whether the position take profits were set
pub async fn set_position_stop_losses(pool: &PgPool, token_address: &str, user_tg_id: &str, take_profits: Vec<(f64, f64)>) -> Result<()> {

    // Sort take_profits by % down (first element of tuple) in descending order
    let mut sorted_stop_losses = take_profits;
    sorted_stop_losses.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    /// Adding to db
    let stop_losses_json = serde_json::to_value(sorted_stop_losses).unwrap();
    sqlx::query("UPDATE positions SET take_profits = $1 WHERE token_address = $2 AND tg_user_id = $3")
    .bind(stop_losses_json)
    .bind(token_address)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_stop_loss_from_position(pool: &PgPool, token_address: &str, user_tg_id: &str, stop_loss: (f64, f64)) -> Result<()> {
    println!("@remove_stop_loss_from_position/ stop_loss to remove: {:?}", stop_loss);
    let mut position_stop_losses = get_position_stop_losses(pool, token_address, user_tg_id).await?;
    println!("@remove_stop_loss_from_position/ position_stop_losses: {:?}", position_stop_losses);


    // Remove the stop loss that matches
    position_stop_losses.retain(|&sl| sl != stop_loss); 
    println!("@remove_stop_loss_from_position/ position_stop_losses after retaining: {:?}", position_stop_losses);

    // Sort by loss % (ascending order)
    position_stop_losses.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!("@remove_stop_loss_from_position/ position_stop_losses after sorting: {:?}", position_stop_losses);

    // Update in db
    println!("@remove_stop_loss_from_position/ setting position_stop_losses");
    set_position_stop_losses(pool, token_address, user_tg_id, position_stop_losses.clone()).await?;
    println!("@remove_stop_loss_from_position/ position_stop_losses after setting: {:?}", position_stop_losses);

    Ok(())
}

/// Deletes a position
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the position was deleted
pub async fn delete_position(pool: &PgPool, token_address: &str, user_tg_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM positions WHERE token_address = $1 AND tg_user_id = $2")
    .bind(token_address)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}


/// Get a position
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the position was deleted
pub async fn get_position(pool: &PgPool, token_address: &str, user_tg_id: &str) -> Result<Position> {
    let query_result = sqlx::query("
    GET * from position WHERE token_address = $1 AND tg_user_id $2")
    .bind(token_address)
    .bind(user_tg_id)
    .fetch_one(pool)
    .await?;
    let take_profits_json: serde_json::Value = query_result.get("take_profits");
    let take_profits = serde_json::from_value(take_profits_json).unwrap_or_default();
    let stop_losses_json: serde_json::Value = query_result.get("stop_losses");
    let stop_losses = serde_json::from_value(stop_losses_json).unwrap_or_default();

    Ok(
        Position { id: query_result.get("id"), tg_user_id: query_result.get("tg_user_id"), token_address: query_result.get("token_address"), amount: query_result.get("amount"), mc_entry: query_result.get("mc_entry"), entry_price: query_result.get("entry_price"), created_at: query_result.get("created_at"), chat_id: query_result.get("chat_id"), sol_entry: query_result.get("sol_entry"), ui_amount: query_result.get("ui_amount"),
        take_profits,
        stop_losses,
        completed: query_result.get("completed")
        }
    )
}


/// Get all positions for a user
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<Position> representing the positions
pub async fn get_positions_by_user_tg_id(pool: &PgPool, user_tg_id: &str) -> Result<Vec<Position>> {
    let positions = sqlx::query("SELECT * FROM positions WHERE tg_user_id = $1")
        .bind(user_tg_id)
        .fetch_all(pool)
        .await?;

    let mut positions_vec = Vec::new();
    for position in positions {
        let take_profits: Vec<(f64, f64)> = position.get::<Option<serde_json::Value>, _>("take_profits")
            .map_or_else(Vec::new, |v| serde_json::from_value(v).unwrap_or_default());
        let stop_losses: Vec<(f64, f64)> = position.get::<Option<serde_json::Value>, _>("stop_losses")
            .map_or_else(Vec::new, |v| serde_json::from_value(v).unwrap_or_default());

        positions_vec.push(Position {
            id: position.get("id"),
            tg_user_id: position.get("tg_user_id"),
            token_address: position.get("token_address"),
            amount: position.get("amount"),
            mc_entry: position.get("mc_entry"),
            entry_price: position.get("entry_price"),
            created_at: position.get("created_at"),
            chat_id: position.get("chat_id"),
            sol_entry: position.get("sol_entry"),
            ui_amount: position.get("ui_amount"),
            take_profits,
            stop_losses,
            completed: position.get("completed")
        });
    }
    Ok(positions_vec)
}


/// Set the user jito tip amount
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `jito_tip_amount` - The Jito tip amount
/// 
/// # Returns
/// 
/// A result indicating whether the user jito tip amount was set
pub async fn set_user_jito_tip_amount(pool: &PgPool, user_tg_id: &str, jito_tip_amount: i32) -> Result<()> {
    println!("@set_user_jito_tip_amount/ jito_tip_amount: {:?}", jito_tip_amount);
    println!("@set_user_jito_tip_amount/ user_tg_id: {:?}", user_tg_id);
    sqlx::query("UPDATE user_settings SET jito_tip_amount = $1 WHERE tg_id = $2")
    .bind(jito_tip_amount)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}


/// Marks a position as completed
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `token_address` - The token address
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the position was marked as completed
pub async fn mark_position_completed(pool: &PgPool, token_address: &str, user_tg_id: &str) -> Result<()> {
    sqlx::query("UPDATE positions SET completed = true WHERE token_address = $1 AND tg_user_id = $2")
    .bind(token_address)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Sets the user settings active complete positions
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `active_complete_positions` - The active complete positions
/// 
/// # Returns
/// 
/// A result indicating whether the user settings active complete positions were set
pub async fn set_user_settings_active_complete_positions(pool: &PgPool, user_tg_id: &str, active_complete_positions: String) -> Result<()> {
    sqlx::query("UPDATE user_settings SET active_complete_positions = $1 WHERE tg_id = $2")
    .bind(active_complete_positions)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the active positions
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<Position> representing the active positions
pub async fn get_active_positions(pool: &PgPool, user_tg_id: &str) -> Result<Vec<Position>> {
    let positions = sqlx::query("SELECT * FROM POSITIONS WHERE tg_user_id = $1 AND completed = false")
    .bind(user_tg_id)
    .fetch_all(pool)
    .await?;
    let mut positions_vec = Vec::new();
    for position in positions {
        let take_profits_value = position.get::<Option<serde_json::Value>, _>("take_profits");
        let stop_losses_value = position.get::<Option<serde_json::Value>, _>("stop_losses");

        let take_profits: Vec<(f64, f64)> = match take_profits_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };

        let stop_losses: Vec<(f64, f64)> = match stop_losses_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };
        positions_vec.push(Position {
            id: position.get("id"),
            tg_user_id: position.get("tg_user_id"),
            token_address: position.get("token_address"),
            amount: position.get("amount"),
            mc_entry: position.get("mc_entry"),
            entry_price: position.get("entry_price"),
            created_at: position.get("created_at"),
            chat_id: position.get("chat_id"),
            sol_entry: position.get("sol_entry"),
            ui_amount: position.get("ui_amount"),
            take_profits,
            stop_losses,
            completed: false
        });
    }
    Ok(positions_vec)
}

/// Get the complete positions
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Vec<Position> representing the complete positions
pub async fn get_complete_positions(pool: &PgPool, user_tg_id: &str) -> Result<Vec<Position>> {
    let positions = sqlx::query("SELECT * FROM POSITIONS WHERE tg_user_id = $1 AND completed = true")
    .bind(user_tg_id)
    .fetch_all(pool)
    .await?;
    let mut positions_vec = Vec::new();
    for position in positions {
        let take_profits_value = position.get::<Option<serde_json::Value>, _>("take_profits");
        let stop_losses_value = position.get::<Option<serde_json::Value>, _>("stop_losses");

        let take_profits: Vec<(f64, f64)> = match take_profits_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };

        let stop_losses: Vec<(f64, f64)> = match stop_losses_value {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => Vec::new(),
        };
        positions_vec.push(Position {
            id: position.get("id"),
            tg_user_id: position.get("tg_user_id"),
            token_address: position.get("token_address"),
            amount: position.get("amount"),
            mc_entry: position.get("mc_entry"),
            entry_price: position.get("entry_price"),
            created_at: position.get("created_at"),
            chat_id: position.get("chat_id"),
            sol_entry: position.get("sol_entry"),
            ui_amount: position.get("ui_amount"),
            take_profits,
            stop_losses,
            completed: false
        });
    }
    Ok(positions_vec)
}


/// Get a refferal
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A Refferal struct representing the refferal
pub async fn get_refferal(pool: &PgPool, user_tg_id: &str) -> Result<Option<Refferal>> {
    let refferal = sqlx::query("SELECT * FROM refferals WHERE user_tg_id = $1")
    .bind(user_tg_id)
    .fetch_one(pool)
    .await?;

    Ok(Some(Refferal{
        id: refferal.get("id"),
        user_tg_id: refferal.get("user_tg_id"),
        uuid: refferal.get("uuid"),
        users_referred: refferal.get("users_referred"),
        referral_rebates: refferal.get("referral_rebates"),
        total_rewards: refferal.get("total_rewards")
    }))
}

/// Set the refferal users referred
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `users_referred` - The number of users referred
/// 
/// # Returns
/// 
/// A result indicating whether the refferal users referred was set
pub async fn set_refferal_users_referred(pool: &PgPool, user_tg_id: &str, users_referred: i32) -> Result<()> {
    sqlx::query("UPDATE refferals SET users_referred = $1 WHERE user_tg_id = $2")
    .bind(users_referred)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set the refferal referral rebates
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `referral_rebates` - The referral rebates
/// 
/// # Returns
/// 
/// A result indicating whether the refferal referral rebates was set
pub async fn set_refferal_referral_rebates(pool: &PgPool, user_tg_id: &str, referral_rebates: i32) -> Result<()> {
    sqlx::query("UPDATE refferals SET referral_rebates = $1 WHERE user_tg_id = $2")
    .bind(referral_rebates)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set the refferal total rewards
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// * `total_rewards` - The total rewards
/// 
/// # Returns
/// 
/// A result indicating whether the refferal total rewards was set
pub async fn set_refferal_total_rewards(pool: &PgPool, user_tg_id: &str, total_rewards: String) -> Result<()> {
    sqlx::query("UPDATE refferals SET total_rewards = $1 WHERE user_tg_id = $2")
    .bind(total_rewards)
    .bind(user_tg_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Create a refferal
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A result indicating whether the refferal was created
pub async fn create_refferal(pool: &PgPool, user_tg_id: &str) -> Result<()> {
    sqlx::query("INSERT INTO refferals (user_tg_id, uuid) VALUES ($1, $2)")
    .bind(user_tg_id)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Check if a user has a refferal
/// 
/// # Arguments
/// 
/// * `pool` - The PostgreSQL connection pool
/// * `user_tg_id` - The user's Telegram ID
/// 
/// # Returns
/// 
/// A boolean indicating whether the user has a refferal
pub async fn check_user_has_refferal(pool: &PgPool, user_tg_id: &str) -> Result<bool> {
    let refferal = get_refferal(pool, user_tg_id).await?;
    Ok(refferal.is_some())
}