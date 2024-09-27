use anyhow::Result;
use redis::Client;
use redis::Connection;

pub async fn get_redis_connection() -> Result<Connection> {
    let client = Client::open("redis://redis:6379")?;
    let connection = client.get_connection()?;
    Ok(connection)
}

