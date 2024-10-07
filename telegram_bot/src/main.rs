use teloxide::prelude::*;
use teloxide::types::MessageKind;
use telegram_bot::{call, get_valid_solana_address, is_pnl_command, is_lb_command, pnl, leaderboard, there_is_valid_solana_address};
mod db;
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();
    
    
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        let con = db::get_connection();
        db::configure_db(&con);
        if let MessageKind::Common(ref common) = msg.kind {
            if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_private() || msg.chat.is_channel() {
                if let Some(text) = msg.text() {
                    log::info!("Received a message in a chat: {:?}", text);
                    log::info!("is valid solana address: {:?}", there_is_valid_solana_address(text));
                    if is_pnl_command(text) {
                        match pnl(&msg, &bot).await {
                            Ok(_) => (),
                            Err(e) => log::error!("Failed to pnl: {:?}", e),
                        }
                    }
                    else if is_lb_command(text) {
                        match leaderboard(&msg, &bot).await {
                            Ok(_) => (),
                            Err(e) => log::error!("Failed to leaderboard: {:?}", e),
                        }
                    }
                    else if there_is_valid_solana_address(text) {
                        let address = get_valid_solana_address(text);
                        match address {
                            Some(address) => {
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
        }
        Ok(())
    })
    .await;
}