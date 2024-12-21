use anyhow::Result;
use redis::{Commands, Connection};

use crate::models::copy_trade::CopyTradeWallet;

/// Get a connection to Redis
///
/// # Returns
/// - `Connection`: The Redis connection
pub fn get_redis_connection() -> Connection {
    // Connect to Redis using the container hostname (default port is 6379)
    let redis_client = redis::Client::open("redis://telegram_app_redis:6379")
        .expect("Couldn't create redis client.");
    let con = redis_client
        .get_connection()
        .expect("Couldn't create connection to redis.");

    con
}

/// Fetch copy trade wallets from Redis
///
/// # Parameters
/// - `conn`: &mut redis::Connection - Mutable reference to the Redis connection
///
/// # Returns
/// - `Result<Vec<CopyTradeWallet>>`: A vector of CopyTradeWallet structs or an error
pub fn get_copy_trade_wallets(conn: &mut redis::Connection) -> Result<Vec<CopyTradeWallet>> {
    let pattern = "user:*:copy_trade_wallet:*";
    let mut cursor = 0;
    let mut keys = Vec::new();
    let mut copy_trade_wallets: Vec<CopyTradeWallet> = Vec::new();

    // Scan Redis for matching keys
    loop {
        let (new_cursor, mut result): (i64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .query(conn)?;

        keys.append(&mut result);
        cursor = new_cursor;

        if cursor == 0 {
            break;
        }
    }

    // Process each key and create CopyTradeWallet structs
    for key in keys {
        let copy_trade_address: String = conn.hget(key.clone(), "copy_trade_address").unwrap();
        let status: String = conn.hget(key.clone(), "status").unwrap();
        let account_address: String = conn.hget(key.clone(), "account_address").unwrap();
        let buy_amount: f64 = conn.hget(key.clone(), "buy_amount").unwrap();

        let wallet = CopyTradeWallet {
            copy_trade_address: copy_trade_address.clone(),
            account_address: account_address.clone(),
            buy_amount,
            status: status == "active",
        };
        copy_trade_wallets.push(wallet);
    }

    Ok(copy_trade_wallets)
}
