use sqlite::Connection;
use std::sync::{Arc, Mutex};
use sqlite::State;
use anyhow::Result;
pub fn get_connection() -> Connection {
    sqlite::open("db.sqlite").unwrap()
}
use crate::utils::helpers::check_period_for_leaderboard;
pub fn configure_db(connection: &Connection) {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tg_id TEXT NOT NULL UNIQUE,
            username TEXT
        );"
    ).unwrap();

    connection.execute(
        "CREATE TABLE IF NOT EXISTS calls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            mkt_cap TEXT,
            token_address TEXT,
            token_mint TEXT,
            token_symbol TEXT,
            price TEXT,
            user_tg_id INTEGER,
            chat_id TEXT,
            message_id TEXT,
            chain TEXT,
            FOREIGN KEY (user_tg_id) REFERENCES users (tg_id)
        );"
    ).unwrap();
}


#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub tg_id: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Call {
    pub id: u64,
    pub time: String,
    pub mkt_cap: String,
    pub price: String,
    pub token_address: String,
    pub token_mint: String,
    pub token_symbol: String,
    pub user_tg_id: String,
    pub chat_id: String,
    pub message_id: String,
    pub chain: String,
}

/// Get a user by telegram id
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `tg_id` - The user's telegram id
/// 
/// # Returns
/// 
/// An optional user
pub fn get_user(connection: &Connection, tg_id: &str) -> Option<User> {
    let query = "SELECT * FROM users WHERE tg_id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, tg_id)).unwrap();
    
    if let Ok(State::Row) = stmt.next() {
        Some(User {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            username: stmt.read::<String, _>("username").unwrap(),
            tg_id: stmt.read::<String, _>("tg_id").unwrap(),
        })
    } else {
        None
    }
}

/// Add a user to the database
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `tg_id` - The user's telegram id
/// * `username` - The user's username
/// 
/// # Returns
/// 
/// An empty result
pub fn add_user(connection: &Connection, tg_id: &str, username: &str) -> Result<()> {
    let query = "INSERT INTO users (tg_id, username) VALUES (?, ?)";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, tg_id)).unwrap();
    stmt.bind((2, username)).unwrap();
    stmt.next().unwrap();
    Ok(())
}


/// Add a call to the database
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `tg_id` - The user's telegram id
/// * `mkt_cap` - The market cap of the token
/// * `token_address` - The token address
/// * `token_mint` - The token mint
/// * `token_symbol` - The token symbol
/// * `price` - The price of the token
/// * `chat_id` - The chat id
/// * `message_id` - The message id
/// * `chain` - The chain of the token
/// 
/// # Returns
/// 
/// The id of the call
pub fn add_call(
    connection: &Connection, 
    tg_id: &str, 
    mkt_cap: &str, 
    token_address: &str, 
    token_mint: &str,
    token_symbol: &str,
    price: &str, 
    chat_id: &str,
    message_id: &str,
    chain: &str
) -> Result<u64> {
    let query = "INSERT INTO calls (user_tg_id, mkt_cap, token_address, token_mint, token_symbol, price, chat_id, message_id, chain) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
    
    // Prepare and execute the INSERT statement
    let mut stmt = connection.prepare(query)?;
    stmt.bind((1, tg_id)).unwrap();
    stmt.bind((2, mkt_cap)).unwrap();
    stmt.bind((3, token_address)).unwrap();
    stmt.bind((4, token_mint)).unwrap();
    stmt.bind((5, token_symbol)).unwrap();
    stmt.bind((6, price)).unwrap();
    stmt.bind((7, chat_id)).unwrap();
    stmt.bind((8, message_id)).unwrap();
    stmt.bind((9, chain)).unwrap();
    stmt.next().unwrap();  // Execute the insert

    // Query to get the last inserted row ID using SQLite's built-in function
    let mut stmt = connection.prepare("SELECT last_insert_rowid()")?;
    stmt.next()?;  // Move to the result row
    
    // Get the last inserted row ID
    let row_id: i64 = stmt.read(0)?;

    Ok(row_id as u64)
}

/// Get the first call of a token in a chat
/// 
/// # Arguments
/// * `connection` - The database connection
/// * `token_address` - The token address
/// * `chat_id` - The chat id
/// 
/// # Returns
/// 
/// An optional first call
pub fn get_first_call_by_token_address(connection: &Connection, token_address: &str, chat_id: &str) -> Option<Call> {
    let query = "SELECT * FROM calls WHERE token_address = ? AND chat_id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, token_address)).unwrap();
    stmt.bind((2, chat_id)).unwrap();
    if let Ok(State::Row) = stmt.next() {
        Some(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        })
    } else {
        None
    }
}

/// Get a call by id
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `id` - The call id
/// 
/// # Returns
///
/// An optional call
pub fn get_call_by_id(connection: &Connection, id: u64) -> Option<Call> {
    let query = "SELECT * FROM calls WHERE id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, id as i64)).unwrap();
    
    if let Ok(State::Row) = stmt.next() {
        Some(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        })
    } else {
        None
    }
}

/// Get all calls made in a channel
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `chat_id` - The chat id
/// 
/// # Returns
///
/// A vector of calls
pub fn get_all_calls_chat_id(connection: &Connection, chat_id: &str) -> Vec<Call> {
    let query = "SELECT * FROM calls WHERE chat_id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, chat_id)).unwrap();
    
    let mut calls = Vec::new();
    while let Ok(State::Row) = stmt.next() {
        calls.push(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        });
    }
    calls
}

/// Get all calls made in a channel in the last x days
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `chat_id` - The chat id
/// * `days` - The number of days to get the calls
/// 
/// # Returns
/// 
/// A vector of calls
pub fn get_channel_calls_last_x_days(connection: &Connection, chat_id: &str, days: u32) -> Vec<Call> {
    let query = format!("
        SELECT * FROM calls 
        WHERE time >= datetime('now', '-{days} day') AND chat_id = ?
    ");
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, chat_id)).unwrap();
    let mut calls = Vec::new();
    while let Ok(State::Row) = stmt.next() {
        calls.push(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        });
    }
    calls
}

/// Get all calls made in a channel in the last x hours
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `chat_id` - The chat id
/// * `hours` - The number of hours to get the calls
/// 
/// # Returns
/// 
/// A vector of calls
pub fn get_channel_calls_last_x_hours(connection: &Connection, chat_id: &str, hours: u32) -> Vec<Call> {
    let query = format!("
        SELECT * FROM calls 
        WHERE time >= datetime('now', '-{hours} hour') AND chat_id = ?
    ");
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, chat_id)).unwrap();
    let mut calls = Vec::new();
    while let Ok(State::Row) = stmt.next() {
        calls.push(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        });
    }
    calls
}

/// Get all calls made in a channel in the last x months
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `chat_id` - The chat id
/// * `months` - The number of months to get the calls
/// 
/// # Returns
/// 
/// A vector of calls
pub fn get_channel_calls_last_x_months(connection: &Connection, chat_id: &str, months: u32) -> Vec<Call> {
    let query = format!("
        SELECT * FROM calls 
        WHERE time >= datetime('now', '-{months} month') AND chat_id = ?
    ");
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, chat_id)).unwrap();
    let mut calls = Vec::new();
    while let Ok(State::Row) = stmt.next() {
        calls.push(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        });
    }
    calls
}

/// Get all calls made in a channel in the last x years
///
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `chat_id` - The chat id
/// * `years` - The number of years to get the calls
/// 
/// # Returns
/// 
/// A vector of calls
pub fn get_channel_calls_last_x_years(connection: &Connection, chat_id: &str, years: u32) -> Vec<Call> {
    let query = format!("
        SELECT * FROM calls 
        WHERE time >= datetime('now', '-{years} year') AND chat_id = ?
    ");
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, chat_id)).unwrap();
    let mut calls = Vec::new();
    while let Ok(State::Row) = stmt.next() {
        calls.push(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        });
    }
    calls
}




/// Get all calls made by a user
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `user_tg_id` - The user's telegram id
/// 
/// # Returns
/// 
/// A vector of calls
pub fn get_all_calls_user_tg_id(connection: &Connection, user_tg_id: &str) -> Vec<Call> {
    let query = "SELECT * FROM calls WHERE user_tg_id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, user_tg_id)).unwrap();
    let mut calls = Vec::new();
    while let Ok(State::Row) = stmt.next() {
        calls.push(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        });
    }
    calls
}

/// Check if a call was already made in a chat
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `token_address` - The token address
/// * `chat_id` - The chat id
/// 
/// # Returns
// 
/// A boolean indicating if the call was already made
pub fn is_first_call(connection: &Arc<Mutex<Connection>>, token_address: &str, chat_id: &str) -> bool {
    // Define the query
    let query = "SELECT COUNT(*) FROM calls WHERE token_address = ? AND chat_id = ?";

    // Prepare the statement
    let conn = connection.lock().unwrap();
    let mut stmt = conn.prepare(query).unwrap();

    // Bind the parameters
    stmt.bind((1, token_address)).unwrap();
    stmt.bind((2, chat_id)).unwrap();

    // Execute the query and check if it's the first call
    let result = stmt.next().unwrap();
    
    if result == sqlite::State::Row {
        let count: i64 = stmt.read::<i64, _>(0).unwrap();
        count == 0  // Check if the count is 0 for the first call
    } else {
        false
    }
}


/// Get the first call of a token in a chat
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `token_address` - The token address
/// * `chat_id` - The chat id
/// 
/// # Returns
/// 
/// An optional call
pub fn get_first_call_token_chat(connection: &Connection, token_address: &str, chat_id: &str) -> Option<Call> {
    let query = "SELECT * FROM calls WHERE token_address = ? AND chat_id = ? ORDER BY time ASC LIMIT 1";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, token_address)).unwrap();
    stmt.bind((2, chat_id)).unwrap();
    if let Ok(State::Row) = stmt.next() {
        Some(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_mint: stmt.read::<String, _>("token_mint").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
            message_id: stmt.read::<String, _>("message_id").unwrap(),
            chain: stmt.read::<String, _>("chain").unwrap(),
        })
    } else {
        None
    }
}

/// Delete a call from the database
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `call_id` - The call id
/// 
/// # Returns
/// 
/// Ok(()) if the call was deleted
pub fn delete_call(connection: &Connection, call_id: u64) -> Result<()> {
    let query = "DELETE FROM calls WHERE id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, call_id as i64)).unwrap();
    stmt.next().unwrap();
    Ok(())
}

/// Get the number of calls a user made in the last 24 hours
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `user_tg_id` - The user's telegram id
/// 
/// # Returns
/// 
/// The number of calls made by the user in the last 24 hours
pub fn get_qtd_calls_user_made_in_24hrs(connection: &Arc<Mutex<Connection>>, user_tg_id: &str) -> usize {
    let conn = connection.lock().unwrap();
    let query = "SELECT COUNT(*) FROM calls WHERE user_tg_id = ? AND time >= datetime('now', '-24 hour')";
    let mut stmt = conn.prepare(query).unwrap();
    stmt.bind((1, user_tg_id)).unwrap();
    if let Ok(State::Row) = stmt.next() {
        stmt.read::<i64, _>(0).unwrap() as usize
    } else {
        0
    }
}


/// Get the user from a call by the call_id
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `call_id` - The call id
/// 
/// # Returns
/// 
/// An optional user
pub fn get_user_from_call(connection: &Connection, call_id: &str) -> Option<User> {
    let query = "SELECT users.id, users.username, users.tg_id 
                 FROM calls 
                 JOIN users ON calls.user_tg_id = users.tg_id 
                 WHERE calls.id = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, call_id)).unwrap();
    if let Ok(State::Row) = stmt.next() {
        Some(User {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            username: stmt.read::<String, _>("username").unwrap(),
            tg_id: stmt.read::<String, _>("tg_id").unwrap(),
        })
    } else {
        None
    }
}


/// Get the user call count for a user
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `user_tg_id` - The user's telegram id
/// * `chat_id` - The chat id
/// * `period` - The period to get the call count
/// 
/// # Returns
/// 
/// The number of calls made by the user in the last period
pub fn get_user_call_count_for_user_chat_with_period(connection: &Connection, user_tg_id: &str, chat_id: &str, period: &str) -> usize {
    let (number, unit) = match check_period_for_leaderboard(period) {
        Some(p) => p,
        None => return 0, // Invalid period
    };

    let time_expr = match unit {
        "h" => format!("-{} hours", number),
        "d" => format!("-{} days", number),
        "w" => format!("-{} weeks", number),
        "y" => format!("-{} years", number),
        _ => return 0, // Invalid unit
    };

    let query = "SELECT COUNT(DISTINCT token_symbol) 
        FROM calls 
        WHERE user_tg_id = ? 
        AND chat_id = ? 
        AND datetime(time) >= datetime('now', ?)";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, user_tg_id)).unwrap();
    stmt.bind((2, chat_id)).unwrap();
    stmt.bind((3, time_expr.as_str())).unwrap();


    if let Ok(State::Row) = stmt.next() {
        stmt.read::<i64, _>(0).unwrap() as usize
    } else {
        0
    }
}

/// Get the number of calls in a chat in the last period
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `chat_id` - The chat id
/// * `period` - The period to get the call count
/// 
/// # Returns
/// 
/// The number of calls made in the last period
pub fn get_chat_call_count_with_period(connection: &Connection, chat_id: &str, period: &str) -> usize {
    let (number, unit) = match check_period_for_leaderboard(period) {
        Some(p) => p,
        None => return 0, // Invalid period
    };
    let time_expr = match unit {
        "h" => format!("-{} hours", number),
        "d" => format!("-{} days", number),
        "w" => format!("-{} weeks", number),
        "y" => format!("-{} years", number),
        _ => return 0, // Invalid unit
    };
    let query = "SELECT COUNT(*) FROM calls WHERE chat_id = ? AND datetime(time) >= datetime('now', ?)";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, chat_id)).unwrap();
    stmt.bind((2, time_expr.as_str())).unwrap();
    if let Ok(State::Row) = stmt.next() {
        stmt.read::<i64, _>(0).unwrap() as usize
    } else {
        0
    }
}