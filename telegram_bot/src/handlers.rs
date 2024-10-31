use teloxide::dispatching::dialogue::GetChatId;
use teloxide::payloads::SendMessageSetters;
use teloxide::payloads::EditMessageReplyMarkupSetters;
use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use teloxide::types::CallbackQuery;
use teloxide::types::Message;
use teloxide::Bot;
use serde::{Serialize, Deserialize};
use crate::db::*;
use crate::utils::helpers::*;
use axum::extract::State;
use crate::*;
use crate::commands::*;
use teloxide::payloads::AnswerCallbackQuerySetters;
use teloxide::payloads::EditMessageTextSetters;
use teloxide::prelude::Requester;
use chrono::{DateTime, Utc};
use regex::Regex;
use axum::extract::Path;
use std::sync::Arc;
use sqlx::Pool;
use sqlx::Postgres;
pub type SafePool = Arc<Pool<Postgres>>;

pub async fn handle_callback_clear_call( bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery) -> Result<()> {
    if let Some(ref message) = query.message {
        match message {
            teloxide::types::MaybeInaccessibleMessage::Regular(msg) => {
                bot.delete_message(msg.chat.id, msg.id).await?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub async fn handle_callback_refresh_call(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery, pool: SafePool) -> Result<()> {
    let call_id = data.strip_prefix("refresh:").unwrap_or_default();
    let call = crate::db::get_call_by_id(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
    let token_pair_token_address = get_pair_token_pair_and_token_address(&call.token_mint).await?;
    let token_address = token_pair_token_address["tokenAddress"].as_str().unwrap_or("");
    let chain = token_pair_token_address["chainName"].as_str().unwrap_or("");
    let scanner_response = get_scanner_search(
&call.token_mint).await?;

    let created_datetime_str = scanner_response["pair"]["pairCreatedAt"].as_str().unwrap_or("");
    let datetime: DateTime<Utc> = created_datetime_str.parse().expect("Failed to parse datetime.");
    let unix_timestamp_milis = datetime.timestamp_millis();

    let ath_response = get_ath(unix_timestamp_milis, &call.token_mint, &call.chain).await?;
    log::info!("ath_response: {:?}", ath_response);
    let holders_response = get_holders(token_address).await?;
    let user = get_user(&pool, call.user_tg_id.as_str()).await?;
    if let Some(ref message) = query.message {
        match message {
            teloxide::types::MaybeInaccessibleMessage::Regular(msg) => {
                let call_info_str = utils::helpers::get_call_info(&call.token_address.clone(), &pool, msg).await?;
                let call_message = call_message(
                    &pool,
                    &ath_response,
                    &holders_response,
                    &scanner_response,
                    call_info_str,
                    user,
                    chain
                ).await;
                let keyboard = create_call_keyboard_after_just_scanning(call_id, call.token_address.as_str());
                bot.edit_message_text(msg.chat.id, msg.id, call_message)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .link_preview_options(teloxide::types::LinkPreviewOptions {
                        is_disabled: true,
                        url: None,
                        prefer_small_media: false,
                        prefer_large_media: false,
                        show_above_text: false,
                    })
                    .reply_markup(keyboard)
                    .await?;
            }
            _ => {}
        }
    }


    Ok(())
}


/// Handle the address
/// 
/// # Arguments
/// 
/// * `text` - The text to handle
/// 
/// # Returns
/// 
/// The token address
pub async fn address_handler(text: &str) -> Result<String> {
    let is_solana_address = there_is_valid_solana_address(text);
    let is_eth_address = there_is_valid_eth_address(text);
    if is_solana_address {
        let address = get_valid_solana_address(text);
        Ok(address.expect("No valid address found"))
    } else if is_eth_address {
        let address = get_valid_eth_address(text);
        Ok(address.expect("No valid address found"))
    } else {
        Err(anyhow::anyhow!("No valid address found"))
    }
}


/// Handle the callback query to delete a call
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The bot structure
/// * `query` - The callback query structure
/// 
/// # Returns
/// 
/// * `Ok(())` - The operation was successful
/// * `Err(e)` - The operation failed
pub async fn handle_callback_del_call(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery, pool: SafePool) -> Result<()> {
    log::info!("Deleting call...");
    // Extract the call ID
    let user_tg_id =  query.from.id.to_string();
    let call_id = data.strip_prefix("del_call:").unwrap_or_default();
    let call_user = get_user_from_call(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
    let call = get_call_by_id(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
    if call_user.tg_id == user_tg_id {
        if let Ok(call_id_num) = call_id.parse::<i64>() {
            // Attempt to delete the call
            match delete_call(&pool, call_id_num).await {
                Ok(_) => {
                    if let Some(ref message) = query.message {
                        // Edit the message with and put just scanning text on the call 
                        let call_info_regex = Regex::new(r"(?s)🔥 First Call.*?🎉|😈.*?@.*?\n").unwrap();
                        match message {
                            teloxide::types::MaybeInaccessibleMessage::Regular(msg) => {
                                match msg.text() {
                                    Some(text) => {
                                        let updated_text = call_info_regex.replace(text, "‼️ Just Scanning...");
                                        // Create the buttons
                                        let keyboard = create_call_keyboard_after_just_scanning(call_id.to_string().as_str(), call.token_address.as_str());

                                        bot.edit_message_text(msg.chat.id, msg.id, updated_text.to_string())
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(keyboard)
                                            .await?;
                                    }
                                    None => {}
                                }
                            },
                            teloxide::types::MaybeInaccessibleMessage::Inaccessible(_) => {
                                {}
                            },
                        };
        
                        bot.answer_callback_query(query.id.clone())
                            .text("Call deleted successfully!")
                            .await?;
                    }
                },
                Err(e) => {
                    log::error!("Failed to delete call {}: {:?}", call_id_num, e);
                    bot.answer_callback_query(query.id.clone())
                        .text("Failed to delete call.")
                        .await?;
                },
            }
            
        } else {
            log::error!("Invalid call ID: {}", call_id);
            bot.answer_callback_query(query.id.clone())
                .text("Invalid call ID.")
                .await?;
        }
    } else {
        bot.answer_callback_query(query.id.clone())
            .text("❌ Only the user who sent this call can use the button.")
            .show_alert(true)
            .await?; 
    }
    Ok(())
}


/// Handle all the messages in all the chats
/// 
/// # Arguments
/// 
/// * `bot` - The bot
/// * `msg` - The message
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn handle_message(
    bot: Bot, 
    msg: Message, 
    pool: SafePool
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Handling message...");
    
    if let Some(text) = msg.text() {
        if let Some(reply_to_message) = msg.reply_to_message() { 
            if reply_to_message.text().unwrap_or_default().starts_with("Enter the amount of SOL to buy") {
                if let Ok(amount) = text.parse::<f64>() {
                    set_user_buy_amount(&pool, msg.from.as_ref().unwrap().id.to_string().as_str(), amount.to_string().as_str()).await.unwrap();
                    bot.send_message(msg.chat.id, format!("Amount set to: {}", amount)).await?;
                    let last_token = get_user_last_sent_token(&pool, msg.from.as_ref().unwrap().id.to_string().as_str()).await.unwrap();
                    token_address_buy_info_handler(&last_token, &bot, &msg, &pool).await?;
                } else {
                    bot.send_message(msg.chat.id, "Invalid amount").await?;
                }
            }
            else if reply_to_message.text().unwrap_or_default().starts_with("Enter the slippage tolerance") {
                let mut slippage_tolerance = text.parse::<f64>().unwrap_or(0.18);
                slippage_tolerance = slippage_tolerance / 100.0;
                set_user_slippage_tolerance(&pool, msg.from.as_ref().unwrap().id.to_string().as_str(), slippage_tolerance.to_string().as_str()).await.unwrap();
                bot.send_message(msg.chat.id, format!("Slippage tolerance set to: {}%", slippage_tolerance * 100.0)).await?;
                let last_token = get_user_last_sent_token(&pool, msg.from.as_ref().unwrap().id.to_string().as_str()).await.unwrap();
                token_address_buy_info_handler(&last_token, &bot, &msg, &pool).await?;
            }
            else if reply_to_message.text().unwrap_or_default().starts_with("Enter the sell percentage") {
                if let Ok(sell_percentage) = text.parse::<f64>() {
                    set_user_sell_percentage(&pool, msg.from.as_ref().unwrap().id.to_string().as_str(), sell_percentage.to_string().as_str()).await.unwrap();
                    bot.send_message(msg.chat.id, format!("Sell percentage set to: {}%", sell_percentage)).await?;
                    sell_token_page(&msg, &bot, &pool).await?;
                } else {
                    bot.send_message(msg.chat.id, "Invalid sell percentage").await?;
                }
            }
            else if reply_to_message.text().unwrap_or_default().starts_with("Enter the gas fee") {
                let gas_lamports: i32 = match text.parse::<f64>() {
                    Ok(sol_amount) => utils::helpers::sol_to_lamports_i32(sol_amount),
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid gas fee").await?;
                        return Ok(());
                    }
                };
                set_user_gas_lamports(&pool, msg.from.as_ref().unwrap().id.to_string().as_str(), gas_lamports).await?;
                bot.send_message(msg.chat.id, format!("Gas fee set to: {} SOL", utils::helpers::lamports_to_sol(gas_lamports))).await?;
            }
            else if reply_to_message.text().unwrap_or_default().starts_with("Send '<multiplier>,<%_token_position_amount_to_sell>' (eg: '1.5,100' that means if the price goes up 1.5x, sell 100% of the position)") {
                println!("@handle_message/ text: {:?}", text);
                let take_profits = match parse_take_profit_message(text) {
                    Ok(tp) => {
                        tp
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, "Invalid take profit format. Please use format: '<multiplier>, <%_token_position_amount_to_sell>'").await?;
                        return Ok(());
                    }
                };
                println!("@handle_message/ take_profits: {:?}", take_profits);
                println!("@handle_message/ adding to user settings");
                add_user_take_profit_user_settings(msg.clone().from.unwrap().id.to_string().as_str(), take_profits, &pool).await?;
                println!("@handle_message/ added to user settings");
                bot.send_message(msg.chat.id, "Take profit set").await?;
                let last_token = get_user_last_sent_token(&pool, msg.from.as_ref().unwrap().id.to_string().as_str()).await.unwrap();
                match token_address_buy_info_handler(last_token.as_str(), &bot, &msg, &pool).await {
                    Ok(_) => (),
                    Err(e) => log::error!("Failed to open buy menu for token address: {:?}", e),
                }
            }
            else if reply_to_message.text().unwrap_or_default().starts_with("Send '<%down>,<%_token_position_amount_to_sell>' (eg: '10,100' that means if the price goes down 10%, sell 100% of the position)") {
                println!("@handle_message/ text: {:?}", text);
                let stop_loss = match parse_stop_loss_message(text) {
                    Ok(sl) => sl,
                    Err(e) => {
                        bot.send_message(msg.chat.id, "Invalid stop loss format. Please use format: '<%_down>,<%_token_position_amount_to_sell>'").await?;
                        return Ok(());
                    }
                };
                println!("@handle_message/ stop_loss: {:?}", stop_loss);
                add_user_stop_loss_user_settings(msg.clone().from.unwrap().id.to_string().as_str(), stop_loss, &pool).await?;
                bot.send_message(msg.chat.id, "Stop loss set").await?;
                let last_token = get_user_last_sent_token(&pool, msg.from.as_ref().unwrap().id.to_string().as_str()).await.unwrap();
                match token_address_buy_info_handler(last_token.as_str(), &bot, &msg, &pool).await {
                    Ok(_) => (),
                    Err(e) => log::error!("Failed to open buy menu for token address: {:?}", e),
                }   
            }
        }
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
                else if text.starts_with("/start sell_token_") {
                    match sell_token_page(&msg, &bot, &pool).await {
                        Ok(_) => (),
                        Err(e) => log::error!("Failed to sell token: {:?}", e),
                    }
                }
                else if text.starts_with("/start") {
                    let user_tg_id = msg.from.as_ref().unwrap().id.to_string();
                    let username = msg.from.as_ref().unwrap().username.clone().unwrap_or("Unknown username".to_string());
                    let chat_id = msg.chat.id;
                    match start(&bot, &user_tg_id, &username, chat_id, &pool).await {
                        Ok(_) => (),
                        Err(e) => log::error!("Failed to start: {:?}", e),
                    }
                }
                else if there_is_valid_solana_address(text) || there_is_valid_eth_address(text) {
                    match token_address_buy_info_handler(text, &bot, &msg, &pool).await {
                        Ok(_) => (),
                        Err(e) => log::error!("Failed to buy token address: {:?}", e),
                    }
                }
        }
        // Check if there's a valid solana address in the message
        else if there_is_valid_solana_address(text) || there_is_valid_eth_address(text) {
            // Get the valid solana address
            let address = address_handler(text).await?;
            let call_info_str = get_call_info(&address.clone(), &pool, &msg).await?;
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
/// 
/// # Arguments
/// 
/// * `bot` - The bot
/// * `query` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn handle_callback_query(
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
            match handle_callback_refresh_call(data.to_string(), &bot, &query, pool).await {
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
        else if data == "buy" {
            match handle_buy_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to buy: {:?}", e),
            }
        }
        else if data.starts_with("buy:") {
            match handle_execute_buy_sol_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to buy: {:?}", e),
            }
        }
        else if data.starts_with("amount:") && !data.ends_with("custom") {
            match handle_set_buy_amount_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to buy: {:?}", e),
            }
        }
        else if data.starts_with("toggle_swap_limit:") {
            match handle_toggle_swap_limit_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to toggle swap limit: {:?}", e),
            }
        }
        else if data == "amount:custom" {
            match handle_set_custom_buy_amount_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to set custom buy amount: {:?}", e),
            }
        }
        else if data == "sell_percentage:custom" {
            match handle_set_custom_sell_percentage_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to set custom sell percentage: {:?}", e),
            }
        }
        else if data.starts_with("delete_take_profit:") {
            match handle_delete_take_profit_user_settings_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to delete take profit: {:?}", e),
            }
        }
        else if data.starts_with("delete_stop_loss:") {
            match handle_delete_stop_loss_user_settings_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to delete stop loss: {:?}", e),
            }
        }
        else if data == "back" {
            let user_tg_id = query.from.id.to_string();
            let user = get_user(&pool, &user_tg_id).await?;
            start(&bot, &user_tg_id, &user.username.clone().unwrap_or("Unknown username".to_string()), query.message.as_ref().unwrap().chat().id, &pool).await?;
        }
        else if data == "set_custom_slippage" {
            match handle_set_custom_slippage_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to set custom slippage: {:?}", e),
            }
        }
        else if data == "set_custom_gas" {
            match handle_set_custom_gas_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to set custom gas: {:?}", e),
            }
        }
        else if data == "positions" {
            match handle_positions_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to handle positions callback: {:?}", e),
            }
        }
        else if data == "sell_page" {
            match handle_sell_choose_token_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to handle sell page callback: {:?}", e),
            }
        }
        else if data.starts_with("sell:") {
            match handle_execute_sell_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to sell: {:?}", e),
            }
        }
        else if data.starts_with("sell_percentage:") {
            match handle_set_sell_percentage_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to set sell percentage: {:?}", e),
            }
        }
        else if data.starts_with("settings") {
            match handle_settings_callback(data.to_string(), &bot, &query, &pool).await { 
                Ok(_) => (),
                Err(e) => log::error!("Failed to handle settings callback: {:?}", e),
            }
        }
        else if data.starts_with("delete_take_profit:") {
            match handle_delete_take_profit_user_settings_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to delete take profit: {:?}", e),
            }
        }
        else if data.starts_with("add_take_profit") {
            match handle_add_take_profit_user_settings_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to add take profit: {:?}", e),
            }
        }
        else if data.starts_with("add_stop_loss") {
            match handle_add_stop_loss_user_settings_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to add stop loss: {:?}", e),
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

/// Get the user calls
/// 
/// # Arguments
/// 
/// * `tg_user_id` - The Telegram user ID
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A JSON response with the user calls
pub async fn get_user_calls_handler(
    Path(tg_user_id): Path<i64>,
    State(pool): State<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    match get_user_calls(tg_user_id, pool).await {
        Ok(calls) => {
            match serde_json::to_string(&calls) {
                Ok(json) => (StatusCode::OK, Json(json)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize calls: {}", e)).into_response(),
            }
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get calls: {}", e)).into_response(),
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PostUserRequest {
    pub tg_id: String,
    pub username: Option<String>,
    pub turnkey_info: TurnkeyInfo,
    pub solana_address: String,
    pub eth_address: String,
}

/// Post add user handler
/// 
/// # Arguments
/// 
/// * `pool` - The database pool
/// * `user` - The user request
/// 
/// # Returns
/// 
/// A response indicating the success of the operation
pub async fn post_add_user_handler(
    State(pool): State<Arc<Pool<Postgres>>>,
    Json(user): Json<PostUserRequest>
) -> impl IntoResponse {
    println!("@add_user/ Request received!");
    println!("@add_user/ post_data: {:?}", user);
    let user_exists = match user_exists(&pool, &user.tg_id).await {
        Ok(exists) => exists,
        Err(e) => {
            println!("@add_user/ error checking if user exists: {:?}", e);
            false
        }
    };
    println!("@add_user/ user_exists: {:?}", user_exists);

    if user_exists {
        let user_id = match get_user_id_by_tg_id(&pool, &user.tg_id).await {
            Ok(id) => id,
            Err(e) => {
                println!("@add_user/ error getting user id: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not get user id").into_response();
            }
        };
        match update_user(&pool, User { id: user_id, username: user.username, tg_id: user.tg_id, turnkey_info: user.turnkey_info, solana_address: Some(user.solana_address), eth_address: Some(user.eth_address) }).await {
            Ok(_) => println!("@add_user/ user updated in the db."),
            Err(e) => {
                println!("@add_user/ error updating user in the db: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not update user in the db").into_response();
            }
        }
    } else {
        match add_user_post(&pool, user.clone()).await {
            Ok(_) => println!("@add_user/ user added to the db."),
            Err(e) => {
                println!("@add_user/ error adding user to the db: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not add user to the db").into_response();
            }
        }
        match create_user_settings_default(&pool, &user.tg_id).await {
            Ok(_) => println!("@add_user/ user settings added to the db."),
            Err(e) => {
                println!("@add_user/ error adding user settings to the db: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not add user settings to the db").into_response();
            }
        }
    }
    (StatusCode::OK, "User added/updated in the db.").into_response()
}

/// Get positions handler
/// 
/// # Arguments
/// 
/// * `user_tg_id` - The user's Telegram ID
/// * `pool` - The PostgreSQL connection pool
/// 
/// # Returns
/// 
/// A JSON response with the positions
pub async fn get_positions_handler(
    Path(user_tg_id): Path<String>, 
    State(pool): State<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    let positions = get_positions_by_user_tg_id(&pool, &user_tg_id).await.expect("Could not get positions");
    (StatusCode::OK, Json(positions)).into_response()
}

/// Handle buy callback
/// 
/// # Description
/// 
/// Handle the buy callback from the tg bot.
/// Sends a message to the user with force reply to enter the token address
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn handle_buy_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    println!("@buy_callback/ data: {:?}", data);
    let user_id = q.from.id.to_string();
    println!("@buy_callback/ user_id: {:?}", user_id);
    let chat_id = q.message.as_ref().unwrap().chat().id;
    bot.send_message(chat_id, "Enter a token address to buy")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Enter the token address".to_string()), selective: false})
    .await?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TurnkeyUser {
    pub api_public_key: String,
    pub api_private_key: String,
    pub organization_id: String,
    pub public_key: String
}


#[derive(Serialize, Deserialize)]
pub struct SwapSolRequest {
    pub user: TurnkeyUser,
    pub user_public_key: String,
    pub priorization_fee_lamports: u64,
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage: f64,
}

/// Handle execute buy sol callback
/// 
/// # Description
/// 
/// Execute swap buy sol transaction
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn handle_execute_buy_sol_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    println!("@handle_execute_buy_sol_callback/ data: {:?}", data);
    let token_address = get_user_last_sent_token(pool, &q.from.id.to_string()).await?;
    let user_id = q.from.id.to_string();
    println!("@handle_execute_buy_sol_callback/ user_id: {:?}", user_id);

    let response = match execute_swap(pool, "So11111111111111111111111111111111111111112", token_address.as_str(), user_id, q.chat_id().expect("Chat ID not found").to_string().as_str()).await {
        Ok(r) => r,
        Err(e) => {
            println!("@handle_execute_buy_sol_callback/ error executing swap: {:?}", e);
            bot.send_message(q.chat_id().expect("Chat ID not found"), format!("❌ Failed to buy: {}", e)).await?;
            return Err(e.into());
        }
    };

    println!("@handle_execute_buy_sol_callback/ checking if response is success");
    if response.status().is_success() {
        println!("@handle_execute_buy_sol_callback/ response is success");
        let json_response = response.json::<serde_json::Value>().await?;
        println!("@handle_execute_buy_sol_callback/ json_response: {:?}", json_response);
        if let Some(transaction) = json_response["transaction"].as_str() {
            println!("@handle_execute_buy_sol_callback/ transaction signature found on response: {:?}", transaction);
            bot.send_message(q.message.as_ref().unwrap().chat().id, format!("https://solscan.io/tx/{}", transaction)).await?;
        } else {
            println!("@handle_execute_buy_sol_callback/ transaction signature not found on response");
            bot.send_message(q.message.as_ref().unwrap().chat().id, "Transaction ID not found in solana app response.".to_string()).await?;
        }
    } else {
        println!("@handle_execute_buy_sol_callback/ response is not success");
        bot.send_message(q.message.as_ref().unwrap().chat().id, format!("Failed to buy: {}", response.text().await?)).await?;
        println!("@handle_execute_buy_sol_callback/ response is not success");
    }   
    println!("@handle_execute_buy_sol_callback/ done");
    Ok(())
}


/// Create the buy state on tg bot after receiving a token address
/// 
/// # Arguments
/// 
/// * `text` - The token address
/// * `bot` - The Telegram bot
/// * `msg` - The message
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn token_address_buy_info_handler(text: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message, pool: &SafePool) -> Result<()> {
    println!("@buy_sol_token_address_handler/ text: {:?}", text);
    let user = get_user(&pool, &msg.from.as_ref().unwrap().id.to_string()).await?;
    println!("@buy_sol_token_address_handler/ user: {:?}", user);
    let sol_balance = get_wallet_sol_balance(&user.solana_address.expect("Solana address not found").as_str()).await?;
    println!("@buy_sol_token_address_handler/ sol_balance: {:?}", sol_balance);
    let sol_balance_usd = sol_to_usd(sol_balance.parse::<f64>().unwrap_or(0.0)).await?;
    let token_address= address_handler(text).await?;
    println!("@buy_sol_token_address_handler/ token_address: {:?}", token_address);
    println!("@buy_sol_token_address_handler/ setting last sent token");
    set_user_last_sent_token(&pool, &user.tg_id, token_address.as_str()).await?;
    println!("@buy_sol_token_address_handler/ last sent token set");
    println!("@buy_sol_token_address_handler/ creating keyboard");
    let keyboard = create_sol_buy_swap_keyboard(&pool, user.tg_id.to_string().as_str()).await;
    println!("@buy_sol_token_address_handler/ keyboard created");
    println!("@buy_sol_token_address_handler/ sending scanner request");
    let scanner_response = get_scanner_search(token_address.as_str()).await?;
    println!("@buy_sol_token_address_handler/ received response");

    // token info
    let token_symbol = scanner_response["pair"]["token1Symbol"].as_str().unwrap_or("N/A").to_uppercase();
    let token_name = scanner_response["pair"]["token1Name"].as_str().unwrap_or("N/A");
    let token_usd_price = format!("{:.8}", scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0)).parse::<f64>().unwrap_or(0.0);
    let mkt_cap: String = format_number(scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let lp = scanner_response["pair"]["burnedAmount"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let burnt = if scanner_response["pair"]["burnedAmount"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) == scanner_response["pair"]["burnedSupply"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) { "✓" } else { "x" };
    let renounced = if scanner_response["pair"]["renounced"].as_bool().unwrap_or(false) { "✓" } else { "x" };

    bot.send_message(
        msg.chat.id, 
        format!(
            "Swap ${token_symbol}📈 - ({token_name})\n\
            <code> {token_address}</code> (Tap to copy)\n\
            • SOL Balance: {sol_balance} (${sol_balance_usd}) [TransferSOL]\n\
            • Price: <b>${token_usd_price}</b> LP: <b>${lp}</b> MC: <b>${mkt_cap}</b>\n\
            • Renounced: {renounced} Burnt: {burnt}
            "
        )
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}

/// Handle toggle swap or limit callback
/// 
/// # Description
/// 
/// Toggle the swap or limit state on the tg bot
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn  handle_toggle_swap_limit_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let user_tg_id = q.from.id.to_string();
    let msg_id = q.message.as_ref().unwrap().id();
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let limit_or_swap = data.strip_prefix("toggle_swap_limit:").unwrap_or("swap");
    set_user_swap_or_limit(&pool, &user_tg_id, limit_or_swap).await?;
    let keyboard = create_sol_buy_swap_keyboard( &pool, &user_tg_id).await;
    bot.edit_message_reply_markup(chat_id, msg_id)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}


/// Handle set buy amount callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_set_buy_amount_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let token_address = get_user_last_sent_token(&pool, &q.from.id.to_string()).await?;
    let user_tg_id = q.from.id.to_string();
    let msg_id = q.message.as_ref().unwrap().id();
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let buy_amount = data.strip_prefix("amount:").unwrap_or("0.2");
    set_user_buy_amount(&pool, &user_tg_id, buy_amount).await?;
    let keyboard = create_sol_buy_swap_keyboard(&pool, &user_tg_id).await;
    bot.edit_message_reply_markup(chat_id, msg_id)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}

/// Handle set sell percentage callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_set_sell_percentage_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    println!("@handle_set_sell_percentage_callback/ data: {:?}", data);
    let user_tg_id = q.from.id.to_string();
    let msg_id = q.message.as_ref().unwrap().id();
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let sell_percentage = data.strip_prefix("sell_percentage:").unwrap_or("10");
    println!("@handle_set_sell_percentage_callback/ sell_percentage: {:?}", sell_percentage);
    set_user_sell_percentage(&pool, &user_tg_id, sell_percentage).await?;
    let last_token_address = get_user_last_sent_token(&pool, &user_tg_id).await?;
    println!("@handle_set_sell_percentage_callback/ last_token_address: {:?}", last_token_address);
    println!("@handle_set_sell_percentage_callback/ creating keyboard");
    let keyboard = create_sol_sell_swap_keyboard(&pool, &user_tg_id, last_token_address.as_str()).await?;
    println!("@handle_set_sell_percentage_callback/ keyboard created");
    bot.edit_message_reply_markup(chat_id, msg_id)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}


/// Handle set custom buy amount callback
/// 
/// # Description
/// 
/// Set the custom buy amount on the tg bot
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_set_custom_buy_amount_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Enter the amount of SOL to buy")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Enter the amount of SOL to buy".to_string()), selective: false})
    .await?;
    Ok(())
}
/// Handle set sell_percenteage amount callback
/// 
/// # Description
/// 
/// Set the sell percentage amount on the tg bot
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_set_custom_sell_percentage_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Enter the sell percentage")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Enter the sell percentage".to_string()), selective: false})
    .await?;
    Ok(())
}


/// Handle set custom slippage callback
/// 
/// # Description
/// 
/// Set the custom slippage on the tg bot
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation    
async fn handle_set_custom_slippage_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Enter the slippage tolerance format integer (10 for 10%)")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Enter the slippage tolerance".to_string()), selective: false})
    .await?;
    Ok(())
}


/// handle positions callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_positions_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let message = create_positions_message(&q.from.id.to_string(), pool).await?;
    let keyboard = create_positions_keyboard(&q.from.id.to_string(), pool).await?;
    bot.send_message(q.message.as_ref().unwrap().chat().id, message)
    .parse_mode(teloxide::types::ParseMode::Html)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}

/// Handle sell callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_sell_choose_token_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let user_tg_id = q.from.id.to_string();
    let user = get_user(&pool, &user_tg_id).await?;
    let tokens_balance = get_positions_balance(&user.clone().solana_address.expect("Solana address not found").as_str()).await?;
    let has_any_token = tokens_balance["tokens"].as_array().unwrap_or(&Vec::new()).len() > 0;
    let sol_balance = get_wallet_sol_balance(&user.solana_address.expect("Solana address not found").as_str()).await?;
    let sol_balance_usd = sol_to_usd(sol_balance.parse::<f64>().unwrap_or(0.0)).await?;
    let mut tokens_str = String::new();
    for token in tokens_balance["tokens"].as_array().unwrap_or(&Vec::new()) {
        if token["token_ui_amount"].as_f64().unwrap_or(0.0) > 0.0 {
            tokens_str.push_str(&format!("{} <a href=\"https://t.me/sj_copyTradebot?start=sell_token_{}\">Sell</a>\n", token["mint"].as_str().unwrap_or("N/A"), token["mint"].as_str().unwrap_or("N/A")));
        }
    }
    if has_any_token {
        bot.send_message(q.message.as_ref().unwrap().chat().id,format!("
        <b>Select the token to sell</b>\n\
        SOL BALANCE: <code> {sol_balance:.6} SOL</code> (${sol_balance_usd:.2})\n\
        {tokens_str}
        "))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    } else {
        bot.send_message(q.message.as_ref().unwrap().chat().id,format!("
        <b>Select a token to sell:</b>\n\
        SOL BALANCE: <code> {sol_balance:.6} SOL</code> (${sol_balance_usd:.2})\n\
        No token holdings found
        "))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    }

    println!("@handle_sell_callback/ tokens_balance: {:?}", tokens_balance);

    Ok(())
}

/// Handle sell callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_execute_sell_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    println!("@handle_execute_sell_callback/ data: {:?}", data);
    let token_address = data.split(":").nth(1).unwrap_or("N/A").to_string();
    println!("@handle_execute_sell_callback/ token_address: {:?}", token_address);
    let user_id = q.from.id.to_string();
    println!("@handle_execute_sell_callback/ user_id: {:?}", user_id);
    let response = match execute_swap(&pool, &token_address, "So11111111111111111111111111111111111111112", user_id, q.chat_id().expect("Chat ID not found").to_string().as_str()).await {
        Ok(r) => r,
        Err(e) => {
            println!("@handle_execute_sell_callback/ error executing swap: {:?}", e);
            bot.send_message(q.message.as_ref().unwrap().chat().id, format!("❌ Failed to sell: {}", e)).await?;
            return Ok(());
        }
    };
    if response.status().is_success() {
        println!("@handle_execute_sell_callback/ response is success");
        let json_response = response.json::<serde_json::Value>().await?;
        bot.send_message(q.message.as_ref().unwrap().chat().id, format!("https://solscan.io/tx/{}", json_response["transaction"].as_str().unwrap_or("N/A"))).await?;
    } else {
        let error_text = response.text().await?;
        println!("@handle_execute_sell_callback/ response is not success: {}", error_text);
        bot.send_message(q.message.as_ref().unwrap().chat().id, format!("Failed to sell: {}", error_text)).await?;
    }   
    Ok(())
}

/// Handle change gas lamports callback
/// 
/// # Description
/// 
/// Change the gas lamports on the tg bot by sending a message with force reply that will be checked by the message handler when
/// a reply with the value is sent
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
async fn handle_change_gas_lamports_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Enter the gas lamports")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Enter the gas lamports".to_string()), selective: false})
    .await?;
    Ok(())
}


/// Handle settings callback
/// 
/// # Description
/// 
/// Show the user settings on the tg bot
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_settings_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    if !user_has_settings(&pool, &q.from.id.to_string()).await? {
        create_user_settings_default(&pool, &q.from.id.to_string()).await?;
    }
    let user_settings = get_user_settings(&pool, &q.from.id.to_string()).await?;
    let keyboard = create_settings_keyboard(user_settings);
    let user = get_user(&pool, &q.from.id.to_string()).await?;
    bot.send_message(
        q.message.as_ref().unwrap().chat().id,
        format!(
        "<b>Settings:</b>\n\
        <code>{}</code>\n\n\
        GAS Fee and MEV Tip will affect transaction speed. MEV Tip will only be used when Anti-MEV is turned on. Please set GAS Fee and MEV Tip reasonably.\n\n\
        RAY Slippage:\n\
        When you initiate a trade, your purchase amount is fixed, and the number of tokens you receive will decrease if the price rises. (If you set the slippage to 50%, then you will get 50% of the tokens, your cost will be 1/50%=2, and you will buy the token at a maximum of 2 times the price.)\n\n\
        PUMP Slippage:\n\
        When you initiate a trade, the number of tokens you receive is fixed, and the amount of SOL you spend will increase if the price rises. (If your slippage is set to 60%, the maximum sol you will spend is 1/(1-60%) = 2.5x SOL)
        ", user.solana_address.expect("Solana address not found").as_str()
        )
    )
    .reply_markup(keyboard)
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
    Ok(())
}

/// Handle set custom gas callback
/// 
/// # Description
/// 
/// Set the custom gas lamports on the tg bot by sending a message with force reply that will be checked by the message handler when
/// a reply with the value is sent
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_set_custom_gas_callback(_data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, _pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Enter the gas fee")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Enter the gas fee in SOL".to_string()), selective: false})
    .await?;
    Ok(())
}

/// Handle add take profit callback
/// 
/// # Description
/// 
/// Add a take profit on the tg bot by sending a message with force reply that will be checked by the message handler when
/// a reply with the value is sent
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool    
/// 
/// # Returns
/// 
/// A result indicating the success of the operation    
async fn handle_add_take_profit_user_settings_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Send '<multiplier>,<%_token_position_amount_to_sell>' (eg: '1.5,100' that means if the price goes up 1.5x, sell 100% of the position)")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Send <multiplier to leave>, <% to sell> ".to_string()), selective: false})
    .await?;
    Ok(())
}

/// handle add stop loss callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_add_stop_loss_user_settings_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    bot.send_message(q.message.as_ref().unwrap().chat().id, "Send '<%down>,<%_token_position_amount_to_sell>' (eg: '10,100' that means if the price goes down 10%, sell 100% of the position)")
    .reply_markup(teloxide::types::ForceReply{force_reply: teloxide::types::True, input_field_placeholder: Some("Send <% down>,<% to sell> ".to_string()), selective: false})
    .await?;
    Ok(())
}


/// Handle delete take profit user settings callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_delete_take_profit_user_settings_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let user_tg_id = q.from.id.to_string();
    // data = "delete_take_profit:<multiplier>_<percentage_to_sell>"k
    let multiplier_and_percentage_to_sell= data.split(":").nth(1).unwrap_or("N/A");
    let multiplier = multiplier_and_percentage_to_sell.split("_").nth(0).unwrap_or("N/A").parse::<f64>().unwrap_or(0.0);
    println!("@handle_delete_take_profit_user_settings_callback/ multiplier: {:?}x", multiplier);
    let percentage_to_sell = multiplier_and_percentage_to_sell.split("_").nth(1).unwrap_or("N/A").parse::<f64>().unwrap_or(0.0);

    println!("@handle_delete_take_profit_user_settings_callback/ percentage_to_sell: {:?}%", percentage_to_sell);
    db::delete_user_settings_take_profit(&pool, (multiplier, percentage_to_sell), &user_tg_id).await?;
    println!("@handle_delete_take_profit_user_settings_callback/ take_profit removed from take_profit array in user settings");

    println!("@handle_delete_take_profit_user_settings_callback/ getting last token address from user settings");
    let last_token_address = get_user_last_sent_token(&pool, &user_tg_id).await?;
    println!("@handle_delete_take_profit_user_settings_callback/ last token address: {:?}", last_token_address);

    // To dsiplay buy menu again we need to send the last token address to the token_address_buy_info_handler
    println!("@handle_delete_take_profit_user_settings_callback/ sending last token address to token_address_buy_info_handler");
    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(msg)) = q.message.as_ref() {
        token_address_buy_info_handler(last_token_address.as_str(), bot, msg, pool).await?;
    }
    Ok(())
}


/// Handle delete stop loss user settings callback
/// 
/// # Arguments
/// 
/// * `data` - The callback data
/// * `bot` - The Telegram bot
/// * `q` - The callback query
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
async fn handle_delete_stop_loss_user_settings_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let user_tg_id = q.from.id.to_string();
    let multiplier_and_percentage_to_sell= data.split(":").nth(1).unwrap_or("N/A");
    let multiplier = multiplier_and_percentage_to_sell.split("_").nth(0).unwrap_or("N/A").parse::<f64>().unwrap_or(0.0);
    let percentage_to_sell = multiplier_and_percentage_to_sell.split("_").nth(1).unwrap_or("N/A").parse::<f64>().unwrap_or(0.0);
    db::delete_user_settings_stop_loss(&pool, (multiplier, percentage_to_sell), &user_tg_id).await?;
    Ok(())
}