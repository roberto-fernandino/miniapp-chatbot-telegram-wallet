use sqlx::{PgPool, postgres::PgPoolOptions};
use sqlx::{Encode, Pool};
use sqlx::Postgres;
use crate::handlers::PostUserRequest;
use crate::utils::helpers::check_period_for_leaderboard;
use serde::Serialize;
use sqlx::Row;
use anyhow::Result;
use std::env;
use std::sync::Arc;

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
    pub api_public_key: String,
    pub api_private_key: String,
    pub suborg_id: String,
    pub wallet_id: String,
}

/// Represents a user in the system.
#[derive(Debug, Serialize, Clone)]
pub struct User {
    pub id: i32,
    pub username: Option<String>,
    pub tg_id: String,
    pub turnkey_info: TurnkeyInfo,
    pub solana_address: String,
    pub eth_address: String,
}

/// Represents a call in the system.
#[derive(Debug, Serialize, Clone)]
pub struct Call {
    pub id: i32,
    pub time: String,
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
    let q = "INSERT INTO users (tg_id, username) VALUES ($1, $2) ON CONFLICT (tg_id) DO NOTHING";

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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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
        WHERE user_tg_id = $1 AND time >= NOW() - INTERVAL '24 hours'
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
            api_public_key: user.get("api_public_key"),
            api_private_key: user.get("api_private_key"),
            suborg_id: user.get("suborg_id"),
            wallet_id: user.get("wallet_id"),
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

    let time_str: &str = call.get("time");
    Ok(Call {
        id: call.get("id"),
        time: time_str.to_string(),
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
pub async fn get_all_calls_user_tg_id(pool: &PgPool, user_tg_id: &str) -> Result<Vec<Call>> {
    let query = "SELECT id, time, mkt_cap, price, token_address, token_mint, token_symbol, user_tg_id, chat_id, message_id, chain FROM calls WHERE user_tg_id = $1";
    let calls = sqlx::query(query)
        .bind(user_tg_id)
        .fetch_all(pool)
        .await?;
    
    let mut calls_vec: Vec<Call> = Vec::new();
    for call in calls {
        let time_str: &str = call.get("time");
        calls_vec.push(Call {
            id: call.get("id"),
            time: time_str.to_string(),
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


pub async fn get_user_id_by_tg_id(pool: &PgPool, tg_id: &str) -> Result<i32> {
    let id: i32 = sqlx::query_scalar("SELECT id FROM users WHERE tg_id = $1")
    .bind(tg_id)
    .fetch_one(pool)
    .await?;
    Ok(id)
}   



pub async fn add_user(pool: &PgPool, post_user_request: PostUserRequest) -> Result<()> {
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