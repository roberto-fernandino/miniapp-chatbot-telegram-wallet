use anyhow::Result;
use reqwest::Response;
use crate::db::get_user_settings_take_profits;
use crate::handlers::{TurnkeyUser, SwapSolRequest};
use chrono::{DateTime, Utc};
use teloxide::types::ChatId;
use crate::*;
use crate::db::{Call, PnlCall, ResponsePaylod, CallWithAth, create_user_with_tg_id_and_username};
use reqwest::Client;
use std::net::SocketAddr;
use handlers::{get_user_calls_handler, post_add_user_handler};
use crate::db;
use crate::utils::helpers::*;
use axum::Router;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use serde_json::Value;
use crate::utils::helpers::create_call_keyboard;
use crate::utils::helpers::get_pair_token_pair_and_token_address;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
pub type SafePool = Arc<Pool<Postgres>>;

/// Get user calls with ATH
/// 
/// # Arguments
/// 
/// * `req` - The request object
/// 
/// # Returns
/// 
/// * `String` - A json string with the calls and the ATH
pub async fn get_user_calls(user_tg_id: i64, pool: SafePool) -> Result<String> {
    let calls_without_ath = db::get_all_user_firsts_calls_by_user_tg_id(&pool, user_tg_id.to_string().as_str()).await?;
    let mut calls_with_ath = Vec::new();
    let user = db::get_user(&pool, user_tg_id.to_string().as_str()).await?;
    for call in calls_without_ath {
        // getting token information
        let chain = call.clone().chain;
        let scanner_response = get_scanner_search(&call.clone().token_address).await?;
        let ath = get_ath(call.clone().time.timestamp_millis(), &call.clone().token_address, &chain).await?;
        let ath_price = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let total_supply = scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

        let ath_mkt_cap = ath_price * total_supply;
        let multiplier;
        if ath_mkt_cap != 0.0 {
            multiplier = ath_price / call.clone().price.parse::<f64>().unwrap_or(0.0);
        } else {
            multiplier = 0.0;
        }
        
        let call_with_ath = CallWithAth {
            call: call.clone(),
            multiplier,
            ath_after_call: ath_mkt_cap,
        };
        calls_with_ath.push(call_with_ath);
    }
    println!("calls_with_ath: {:?}", calls_with_ath);
    let payload = ResponsePaylod { calls: calls_with_ath, username: user.username.clone().unwrap_or("Unknown username".to_string()) };
    println!("@get_user_calls/ Response payload: {:?}", payload);
    Ok(serde_json::to_string(&payload)?)
}

pub async fn start(bot: &teloxide::Bot, user_tg_id: &str, username: &str, chat_id: ChatId, pool: &SafePool) -> Result<()> {
    println!("@start");
  
    let is_user_registered_in_mini_app = db::is_user_registered_in_mini_app(&pool, &user_tg_id, &username).await?;
    println!("@start/ is_user_registered_in_mini_app: {:?}", is_user_registered_in_mini_app);
    if is_user_registered_in_mini_app {
        let user = db::get_user(&pool, user_tg_id).await?;
        println!("@start/ user: {:?}", user);
        let keyboard = create_main_menu_keyboard();
        let sol_balance = get_wallet_sol_balance(user.solana_address.clone().expect("Solana address not found").as_str()).await?;
        let sol_balance_usd = sol_to_usd(sol_balance.parse::<f64>().unwrap_or(0.0)).await?;
        println!("@start/ sol_balance: {:?} usd: {:?}", sol_balance, sol_balance_usd);
        println!("@start/ sending message");
        bot.send_message(
            chat_id,
            format!("Solana Wallet address:\n\
            <code>{}</code>\n\
            SOL Balance: <b>{} SOL (${})</b>\n\n\
            You can send SOL to this address or import your existing wallet.\n\n\
        💵 Join our Telegram group <a href=\"https://t.me/dexcelerateapp\">Dexcelerate Lounge</a> for the state-of-the-art trading platform.", user.solana_address.expect("Solana address not found"), sol_balance, sol_balance_usd)
    )
    .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(keyboard)
            .await?;
    } else {
        println!("@start/ sending message");
        bot.send_message(chat_id, "
        Welcome to Dexcelerate Telegram bot, the best way to manage your calls and your portfolio directly from your Telegram account.\n\n\
        You're not registered in the mini app yet.\n\n\
        Please, register in the mini app to use me.\n\n\
        You can either register in the mini app by clicking <a href=\"https://t.me/sj_copyTradebot/app\">here</a> or by clicking the <b>Wallet</b> button below close to the keyboard.\n\
        After registering in the mini app, you can start using our service by the app or by the bot here by using the /start command.
        ")
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    }
    Ok(())
}

pub async fn run_axum_server(pool: SafePool) {
       let app = Router::new()
       .route(
           "/user_calls/:tg_user_id",
           axum::routing::get(get_user_calls_handler),
       )
       .route(
        "/add_user",
        axum::routing::post(post_add_user_handler),
       )
       .with_state(pool);
   
       let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), 2020); // Updated to use SocketAddr::new
       println!("Axum server running on {:?}", addr);
       
   
       axum::Server::bind(&addr)
           .serve(app.into_make_service())
           .await
           .unwrap();
   }

/// Get the scanner search
/// 
/// # Arguments
/// 
/// * `pair_address` - The pair address
/// * `token_address` - The token address
/// 
/// # Returns
/// 
/// A JSON object containing the scanner search
pub async fn get_scanner_search(token_mint_address: &str) -> Result<Value> {
    let pair_token_pair_and_token_address = get_pair_token_pair_and_token_address(token_mint_address).await?;
    let pair_address = pair_token_pair_and_token_address["pairAddress"].as_str().unwrap_or("");
    let token_address = pair_token_pair_and_token_address["tokenAddress"].as_str().unwrap_or("");
    let chain = pair_token_pair_and_token_address["chainName"].as_str().unwrap_or("");
    let client = Client::new();
    let url = format!("https://api-rs.dexcelerate.com/scanner/{}/{}/{}/pair-stats", chain, pair_address, token_address);
    log::info!("url: {:?}", url);
    let response = client.get(url)
        .send()
        .await?;

    // Check if the response status is success
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch data: HTTP {}", response.status()));
    }

    // Read the response body as a string
    let body = response.text().await?;
    if body.is_empty() {
        log::error!("Received empty response body");
        return Err(anyhow::anyhow!("Received empty response body"));
    }

    // Parse the response body as JSON
    let json: Value = serde_json::from_str(&body)?;

    Ok(json)
}

/// Check the PNL call
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `mkt_cap` - The market cap of the token
/// * `token_address` - The address of the token
/// * `chat_id` - The chat ID of the user who made the call
/// 
/// # Returns
/// 
/// A Result containing the PNL call or an error
pub async fn check_pnl_call(pool: &SafePool, mkt_cap: &str, token_address: &str, chat_id: &str) -> Result<PnlCall> {
    let call: Call = db::get_first_call_by_token_address(&pool, token_address, chat_id).await?;
    
    let mkt_cap_i = call.mkt_cap.parse::<f64>().unwrap_or(0.0);
    let mkt_cap_n = mkt_cap.parse::<f64>().unwrap_or(0.0);
    
    let percent = if mkt_cap_i != 0.0 {
        ((mkt_cap_n - mkt_cap_i) / mkt_cap_i) * 100.0
    } else {
        0.0
    };
    let percent_str = format!("{:.2}", percent);
    Ok(PnlCall {
        call_id: call.id as i64,
        percent: percent_str,
        token_address: call.token_address,
        mkt_cap: mkt_cap.to_string(),
    })
}

/// Create and send pnl message
/// 
/// # Arguments
/// 
/// * `msg` - The message to check
/// * `bot` - The bot to send the message to
/// 
/// # Returns  
/// 
/// An Ok result
pub async fn pnl(msg: &teloxide::types::Message, bot: &teloxide::Bot, pool: &SafePool) -> Result<()> {
    let chat_id = msg.chat.id.to_string();
    let text = msg.text().unwrap().to_string(); 
    let token_address = text.split(" ").nth(1).unwrap_or("");
    // Check if the token address is valid
    if there_is_valid_solana_address(token_address) {
        // Get the pair address and token address
        match get_pair_token_pair_and_token_address(token_address).await {
            Ok(token_pair_and_token) => {
            let pair_address = token_pair_and_token["pairAddress"].as_str().unwrap_or("");
            let token_address_pair_address = token_pair_and_token["tokenAddress"].as_str().unwrap_or("");
            // scan the pair address and token address 
            match get_scanner_search(token_address).await {
                // if the scanner search is ok, get the mkt cap and symbol
                Ok(scanner_search) => {
                    let mkt_cap = scanner_search["pair"]["fdv"].as_str().unwrap_or("0");
                    let symbol = scanner_search["pair"]["token1Symbol"].as_str().unwrap_or("");
                    // check the pnl call
                    match check_pnl_call(&pool, mkt_cap, token_address_pair_address, chat_id.as_str()).await {
                        Ok(pnl_call) => {
                            // send the pnl message
                            bot.send_message(msg.chat.id, pnl_message(&pool, pnl_call, symbol, pair_address).await).parse_mode(teloxide::types::ParseMode::Html).await?;
                        }
                        Err(e) => {
                            log::error!("Failed to check PNL call: {:?}", e);
                            bot.send_message(msg.chat.id, "Failed to check PNL call").await?;
                        }
                    }
                }
                Err(_) => {
                    bot.send_message(msg.chat.id, "Failed to get scanner search").await?;
                } 
            } 
        }
        Err(_) => {}
        }
    } else {
        log::warn!("Received a message without text");
    }
    Ok(())
}

/// Get the holders of a token
/// 
/// # Arguments
/// 
/// * `address` - The address of the token
/// 
/// # Returns
///
/// A Result containing the holders or an error
pub async fn get_holders(address: &str) -> Result<Value> {
    let client = Client::new();
    let url = format!("https://api-rs.dexcelerate.com/token/SOL/{}/holders", address);
    let response = client.get(url)
        .send()
        .await?;
    let json: Value = response.json().await?;
    Ok(json)
}

/// Make a call
/// 
/// # Arguments
/// 
/// * `address` - The address of the token
/// * `bot` - The bot to send the message to
/// * `msg` - The message to send
/// 
/// # Returns
/// 
/// An Ok result
pub async fn call(address: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message, call_info_str: String, pool: &SafePool) -> Result<()> {
    // Get the pair address and token address
    match get_pair_token_pair_and_token_address(address).await {
        Ok(token_pair_and_token_address) => {
            let pair_address = token_pair_and_token_address["pairAddress"].as_str().unwrap_or("");
            let token_address = token_pair_and_token_address["tokenAddress"].as_str().unwrap_or("");
            let chain = token_pair_and_token_address["chainName"].as_str().unwrap_or("");
            // Check if the pair address and token address are valid
            if pair_address.is_empty() || token_address.is_empty() {
                log::error!("Invalid pair or token address");
                bot.send_message(msg.chat.id, "Invalid pair or token address").await?;
            } else {
                // Get the user ID
                let user_id = msg.clone().from.expect("Could not get the user from the message").id.to_string();
                let user_id_str = user_id.as_str();
                // Get the user
                let user = db::get_user(&pool, user_id_str).await;
                if user.is_err() {
                    create_user_with_tg_id_and_username(pool, user_id_str, Some(msg.from.clone().unwrap().username.clone().unwrap_or("Unknown".to_string()).as_str())).await?;
                    log::error!("User not found in database");
                }
                // If the user is not in the database, add them
                match user {
                    Err(_) => {
                        // User not found, attempt to add them
                        match db::create_user_with_tg_id_and_username(&pool, user_id_str, Some(msg.from.clone().expect("Could not get the user from the message").username.clone().unwrap_or("Unknown".to_string()).as_str())).await {
                            Ok(_) => {
                                log::info!("User added to database");
                            }
                            Err(e) => {
                                log::error!("Failed to add user to database: {:?}", e);
                            }
                        }
                    }
                    Ok(_) => {}
                }
                // Get the scanner search
                match get_scanner_search(address).await {
                    Ok(scanner_search) => {
                        // Parse datetime
                        let created_datetime_str = scanner_search["pair"]["pairCreatedAt"].as_str().unwrap_or("");
                        let datetime: DateTime<Utc> = created_datetime_str.parse().expect("Failed to parse datetime.");
                        let unix_timestamp_milis = datetime.timestamp_millis();

                        let ath_response = get_ath(unix_timestamp_milis, address, chain).await?;
                        let holders_response = get_holders(address).await?;
                        let chat_id = msg.clone().chat.id.to_string();
                        // Add the call to the database
                        let call_id = match db::add_call(
                            &pool, 
                            &chrono::Utc::now().to_rfc3339(),
                            user_id_str,
                            &scanner_search["pair"]["fdv"].as_str().unwrap_or("0"), 
                            token_address,
                            address,
                            &scanner_search["pair"]["token1Symbol"].as_str().unwrap_or(""),
                            &scanner_search["pair"]["pairPrice1Usd"].as_str().unwrap_or("0"),
                            chat_id.as_str(),
                            &msg.id.to_string(),
                            chain,
                            Some(msg.from.clone().unwrap().username.clone().unwrap_or("Unknown".to_string()).as_str())
                        ).await {
                            Ok(id) => {
                                id
                            }
                            Err(e) => {
                                log::error!("Failed to add call to database: {:?}", e);
                                0
                            }
                        };
                        
                        // BUTTONS MANAGEMENT
                        
                       
                        let keyboard = create_call_keyboard(call_info_str.as_str(), call_id.to_string().as_str(), token_address, user_id_str);
                        
                        
                        // Send the call message
                        bot.send_message(
                            msg.chat.id,
                            call_message(
                                &pool,
                                &ath_response,
                                &holders_response,
                                &scanner_search,
                                call_info_str,
                                user.unwrap(),
                                chain
                            ).await
                        )
                        .reply_parameters(teloxide::types::ReplyParameters { message_id: msg.id, chat_id: None, allow_sending_without_reply: Some(true), quote: None, quote_parse_mode: None, quote_entities: None, quote_position: None })
                        .reply_markup(keyboard)
                        .link_preview_options(teloxide::types::LinkPreviewOptions {
                            is_disabled: true,
                            url: None,
                            prefer_small_media: false,
                            prefer_large_media: false,
                            show_above_text: false,
                        })
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    }
                    Err(e) => {
                        log::error!("Failed to get scanner search: {:?}", e);
                        bot.send_message(msg.chat.id, "Failed to get scanner search").await?;
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Failed to get token pair and token: {:?}", e);
            bot.send_message(msg.chat.id, "Failed to get token pair and token").await?;
        }
    }
    Ok(())
}

/// Get the leaderboard
/// 
/// # Arguments
/// 
/// * `msg` - The message to get the leaderboard from
/// * `bot` - The bot to send the message to
/// 
/// # Returns
/// 
/// An Ok result
pub async fn leaderboard(msg: &teloxide::types::Message, bot: &teloxide::Bot, pool: &SafePool) -> Result<()> {
    let period = utils::helpers::check_period(msg.text().unwrap());
    let chat_id = msg.chat.id.to_string();
    let mut calls: Vec<Call> = vec![];
    let mut period_str: String = String::new();
    match period {
        Some(period) => {
           if period == "Hours" {
                let hours = utils::helpers::extract_hours(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{hours}h");
                calls = db::get_channel_calls_last_x_hours(&pool, chat_id.as_str(), hours as i32).await?;
                log::info!("Calls: {:?}", calls.len());
           }
           if period == "Days"  {
                let days = utils::helpers::extract_days(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{days}d");
                calls = db::get_channel_calls_last_x_days(&pool, chat_id.as_str(), days as i32).await?;
           }
           if period == "Months" {
                let months = utils::helpers::extract_months(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{months}m");
                calls = db::get_channel_calls_last_x_months(&pool, chat_id.as_str(), months as i32).await?;
            }
            if period == "Years"{
                let years = utils::helpers::extract_years(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{years}y");
                calls = db::get_user_calls_last_x_years(&pool, chat_id.as_str(), years as i32).await?;
           }
        }
        None => {
            period_str = "1d".to_string();
            calls = db::get_channel_calls_last_x_days(&pool, chat_id.as_str(), 1).await?;
        }
    }
    let mut lb = Vec::new();
    let mut unique_tokens = std::collections::HashSet::new();
    for call in calls {
        // Check if the token is already in the unique_tokens set
        if unique_tokens.insert(call.token_address.clone()) {
            // If the token is not in the set, add it and process the call
            let ath = get_ath(
                call.clone().time.timestamp_millis(),
                call.clone().token_address.as_str(),
                call.clone().chain.as_str()).await?;

            let ath_after_call = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

            let multiplier = ath_after_call / call.price.parse::<f64>().unwrap_or(0.0);

            let call_with_ath = CallWithAth {
                call: call,
                ath_after_call: ath_after_call,
                multiplier: multiplier,
            };
            lb.push(call_with_ath.clone());
        }
    }

    // Sort by multiplier in descending order
    lb.sort_by(|a, b| b.multiplier.partial_cmp(&a.multiplier).unwrap_or(std::cmp::Ordering::Equal));
    lb = lb.into_iter().take(10).collect();


    bot.send_message(msg.chat.id, leaderboard_message(lb, period_str, msg.chat.first_name().unwrap_or(""), &pool).await?)
    .reply_parameters(teloxide::types::ReplyParameters { message_id: msg.id, chat_id: None, allow_sending_without_reply: Some(true), quote: None, quote_parse_mode: None, quote_entities: None, quote_position: None })
    .parse_mode(teloxide::types::ParseMode::Html).await?;

    Ok(())
}

/// Get the best call for a user
/// 
/// # Arguments
/// 
/// * `user_tg_id` - The user Telegram ID
/// 
/// # Returns
/// 
/// An Option containing the best call as a CallWithAth struct
pub async fn best_call_user(user_tg_id: &str, pool: &SafePool) -> Result<Option<CallWithAth>> {
    let user_calls = db::get_all_calls_user_tg_id(&pool, user_tg_id).await?;
    let mut best_call: Option<CallWithAth> = None;
    let mut count = 0;
    for call in user_calls {
        let ath = get_ath(call.clone().time.timestamp_millis(), call.clone().token_address.as_str(), call.clone().chain.as_str()).await?;
        let ath_after_call = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let multiplier = ath_after_call / call.clone().price.parse::<f64>().unwrap_or(0.0);
        
        if count == 0 {
            best_call = Some(CallWithAth {
                call: call.clone(),
                ath_after_call: ath_after_call,
                multiplier: multiplier,
            });
        } else if let Some(ref current_best) = best_call {
            if multiplier > current_best.multiplier {
                best_call = Some(CallWithAth {
                    call: call.clone(),
                    ath_after_call: ath_after_call,
                    multiplier: multiplier,
                });
            }
        }
        count += 1;
    }
    Ok(best_call)
}

/// Get the user stats
/// 
/// # Arguments
/// 
/// * `user_tg_id` - The user Telegram ID
/// * `bot` - The bot to send the message to
/// * `msg` - The message to send the stats to
/// 
/// # Returns
/// 
/// An Ok result
pub async fn user_stats(user_tg_id: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message, pool: &SafePool) -> Result<()> {
    log::info!("User stats called");
    let user_calls = db::get_all_calls_user_tg_id(&pool, user_tg_id).await?;
    let user = db::get_user(&pool, user_tg_id).await?;
    let username = user.username.unwrap_or("Unknown".to_string());
    let calls_count = user_calls.len();
    let mut call_lb = Vec::new();   
    let mut seen_tokens = std::collections::HashSet::new(); // Track seen tokens
    for call in user_calls {
        if seen_tokens.contains(&call.clone().token_symbol.clone()) {
            continue; // Skip if token has already been processed
        }
        seen_tokens.insert(call.clone().token_symbol.clone()); // Mark token as seen

        let ath = get_ath(call.clone().time.timestamp_millis(), call.clone().token_address.as_str(), call.clone().chain.as_str()).await?;
        let ath_after_call = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let multiplier = ath_after_call / call.clone().price.parse::<f64>().unwrap_or(0.0);
        call_lb.push(CallWithAth {
            call: call.clone(),
            ath_after_call: ath_after_call,
            multiplier: multiplier,
        });
    }

    // Sort descending multiplier
    call_lb.sort_by(|a, b| b.multiplier.partial_cmp(&a.multiplier).unwrap_or(std::cmp::Ordering::Equal));

    let mut learderboard_string = String::new();
    let mut count = 1;
    // Create the user leaderboard string
    let mut percent_sum: f64 = 0.0;
    let mut hits = 0;
    for call in call_lb {
        let multiplier = call.multiplier;
        percent_sum += multiplier * 100.0;
        if multiplier > 2.0 {
            hits += 1;
        }
        if count == 1 {
            learderboard_string.push_str(&format!("👑🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ${} [<b>{:.2}x</b>]\n", count, call.call.token_symbol, multiplier));
        } else if count == 2 {
            learderboard_string.push_str(&format!("🥈🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ${} [<b>{:.2}x</b>]\n", count, call.call.token_symbol, multiplier));
        } else if count == 3 {
            learderboard_string.push_str(&format!("🥉🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ${} [<b>{:.2}x</b>]\n", count, call.call.token_symbol, multiplier));
        } else if multiplier < 1.2 {
            learderboard_string.push_str(&format!("😭🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ${} [<b>{:.2}x</b>]\n", count, call.call.token_symbol, multiplier));
        } else if count > 3 {
            learderboard_string.push_str(&format!("😎 🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ${} [<b>{:.2}x</b>]\n", count, call.call.token_symbol, multiplier));
        }
        count += 1;
    }
    let hit_rate = hits as f64 / count as f64 * 100.0;
    percent_sum -= 100.0;
    let multipliers_sum = percent_sum / 100.0;
    let multipliers_avg = percent_sum / 100.0 / count as f64;

    bot.send_message(msg.chat.id,user_stats_message(username, calls_count, multipliers_sum, multipliers_avg, learderboard_string, hit_rate)).parse_mode(teloxide::types::ParseMode::Html).await?;
    Ok(())
}

/// Sell token
/// 
/// # Description
/// This function sends a message to the user with the token info and a keyboard to sell the token
/// 
/// # Arguments
/// 
/// * `msg` - The message to sell the token
/// * `bot` - The bot to send the message to
/// * `pool` - The database pool
/// 
/// # Returns
/// 
/// An Ok result
pub async fn sell_token_page(msg: &teloxide::types::Message, bot: &teloxide::Bot, pool: &SafePool) -> Result<()> {
    let token_address = msg.text().unwrap().split("sell_token_").nth(1).unwrap_or("");
    let user_tg_id = msg.from.as_ref().unwrap().id.to_string();
    let user = db::get_user(&pool, &user_tg_id).await?;
    let scanner_response = get_scanner_search(token_address).await?;
    let token_symbol = scanner_response["pair"]["token1Symbol"].as_str().unwrap_or("");
    let token_price = scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let token_amount = get_token_amount(&user.solana_address.clone().unwrap_or("".to_string()), token_address).await?;
    let token_name = scanner_response["pair"]["token1Name"].as_str().unwrap_or("");
    let keyboard = create_sol_sell_swap_keyboard(&pool, user_tg_id.as_str(), token_address).await?;
    let sol_balance = get_wallet_sol_balance(&user.solana_address.clone().unwrap_or("".to_string())).await?.parse::<f64>().unwrap_or(0.0);
    let fdv = format_number(scanner_response["pair"]["fdv"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    
    bot.send_message(
        msg.chat.id, 
        format!("
       Swap ${token_symbol} 📈 - ({token_name})\n\
       • SOL Balance: {:.6}\n\
       • {token_symbol} Balance: {token_amount:.3}\n\
       • Price: ${token_price:.6} MC: ${fdv}\n\
        ", sol_balance)
    )
    .reply_markup(keyboard)
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;
    Ok(())
}


/// Execute a swap
/// 
/// # Arguments
/// 
/// * `pool` - The database pool
/// * `input_token` - The input token
/// * `output_token` - The output token
/// * `user_tg_id` - The user Telegram ID
/// 
/// # Returns
/// 
/// A Response
pub async fn execute_swap(pool: &SafePool, input_token: &str, output_token: &str, user_tg_id: String) -> Result<Response> {
    println!("@execute_swap: Starting execution");
    println!("@execute_swap: User Telegram ID: {}", user_tg_id);
    println!("@execute_swap: Input token: {}", input_token);
    println!("@execute_swap: Output token: {}", output_token);

    let user_settings = match db::get_or_create_user_settings(pool, &user_tg_id).await {
        Ok(settings) => {
            println!("@execute_swap: User settings retrieved successfully");
            settings
        },
        Err(e) => {
            println!("@execute_swap: Error getting or creating user settings: {:?}", e);
            return Err(anyhow::anyhow!("Failed to get or create user settings: {}", e));
        }
    };

    let user = match db::get_user(&pool, &user_tg_id).await {
        Ok(u) => {
            println!("@execute_swap: User retrieved successfully");
            u
        },
        Err(e) => {
            println!("@execute_swap: Error getting user: {:?}", e);
            return Err(anyhow::anyhow!("Failed to get user: {}", e));
        }
    };

    let input_token_amount: f64;
    if input_token == "So11111111111111111111111111111111111111112" {
        input_token_amount = user_settings.buy_amount.parse::<f64>().unwrap_or(0.2);
    } else {
        match get_token_amount(&user.solana_address.clone().unwrap_or("".to_string()), input_token).await {
            Ok(amount) => {
                println!("@execute_swap/ token_amount: {:?}", amount);
                input_token_amount = amount * user_settings.sell_percentage.parse::<f64>().expect("Sell percentage is not a number") / 100.0;
            },
            Err(e) => {
                println!("@execute_swap: Error getting token amount: {:?}", e);
                return Err(anyhow::anyhow!("Failed to get token amount: {}", e));
            }
        }
    }
    let slippage = user_settings.slippage_tolerance.parse::<f64>().unwrap_or(0.5);
    println!("@execute_swap: input_token_amount: {:?}, slippage: {:?}", input_token_amount, slippage);

    let solana_address = user.solana_address.clone();
    println!("@execute_swap: solana_address: {:?}", solana_address);
    
    let turnkey_user = TurnkeyUser {
        api_public_key: user.turnkey_info.api_public_key.clone().expect("API public key not found"),
        api_private_key: user.turnkey_info.api_private_key.clone().expect("API private key not found"),
        organization_id: user.turnkey_info.suborg_id.clone().expect("Suborg ID not found"),
        public_key: solana_address.clone().expect("Solana address not found").to_string(),
    };
    println!("@execute_swap: turnkey_user created successfully");

    println!("@execute_swap: Preparing request");
    let request: SwapSolRequest = if input_token == "So11111111111111111111111111111111111111112" {
        SwapSolRequest {
            user: turnkey_user,
            user_public_key: user.solana_address.clone().expect("Solana address not found").to_string(),
            priorization_fee_lamports: user_settings.gas_lamports as u64,
            output_mint: output_token.to_string(),
            input_mint: input_token.to_string(),
            amount: sol_to_lamports_u64(input_token_amount),
            slippage: slippage * 100.0,
        }
    } else {
        SwapSolRequest {
            user: turnkey_user,
            user_public_key: user.solana_address.clone().expect("Solana address not found").to_string(),
            priorization_fee_lamports: user_settings.gas_lamports as u64,
            output_mint: output_token.to_string(),
            input_mint: input_token.to_string(),
            amount: input_token_amount as u64,
            slippage,
        }
    };

    let client = reqwest::Client::new();
    let url = format!("{}/sol/swap", "http://solana_app:3030");
    println!("@execute_swap: Sending request to url: {:?}", url);

    // response.data = {transaction: "tx_hash"}
    let response = match client.post(url).json(&request).send().await {
        Ok(res) => {
            println!("@execute_swap: Response received successfully");
            res 
        },
        Err(e) => {
            println!("@execute_swap: Error sending request: {:?}", e);
            return Err(anyhow::anyhow!("Failed to send request: {}", e));
        }
    };
    println!("@execute_swap: input_token: {:?}", input_token);
    if response.status().is_success() { 
        
        // If the input token is SOL = buy
        if input_token == "So11111111111111111111111111111111111111112" {
            println!("@execute_swap: input_token is SOL, creating position");
            let take_profits = get_user_settings_take_profits(pool, &user_tg_id).await?;
            println!("@execute_swap: take_profits: {:?}", take_profits);

            let stop_losses = db::get_user_settings_stop_losses(pool, &user_tg_id).await?;
            println!("@execute_swap: stop_losses: {:?}", stop_losses);

            println!("@execute_swap: getting scanner response");
            let scanner_response = get_scanner_search(input_token).await?;

            let token_price = scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            println!("@execute_swap: token_price: {:?}", token_price);

            let token_amount_in_wallet = get_token_amount_in_wallet(&user.solana_address.clone().unwrap_or("".to_string()), input_token).await?;
            println!("@execute_swap: token_amount_in_wallet: {:?}", token_amount_in_wallet);

            println!("@execute_swap: inserting position");
            db::insert_position(pool, &user_tg_id, input_token, take_profits, stop_losses, token_amount_in_wallet, token_price).await?;
            println!("@execute_swap: position inserted");
        } 
    }
    Ok(response)
}


/// Execute a swap take profit
/// 
/// # Arguments
/// 
/// * `pool` - The database pool
/// * `user_tg_id` - The user Telegram ID
/// * `take_profit` - The take profit
/// * `input_token` - The input token
/// * `output_token` - The output token
/// 
/// # Returns
/// 
/// A Result indicating whether the swap take profit was executed successfully
pub async fn execute_swap_take_profit(pool: &SafePool, user_tg_id: String, take_profit: (f64, f64), input_token: &str, output_token: &str) -> Result<()> {
    println!("@execute_swap_take_profit: Starting execution");
    println!("@execute_swap_take_profit: User Telegram ID: {}", user_tg_id);
    println!("@execute_swap_take_profit: Input token: {}", input_token);
    println!("@execute_swap_take_profit: Output token: {}", output_token);

    // Get user
    let user = match db::get_user(&pool, &user_tg_id).await {
        Ok(u) => {
            println!("@execute_swap_take_profit: User retrieved successfully");
            u
        },
        Err(e) => {
            println!("@execute_swap_take_profit: Error getting user: {:?}", e);
            return Err(anyhow::anyhow!("Failed to get user: {}", e));
        }
    };
    let user_settings = db::get_or_create_user_settings(pool, &user_tg_id).await?;
    
    println!("@execute_swap_take_profit: Getting token amount in wallet");
    let token_amount_in_wallet = get_token_amount_in_wallet(&user.solana_address.clone().unwrap_or("".to_string()), input_token).await?;
    println!("@execute_swap_take_profit: Token amount in wallet: {:?}", token_amount_in_wallet);

    // Calculate the amount to sell
    println!("@execute_swap_take_profit: Calculating amount to sell");
    let amount_to_sell = token_amount_in_wallet * take_profit.1 / 100.0;
    println!("@execute_swap_take_profit: Amount to sell: {:?}", amount_to_sell);

    println!("@execute_swap_take_profit: Getting slippage tolerance");
    let slippage = user_settings.slippage_tolerance.parse::<f64>().unwrap_or(0.5);
    println!("@execute_swap_take_profit: Slippage: {:?}", slippage);

    println!("@execute_swap_take_profit: Creating turnkey user");
    let turnkey_user = TurnkeyUser {
        api_public_key: user.turnkey_info.api_public_key.clone().expect("API public key not found"),
        api_private_key: user.turnkey_info.api_private_key.clone().expect("API private key not found"),
        organization_id: user.turnkey_info.suborg_id.clone().expect("Suborg ID not found"),
        public_key: user.solana_address.clone().expect("Solana address not found").to_string(),
    };
    println!("@execute_swap_take_profit: turnkey_user created successfully");
    let request = SwapSolRequest {
            user: turnkey_user,
            user_public_key: user.solana_address.clone().expect("Solana address not found").to_string(),
            priorization_fee_lamports: user_settings.gas_lamports as u64,
            output_mint: output_token.to_string(),
            input_mint: input_token.to_string(),
            amount: amount_to_sell as u64,
            slippage,
        };

    println!("@execute_swap_take_profit: Sending request");
    let client = reqwest::Client::new();
    let url = format!("{}/sol/swap", "http://solana_app:3030");
    let response = client.post(url).json(&request).send().await?;
    println!("@execute_swap_take_profit: Response received: {:?}", response);

    Ok(())
}