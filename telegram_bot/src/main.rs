use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, Message};
use teloxide::{dispatching::UpdateFilterExt, Bot};
mod utils;
mod db;
use telegram_bot::*;
use telegram_bot::format_number;
use std::sync::Arc;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::postgres::PgPoolOptions;

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

    let shared_pool = Arc::new(pool);
    // Spawn the Tide server on a separate task using Tokio runtime.
    let tide_pool = shared_pool.clone();
    tokio::spawn(async move {
        run_tide_server(tide_pool).await;
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

async fn handle_message(
    bot: Bot, 
    msg: Message, 
    pool: SafePool
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Handling message...");
    
    if let Some(text) = msg.text() {
        if is_pnl_command(text) {
            log::info!("Message is a pnl command");
            match pnl(&msg, &bot, pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to pnl: {:?}", e),
            }
        }
        else if utils::helpers::is_lb_command(text) {
            match leaderboard(&msg, &bot, pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to leaderboard: {:?}", e),
            }
        }
        else if utils::helpers::is_start_command(text) {
            match start(&bot, &msg).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to start: {:?}", e),
            }
        }
        else if msg.chat.is_private() {
                if text.starts_with("/start user_") {
                    // get the user id
                    if let Some(user_id) = text.strip_prefix("/start user_") {
                        // get the user stats
                        match user_stats(user_id, &bot, &msg, &pool).await {
                            Ok(_) => (),
                            Err(e) => log::error!("Failed to user stats: {:?}", e),
                        }
                    }
                }
        }
        // Check if there's a valid solana address in the message
        else if there_is_valid_solana_address(text) || there_is_valid_eth_address(text) {
            // Get the valid solana address
            let address = utils::helpers::address_handler(text).await?;
            let call_info_str = utils::helpers::get_call_info(&address.clone(), &pool, &msg).await?;
            // Call the address
            match call(&address, &bot, &msg, call_info_str, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to call: {:?}", e),
            }
        }   
        
    }
    Ok(())
}

/// Handles callback queries by delegating to specific handlers.
async fn handle_callback_query(
    bot: Bot, 
    query: CallbackQuery, 
    pool: SafePool
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(data) = query.data.as_ref() {
        if data.starts_with("del_call:") {
            match handle_callback_del_call(data.to_string(), &bot, &query, pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to delete call: {:?}", e),
            }
        } 
        else if data.starts_with("refresh:") {
            match handle_callback_refresh(data.to_string(), &bot, &query, pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to refresh: {:?}", e),
            }
        }
        else if data.starts_with("clear_call:") {
            match handle_callback_clear_call(&bot, &query).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to clear call: {:?}", e),
            }
        }
        else {
            log::info!("Unrecognized callback query data: {}", data);
        }
    } else {
        log::info!("Callback query without data");
    }
    
    Ok(())
}

/// Runs the Tide server.
async fn run_tide_server(pool: SafePool) {
    let mut app = tide::new();
    println!("Tide bot server running.");
    app.at("/user_calls/:tg_user_id").get(move |req| {
        let pool = pool.clone();
        async move { get_user_calls(req, pool).await }
    });
    log::info!("Starting Tide server on port 2020...");
    if let Err(e) = app.listen("0.0.0.0:2020").await {
        log::error!("Failed to start Tide server: {}", e);
    }
}
