use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, Message};
use teloxide::{dispatching::UpdateFilterExt, Bot};
mod utils;
mod db;
use telegram_bot::*;
use crate::get_user_calls;
use telegram_bot::format_number;
use std::sync::{Arc, Mutex};
use sqlite::Connection;
use tokio::spawn;

pub type SafeConnection = Arc<Mutex<Connection>>;

pub fn get_safe_connection() -> SafeConnection {
    Arc::new(Mutex::new(db::get_connection()))
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");
    tokio::spawn(async {
        run_tide_server().await;
    });

    let bot = Bot::from_env();
    db::configure_db(&db::get_connection());

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(|bot: Bot, msg: Message| async move {
            handle_message(bot, msg).await
        }))
        .branch(Update::filter_callback_query().endpoint(|bot: Bot, q: CallbackQuery| async move {
            handle_callback_query(bot, q).await
        }));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_message(bot: Bot, msg: Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Handling message...");
    // Check if the message is a pnl command
    let con = get_safe_connection();
    if let Some(text) = msg.text() {
        if is_pnl_command(text) {
            log::info!("Message is a pnl command");
            // Get the pnl
            match pnl(&msg, &bot).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to pnl: {:?}", e),
            }
        }
        // Check if the message is a leaderboard command
        else if utils::helpers::is_lb_command(text) {
            // Get the leaderboard
            match leaderboard(&msg, &bot).await {
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
                        match user_stats(user_id, &bot, &msg).await {
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
            let call_info_str = utils::helpers::get_call_info(&address.clone(), &con, &msg).await?;
            // Call the address
            match call(&address, &bot, &msg, call_info_str).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to call: {:?}", e),
            }
        }   
        
    }
    Ok(())
}

/// Handle all the callback queries
/// 
/// # Arguments
/// 
/// * `bot` - The bot structure
/// * `query` - The callback query structure
/// * `msg` - The message structure
/// 
/// # Returns
/// 
/// * `Ok(())` - The operation was successful
/// * `Err(e)` - The operation failed
async fn handle_callback_query(bot: Bot, query: CallbackQuery) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
     
    if let Some(data) = query.data.as_ref() {
        if data.starts_with("del_call:") {
            match handle_callback_del_call(data.to_string(), &bot, &query).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to delete call: {:?}", e),
            }
        } 
        if data.starts_with("refresh:"){
            match handle_callback_refresh(data.to_string(), &bot, &query).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to refresh: {:?}", e),
            }
        }
        else {
            log::info!("Unrecognized callback query data: {}", data);
        }
        if data.starts_with("clear_call:"){
            match handle_callback_clear_call(data.to_string(), &bot, &query).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to clear call: {:?}", e),
            }
        }
    } else {
        log::info!("Callback query without data");
    }
    
    Ok(())
}

async fn run_tide_server() {
    let mut app = tide::new();
    println!("Tide bot server running.");
    app.at("/user_calls/:tg_user_id").get(get_user_calls);
    log::info!("Starting Tide server on port 2020...");
    if let Err(e) = app.listen("0.0.0.0:2020").await {
        log::error!("Failed to start Tide server: {}", e);
    }
}
