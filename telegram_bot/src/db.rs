use sqlite::Connection;
use sqlite::State;
use anyhow::Result;
pub fn get_connection() -> Connection {
    sqlite::open("db.sqlite").unwrap()
}

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
            token_symbol TEXT,
            price TEXT,
            user_tg_id INTEGER,
            chat_id TEXT,
            FOREIGN KEY (user_tg_id) REFERENCES users (tg_id)
        );"
    ).unwrap();
}


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
    pub token_symbol: String,
    pub user_tg_id: String,
    pub chat_id: String,
}

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

pub fn add_user(connection: &Connection, tg_id: &str, username: &str) -> Result<()> {
    let query = "INSERT INTO users (tg_id, username) VALUES (?, ?)";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, tg_id)).unwrap();
    stmt.bind((2, username)).unwrap();
    stmt.next().unwrap();
    Ok(())
}


pub fn add_call(
    connection: &Connection, 
    tg_id: &str, 
    mkt_cap: &str, 
    token_address: &str, 
    token_symbol: &str, 
    price: &str, 
    chat_id: &str
) -> Result<u64> {
    let query = "INSERT INTO calls (user_tg_id, mkt_cap, token_address, token_symbol, price, chat_id) VALUES (?, ?, ?, ?, ?, ?)";
    
    // Prepare and execute the INSERT statement
    let mut stmt = connection.prepare(query)?;
    stmt.bind((1, tg_id)).unwrap();
    stmt.bind((2, mkt_cap)).unwrap();
    stmt.bind((3, token_address)).unwrap();
    stmt.bind((4, token_symbol)).unwrap();
    stmt.bind((5, price)).unwrap();
    stmt.bind((6, chat_id)).unwrap();
    stmt.next().unwrap();  // Execute the insert

    // Query to get the last inserted row ID using SQLite's built-in function
    let mut stmt = connection.prepare("SELECT last_insert_rowid()")?;
    stmt.next()?;  // Move to the result row
    
    // Get the last inserted row ID
    let row_id: i64 = stmt.read(0)?;

    Ok(row_id as u64)
}

pub fn get_call(connection: &Connection, token_address: &str, chat_id: &str) -> Option<Call> {
    let query = "SELECT * FROM calls WHERE token_address = ?";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, token_address)).unwrap();
    
    if let Ok(State::Row) = stmt.next() {
        Some(Call {
            id: stmt.read::<i64, _>("id").unwrap() as u64,
            time: stmt.read::<String, _>("time").unwrap(),
            mkt_cap: stmt.read::<String, _>("mkt_cap").unwrap(),
            token_address: stmt.read::<String, _>("token_address").unwrap(),
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            price: stmt.read::<String, _>("price").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
        })
    } else {
        None
    }
}

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
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
        })
    } else {
        None
    }
}


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
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
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
pub fn get_channel_calls_last_x_days(connection: &Connection, chat_id: &str, days: i32) -> Vec<Call> {
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
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
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
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
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
pub fn is_first_call(connection: &Connection, token_address: &str, chat_id: &str) -> bool {
    // Define the query
    let query = "SELECT COUNT(*) FROM calls WHERE token_address = ? AND chat_id = ?";

    // Prepare the statement
    let mut stmt = connection.prepare(query).unwrap();

    // Bind the parameters
    stmt.bind((1, token_address)).unwrap();
    stmt.bind((2, chat_id)).unwrap();

    // Execute the query and check if it's the first call
    let result = stmt.next().unwrap();
    
    if result == sqlite::State::Row {
        let count: i64 = stmt.read::<i64, _>(0).unwrap();
        count == 1
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
            token_symbol: stmt.read::<String, _>("token_symbol").unwrap(),
            user_tg_id: stmt.read::<String, _>("user_tg_id").unwrap(),
            chat_id: stmt.read::<String, _>("chat_id").unwrap(),
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
pub fn get_qtd_calls_user_made_in_24hrs(connection: &Connection, user_tg_id: &str) -> usize {
    let query = "SELECT COUNT(*) FROM calls WHERE user_tg_id = ? AND time >= datetime('now', '-24 hour')";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, user_tg_id)).unwrap();
    if let Ok(State::Row) = stmt.next() {
        stmt.read::<i64, _>(0).unwrap() as usize
    } else {
        0
    }
}
