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
                    match start(&bot, &msg, &pool).await {
                        Ok(_) => (),
                        Err(e) => log::error!("Failed to start: {:?}", e),
                    }
                }
                else if there_is_valid_solana_address(text) || there_is_valid_eth_address(text) {
                    match buy_sol_token_address_handler(text, &bot, &msg, &pool).await {
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
        else if data == "buy" {
            match handle_buy_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to buy: {:?}", e),
            }
        }
        else if data.starts_with("buy_") {
            match handle_execute_buy_sol_callback(data.to_string(), &bot, &query, &pool).await {
                Ok(_) => (),
                Err(e) => log::error!("Failed to buy: {:?}", e),
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
        match update_user(&pool, User { id: user_id, username: user.username, tg_id: user.tg_id, turnkey_info: user.turnkey_info, solana_address: user.solana_address, eth_address: user.eth_address }).await {
            Ok(_) => println!("@add_user/ user updated in the db."),
            Err(e) => {
                println!("@add_user/ error updating user in the db: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not update user in the db").into_response();
            }
        }
    } else {
        match add_user(&pool, user.clone()).await {
            Ok(_) => println!("@add_user/ user added to the db."),
            Err(e) => {
                println!("@add_user/ error adding user to the db: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not add user to the db").into_response();
            }
        }
        match upsert_user_settings(&pool, &user.tg_id, "0.18", "10", "swap").await {
            Ok(_) => println!("@add_user/ user settings added to the db."),
            Err(e) => {
                println!("@add_user/ error adding user settings to the db: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Could not add user settings to the db").into_response();
            }
        }
    }
    (StatusCode::OK, "User added/updated in the db.").into_response()
}

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

#[derive(Serialize, Deserialize)]
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
}

pub async fn handle_execute_buy_sol_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let sol_amount = data.strip_prefix("buy_")
        .and_then(|s| s.strip_suffix("_sol"))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let user_id = q.from.id.to_string();
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user = get_user(&pool, &user_id).await?;
    let solana_address = user.solana_address;
    let turnkey_user = TurnkeyUser {
        api_public_key: user.turnkey_info.api_public_key,
        api_private_key: user.turnkey_info.api_private_key,
        organization_id: user.turnkey_info.suborg_id,
        public_key: solana_address,
    };


    Ok(())


}
pub async fn buy_sol_token_address_handler(text: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message, pool: &SafePool) -> Result<()> {
    let user = get_user(&pool, &msg.from.as_ref().unwrap().id.to_string()).await?;
    let sol_balance = get_wallet_sol_balance(&user.solana_address).await?;
    let token_address= address_handler(text).await?;
    let keyboard = create_sol_swap_keyboard(token_address.as_str(), &pool, user.id.to_string().as_str()).await;
    let token_pair_and_token_address  = get_pair_token_pair_and_token_address(token_address.as_str()).await?;
    let scanner_response = get_scanner_search(token_pair_and_token_address["pairAddress"].as_str().unwrap_or(""), token_pair_and_token_address["tokenAddress"].as_str().unwrap_or(""), token_pair_and_token_address["chainName"].as_str().unwrap_or("")).await?;


    // token info
    let token_symbol = scanner_response["pair"]["token1Symbol"].as_str().unwrap_or("N/A").to_uppercase();
    let token_name = scanner_response["pair"]["token1Name"].as_str().unwrap_or("N/A");
    let token_usd_price = format!("{:.8}", scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0)).parse::<f64>().unwrap_or(0.0);
    let mkt_cap: String = format_number(scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let lp = scanner_response["pair"]["burnedAmount"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let renounced = if scanner_response["pair"]["renounced"].as_bool().unwrap_or(false) { "✓" } else { "x" };

    bot.send_message(
        msg.chat.id, 
        format!(
            "Swap ${token_symbol}📈 - ({token_name})\n\
            <code> {token_address}</code> (Tap to copy)\n\
            • SOL Balance: {sol_balance} ($not_implemented_yet) [TransferSOL]\n\
            • Price: <b>${token_usd_price}</b> LP: <b>${lp}</b> MC: <b>${mkt_cap}</b>\n\
            • Renounced: {renounced} Burnt: {lp}
            "
        )
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}

/// Handle toggle swap or limit callback
async fn  handle_toggle_swap_limit_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let user_tg_id = q.from.id.to_string();
    let msg_id = q.message.as_ref().unwrap().id();
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let limit_or_swap = data.strip_prefix("toggle_swap_limit:").unwrap_or("swap");
    set_user_swap_or_limit(&pool, &user_tg_id, limit_or_swap).await?;
    let token_address = data.split(":").nth(1).unwrap_or("");
    let keyboard = create_sol_swap_keyboard(token_address, &pool, &user_tg_id).await;
    bot.edit_message_reply_markup(chat_id, msg_id)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}


async fn handle_set_buy_amount_callback(data: String, bot: &teloxide::Bot, q: &teloxide::types::CallbackQuery, pool: &SafePool) -> Result<()> {
    let token_address = data.split(":").nth(1).unwrap_or("");
    let user_tg_id = q.from.id.to_string();
    let msg_id = q.message.as_ref().unwrap().id();
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let buy_amount = data.strip_prefix("set_buy_amount:").unwrap_or("0.2");
    set_user_buy_amount(&pool, &user_tg_id, buy_amount).await?;
    let keyboard = create_sol_swap_keyboard(token_address, &pool, &user_tg_id).await;
    bot.edit_message_reply_markup(chat_id, msg_id)
    .reply_markup(keyboard)
    .await?;
    Ok(())
}