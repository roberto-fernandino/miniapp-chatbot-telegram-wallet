use commands::execute_swap;
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
        run_axum_server(axum_pool).await;
    });

    /// Check positions
    let positions_pool = shared_pool.clone();
    tokio::spawn(async move {
        check_positions(positions_pool).await;
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

async fn check_positions(pool: SafePool) {
    let url = "wss://pumpportal.fun/api/data";
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect to pumpportal");
    let (mut pump_write, mut pump_read) = ws_stream.split();
    let all_positions = db::get_all_positions(&pool).await.unwrap();
    let mut raydium_tokens_to_watch = vec![];
    let mut pumpfun_tokens_to_watch = vec![];
    let mut positions_raydium: Vec<db::Position> = vec![];
    let mut positions_pumpfun: Vec<db::Position> = vec![];

    let tokens = all_positions.iter().map(|p| p.token_address.clone()).collect::<Vec<String>>();

    raydium_tokens_to_watch = check_raydiums_tokens(tokens.clone()).await.unwrap();
    pumpfun_tokens_to_watch = tokens.clone()
        .iter()
        .filter(|token| !raydium_tokens_to_watch.contains(token))
        .cloned()
        .collect::<Vec<String>>();
    
    let pump_payload = PumpPayload {
        method: "subscribeTokenTrade".to_string(),
        keys: pumpfun_tokens_to_watch
    };
    let pump_payload_json = serde_json::to_string(&pump_payload).unwrap();
    pump_write.send(WsMessage::Text(pump_payload_json)).await.unwrap();

    tokio::spawn(async move {
        while let Some(msg) = pump_read.next().await {
            println!("Message received: {:?}", msg);
        }
    });
        // New loop to check prices every 3 seconds
    loop {
        // Fetch current prices (this is a placeholder, replace with actual fetching logic)
        let current_prices = crate::utils::helpers::check_raydium_tokens_prices(tokens.clone()).await;
        // Ensure current_prices is successfully retrieved
        let current_prices = match current_prices {
            Ok(prices) => prices,
            Err(e) => {
                eprintln!("Error fetching current prices: {:?}", e);
                continue; // Skip this iteration if there's an error
            }
        };

        // Check if any position has reached its target price
        for position in &all_positions {
            if let Some(current_price) = current_prices.get(&position.token_address) {
                let current_price_float = current_price.parse::<f64>().unwrap();
                if current_price_float >= (position.take_profits[0].0 * position.entry_price) {
                    // Handle the event (e.g., notify user, update database, etc.)
                    println!("Target price reached for position: {:?}", position);
                    db::delete_position_target_reached(&pool, &position.token_address, &position.tg_user_id, (position.take_profits[0].0, position.take_profits[0].1)).await.unwrap();
                    execute_swap(&pool, &position.token_address, "So11111111111111111111111111111111111111112", position.tg_user_id.clone()).await.unwrap();
                }
            }
        }
        // Wait for 3 seconds before the next check
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
}
