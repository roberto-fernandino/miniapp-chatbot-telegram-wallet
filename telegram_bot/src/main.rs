use teloxide::prelude::*;
use teloxide::types::MessageKind;
use telegram_bot::{call, get_valid_solana_address, is_pnl_command, is_lb_command, pnl, leaderboard, there_is_valid_solana_address, user_stats};
mod db;
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();
    
    // Start the telegram bot
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        // Get the database connection
        let con = db::get_connection();
        // Configure the database
        db::configure_db(&con);

        // Check if the message is a common message
        if let MessageKind::Common(ref common) = msg.kind {
            // Check if the message is in a group, supergroup, or channel
            if msg.chat.is_group() || msg.chat.is_supergroup()  || msg.chat.is_channel() {
                // Check if the message has text
                if let Some(text) = msg.text() {
                    // Check if the message is a pnl command
                    if is_pnl_command(text) {
                        // Get the pnl
                        match pnl(&msg, &bot).await {
                            Ok(_) => (),
                            Err(e) => log::error!("Failed to pnl: {:?}", e),
                        }
                    }
                    // Check if the message is a leaderboard command
                    else if is_lb_command(text) {
                        // Get the leaderboard
                        match leaderboard(&msg, &bot).await {
                            Ok(_) => (),
                            Err(e) => log::error!("Failed to leaderboard: {:?}", e),
                        }
                    }
                    // Check if there's a valid solana address in the message
                    else if there_is_valid_solana_address(text) {
                        // Get the valid solana address
                        let address = get_valid_solana_address(text);
                        match address {
                            Some(address) => {
                                // Call the address
                                match call(&address, &bot, &msg).await {
                                    Ok(_) => (),
                                    Err(e) => log::error!("Failed to call: {:?}", e),
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
            // Check if the message is a user stats command
            if msg.chat.is_chat() {
                if let Some(text) = msg.text() {
                    if text.starts_with("/start user_") {
                        // Get the user tg id
                        if let Some(user_tg_id) = text.strip_prefix("/start user_") {
                            // Send the user stats
                            match user_stats(user_tg_id, &bot, &msg).await {
                                Ok(_) => (),
                                Err(e) => log::error!("Failed to user_stats: {:?}", e),
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    })
    .await;
}