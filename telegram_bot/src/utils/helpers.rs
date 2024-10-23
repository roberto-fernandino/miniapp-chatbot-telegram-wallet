use chrono::TimeZone;
use serde_derive::Serialize;
use reqwest::Url;
use chrono::TimeDelta;
use serde_json::Value;
use crate::db::*;
use regex::Regex;
use chrono::Duration;
use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
use reqwest::Client;
use sqlx::PgPool;
use anyhow::Result;
use crate::commands::get_scanner_search;
use teloxide::types::Message;
use chrono::{NaiveDateTime, Utc, DateTime};


/// Check the period from a lb command
/// 
/// # Arguments
/// 
/// * `text` - The command text to check
/// 
/// # Returns
/// 
/// An Option containing the period or None if no period is found
pub fn check_period(text: &str) -> Option<String> {
    if text.ends_with("d") || text.ends_with("D") {
        Some("Days".to_string())
    } else if text.ends_with("w") || text.ends_with("W") {
        Some("Weeks".to_string())
    } else if text.ends_with("m") || text.ends_with("M") {
        Some("Months".to_string())
    } else if text.ends_with("y") || text.ends_with("Y") {
        Some("Years".to_string())
    } else if text.ends_with("h") || text.ends_with("H") {
        Some("Hours".to_string())
    } else {
        None
    }
}

/// Check the period for a leaderboard command
/// 
/// # Arguments
/// 
/// * `text` - The command text to check
/// 
/// # Returns
/// 
/// An Option containing a tuple of the number and unit
pub fn check_period_for_leaderboard(text: &str) -> Option<(u32, &str)> {
    let re = regex::Regex::new(r"(\d+)([hdwy])$").unwrap();
    re.captures(text).and_then(|cap| {
        let number = cap.get(1)?.as_str().parse::<u32>().ok()?;
        let unit = cap.get(2)?.as_str();
        Some((number, unit))
    })
}

/// Check if the message is a lb command
/// 
/// # Arguments
/// 
/// * `message` - The message to check
/// 
/// # Returns
/// 
/// A boolean indicating if the message is a ranking command
pub fn is_lb_command(message: &str) -> bool {
    message.starts_with("/lb")
}


/// Extract the days from a lb command
/// 
/// # Arguments
/// 
/// * `command` - The command to extract the days from
/// 
/// # Returns
/// 
/// An Option containing the days or None
pub fn extract_days(command: &str) -> Option<u32> {
    let re = regex::Regex::new(r"/lb (\d+)d").unwrap();
    re.captures(command)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u32>().ok()))
        .flatten()
}

/// Extract the hours from a lb command
/// 
/// # Arguments
/// 
/// * `command` - The command to extract the hours from
/// 
/// # Returns
/// 
/// An Option containing the hours or None
pub fn extract_hours(command: &str) -> Option<u32> {
    let re = regex::Regex::new(r"/lb (\d+)h").unwrap();
    re.captures(command)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u32>().ok()))
        .flatten()
}

/// Extract the months from a lb command
/// 
/// # Arguments
/// 
/// * `command` - The command to extract the months from
/// 
/// # Returns
/// 
/// An Option containing the months or None
pub fn extract_months(command: &str) -> Option<u32> {
    let re = regex::Regex::new(r"/lb (\d+)m").unwrap();
    re.captures(command)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u32>().ok()))
        .flatten()
}

/// Extract the years from a lb command
/// 
/// # Arguments
/// 
/// * `command` - The command to extract the years from
/// 
/// # Returns
/// 
/// An Option containing the years or None
pub fn extract_years(command: &str) -> Option<u32> {
    let re = regex::Regex::new(r"/lb (\d+)y").unwrap();
    re.captures(command)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u32>().ok()))
        .flatten()
}

/// Convert a time string to a timestamp
/// 
/// # Arguments
/// 
/// * `time` - The time string
/// 
/// # Returns
/// 
/// A Result containing an i64 timestamp or a Box<dyn std::error::Error>
pub async fn async_time_to_timestamp(time: String) -> Result<i64, Box<dyn std::error::Error>> {
    // Parse the RFC 3339 formatted string into a DateTime<Utc>
    let datetime: DateTime<Utc> = DateTime::parse_from_rfc3339(&time)
        .map_err(|e| {
            eprintln!("Failed to parse datetime: {}", e);
            e // Propagate the error
        })?
        .with_timezone(&Utc); // Convert to UTC if necessary

    Ok(datetime.timestamp_millis())
}

pub fn time_to_timestamp(time: &str) -> i64 {
    let format = "%Y-%m-%d %H:%M:%S";
    let naive_datetime = NaiveDateTime::parse_from_str(time, format)
        .expect("Failed to parse datetime.");
    let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    datetime.timestamp_millis()
}

/// Get the call info
/// 
/// # Arguments
/// 
/// * `address` - The address to get the call info from
/// * `con` - The database connection
/// * `msg` - The message structure
/// 
/// # Returns
/// 
/// A string containing the call info
pub async fn get_call_info(address: &String, pool: &PgPool, msg: &Message) -> Result<String> {
    // First call info
    let mut call_info_str = String::new();
    let is_first_call = crate::db::is_first_call(&pool,address.as_ref(), msg.chat.id.to_string().as_str()).await?;
    let token_pair_and_token_address  = get_pair_token_pair_and_token_address(address).await?;
    let scanner_response = get_scanner_search(token_pair_and_token_address["pairAddress"].as_str().unwrap_or(""), token_pair_and_token_address["tokenAddress"].as_str().unwrap_or(""), token_pair_and_token_address["chainName"].as_str().unwrap_or("")).await?;
    let mkt_cap = scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    if !is_first_call {
        let chat_id_str = msg.chat.id.to_string();
        let first_call = {
            crate::db::get_first_call_token_chat(&pool, address.as_ref(), chat_id_str.as_str())
        };

        // Calculating the % change
        let first_call = first_call.await.expect("First call not found");
        let first_call_mkt_cap = first_call.mkt_cap.parse::<f64>().unwrap_or(0.0);
        let percentage_change = format_number(((mkt_cap - first_call_mkt_cap) / first_call_mkt_cap) * 100.0);


        // Calculating the age of the call
        let timestamp = first_call.time.timestamp_millis();
        let call_time = Utc.timestamp_millis_opt(timestamp).unwrap();
        let current_time = Utc::now();
        let time_delta: TimeDelta = current_time.signed_duration_since(call_time);
        let user_called_first = {
                crate::db::get_user(&pool, first_call.user_tg_id.as_str()).await.expect("User not found")
            };
        call_info_str.push_str(&format!("😈 <a href=\"https://t.me/sj_copyTradebot?start=user_{}\"><i><b>{}</b></i></a> @ {} <b>[{}%]</b> ({})", first_call.user_tg_id,  user_called_first.username.unwrap_or("Unknown".to_string()), format_number(first_call.mkt_cap.parse::<f64>().unwrap_or(0.0)), percentage_change, format_age(time_delta)));
    } 
    Ok(call_info_str)
}

pub fn is_start_command(text: &str) -> bool {
    text.starts_with("/start")
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



/// Create main menu keyboard
/// 
/// # Returns
/// 
/// A InlineKeyboardMarkup struct to be used in the ReplyMarkup on the bot
pub fn create_main_menu_keyboard() -> InlineKeyboardMarkup {
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = vec![];
    buttons.push(vec![
        InlineKeyboardButton::callback("Buy", "buy"),
        InlineKeyboardButton::callback("Sell", "sell")
    ]);
    buttons.push(vec![
        InlineKeyboardButton::callback("🛫 Copy Trade", "copy_trade")
    ]);
    buttons.push(vec![
        InlineKeyboardButton::callback("💴 Smart Wallet", "smart_wallet")
    ]);
    buttons.push(vec![
        InlineKeyboardButton::callback("Limit orders", "limit_orders"), 
        InlineKeyboardButton::callback("Auto sell", "auto_sell")
    ]);
    buttons.push(vec![
        InlineKeyboardButton::callback("Positions", "positions"),
        InlineKeyboardButton::callback("Wallet", "wallet"),
        InlineKeyboardButton::callback("Help", "help"),
    ]);

    buttons.push(vec![
        InlineKeyboardButton::callback("Settings", "settings"),
        InlineKeyboardButton::callback("💰 Referrals", "referrals"),
    ]);

    InlineKeyboardMarkup::new(buttons)
}

/// Create the swap keyboard
/// 
/// # Returns
/// 
/// A InlineKeyboardMarkup struct to be used in the ReplyMarkup on the bot
pub async fn create_sol_swap_keyboard(pool: &PgPool, user_tg_id: &str) -> InlineKeyboardMarkup {
    let user_has_settings = check_if_user_has_settings(&pool, user_tg_id).await.expect("Failed to check if user has settings");
    if !user_has_settings {
        create_user_settings_default(&pool, user_tg_id).await.expect("Failed to create user settings");
    }
    let user_settings = get_user_settings(&pool, user_tg_id).await.expect("User settings not found");
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = vec![];
    
    buttons.push(vec![
        InlineKeyboardButton::callback("← Back", "back"),
        InlineKeyboardButton::callback("Smart Money", "smart_money"),
        InlineKeyboardButton::callback("↻ Refresh", "refresh"),
    ]);


    let swap_or_limit = user_settings.swap_or_limit.as_str();
    buttons.push(vec![
        InlineKeyboardButton::callback(
            if swap_or_limit == "swap" { "✅ Swap" } else { "Swap" },
            "toggle_swap_limit:swap"
        ),
        InlineKeyboardButton::callback(
            if swap_or_limit == "limit" { "✅ Limit Orders" } else { "Limit Orders" },
            "toggle_swap_limit:limit"
        ),
    ]);

    let buy_amount = user_settings.buy_amount.as_str();
    let global_amounts = vec!["0.2", "0.5", "1", "2", "5"];
    let buy_amounts = vec!["0.2", "0.5", "1"];
    let row1 = buy_amounts.iter().map(|&amount| {
        let is_selected = user_settings.buy_amount == amount;
        InlineKeyboardButton::callback(
            if is_selected { format!("✅ Buy {} SOL", amount) } else { format!("Buy {} SOL", amount) },
            format!("amount:{}", amount)
        )
    }).collect::<Vec<_>>();
    buttons.push(row1);
    

    let buy_amounts2 = vec!["2", "5"];
    let mut row2 = buy_amounts2.iter().map(|&amount| {
        InlineKeyboardButton::callback(format!("Buy {} SOL", amount), format!("amount:{}", amount))
    }).collect::<Vec<_>>();

    if !global_amounts.contains(&buy_amount)  {
        row2.push(InlineKeyboardButton::callback(format!("✅ Buy {} 🖌 SOL", buy_amount), "amount:custom"));
    } else {
        row2.push(InlineKeyboardButton::callback("Buy X SOL 🖌 ", "amount:custom"));
    }
    buttons.push(row2);
    
    let slippage = user_settings.slippage_tolerance.parse::<f64>().unwrap_or(0.18);
    buttons.push(vec![
        InlineKeyboardButton::callback(if slippage == 0.18 { "✅ 18% Slippage" } else { "18% Slippage" }, "_"),
        InlineKeyboardButton::callback(if slippage != 0.18 { format!("✅ {}% Slippage 📝", slippage * 100.0) } else { "X Slippage 🖌".to_string() }, "set_custom_slippage")
    ]);
    
    buttons.push(vec![
        InlineKeyboardButton::callback("Buy", format!("buy:{}", buy_amount)),
    ]);

    InlineKeyboardMarkup::new(buttons)
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
/// * `num - The number to format
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
pub fn calculate_liquidity(pair0_reserve_usd: f64, pair1_reserve_usd: f64) -> f64 {
    pair0_reserve_usd + pair1_reserve_usd
}

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
    let liquidity: String = format_number(calculate_liquidity(pair_reserves0.parse::<f64>().unwrap_or(0.0), pair_reserves1.parse::<f64>().unwrap_or(0.0)));
    
    let volume = format_number(scanner_response["pairStats"]["twentyFourHour"]["volume"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let mkt_cap: String = format_number(scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    log::info!("mkt_cap: {}", mkt_cap);

   //  If is first call, call_info_str com empty from @call function, so we need to add the first call info
    if call_info_str == "" {
        call_info_str = format!("🔥 First Call <a href=\"https://t.me/sj_copyTradebot?start=user_{}\"><i><b>{}</b></i></a> @ {}\n",user.id,  user.username.unwrap_or("N/A".to_string()), mkt_cap);
        call_info_str.push_str(&format!("└ Calls today: {} 🎉", crate::db::get_qtd_calls_user_made_in_24hrs(&pool, user.tg_id.as_str()).await.unwrap_or(0)));
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
        ⛰️ ATH: <code>${ath}</code> <code>[{ath_date}]</code>\n\
        🚀 1H: <code>{one_hour_change_str}%</code> . <code>${buy_volume}</code> 🅑 {buys} 🅢 {sells}\n\
        {holders_str}\n\
        LP: {lp} Mint:{verified}\n\
        {links_section}\
        <code>{token_address}</code>\n\n\
        {call_info_str}\n\n\
        🏆 <a href=\"https://app.dexcelerate.com/terminal/SOL/{token_address}\">See on #1 dex</a>\n\
        ")
}



/// Creates the pnl message
/// 
/// # Description
/// 
///  Format the message pretty
/// 
/// # Arguments
/// 
/// * `pool` - The database connection
/// * `pnl_call` - The PNL call information
/// * `symbol` - The symbol of the token
/// * `pair_address` - The address of the pair
/// 
/// # Returns
/// 
/// A string containing the formatted PNL message
pub async fn pnl_message(pool: &SafePool, pnl_call: PnlCall, symbol: &str, pair_address: &str) -> String {
    let call = crate::db::get_call_by_id(&pool, pnl_call.call_id as i64).await.expect("Call not found");
    let user = crate::db::get_user(&pool, call.user_tg_id.as_str()).await.expect("User not found");
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
        let user = crate::db::get_user(&pool, call.call.user_tg_id.as_str()).await?;
        let user_tg_id = user.tg_id;
        let username = user.username.unwrap_or("Unknown".to_string());
        let calls_count_user = crate::db::get_user_call_count_for_user_chat_with_period(&pool, call.call.user_tg_id.as_str(), call.call.chat_id.as_str(), period_str.as_str()).await?;
        let calls_count_chat = crate::db::get_chat_call_count_with_period(&pool, call.call.chat_id.as_str(), period_str.as_str()).await?;
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


/// Requests the SOL balance of a wallet in the solana_app connected to the node
/// 
/// # Arguments
/// 
/// * `address` - The address of the wallet
/// 
/// # Returns
/// 
/// A f64 representing the SOL balance
pub async fn get_wallet_sol_balance(address: &str) -> Result<String> {
    println!("@get_wallet_sol_balance/ address: {:?}", address);
    let client = reqwest::Client::new();
    let response = client.get(
        format!("http://solana_app:3030/get_wallet_sol_balance/{address}")
    )
    .send()
    .await?;
    let response_json = response.json::<serde_json::Value>().await?;
    println!("@get_wallet_sol_balance/ response_json: {:?}", response_json);
    let balance = response_json["balance"].as_f64().unwrap_or(0.0);
    println!("@get_wallet_sol_balance/ balance: {:?}", balance);
    Ok(balance.to_string())
}


pub async fn sol_to_usd(sol_amount: f64) -> Result<f64> {
    let client = reqwest::Client::new();
    let response = client.get(
        format!("https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd")
    )
    .send()
    .await?;
    let response_json = response.json::<serde_json::Value>().await?;
    Ok(response_json["solana"]["usd"].as_f64().unwrap_or(0.0) * sol_amount)
}

pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * 1_000_000_000.0) as u64
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

/// Create the positions message
/// 
/// # Arguments
/// 
/// * `user_tg_id` - The user Telegram ID
/// * `pool` - The database connection
/// 
/// # Returns
/// 
/// A String representing the positions message
pub async fn create_positions_message(user_tg_id: &str, pool: &SafePool) -> Result<String> {
    if crate::db::user_exists(pool, user_tg_id).await? {
        let user = crate::db::get_user(&pool, user_tg_id).await?;
        let solana_wallet_address = user.solana_address.expect("User has no solana address");
        let client = reqwest::Client::new();
        let response = client.get(
            format!("http://solana_app:3030/get_positions/{solana_wallet_address}")
        )
        .send()
        .await?;
        let sol_balance = get_wallet_sol_balance(&solana_wallet_address).await?;
        let sol_balance_usd = sol_to_usd(sol_balance.parse::<f64>().unwrap_or(0.0)).await?;
        let response_json = response.json::<serde_json::Value>().await?;
        let sol_token_balance = response_json["total_sol_balance"].as_f64().unwrap_or(0.0);
        let sol_token_balance_usd = sol_to_usd(sol_token_balance).await?;
        let mut tokens_balance_str = String::new();
        for token in response_json["tokens"].as_array().unwrap_or(&Vec::new()).iter() {
            let mint = token["mint"].as_str().unwrap_or("N/A");
            let sol_amount = token["sol_amount"].as_f64().unwrap_or(0.0);
            let token_ui_amount = token["token_ui_amount"].as_f64().unwrap_or(0.0);
            tokens_balance_str.push_str(&format!("<code>{}</code>: {} SOL - {}\n", mint, sol_amount, token_ui_amount));
        }
        Ok(format!(
            "<b>Manage tokens:</b>\n\
            SOL Balance: <b> {} SOL (${})</b>\n\
            Token balance: <b> {} SOL</b> (${})\n\
            {tokens_balance_str}
            ", sol_balance, sol_balance_usd, sol_token_balance, sol_token_balance_usd)
        )
    } else {
        Err(anyhow::anyhow!("User not found"))
    }
}
