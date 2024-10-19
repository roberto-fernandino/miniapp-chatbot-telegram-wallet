use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use teloxide::types::CallbackQuery;
use teloxide::types::Message;
use teloxide::Bot;
use crate::db::*;
use crate::utils::helpers::*;
use axum::extract::State;
use crate::*;
use crate::commands::*;
use teloxide::payloads::AnswerCallbackQuerySetters;
use teloxide::payloads::EditMessageTextSetters;
use teloxide::prelude::Requester;
use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
use reqwest::Url;
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

pub async fn handle_callback_refresh(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery, pool: SafePool) -> Result<()> {
    let call_id = data.strip_prefix("refresh:").unwrap_or_default();
    let call = crate::db::get_call_by_id(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
    let token_pair_token_address = get_pair_token_pair_and_token_address(&call.token_mint).await?;
    let pair_address = token_pair_token_address["pairAddress"].as_str().unwrap_or("");
    let token_address = token_pair_token_address["tokenAddress"].as_str().unwrap_or("");
    let chain = token_pair_token_address["chainName"].as_str().unwrap_or("");
    let scanner_response = get_scanner_search(
        pair_address,
        token_address,
        chain
    ).await?;

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
/// A tuple containing the address and the chain
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

/// Create the call buttons
/// 
/// # Arguments
/// 
/// * `call_info_str` - The call info string
/// * `call_id` - The call ID
/// * `mini_app_url` - The mini app URL
/// 
/// # Returns
/// 
/// A InlineKeyboardMarkup struct
pub fn create_call_keyboard(call_info_str: &str, call_id: &str, token_address: &str, user_tg_id: &str) -> InlineKeyboardMarkup {
    let swap_mini_app_url = Url::parse(&format!("https://t.me/sj_copyTradebot/app?start=tokenCA={}", token_address)).expect("Invalid Swap URL");
    let copy_mini_app_url = Url::parse(&format!("https://t.me/sj_copyTradebot/app?start=copyUser={}", user_tg_id)).expect("Invalid Copy Caller URL");
    log::info!("mini_app_url: {:?}", swap_mini_app_url);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = vec![];
    // Call info == "" means that is firt call
    if call_info_str == "" {
        buttons.push(
            vec![InlineKeyboardButton::callback("🔭 Just Scanning", format!("del_call:{}", call_id))
            ]
        );
    }
    buttons.push(
        vec![
            InlineKeyboardButton::url("💳 Buy now", swap_mini_app_url), 
            InlineKeyboardButton::url("Copy", copy_mini_app_url)
        ]
    );
    buttons.push(
        vec![
            InlineKeyboardButton::callback("🔄 Refresh", format!("refresh:{}", call_id)), 
            InlineKeyboardButton::callback("🆑 Clear", format!("clear_call:{}", call_id))
            ]
        );
    InlineKeyboardMarkup::new(buttons)
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


pub async fn handle_message(
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
                else if text.starts_with("/start") {
                    match start(&bot, &msg).await {
                        Ok(_) => (),
                        Err(e) => log::error!("Failed to start: {:?}", e),
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
