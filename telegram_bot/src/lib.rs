use crate::utils::helpers::check_period;
use teloxide::types::ReplyMarkup::InlineKeyboard;
use reqwest::Url;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, WebAppInfo};
use chrono::Duration;
use chrono::{DateTime, Utc};
use teloxide::prelude::*;
use sqlite::Connection;
use reqwest::Client;
use anyhow::Result;
use serde_json::Value;
mod utils;
mod db;
use db::{get_user_from_call, Call, User};
use regex::Regex;

/// Check if there's a valid solana address in a text
/// 
/// # Arguments
/// 
/// * `message` - The message to check
/// 
/// # Returns
/// 
/// A boolean indicating if the address is a valid solana address
pub fn there_is_valid_solana_address(message: &str) -> bool {
    let re = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();
    re.is_match(message)

}

/// Get the valid solana address from a text
/// 
/// # Arguments
/// 
/// * `text` - The text to get the valid solana address from
/// 
/// # Returns
/// 
/// An Option containing the valid solana address
pub fn get_valid_solana_address(text: &str) -> Option<String> {
    let re = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();
    if let Some(mat) = re.find(text) {
        Some(mat.as_str().to_string())
    } else {
        None
    }
}

/// Check if the message is a pnl command
/// 
/// # Arguments
/// 
/// * `message` - The message to check
/// 
/// # Returns
/// 
/// A boolean indicating if the message is a pnl command
pub fn is_pnl_command(message: &str) -> bool {
    message.starts_with("/pnl")
}

/// Get the pair address and token address
/// 
/// # Arguments
/// 
/// * `address` - The address to get the pair address and token address for
/// 
/// # Returns
/// 
/// A JSON object containing the pair address and token address
pub async fn get_pair_token_pair_and_token_address(address: &str) -> Result<Value> {
    let client = Client::new();
    let response = client.get(format!("https://api-rs.dexcelerate.com/pair/{}/pair-and-token", address))
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    Ok(json)
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
pub async fn get_scanner_search(pair_address: &str, token_address: &str) -> Result<Value> {
    let client = Client::new();
    let url = format!("https://api-rs.dexcelerate.com/scanner/SOL/{}/{}/pair-stats", pair_address, token_address);
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

/// Get the ATH of a token
/// 
/// # Arguments
/// 
/// * `timestamp` - The timestamp to get the ATH for
/// * `token_address` - The address of the token
/// 
/// # Returns
/// 
/// A JSON object containing the ATH
pub async fn get_ath(timestamp: i64, token_address: &str) -> Result<Value> {
    let url = format!("https://api-rs.dexcelerate.com/token/SOL/{}/ath?timestamp={}", token_address, timestamp);
    let client = Client::new();
    let response = client.get(url)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    Ok(json)
}

/// Format the number to a more readable format
/// 
/// # Arguments
/// 
/// * `num` - The number to format
/// 
/// # Returns
/// 
/// A string containing the formatted number
pub fn format_number(num: f64) -> String {
    if num >= 1_000_000.0 {
        format!("{:.1}M", num / 1_000_000.0)
    } else if num >= 1_000.0 {
        format!("{:.1}k", num / 1_000.0)
    } else {
        format!("{:.0}", num)
    }
}

/// Calculate the liquidity of a pair
/// 
/// # Arguments
/// 
/// * `pair0_reserve_usd` - The reserve of the first token in USD
/// * `pair1_reserve_usd` - The reserve of the second token in USD
/// 
/// # Returns
/// 
fn calculate_liquidity(pair0_reserve_usd: f64, pair1_reserve_usd: f64) -> f64 {
    pair0_reserve_usd + pair1_reserve_usd
}

/// Format the time ago of a datetime
/// 
/// # Arguments
/// 
/// * `datetime_str` - The datetime string to format
/// 
/// # Returns
/// 
/// A string containing the time ago
fn time_ago(datetime_str: &str) -> String {
    // Parse the input string into a DateTime<Utc> object
    let datetime = match DateTime::parse_from_rfc3339(datetime_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return "Invalid datetime format".to_string(),
    };

    // Get the current time in UTC
    let now = Utc::now();

    // Calculate the duration between the current time and the input time
    let duration = now.signed_duration_since(datetime);

    // Format the duration into a human-readable string
    format_duration(duration)
}

/// Format the age of a token
/// 
/// # Arguments
/// 
/// * `datetime_str` - The datetime string to format
/// 
/// # Returns
/// 
/// A string containing the age
fn age_token(datetime_str: &str) -> String {
    // Parse the input string into a DateTime<Utc> object
    let datetime = match DateTime::parse_from_rfc3339(datetime_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return "Invalid datetime format".to_string(),
    };

    // Get the current time in UTC
    let now = Utc::now();

    // Calculate the duration between the current time and the input time
    let duration = now.signed_duration_since(datetime);

    // Format the duration into a human-readable string
    format_age(duration)
}

/// Format the duration of anything that can be represented  as a Duration
/// 
/// # Arguments
/// 
/// * `duration` - The duration of the object
/// 
/// # Returns
/// 
/// A string containing the formatted duration
fn format_duration(duration: Duration) -> String {
    if duration.num_seconds() < 60 {
        format!("{}s ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("️{}h ago", duration.num_hours())
    } else if duration.num_days() < 365 {
        format!("️{}d ago", duration.num_days())
    } else {
        format!("️{}y ago", duration.num_days() / 365)
    }
}

/// Format the age of a token
/// 
/// # Arguments
/// 
/// * `duration` - The duration of the token
/// 
/// # Returns
/// 
/// A string containing the formatted age
fn format_age(duration: Duration) -> String {
    if duration.num_seconds() < 60 {
        format!("{}s", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 365 {
        format!("{}d", duration.num_days())
    } else {
        format!("{}y", duration.num_days() / 365)
    }
}

/// Generate the message for a call
/// 
/// # Arguments
/// 
/// * `ath_response` - The response from the API call to get the ATH
/// * `holders_response` - The response from the API call to get the holders
/// * `data` - The response from the API call to get the data
/// * `username` - The username of the user who made the call
/// 
/// # Returns
/// 
/// A string containing the formatted message
pub fn call_message(con: &Connection, ath_response: &Value, holders_response: &Value, scanner_response: &Value,  mut call_info_str: String, user: User) -> String {
    // Main info
    let pair_address = scanner_response["pair"]["pairAddress"].as_str().unwrap_or("");
    let token_symbol = scanner_response["pair"]["token1Symbol"].as_str().unwrap_or("N/A").to_uppercase();
    let token_usd_price = format!("{:.8}", scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0)).parse::<f64>().unwrap_or(0.0);
    let age = age_token(scanner_response["pair"]["pairCreatedAt"].as_str().unwrap_or(""));
    let circulating_supply = scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    
    // Stats
    let fdv = format_number(scanner_response["pair"]["fdv"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));

    // Ath 
    let ath = format_number(ath_response["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * circulating_supply);
    let ath_date = time_ago(ath_response["athTimestamp"].as_str().unwrap_or(""));
    
    // Liq
    let pair_reserves0 = scanner_response["pair"]["pairReserves0Usd"].as_str().unwrap_or("0");
    let pair_reserves1 = scanner_response["pair"]["pairReserves1Usd"].as_str().unwrap_or("0");
    let liquidity = format_number(calculate_liquidity(pair_reserves0.parse::<f64>().unwrap_or(0.0), pair_reserves1.parse::<f64>().unwrap_or(0.0)));
    
    let volume = format_number(scanner_response["pairStats"]["twentyFourHour"]["volume"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let mkt_cap = format_number(scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    log::info!("mkt_cap: {}", mkt_cap);

   //  If is first call, call_info_str com empty from @call function, so we need to add the first call info
    if call_info_str == "" {
        call_info_str = format!("🔥 First Call <a href=\"https://t.me/sj_copyTradebot?start=user_{}\"><i><b>{}</b></i></a> @ {}\n",user.id,  user.username, mkt_cap);
        call_info_str.push_str(&format!("└ Calls today: {} 🎉", db::get_qtd_calls_user_made_in_24hrs(&con, user.tg_id.as_str())));
    }   
    // One hour change
    let one_hour_first = scanner_response["pairStats"]["oneHour"]["first"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let one_hour_last = scanner_response["pairStats"]["oneHour"]["last"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let one_hour_change = if one_hour_first != 0.0 {
        ((one_hour_last / one_hour_first) - 1.0) * 100.0
    } else {
        0.0
    };
    let one_hour_change_str = format!("{:.2}", one_hour_change);
    // 24 hour change
    let twenty_four_hour_first = scanner_response["pairStats"]["twentyFourHour"]["first"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let twenty_four_hour_last = scanner_response["pairStats"]["twentyFourHour"]["last"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let twenty_four_hour_change = if twenty_four_hour_first != 0.0 {
        ((twenty_four_hour_last / twenty_four_hour_first) - 1.0) * 100.0
    } else {
        0.0
    };
    let twenty_four_hour_change_str = format_number(twenty_four_hour_change);
    
    // Info
    let buy_volume = format_number(scanner_response["pairStats"]["oneHour"]["buyVolume"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let buys = scanner_response["pairStats"]["oneHour"]["buys"].as_i64().unwrap_or(0);
    let sells = scanner_response["pairStats"]["oneHour"]["sells"].as_i64().unwrap_or(0);
    let lp = if scanner_response["pair"]["totalLockedRatio"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) > 0.0 { "🔥" } else { "🔴" };
    let token_address = scanner_response["pair"]["token1Address"].as_str().unwrap_or("");
    let verified = if scanner_response["pair"]["isVerified"].as_bool().unwrap_or(false) { "🟢" } else { "🔴" };
    let mut holders_str = String::new();
    let mut count = 0;
    for holder in holders_response["holders"].as_array().unwrap_or(&Vec::new()).iter() {
        if count == 5 {
            break;
        }
        let holder_address = holder["holderAddress"].as_str().unwrap_or("");
        let percent = holder["percent"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * 100.0;
        let percent_str = format!("{:.2}", percent);
        if count == 1 {
            holders_str.push_str(&format!("👥 TH: <a href=\"https://solscan.io/account/{holder_address}\">{percent_str}⋅</a>"));
        } 
        if count > 1 {
            holders_str.push_str(&format!("<a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a>⋅"));
        }
        if count == 4 {
            holders_str.push_str(&format!("<a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a>"));
        }
        count += 1;
    }
    // links
    let twitter = scanner_response["pair"]["linkTwitter"].as_str().unwrap_or("");
    let website = scanner_response["pair"]["linkWebsite"].as_str().unwrap_or("");   
    let telegram = scanner_response["pair"]["linkTelegram"].as_str().unwrap_or("");

    let mut links = String::new();
    if !twitter.is_empty() {
        links.push_str(&format!("<a href=\"{twitter}\">X</a> | "));
    }
    if !website.is_empty() {
        links.push_str(&format!("<a href=\"{website}\">WEB</a> | "));
    }
    if !telegram.is_empty() {
        links.push_str(&format!("<a href=\"{telegram}\">TG</a> | "));
    }

    let links_section = if links.len() > 0 {
        format!("🧰 More {links}\n\n")
    } else {
        String::new()
    };

    format!(
        "🟢 <a href=\"https://app.dexcelerate.com/terminal/SOL/{pair_address}\">{token_symbol}</a> [{mkt_cap}/{twenty_four_hour_change_str}%]\n\
        🌐 Solana @ Raydium\n\
        💰 USD: <code>${token_usd_price}</code>\n\
        💶 MCAP: <code>${mkt_cap}</code> \n\
        💎 FDV: <code>${fdv}</code>\n\
        💦 Liq: <code>${liquidity}</code>\n\
        📊 Vol: <code>${volume}</code> 🕰️ Age: {age} \n\
        ⛰️  ATH: <code>${ath}</code> <code>[{ath_date}]</code>\n\
        📉 1H: <code>{one_hour_change_str}%</code> . <code>${buy_volume}</code> 🅑 {buys} 🅢 {sells}\n\
        {holders_str}\n\
        LP: {lp} Mint:{verified}\n\
        {links_section}\
        <code>{token_address}</code>\n\n\
        {call_info_str}\n\n\
        🏆 <a href=\"https://app.dexcelerate.com/terminal/SOL/{token_address}\">See on #1 dex</a>\n\
        ")
}

/// Struct to hold the PNL call information
#[derive(Debug)]
pub struct PnlCall {
    pub percent: String,
    pub token_address: String,
    pub mkt_cap: String,
    pub call_id: u64,
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
pub fn check_pnl_call(connection: &Connection, mkt_cap: &str, token_address: &str, chat_id: &str) -> Result<PnlCall> {
     let call: Option<Call> = db::get_first_call_by_token_address(connection, token_address, chat_id);
     if let Some(call) = call {
        let mkt_cap_i = call.mkt_cap.parse::<f64>().unwrap_or(0.0);
        let mkt_cap_n = mkt_cap.parse::<f64>().unwrap_or(0.0);
        
        let percent = if mkt_cap_i != 0.0 {
            ((mkt_cap_n - mkt_cap_i) / mkt_cap_i) * 100.0
        } else {
            0.0
        };
        let percent_str = format!("{:.2}", percent);

        Ok(PnlCall {
            percent: percent_str,
            token_address: call.token_address,
            mkt_cap: mkt_cap.to_string(),
            call_id: call.id,
        })
    } else {
        Err(anyhow::anyhow!("Call not found"))
    }
}


/// Generate the PNL message
/// 
/// # Arguments
/// 
/// * `connection` - The database connection
/// * `pnl_call` - The PNL call information
/// * `symbol` - The symbol of the token
/// * `pair_address` - The address of the pair
/// 
/// # Returns
/// 
/// A string containing the formatted PNL message
pub fn pnl_message(connection: &Connection, pnl_call: PnlCall, symbol: &str, pair_address: &str) -> String {
    let call = db::get_call_by_id(connection, pnl_call.call_id).expect("Call not found");
    let user = db::get_user(connection, call.user_tg_id.as_str()).expect("User not found");
    let mkt_cap_called = format_number(call.mkt_cap.parse::<f64>().unwrap_or(0.0));
    let win_loss;
    let percent = pnl_call.percent.parse::<f64>().unwrap_or(0.0);
    let multiplier = if percent >= 0.0 {
        1.0 + (percent / 100.0)
    } else {
        1.0 / (1.0 - (percent / 100.0))
    };

    let multiplier_str = if multiplier >= 2.0 {
        format!("{:.2}x", multiplier)
    } else {
        format!("{:.2}", multiplier)
    };
    let uppercase_symbol = symbol.to_uppercase();
    if pnl_call.percent.starts_with("-") {
       win_loss = "🔴";
    } else {
        win_loss = "🟢";
    }
    
    format!("
    <a href=\"https://app.dexcelerate.com/terminal/SOL/{pair_address}\">${uppercase_symbol}</a>\n\
    {win_loss}  {multiplier_str}x\n\
    💰 Called at: <code>{}</code>\n\
    Called by: @{}\n\
    🏆 <a href=\"https://app.dexcelerate.com/terminal/SOL/{pair_address}\">See on #1 dex</a>\n\
    ",  mkt_cap_called, user.username)
    
}


/// Check the PNL call
/// 
/// # Arguments
/// 
/// * `msg` - The message to check
/// * `bot` - The bot to send the message to
/// 
/// # Returns  
/// 
/// An Ok result
pub async fn pnl(msg: &teloxide::types::Message, bot: &teloxide::Bot) -> Result<()> {
    let con = db::get_connection();
    let chat_id = msg.chat.id.to_string();
    let text = msg.text().unwrap().to_string(); 
    let token_address = text.split(" ").nth(1).unwrap_or("");
    // Check if the token address is valid
    if there_is_valid_solana_address(token_address) {
        // Get the pair address and token address
        match get_pair_token_pair_and_token_address(token_address).await {
            Ok(token_pair_and_token) => {
            let pair_address = token_pair_and_token["pairAddress"].as_str().unwrap_or("");
            let token_address = token_pair_and_token["tokenAddress"].as_str().unwrap_or("");
            // scan the pair address and token address 
            match get_scanner_search(pair_address, token_address).await {
                // if the scanner search is ok, get the mkt cap and symbol
                Ok(scanner_search) => {
                    let mkt_cap = scanner_search["pair"]["fdv"].as_str().unwrap_or("0");
                    let symbol = scanner_search["pair"]["token1Symbol"].as_str().unwrap_or("");
                    // check the pnl call
                    match check_pnl_call(&con, mkt_cap, token_address, chat_id.as_str()) {
                        Ok(pnl_call) => {
                            // send the pnl message
                            bot.send_message(msg.chat.id, pnl_message(&con, pnl_call, symbol, pair_address)).parse_mode(teloxide::types::ParseMode::Html).await?;
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
pub async fn call(address: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message, call_info_str: String) -> Result<()> {
    let con = db::get_connection();
    db::configure_db(&con);
    // Get the pair address and token address
    match get_pair_token_pair_and_token_address(address).await {
        Ok(token_pair_and_token) => {
            let pair_address = token_pair_and_token["pairAddress"].as_str().unwrap_or("");
            let token_address = token_pair_and_token["tokenAddress"].as_str().unwrap_or("");
            // Check if the pair address and token address are valid
            if pair_address.is_empty() || token_address.is_empty() {
                log::error!("Invalid pair or token address");
                bot.send_message(msg.chat.id, "Invalid pair or token address").await?;
            } else {
                // Get the user ID
                let user_id = msg.clone().from.unwrap().id.to_string();
                let user_id_str = user_id.as_str();
                // Get the user
                let mut user = db::get_user(&con, user_id_str);
                // If the user is not in the database, add them
                if user.is_none() {
                    match db::add_user(&con, user_id_str, msg.from.clone().unwrap().username.clone().unwrap_or("Unknown".to_string()).to_string().as_str()) {
                        Ok(_) => {
                            log::info!("User added to database");
                        }
                        Err(e) => {
                            log::error!("Failed to add user to database: {:?}", e);
                        }
                    }
                    user = db::get_user(&con, user_id_str);
                }
                // Get the scanner search
                match get_scanner_search(pair_address, token_address).await {
                    Ok(scanner_search) => {
                        // Parse datetime
                        let created_datetime_str = scanner_search["pair"]["pairCreatedAt"].as_str().unwrap_or("");
                        let datetime: DateTime<Utc> = created_datetime_str.parse().expect("Failed to parse datetime.");
                        let unix_timestamp_milis = datetime.timestamp_millis();

                        let ath_response = get_ath(unix_timestamp_milis, address).await?;
                        let holders_response = get_holders(address).await?;
                        let chat_id = msg.clone().chat.id.to_string();
                        // Add the call to the database
                        let call_id = match db::add_call(
                            &con, 
                            user_id_str, 
                            &scanner_search["pair"]["fdv"].as_str().unwrap_or("0"), 
                            token_address,
                            address,
                            &scanner_search["pair"]["token1Symbol"].as_str().unwrap_or(""),
                            &scanner_search["pair"]["pairPrice1Usd"].as_str().unwrap_or("0"),
                            chat_id.as_str(),
                            &msg.id.to_string()
                        ) {
                            Ok(id) => {
                                id
                            }
                            Err(e) => {
                                log::error!("Failed to add call to database: {:?}", e);
                                0
                            }
                        };
                        
                        // BUTTONS MANAGEMENT
                        
                       
                        let keyboard = create_call_keyboard(call_info_str.as_str(), call_id.to_string().as_str(), token_address);
                        
                        
                        // Send the call message
                        bot.send_message(
                            msg.chat.id,
                            call_message(
                                &con,
                                &ath_response,
                                &holders_response,
                                &scanner_search,
                                call_info_str,
                                user.expect("User not found")
                            )
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

/// Check if the message is a ranking command
/// 
/// # Arguments
/// 
/// * `message` - The message to check
/// 
/// # Returns
/// 
/// A boolean indicating if the message is a ranking command
pub fn is_ranking_command(message: &str) -> bool {
    message.starts_with("/ranking")
}


/// Struct to hold the call with the ATH after the call
/// 
/// # Fields
/// 
/// * `call` - The call
/// * `ath_after_call` - The ATH after the call
/// * `multiplier` - The multiplier
#[derive(Debug, Clone)]
pub struct CallWithAth {
    pub call: Call,
    pub ath_after_call: f64,
    pub multiplier: f64,
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
pub async fn leaderboard(msg: &teloxide::types::Message, bot: &teloxide::Bot) -> Result<()> {
    let period = check_period(msg.text().unwrap());
    let con = db::get_connection();
    let chat_id = msg.chat.id.to_string();
    let mut calls: Vec<Call> = vec![];
    let mut period_str: String = String::new();
    match period {
        Some(period) => {
           if period == "Hours" {
                let hours = utils::helpers::extract_hours(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{hours}h");
                calls = db::get_channel_calls_last_x_hours(&con, chat_id.as_str(), hours);
                log::info!("Calls: {:?}", calls.len());
           }
           if period == "Days"  {
                let days = utils::helpers::extract_days(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{days}d");
                calls = db::get_channel_calls_last_x_days(&con, chat_id.as_str(), days);
           }
           if period == "Months" {
                let months = utils::helpers::extract_months(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{months}m");
                calls = db::get_channel_calls_last_x_months(&con, chat_id.as_str(), months);
            }
            if period == "Years"{
                let years = utils::helpers::extract_years(msg.text().unwrap()).unwrap_or(0);
                period_str = format!("{years}y");
                calls = db::get_channel_calls_last_x_years(&con, chat_id.as_str(), years)
           }
        }
        None => {
            period_str = "1d".to_string();
            calls = db::get_channel_calls_last_x_days(&con, chat_id.as_str(), 1);
        }
    }
    let mut lb = Vec::new();
    let mut unique_tokens = std::collections::HashSet::new();
    for call in calls {
        // Check if the token is already in the unique_tokens set
        if unique_tokens.insert(call.token_address.clone()) {
            // If the token is not in the set, add it and process the call
            let ath = get_ath(
                utils::helpers::time_to_timestamp(call.time.as_str()).await, call.token_address.as_str()
            ).await?;

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


    bot.send_message(msg.chat.id, leaderboard_message(lb, period_str, msg.chat.first_name().unwrap_or(""))).parse_mode(teloxide::types::ParseMode::Html).await?;

    Ok(())
}

/// Get the user call count for a user
/// 
/// # Arguments
/// 
/// * `lb` - The leaderboard
/// * `user_id` - The user ID
/// 
/// # Returns
/// 
/// A usize representing the user call count
fn get_user_call_count_for_user(lb: &[CallWithAth], user_id: &str) -> usize {
    lb.iter()
        .filter(|call| call.call.user_tg_id == user_id)
        .count()
}

/// Get the user calls average multiplier
/// 
/// # Arguments
/// 
/// * `lb` - The leaderboard
/// * `user_tg_id` - The user Telegram ID
/// 
/// # Returns
/// 
/// A f64 representing the user calls average multiplier
fn get_user_average_multiplier(lb: &[CallWithAth], user_tg_id: String) -> f64 {
    let mut count = 0;
    for call in lb {
        if call.call.user_tg_id == user_tg_id {
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let total_multiplier: f64 = lb.iter()
        .map(|call| call.ath_after_call / call.call.price.parse::<f64>().unwrap_or(0.0))
        .sum();
    total_multiplier / count as f64
}

/// Create the leaderboard message
/// 
/// # Arguments
/// 
/// * `lb` - The leaderboard, as a vector of CallWithAth structs
/// * `days` - The number of days
/// * `channel_name` - The channel name
/// 
/// # Returns
/// 
/// A String representing the leaderboard message
pub fn leaderboard_message(lb: Vec<CallWithAth>, period_str: String, channel_name: &str) -> String {
    let con = db::get_connection();
    let mut learderboard_string = String::new();
    let mut count = 1;
    let mut mvp_string = String::new();
    for call in &lb {
        let multiplier = call.ath_after_call / call.call.price.parse::<f64>().unwrap_or(0.0);
        let user = db::get_user(&con, call.call.user_tg_id.as_str()).expect("User not found");
        let user_tg_id = user.tg_id;
        let username = user.username;
        let calls_count = get_user_call_count_for_user(&lb, call.call.user_tg_id.as_str());
        if count == 1 {
            let mvp_average_multiplier = get_user_average_multiplier(&lb, call.call.user_tg_id.to_string());
            mvp_string.push_str(&format!("👑 {}\n", channel_name));
            mvp_string.push_str(&format!("├ <code>MVP:</code>               <b>@{}</b>\n", username));
            mvp_string.push_str(&format!("├ <code>Period:</code>         <b>{}</b>\n", period_str));
            mvp_string.push_str(&format!("├ <code>Calls:</code>           <b>{}</b>\n", calls_count));
            mvp_string.push_str(&format!("└ <code>Return:</code>         <b>{:.2}x</b>\n", mvp_average_multiplier));
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
    format!("
    {mvp_string}\n\
    <blockquote>\
    {learderboard_string}\
    </blockquote>\n\n\
    • TOKEN PNL » /pnl <i>token_address</i>\n\
    • LEADERBOARD » /lb <i>days</i>\n\n\
    🏆 <a href=\"https://app.dexcelerate.com/scanner\">Watch and trade automatically with #1 dex</a>\n
    ")
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
pub async fn best_call_user(user_tg_id: &str) -> Result<Option<CallWithAth>> {
    let con = db::get_connection();
    let user_calls = db::get_all_calls_user_tg_id(&con, user_tg_id);
    let mut best_call: Option<CallWithAth> = None;
    let mut count = 0;
    for call in user_calls {
        let ath = get_ath(utils::helpers::time_to_timestamp(call.time.as_str()).await, call.token_address.as_str()).await?;
        let ath_after_call = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let multiplier = ath_after_call / call.price.parse::<f64>().unwrap_or(0.0);
        
        if count == 0 {
            best_call = Some(CallWithAth {
                call: call.clone(),
                ath_after_call: ath_after_call,
                multiplier: multiplier,
            });
        } else if let Some(ref current_best) = best_call {
            if multiplier > current_best.multiplier {
                best_call = Some(CallWithAth {
                    call: call,
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
pub async fn user_stats(user_tg_id: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message) -> Result<()> {
    log::info!("User stats called");
    let con = db::get_connection();
    let user_calls = db::get_all_calls_user_tg_id(&con, user_tg_id);
    let user = match db::get_user(&con, user_tg_id) {
        Some(user) => user,
        None => return Err(anyhow::Error::msg("User not found")),
    };
    let username = user.username;
    let calls_count = user_calls.len();
    let mut call_lb = Vec::new();   
    let mut seen_tokens = std::collections::HashSet::new(); // Track seen tokens
    for call in user_calls {
        if seen_tokens.contains(&call.token_symbol) {
            continue; // Skip if token has already been processed
        }
        seen_tokens.insert(call.token_symbol.clone()); // Mark token as seen

        let ath = get_ath(utils::helpers::time_to_timestamp(call.time.as_str()).await, call.token_address.as_str()).await?;
        let ath_after_call = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let multiplier = ath_after_call / call.price.parse::<f64>().unwrap_or(0.0);
        call_lb.push(CallWithAth {
            call: call,
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
    log::info!("Multipliers sum: {:?} multipleirs avg: {:?}", multipliers_sum, multipliers_avg);

    bot.send_message(msg.chat.id,user_stats_message(username, calls_count, multipliers_sum, multipliers_avg, learderboard_string, hit_rate)).parse_mode(teloxide::types::ParseMode::Html).await?;
    Ok(())
}

/// Create the user stats message
/// 
/// # Arguments
/// 
/// * `username` - The username
/// * `calls_count` - The number of calls
/// * `best_call_multiplier` - The best call multiplier
/// * `learderboard_string` - The leaderboard string
/// 
/// # Returns
/// 
/// A String representing the user stats message
pub fn user_stats_message(username: String, calls_count: usize, multipliers_sum: f64, multipliers_avg: f64, learderboard_string: String, hit_rate: f64) -> String {
    format!("
    🥷 @{username}\n\
    ├ Calls: <code>{calls_count}</code>\n\
    ├ Hit rate: <code>{hit_rate:.2}%</code>\n\
    └ Return: <code>{multipliers_sum:.2}x</code> ({multipliers_avg:.2}x avg)\n\n\
    <blockquote>\
    {learderboard_string}
    </blockquote>\n\
    ")
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
pub async fn handle_callback_del_call(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery) -> Result<()> {
    log::info!("Deleting call...");
    // Extract the call ID
    let con = db::get_connection();
    let user_tg_id =  query.from.id.to_string();
    let call_id = data.strip_prefix("del_call:").unwrap_or_default();
    let call_user = get_user_from_call(&con, call_id).expect("Could not get user from call.");
    let call = db::get_call_by_id(&con, call_id.parse::<u64>().unwrap()).expect("Could not get call.");
    if call_user.tg_id == user_tg_id {
        if let Ok(call_id_num) = call_id.parse::<u64>() {
            // Get the database connection
            // Attempt to delete the call
            match db::delete_call(&con, call_id_num) {
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
pub fn create_call_keyboard(call_info_str: &str, call_id: &str, token_address: &str) -> InlineKeyboardMarkup {
    let mini_app_url = Url::parse(&format!("https://t.me/sj_copyTradebot/app?start=tokenCA={}", token_address)).expect("Invalid URL");
    log::info!("mini_app_url: {:?}", mini_app_url);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = vec![];
    // Call info == "" means that is firt call
    if call_info_str == "" {
        buttons.push(vec![InlineKeyboardButton::callback("🔭 Just Scanning", format!("del_call:{}", call_id))]);
    }
    buttons.push(vec![InlineKeyboardButton::url("💳 Buy now", mini_app_url)]);
    buttons.push(vec![InlineKeyboardButton::callback("🔄 Refresh", format!("refresh:{}", call_id)), InlineKeyboardButton::callback("🆑 Clear", format!("clear:{}", call_id))]);
    InlineKeyboardMarkup::new(buttons)
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
pub fn create_call_keyboard_after_just_scanning(call_id: &str, token_address: &str) -> InlineKeyboardMarkup {
    let mini_app_url = Url::parse(&format!("https://t.me/sj_copyTradebot/app?start=tokenCA={}", token_address)).expect("Invalid URL");
    log::info!("mini_app_url: {:?}", mini_app_url);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = vec![];
    buttons.push(vec![InlineKeyboardButton::url("💳 Buy now", mini_app_url)]);
    buttons.push(vec![InlineKeyboardButton::callback("🔄 Refresh", format!("refresh:{}", call_id)), InlineKeyboardButton::callback("🆑 Clear", format!("clear:{}", call_id))]);
    InlineKeyboardMarkup::new(buttons)
}


pub async fn handle_callback_refresh(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery) -> Result<()> {
    let call_id = data.strip_prefix("refresh:").unwrap_or_default();
    let con = db::get_connection();
    let call = db::get_call_by_id(&con, call_id.parse::<u64>().unwrap()).expect("Could not get call.");
    let token_pair_token_address = get_pair_token_pair_and_token_address(&call.token_mint).await?;
    let pair_address = token_pair_token_address["pairAddress"].as_str().unwrap_or("");
    let token_address = token_pair_token_address["tokenAddress"].as_str().unwrap_or("");
    let scanner_response = get_scanner_search(
        pair_address,
        token_address
    ).await?;

    let created_datetime_str = scanner_response["pair"]["pairCreatedAt"].as_str().unwrap_or("");
    let datetime: DateTime<Utc> = created_datetime_str.parse().expect("Failed to parse datetime.");
    let unix_timestamp_milis = datetime.timestamp_millis();

    let ath_response = get_ath(unix_timestamp_milis, &call.token_mint).await?;
    log::info!("ath_response: {:?}", ath_response);
    let holders_response = get_holders(token_address).await?;
    let user = db::get_user(&con, call.user_tg_id.as_str()).expect("User not found");
    if let Some(ref message) = query.message {
        match message {
            teloxide::types::MaybeInaccessibleMessage::Regular(msg) => {
                let call_info_str = utils::helpers::get_call_info(&call.token_address.clone(), &con, msg);
                let call_message = call_message(
                    &con,
                    &ath_response,
                    &holders_response,
                    &scanner_response,
                    call_info_str,
                    user,
                );
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