use commands::{execute_swap, execute_swap_take_profit};
use teloxide::prelude::*;
use tungstenite::Message as WsMessage;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio_tungstenite::connect_async;
use teloxide::{dispatching::UpdateFilterExt, Bot};
use utils::helpers::check_raydiums_tokens;
use std::sync::Arc;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::postgres::PgPoolOptions;
use crate::commands::run_axum_server;
use crate::handlers::{handle_message, handle_callback_query};
mod utils;
mod handlers;
mod db;
mod commands;
pub type SafePool = Arc<Pool<Postgres>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    // Initialize the PostgreSQL connection pool.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL is not set"))
        .await
        .expect("Failed to create pool");

    /// Axum server
    let shared_pool = Arc::new(pool);
    // Spawn the Tide server on a separate task using Tokio runtime.
    let axum_pool = shared_pool.clone();
    tokio::spawn(async move {
        println!("@main/ starting axum server");
        run_axum_server(axum_pool).await;
    });

    /// Check positions
    let positions_pool = shared_pool.clone();
    tokio::spawn(async move {
        println!("@main/ running positions_watcher");
        positions_watcher(positions_pool).await;
    });

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handle_message))
        .branch(Update::filter_callback_query().endpoint(handle_callback_query));
    
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![shared_pool.clone()]) // Use shared_pool here
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}



#[derive(Debug, serde::Serialize)]
pub struct PumpPayload {
    method: String,
    keys: Vec<String>,
}

async fn positions_watcher(pool: SafePool) {
    let url = "wss://pumpportal.fun/api/data";
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect to pumpportal");
    let (mut pump_write, mut pump_read) = ws_stream.split();

    // Spawn WebSocket listener
    let pump_read_handle = tokio::spawn(async move {
        while let Some(msg) = pump_read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    // Handle the message (implement your price checking logic here)
                    println!("Message received: {}", text);
                },
                Err(e) => eprintln!("Error receiving message: {:?}", e),
                _ => {}
            }
        }
    });

    loop {
        // Fetch all positions from database
        let all_positions = match db::get_all_positions(&pool).await {
            Ok(positions) => positions,
            Err(e) => {
                eprintln!("Error fetching positions: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                continue;
            }
        };
        if !all_positions.is_empty() {
            println!("@check_positions/ found {} positions open", all_positions.len());
        } else {
            println!("@check_positions/ no positions open");
        }

        let tokens: Vec<String> = all_positions.iter()
            .map(|p| p.token_address.clone())
            .collect();

        // Get updated Raydium tokens
        let raydium_tokens = match check_raydiums_tokens(tokens.clone()).await {
            Ok(tokens) => tokens.into_iter().collect::<std::collections::HashSet<_>>(),
            Err(e) => {
                eprintln!("Error checking Raydium tokens: {:?}", e);
                continue;
            }
        };

        let raydium_positions = all_positions.iter()
            .filter(|p| raydium_tokens.contains(&p.token_address))
            .collect::<Vec<_>>();
        println!("@positions_watcher/ raydium positions: {:?}", raydium_positions);
        

        let pumpfun_positions = all_positions.iter()
            .filter(|p| !raydium_tokens.contains(&p.token_address))
            .collect::<Vec<_>>();
        println!("@positions_watcher/ pumpfun positions: {:?}", pumpfun_positions);

        // Calculate new PumpFun tokens
        let pumpfun_tokens: std::collections::HashSet<String> = tokens.into_iter()
            .filter(|token| !raydium_tokens.contains(token))
            .collect();
       

        // If there are new tokens to subscribe
        if !pumpfun_tokens.is_empty() {
            let pump_payload = PumpPayload {
                method: "subscribeTokenTrade".to_string(),
                keys: pumpfun_tokens.iter().cloned().collect(),
            };
            
            if let Ok(payload_json) = serde_json::to_string(&pump_payload) {
                if let Err(e) = pump_write.send(WsMessage::Text(payload_json)).await {
                    eprintln!("Error sending subscription: {:?}", e);
                }
            }
        }

        // Check Raydium prices
        if let Ok(current_prices) = crate::utils::helpers::check_raydium_tokens_prices(
            raydium_tokens.iter().cloned().collect()
        ).await {
            for position in &all_positions {
                if let Some(current_price) = current_prices.get(&position.token_address) {
                    let current_price_float = current_price.parse::<f64>().unwrap_or_default();
                    if current_price_float >= (position.take_profits[0].0 * position.entry_price) {
                        // Execute take profit
                        if let Err(e) = execute_swap_take_profit(
                            &pool,
                            position.tg_user_id.clone(),
                            (position.take_profits[0].0, position.take_profits[0].1),
                            &position.token_address,
                            "So11111111111111111111111111111111111111112"
                        ).await {
                            eprintln!("Error executing swap: {:?}", e);
                        }

                        if let Err(e) = db::delete_position_target_reached(
                            &pool,
                            &position.token_address,
                            &position.tg_user_id,
                            (position.take_profits[0].0, position.take_profits[0].1)
                        ).await {
                            eprintln!("Error deleting position: {:?}", e);
                            continue;
                        }
                    } 
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
}
