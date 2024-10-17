use anyhow::Result;
use chrono::{NaiveDateTime, DateTime, Utc};
use serde_json::Value;
use redis::Client as RedisClient;
use redis::Connection;
use reqwest::Client;

pub async fn get_redis_connection() -> Result<Connection> {
    let client = RedisClient::open("redis://redis:6379")?;
    let connection = client.get_connection()?;
    Ok(connection)
}




/// Get the ATH of a token
/// 
/// # Arguments
/// 
/// * `timestamp` - The timestamp to get the ATH for
/// * `token_address` - The address of the token
/// 
/// # Returns
/// 
/// A JSON object containing the ATH
pub async fn get_ath(timestamp: i64, token_address: &str) -> Result<Value> {
    let url = format!("https://api-rs.dexcelerate.com/token/SOL/{}/ath?timestamp={}", token_address, timestamp);
    let client = Client::new();
    let response = client.get(url)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    Ok(json)
}

/// Convert a time string to a timestamp
/// 
/// # Arguments
/// 
/// * `time` - The time string
/// 
/// # Returns
/// 
/// An i64 timestamp
pub async fn time_to_timestamp(time: &str) -> i64 {
    let format = "%Y-%m-%d %H:%M:%S";
    let naive_datetime = NaiveDateTime::parse_from_str(time, format)
        .expect("Failed to parse datetime.");
    let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    datetime.timestamp_millis()
}