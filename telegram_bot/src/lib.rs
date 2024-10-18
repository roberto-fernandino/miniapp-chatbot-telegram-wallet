use std::sync::{Arc, Mutex};
use serde::Serialize;
use reqwest::Url;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use chrono::Duration;
use chrono::{DateTime, Utc};
use teloxide::prelude::*;
use reqwest::Client;
use anyhow::Result;
use serde_json::Value;
mod utils;
mod db;
use crate::utils::helpers::time_to_timestamp;
use db::{add_user, get_all_user_firsts_calls_by_user_tg_id, get_user_from_call, Call, User};
use regex::Regex;
use sqlx::Pool;
use sqlx::Postgres;

pub type SafePool = Arc<Pool<Postgres>>;

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

/// Check if there's a valid eth address in a text
/// 
/// # Arguments
/// 
/// * `message` - The message to check
/// 
/// # Returns
/// 
/// A boolean indicating if the address is a valid eth address
pub fn there_is_valid_eth_address(message: &str) -> bool {
    let re = Regex::new(r"(?i)0x[0-9a-f]{40}").unwrap();
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


/// Get valid eth address from a text
/// 
/// # Arguments
/// 
/// * `text` - The text to get the valid eth address from
/// 
/// # Returns
/// 
/// An Option containing the valid eth address
pub fn get_valid_eth_address(text: &str) -> Option<String> {
    let re = Regex::new(r"(?i)0x[0-9a-f]{40}").unwrap();
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
pub async fn get_scanner_search(pair_address: &str, token_address: &str, chain: &str) -> Result<Value> {
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
pub async fn get_ath(timestamp: i64, token_address: &str, chain: &str) -> Result<Value> {
    let url = format!("https://api-rs.dexcelerate.com/token/{}/{}/ath?timestamp={}", chain, token_address, timestamp);
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
pub fn format_age(duration: Duration) -> String {
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
pub async fn call_message(pool: &SafePool, ath_response: &Value, holders_response: &Value, scanner_response: &Value,  mut call_info_str: String, user: User, chain: &str) -> String {
    // Main info
    let pair_address = scanner_response["pair"]["pairAddress"].as_str().unwrap_or("");
    let token_symbol = scanner_response["pair"]["token1Symbol"].as_str().unwrap_or("N/A").to_uppercase();
    let token_name = scanner_response["pair"]["token1Name"].as_str().unwrap_or("N/A");
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
        call_info_str = format!("🔥 First Call <a href=\"https://t.me/sj_copyTradebot?start=user_{}\"><i><b>{}</b></i></a> @ {}\n",user.id,  user.username.unwrap_or("N/A".to_string()), mkt_cap);
        call_info_str.push_str(&format!("└ Calls today: {} 🎉", db::get_qtd_calls_user_made_in_24hrs(&pool, user.tg_id.as_str()).await.unwrap_or(0)));
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
    let buys = format_number(scanner_response["pairStats"]["oneHour"]["buys"].as_i64().unwrap_or(0) as f64);
    let sells = format_number(scanner_response["pairStats"]["oneHour"]["sells"].as_i64().unwrap_or(0) as f64);
    let lp = if scanner_response["pair"]["totalLockedRatio"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) > 0.0 { "🔥" } else { "🔴" };
    let token_address = scanner_response["pair"]["token1Address"].as_str().unwrap_or("");
    let verified = if scanner_response["pair"]["isVerified"].as_bool().unwrap_or(false) { "🟢" } else { "🔴" };
    let top_10_holders_percentage = format_number(holders_response["holders"]
    .as_array()
    .unwrap_or(&Vec::new())
    .iter()
    .skip(1)
    .take(10)  // Take only the first 10 elements
    .map(|h| h["percent"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * 100.0) // Multiply by 100 to convert to percentage
    .sum::<f64>());
    let holders_str = holders_response["holders"]
    .as_array()
    .unwrap_or(&Vec::new())
    .iter()
    .skip(1)
    .take(5)
    .enumerate()
    .map(|(i, holder)| {
        let holder_address = holder["holderAddress"].as_str().unwrap_or("");
        let percent = holder["percent"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * 100.0;
        let percent_str = format!("{:.1}", percent);
        
        match i {
            0 => format!("👥 TH: <a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a>⋅"),
            1..4 => format!("<a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a>⋅"),
            4 => format!("<a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a> <b>({:.2}%)</b>", top_10_holders_percentage),
            _ => String::new()
        }
    })
    .collect::<Vec<String>>()
    .join("");

    // links management
    let twitter = scanner_response["pair"]["linkTwitter"].as_str().unwrap_or("");
    let website = scanner_response["pair"]["linkWebsite"].as_str().unwrap_or("");   
    let telegram = scanner_response["pair"]["linkTelegram"].as_str().unwrap_or("");

    let mut links = String::new();
    let mut link_added = false;
    
    if !twitter.is_empty() {
        links.push_str(&format!("<a href=\"{twitter}\">X</a>"));
        link_added = true;
    }
    if !website.is_empty() {
        if link_added {
            links.push_str(" ⋅ ");
        }
        links.push_str(&format!("<a href=\"{website}\">WEB</a>"));
        link_added = true;
    }
    if !telegram.is_empty() {
        if link_added {
            links.push_str(" ⋅ ");
        }
        links.push_str(&format!("<a href=\"{telegram}\">TG</a>"));
    }
    let token_spawned_at_str: String;
    if chain == "solana" {
        token_spawned_at_str = if token_address.contains("pump") { "💊".to_string() } else { "🟣".to_string() };
    } else if chain == "ethereum" {
        token_spawned_at_str = "🔷".to_string();
    } else {
        token_spawned_at_str = "❗️".to_string();
    }
    let links_section = if links.len() > 0 {
        format!("🧰 More {links}\n\n")
    } else {
        String::new()
    };

    format!(
        "{token_spawned_at_str} <a href=\"https://app.dexcelerate.com/terminal/SOL/{pair_address}\">{token_name}</a> <b>[{mkt_cap}/{twenty_four_hour_change_str}%] ${token_symbol}</b>\n\
        🌐 Solana @ Raydium\n\
        💰 USD: <code>${token_usd_price}</code>\n\
        💶 MCAP: <code>${mkt_cap}</code> \n\
        💎 FDV: <code>${fdv}</code>\n\
        💦 Liq: <code>${liquidity}</code> \n\
        📊 Vol: <code>${volume}</code> 🕰️ Age: <code>{age}</code> \n\
        ⛰️  ATH: <code>${ath}</code> <code>[{ath_date}]</code>\n\
        🚀 1H: <code>{one_hour_change_str}%</code> . <code>${buy_volume}</code> 🅑 {buys} 🅢 {sells}\n\
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
        percent: percent_str,
        token_address: call.token_address,
        mkt_cap: mkt_cap.to_string(),
        call_id: call.id as u64,
    })
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
pub async fn pnl_message(pool: &SafePool, pnl_call: PnlCall, symbol: &str, pair_address: &str) -> String {
    let call = db::get_call_by_id(&pool, pnl_call.call_id as i64).await.expect("Call not found");
    let user = db::get_user(&pool, call.user_tg_id.as_str()).await.expect("User not found");
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
    ",  mkt_cap_called, user.username.unwrap_or("N/A".to_string()))
    
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
pub async fn pnl(msg: &teloxide::types::Message, bot: &teloxide::Bot, pool: SafePool) -> Result<()> {
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
            let chain = token_pair_and_token["chainName"].as_str().unwrap_or("");
            // scan the pair address and token address 
            match get_scanner_search(pair_address, token_address, chain).await {
                // if the scanner search is ok, get the mkt cap and symbol
                Ok(scanner_search) => {
                    let mkt_cap = scanner_search["pair"]["fdv"].as_str().unwrap_or("0");
                    let symbol = scanner_search["pair"]["token1Symbol"].as_str().unwrap_or("");
                    // check the pnl call
                    match check_pnl_call(&pool, mkt_cap, token_address, chat_id.as_str()).await {
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
                let user_id = msg.clone().from.unwrap().id.to_string();
                let user_id_str = user_id.as_str();
                // Get the user
                let user = db::get_user(&pool, user_id_str).await;
                if user.is_err() {
                    add_user(pool, user_id_str, Some(msg.from.clone().unwrap().username.clone().unwrap_or("Unknown".to_string()).as_str())).await?;
                    log::error!("User not found in database");
                }
                // If the user is not in the database, add them
                match user {
                    Err(_) => {
                        // User not found, attempt to add them
                        match db::add_user(&pool, user_id_str, Some(msg.from.clone().unwrap().username.clone().unwrap_or("Unknown".to_string()).as_str())).await {
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
                match get_scanner_search(pair_address, token_address, chain).await {
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
pub async fn leaderboard(msg: &teloxide::types::Message, bot: &teloxide::Bot, pool: SafePool) -> Result<()> {
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
                utils::helpers::async_time_to_timestamp(call.clone().time).await.expect("Failed to parse datetime."), 
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
pub async fn leaderboard_message(lb: Vec<CallWithAth>, period_str: String, channel_name: &str, pool: &SafePool) -> Result<String> {
    let mut learderboard_string = String::new();
    let mut count = 1;
    let mut hits = 0;
    let mut mvp_string = String::new();
    let mut mvp_average_multiplier = 0.0;
    for call in &lb {
        let multiplier = call.ath_after_call / call.call.price.parse::<f64>().unwrap_or(0.0);
        let user = db::get_user(&pool, call.call.user_tg_id.as_str()).await?;
        let user_tg_id = user.tg_id;
        let username = user.username.unwrap_or("Unknown".to_string());
        let calls_count_user = db::get_user_call_count_for_user_chat_with_period(&pool, call.call.user_tg_id.as_str(), call.call.chat_id.as_str(), period_str.as_str()).await?;
        let calls_count_chat = db::get_chat_call_count_with_period(&pool, call.call.chat_id.as_str(), period_str.as_str()).await?;
        if multiplier > 2.0 {
            hits += 1;
        }
        if count == 1 {
            mvp_average_multiplier = get_user_average_multiplier(&lb, call.call.user_tg_id.to_string());
            mvp_string.push_str(&format!("👑 {}\n", channel_name));
            mvp_string.push_str(&format!("├ <code>MVP:</code>               <b>@{}</b>\n", username));
            mvp_string.push_str(&format!("├ <code>Period:</code>         <b>{}</b>\n", period_str));
            mvp_string.push_str(&format!("├ <code>Calls:</code>           <b>{}</b>\n", calls_count_chat));
            
            learderboard_string.push_str(&format!("👑🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ({calls_count_user}): ${} [<b>{:.1}x</b>]\n", count, call.call.token_symbol, multiplier));
        }
        if count == 2 {
            learderboard_string.push_str(&format!("🥈🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ({calls_count_user}): ${} [<b>{:.1}x</b>]\n", count, call.call.token_symbol, multiplier));
        } else if count == 3 {
            learderboard_string.push_str(&format!("🥉🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ({calls_count_user}): ${} [<b>{:.1}x</b>]\n", count, call.call.token_symbol, multiplier));
        } else if multiplier < 1.5  && count > 3{
            learderboard_string.push_str(&format!("😭🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ({calls_count_user}): ${}\n", count, call.call.token_symbol));
        } else if count > 3 && multiplier > 2.0 {
            learderboard_string.push_str(&format!("😎 🟣 <b>{}</b>:<a href=\"https://t.me/sj_copyTradebot?start=user_{user_tg_id}\"><i><b>{username}</b></i></a> ({calls_count_user}): ${} [<b>{:.1}x</b>]\n", count, call.call.token_symbol, multiplier));
        }
        count += 1;
        if count == 10 {
            break;
        }
    }
    let hit_rate = hits as f64 / count as f64 * 100.0;
    mvp_string.push_str(&format!("├ <code>Hit rate:</code>      <b>{:.2}%</b>\n", hit_rate));
    mvp_string.push_str(&format!("└ <code>Return:</code>         <b>{:.2}x</b>\n", mvp_average_multiplier));
    Ok(format!("
    {mvp_string}\n\
    <blockquote>\
    {learderboard_string}\
    </blockquote>\n\n\
    • TOKEN PNL » /pnl <i>token_address</i>\n\
    • LEADERBOARD » /lb <i>Period</i>\n\n\
    🏆 <a href=\"https://app.dexcelerate.com/scanner\">Watch and trade automatically with #1 dex</a>\n
    "))
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
        let ath = get_ath(utils::helpers::async_time_to_timestamp(call.clone().time).await.expect("Failed to parse datetime."), call.clone().token_address.as_str(), call.clone().chain.as_str()).await?;
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

        let ath = get_ath(utils::helpers::async_time_to_timestamp(call.clone().time).await.expect("Failed to parse datetime."), call.clone().token_address.as_str(), call.clone().chain.as_str()).await?;
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
pub async fn handle_callback_del_call(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery, pool: SafePool) -> Result<()> {
    log::info!("Deleting call...");
    // Extract the call ID
    let user_tg_id =  query.from.id.to_string();
    let call_id = data.strip_prefix("del_call:").unwrap_or_default();
    let call_user = get_user_from_call(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
    let call = db::get_call_by_id(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
    if call_user.tg_id == user_tg_id {
        if let Ok(call_id_num) = call_id.parse::<i64>() {
            // Attempt to delete the call
            match db::delete_call(&pool, call_id_num).await {
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
    buttons.push(vec![InlineKeyboardButton::url("💳 Buy now", mini_app_url), InlineKeyboardButton::callback("Copy", format!("copy:{}", call_id))]);
    buttons.push(vec![InlineKeyboardButton::callback("🔄 Refresh", format!("refresh:{}", call_id)), InlineKeyboardButton::callback("🆑 Clear", format!("clear:{}", call_id))]);
    InlineKeyboardMarkup::new(buttons)
}


pub async fn handle_callback_refresh(data: String, bot: &teloxide::Bot, query: &teloxide::types::CallbackQuery, pool: SafePool) -> Result<()> {
    let call_id = data.strip_prefix("refresh:").unwrap_or_default();
    let call = db::get_call_by_id(&pool, call_id.parse::<i64>().expect("Could not parse call id, maybe the value is not a number or to big.")).await?;
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
    let user = db::get_user(&pool, call.user_tg_id.as_str()).await?;
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

pub async fn start(bot: &teloxide::Bot, msg: &teloxide::types::Message) -> Result<()> {
    bot.send_message(msg.chat.id, "Welcome to the bot! Please use the keyboard to interact with the bot.").await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CallHistoryUser
 {
    pub call: Call,
    pub multiplier: f64,
    pub ath: f64,
}


/// Get user calls with ATH
/// 
/// # Arguments
/// 
/// * `req` - The request object
/// 
/// # Returns
/// 
/// * `String` - A json string with the calls and the ATH
pub async fn get_user_calls(req: tide::Request<()>, pool: SafePool) -> tide::Result<String> {
    println!("Getting user calls...");
    let user_tg_id = req.param("tg_user_id")?.to_string();
    println!("user_tg_id: {}", user_tg_id);

   
        let calls_without_ath = get_all_user_firsts_calls_by_user_tg_id(&pool, user_tg_id.as_str()).await?;
        let mut calls_with_ath = Vec::new();
        for call in calls_without_ath {
            let ath = get_ath(utils::helpers::async_time_to_timestamp(call.clone().time).await.expect("Failed to parse datetime."), &call.clone().token_address, &call.clone().chain).await?;
            let ath_price = ath["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let multiplier = ath_price / call.clone().price.parse::<f64>().unwrap_or(0.0);
            let call_with_ath = CallHistoryUser {
                call: call.clone(),
                multiplier,
                ath: ath_price,
            };
            calls_with_ath.push(call_with_ath);
        }
        println!("calls_with_ath: {:?}", calls_with_ath);
    Ok(serde_json::to_string(&calls_with_ath)?)
}

