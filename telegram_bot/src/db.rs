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


pub fn add_call(connection: &Connection, tg_id: &str, mkt_cap: &str, token_address: &str, token_symbol: &str, price: &str, chat_id: &str) -> Result<()> {
    let query = "INSERT INTO calls (user_tg_id, mkt_cap, token_address, token_symbol, price, chat_id) VALUES (?, ?, ?, ?, ?, ?)";
    let mut stmt = connection.prepare(query).unwrap();
    stmt.bind((1, tg_id)).unwrap();
    stmt.bind((2, mkt_cap)).unwrap();
    stmt.bind((3, token_address)).unwrap();
    stmt.bind((4, token_symbol)).unwrap();
    stmt.bind((5, price)).unwrap();
    stmt.bind((6, chat_id)).unwrap();
    stmt.next().unwrap();
    Ok(())
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
