use crate::utils::helpers::get_redis_connection;
use std::collections::HashMap;
use redis::Commands;
use serde::{Serialize, Deserialize};
use anyhow::Result;


pub async fn index(_req: tide::Request<()>) -> tide::Result<String> {
    Ok("Running!".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub language_code: String,
    pub allows_write_to_pm: bool,
    pub wallets_id: Option<Vec<String>>,
}


pub async fn check_user_exists(user_id: &str) -> Result<bool> {
    let mut con = get_redis_connection().await?;
    let user_exists: bool = con.sismember("users", format!("user:{}", user_id))?;
    Ok(user_exists)
}

pub async fn add_or_update_user(mut req: tide::Request<()>) -> tide::Result<String> {
    let mut con = get_redis_connection().await?;
    let user: User = req.body_json().await?;

    // Store user as hash
    con.hset_multiple(
        format!("user:{}", user.id),
        &[
            ("username", &user.username),
            ("first_name", &user.first_name),
            ("last_name", &user.last_name),
            ("language_code", &user.language_code),
            ("allows_write_to_pm", &user.allows_write_to_pm.to_string()),
            ("wallets_id", &user.wallets_id.unwrap_or(vec![]).join(",")),
        ],
    )?;
    println!("user:{} touched", user.id);    

    // Add user ID to the set of all users
    con.sadd("users", format!("user:{}", user.id))?;

    Ok("User added or updated successfully".to_string())
}

pub async fn get_user(req: tide::Request<()>) -> tide::Result<String> {
    println!("get_user");
    let user_id = req.param("id")?;
    let mut con = get_redis_connection().await?;
    let user_hash: HashMap<String, String> = con.hgetall(format!("user:{}", user_id))?;
    println!("user_json: {:?}", user_hash);
    if user_hash.is_empty() {
        return Ok("User not found".to_string());
    }

    let user = User {
        id: user_id.to_string(),
        first_name: user_hash.get("first_name").unwrap().to_string(),
        last_name: user_hash.get("last_name").unwrap().to_string(),
        username: user_hash.get("username").unwrap().to_string(),
        language_code: user_hash.get("language_code").unwrap().to_string(),
        allows_write_to_pm: user_hash.get("allows_write_to_pm").unwrap().to_string().parse::<bool>().unwrap_or(false),
        wallets_id: user_hash.get("wallets_id").map(|w| vec![w.to_string()]),
    };

    Ok(serde_json::to_string(&user)?)
}


pub async fn get_all_users(_req: tide::Request<()>) -> tide::Result<String> {
    println!("get_all_users_id");
    let mut con = get_redis_connection().await?;
    
    // Get all user IDs from the set
    let user_ids: Vec<String> = con.smembers("users")?;
    println!("user_ids: {}", user_ids.len());
    Ok(serde_json::to_string(&user_ids)?)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletPost {
    pub user_id: String,
    pub wallet_id: String,
    pub wallet_name: String,
    pub sol_address: String,
}

pub async fn add_wallet_to_user(mut req: tide::Request<()>) -> tide::Result<String> {
    let post: WalletPost = req.body_json().await?;

    let mut con = get_redis_connection().await?;

    let key = format!("user:{}:wallet_id:{}", post.user_id, post.wallet_id);
    con.hset_multiple (key, &[
        ("wallet_id", post.wallet_id),
        ("sol_address", post.sol_address),
        ("wallet_name", post.wallet_name),
    ])?;

    Ok("Wallet added to user".to_string())
}

