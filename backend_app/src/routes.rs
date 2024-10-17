use crate::utils::helpers::{get_redis_connection, get_ath, time_to_timestamp};
use serde_json::Value;
use crate::db::*;
use std::collections::HashMap;
use redis::Commands;
use serde::{Serialize, Deserialize};


pub async fn index(_req: tide::Request<()>) -> tide::Result<String> {
    Ok("Running!".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CopyTradeWalletPost {
    pub user_id: String,
    pub wallet_id: String,
    pub account_address: String,
    pub buy_amount: String,
    pub copy_trade_address: String,
    pub status: String,
}

pub async fn set_copy_trade_wallet(mut req: tide::Request<()>) -> tide::Result<String> {
    let post: CopyTradeWalletPost = req.body_json().await?;
    let mut con = get_redis_connection().await?;
    
    let key = format!("user:{}:copy_trade_wallet:{}", &post.user_id, &post.copy_trade_address);
    con.hset_multiple(key, &[
        ("user_id", &post.user_id),
        ("wallet_id", &post.wallet_id),
        ("account_address", &post.account_address),
        ("buy_amount", &post.buy_amount),
        ("status", &post.status),
        ("copy_trade_address", &post.copy_trade_address),
        ])?;
        
    let response = surf::get("http://solana_app:3030/resubscribe").recv_string().await;
    match response {
        Ok(_) => {
            println!("Subscribed");
        }
        Err(e) => {
            eprintln!("Failed to update Solana app: {}", e);
            return Ok("Failed to update Solana app".to_string());
        }
    }
    Ok(serde_json::to_string(&post)?)
}

pub async fn delete_copy_trade_wallet(req: tide::Request<()>) -> tide::Result<String> {
    let user_id = req.param("user_id")?;
    let copy_trade_address = req.param("copy_trade_address")?;
    let mut con = get_redis_connection().await?;

    let key = format!("user:{}:copy_trade_wallet:{}", user_id, copy_trade_address);
    con.del(key)?;

    let response = surf::get("http://solana_app:3030/resubscribe").recv_string().await;
    match response {
        Ok(_) => {
            println!("Unsubscribed");
        }
        Err(e) => {
            eprintln!("Failed to update Solana app: {}", e);
            return Ok("Failed to update Solana app".to_string());
        }
    }
    Ok("Copy trade wallet deleted".to_string())
}

pub async fn get_copy_trades(req: tide::Request<()>) -> tide::Result<String> {
    let user_id = req.param("user_id")?;
    let mut con = get_redis_connection().await?;

    let copy_trades_keys: Vec<String> = con.keys(format!("user:{}:copy_trade_wallet:*", user_id))?;
    let mut copy_trades = Vec::new();
    for key in &copy_trades_keys {
        let copy_trade_data: HashMap<String, String> = con.hgetall(key)?;
        copy_trades.push(copy_trade_data);
    }

    Ok(serde_json::to_string(&copy_trades)?)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSessionPost {
    pub user_id: String,
    pub session_end_time: String,
    pub public_key: String,
    pub private_key: String,
}

/// Set user session
/// 
/// # Arguments
/// 
/// * `req` - The request object
/// 
/// # Returns
/// 
/// * `String` - The response message
pub async fn set_user_session(mut req: tide::Request<()>) -> tide::Result<String> {
    let post: UserSessionPost = match req.body_json().await {
        Ok(post) => post,
        Err(e) => {
            eprintln!("Failed to parse request body: {:?}", e);
            return Err(tide::Error::from_str(422, "Invalid request body"));
        }
    };
    
    let mut con = match get_redis_connection().await {
        Ok(con) => con,
        Err(e) => {
            eprintln!("Failed to get Redis connection: {:?}", e);
            return Err(tide::Error::from_str(500, "Internal server error"));
        }
    };

    let key = format!("user:{}:session", &post.user_id);
    con.hset_multiple(key, &[
        ("user_id", &post.user_id),
        ("session_end_time", &post.session_end_time),
        ("public_key", &post.public_key),
        ("private_key", &post.private_key),
    ])?;
    Ok("Session set".to_string())
}

#[derive(Debug, Serialize)]
pub struct CallWithAth {
    pub call: Call,
    pub multiplier: f64,
    pub ath: f64,
}

/// Get user calls with ATH
/// 
/// # Arguments
/// 
/// * `req` - The request object
/// 
/// # Returns
/// 
/// * `String` - A json string with the calls and the ATH
pub async fn get_user_calls(req: tide::Request<()>) -> tide::Result<String> {
    let user_tg_id = req.param("user_tg_id")?;
    let mut con = get_db_connection();
    let calls_without_ath = get_all_user_firsts_calls_by_user_tg_id(&mut con, user_tg_id);
    let mut calls_with_ath = Vec::new();
    for call in calls_without_ath {
        let ath = get_ath(time_to_timestamp(&call.time).await, &call.token_address).await?;
        let ath_price = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let multiplier = ath_price / call.price.parse::<f64>().unwrap_or(0.0);
        let call_with_ath = CallWithAth {
            call,
            multiplier,
            ath: ath_price,
        };
        calls_with_ath.push(call_with_ath);
    }
    Ok(serde_json::to_string(&calls_with_ath)?)
}