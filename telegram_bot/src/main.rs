use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, Message};
use teloxide::{dispatching::UpdateFilterExt, Bot};
mod utils;
mod db;
use telegram_bot::*;
use crate::db::get_user_calls;
use telegram_bot::format_number;
use std::sync::Arc;
use sqlx::Pool;
use sqlx::Postgres;
use tokio::sync::Mutex;
use std::env;

pub type SafePool = Arc<Pool<Postgres>>;

/// Initializes and returns a thread-safe PostgreSQL connection pool.
pub async fn get_safe_pool() -> SafePool {
    let pool = db::get_pool().await.expect("Failed to create PostgreSQL pool");
    Arc::new(pool)
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    // Initialize the PostgreSQL connection pool.
    let pool = get_safe_pool().await;

    // Configure the database (create tables if they don't exist).
    db::configure_db(&pool).await.expect("Failed to configure the database");

    // Spawn the Tide server on a separate task using Tokio runtime.
    let tide_pool = pool.clone();
    tokio::spawn(async move {
        run_tide_server(tide_pool).await;
    });

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(|bot: Bot, msg: Message| async move {
            handle_message(bot, msg, pool.clone()).await
        }))
        .branch(Update::filter_callback_query().endpoint(|bot: Bot, q: CallbackQuery| async move {
            handle_callback_query(bot, q, pool.clone()).await
        }));

    Dispatcher::builder(bot, handler)
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
            match pnl(&msg, &bot, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to pnl: {:?}", e),
            }
        }
        else if utils::helpers::is_lb_command(text) {
            match leaderboard(&msg, &bot, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to leaderboard: {:?}", e),
            }
        }
        // ... [Handle other commands similarly, passing `pool` as needed]
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
            match handle_callback_del_call(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to delete call: {:?}", e),
            }
        } 
        else if data.starts_with("refresh:") {
            match handle_callback_refresh(data.to_string(), &bot, &query, &pool).await {
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
    use tide::prelude::*; // For serde_json

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
