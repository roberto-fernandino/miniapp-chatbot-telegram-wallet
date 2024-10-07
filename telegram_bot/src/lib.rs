use regex::Regex;
use chrono::Duration;
use chrono::{DateTime,Utc, NaiveDateTime};
use teloxide::prelude::*;
use sqlite::Connection;
use reqwest::Client;
use anyhow::Result;
use serde_json::Value;
mod db;
use db::Call;

pub fn there_is_valid_solana_address(address: &str) -> bool {
    let re = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();
    re.is_match(address)
}

pub fn get_valid_solana_address(text: &str) -> Option<String> {
    let re = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();
    if let Some(mat) = re.find(text) {
        Some(mat.as_str().to_string())
    } else {
        None
    }
}

pub fn is_pnl_command(message: &str) -> bool {
    message.starts_with("/pnl")
}


pub async fn get_pair_token_pair_and_token_address(address: &str) -> Result<Value> {
    let client = Client::new();
    let response = client.get(format!("https://api-rs.dexcelerate.com/pair/{}/pair-and-token", address))
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    Ok(json)
}

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

pub async fn get_ath(timestamp: i64, token_address: &str) -> Result<Value> {
    let url = format!("https://api-rs.dexcelerate.com/token/SOL/{}/ath?timestamp={}", token_address, timestamp);
    let client = Client::new();
    let response = client.get(url)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    Ok(json)
}

 fn format_number(num: f64) -> String {
    if num >= 1_000_000.0 {
        format!("{:.1}M", num / 1_000_000.0)
    } else if num >= 1_000.0 {
        format!("{:.1}k", num / 1_000.0)
    } else {
        format!("{:.0}", num)
    }
}

fn calculate_liquidity(pair0_reserve_usd: f64, pair1_reserve_usd: f64) -> f64 {
    pair0_reserve_usd + pair1_reserve_usd
}

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

pub fn call_message(ath_response: &Value, holders_response: &Value, data: &Value, username: Option<String>) -> String {
    // Main info
    let pair_address = data["pair"]["pairAddress"].as_str().unwrap_or("");
    let token_symbol = data["pair"]["token1Symbol"].as_str().unwrap_or("N/A").to_uppercase();
    let token_usd_price = format!("{:.8}", data["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0)).parse::<f64>().unwrap_or(0.0);
    let age = age_token(data["pair"]["pairCreatedAt"].as_str().unwrap_or(""));
    let circulating_supply = data["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);


    // Stats
    let fdv = format_number(data["pair"]["fdv"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));

    // Ath 
    let ath = format_number(ath_response["athTokenPrice"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * circulating_supply);
    let ath_date = time_ago(ath_response["athTimestamp"].as_str().unwrap_or(""));

    // Liq
    let pair_reserves0 = data["pair"]["pairReserves0Usd"].as_str().unwrap_or("0");
    let pair_reserves1 = data["pair"]["pairReserves1Usd"].as_str().unwrap_or("0");
    let liquidity = format_number(calculate_liquidity(pair_reserves0.parse::<f64>().unwrap_or(0.0), pair_reserves1.parse::<f64>().unwrap_or(0.0)));

    let volume = format_number(data["pairStats"]["twentyFourHour"]["volume"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let mkt_cap = format_number(data["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * data["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));

    // One hour change
    let one_hour_first = data["pairStats"]["oneHour"]["first"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let one_hour_last = data["pairStats"]["oneHour"]["last"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let one_hour_change = if one_hour_first != 0.0 {
        ((one_hour_last / one_hour_first) - 1.0) * 100.0
    } else {
        0.0
    };
    let one_hour_change_str = format!("{:.2}", one_hour_change);
    // 24 hour change
    let twenty_four_hour_first = data["pairStats"]["twentyFourHour"]["first"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let twenty_four_hour_last = data["pairStats"]["twentyFourHour"]["last"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let twenty_four_hour_change = if twenty_four_hour_first != 0.0 {
        ((twenty_four_hour_last / twenty_four_hour_first) - 1.0) * 100.0
    } else {
        0.0
    };
    let twenty_four_hour_change_str = format!("{:.2}", twenty_four_hour_change);

    // Info
    let buy_volume = format_number(data["pairStats"]["oneHour"]["buyVolume"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0));
    let buys = data["pairStats"]["oneHour"]["buys"].as_i64().unwrap_or(0);
    let sells = data["pairStats"]["oneHour"]["sells"].as_i64().unwrap_or(0);
    let lp = if data["pair"]["totalLockedRatio"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) > 0.0 { "🔥" } else { "🔴" };
    let token_address = data["pair"]["token1Address"].as_str().unwrap_or("");
    let username = username.unwrap_or("Not found".to_string());
    let verified = if data["pair"]["isVerified"].as_bool().unwrap_or(false) { "🟢" } else { "🔴" };
    let mut holders_str = String::new();
    let mut count = 0;
    for holder in holders_response["holders"].as_array().unwrap_or(&Vec::new()).iter() {
        if count == 5 {
            break;
        }
        let holder_address = holder["holderAddress"].as_str().unwrap_or("");
        let percent = holder["percent"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * 100.0;
        let percent_str = format!("{:.2}", percent);
        let token_balance = holder["balanceFormatted"].as_f64().unwrap_or(0.0);
        let usd_amount = token_balance * token_usd_price;
        let usd_amount_str = format_number(usd_amount);
        if count == 1 {
            holders_str.push_str(&format!("👥 TH: <a href=\"https://solscan.io/account/{holder_address}\">{percent_str}⋅</a>"));
        } 
        if count > 1{
            holders_str.push_str(&format!("<a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a>⋅"));
        }
        if count == 4 {
            holders_str.push_str(&format!("<a href=\"https://solscan.io/account/{holder_address}\">{percent_str}</a>"));
        }
        count += 1;
    }
    // links
    let twitter = data["pair"]["linkTwitter"].as_str().unwrap_or("");
    let website = data["pair"]["linkWebsite"].as_str().unwrap_or("");   
    let telegram = data["pair"]["linkTelegram"].as_str().unwrap_or("");

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
        "🟢 <a href=\"https://app.dexcelerate.com/terminal/SOL/{pair_address}\">{token_symbol}</a> [{mkt_cap}/{twenty_four_hour_change_str}%] 🔼\n\
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
        🪙 <code>{token_address}</code>\n\n\
        👨‍💼 @{username}\n\
        🏆 <a href=\"https://app.dexcelerate.com/terminal/SOL/{token_address}\">See on #1 dex</a>\n\
        "
    )
}

#[derive(Debug)]
pub struct PnlCall {
    pub percent: String,
    pub token_address: String,
    pub mkt_cap: String,
    pub call_id: u64,
}

 pub fn check_pnl_call(connection: &Connection, mkt_cap: &str, token_address: &str, chat_id: &str) -> Result<PnlCall> {
     let call: Option<Call> = db::get_call(connection, token_address, chat_id);
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


pub fn pnl_message(connection: &Connection, pnl_call: PnlCall, symbol: &str, pair_address: &str) -> String {
    let call = db::get_call_by_id(connection, pnl_call.call_id).expect("Call not found");
    let user = db::get_user(connection, call.user_tg_id.as_str()).expect("User not found");
    let mkt_cap_called = format_number(call.mkt_cap.parse::<f64>().unwrap_or(0.0));
    let mkt_cap_now = format_number(pnl_call.mkt_cap.parse::<f64>().unwrap_or(0.0));
    let win_loss;
    let percent = pnl_call.percent.parse::<f64>().unwrap_or(0.0);
    let percent_str = format!("{:.2}", percent);
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
    {win_loss} {percent_str}% | {multiplier_str}x\n\
    🪙 <code>{}</code>\n\n\
    💰 Mkt Cap called: <code>{}</code>\n\
    💰 Mkt Cap now: <code>{}</code>\n\n\
    🔥 Called by: @{}\n\
    ⏰ Time: {} UTC\n\n\
    🏆 <a href=\"https://app.dexcelerate.com/terminal/SOL/{pair_address}\">See on #1 dex</a>\n\
    ",  pnl_call.token_address, mkt_cap_called, mkt_cap_now, user.username, call.time)
    
}

pub async fn pnl(msg: &teloxide::types::Message, bot: &teloxide::Bot) -> Result<()> {
    let con = db::get_connection();
    let chat_id = msg.chat.id.to_string();
    let text = msg.text().unwrap().to_string(); // Clone the text
    let token_address = text.split(" ").nth(1).unwrap_or("");
    if there_is_valid_solana_address(token_address) {
        match get_pair_token_pair_and_token_address(token_address).await {
            Ok(token_pair_and_token) => {
            let pair_address = token_pair_and_token["pairAddress"].as_str().unwrap_or("");
            let token_address = token_pair_and_token["tokenAddress"].as_str().unwrap_or("");
            match get_scanner_search(pair_address, token_address).await {
                Ok(scanner_search) => {
                    let mkt_cap = scanner_search["pair"]["fdv"].as_str().unwrap_or("0");
                    let symbol = scanner_search["pair"]["token1Symbol"].as_str().unwrap_or("");
                    match check_pnl_call(&con, mkt_cap, token_address, chat_id.as_str()) {
                        Ok(pnl_call) => {
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

pub async fn get_holders(address: &str) -> Result<Value> {
    let client = Client::new();
    let url = format!("https://api-rs.dexcelerate.com/token/SOL/{}/holders", address);
    let response = client.get(url)
        .send()
        .await?;
    let json: Value = response.json().await?;
    Ok(json)
}

pub async fn call(address: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message) -> Result<()> {
    let con = db::get_connection();
    db::configure_db(&con);
    match get_pair_token_pair_and_token_address(address).await {
        Ok(token_pair_and_token) => {
            let pair_address = token_pair_and_token["pairAddress"].as_str().unwrap_or("");
            let token_address = token_pair_and_token["tokenAddress"].as_str().unwrap_or("");
            
            if pair_address.is_empty() || token_address.is_empty() {
                log::error!("Invalid pair or token address");
                bot.send_message(msg.chat.id, "Invalid pair or token address").await?;
            } else {
                let user_id = msg.clone().from.unwrap().id.to_string();
                let user_id_str = user_id.as_str();
                let user = db::get_user(&con, user_id_str);
                if user.is_none() {
                    match db::add_user(&con, user_id_str, msg.from.clone().unwrap().username.clone().unwrap_or("Unknown".to_string()).to_string().as_str()) {
                        Ok(_) => {
                            log::info!("User added to database");
                        }
                        Err(e) => {
                            log::error!("Failed to add user to database: {:?}", e);
                        }
                    }
                }
                match get_scanner_search(pair_address, token_address).await {
                    Ok(scanner_search) => {
                        // Parse datetime
                        let created_datetime_str = scanner_search["pair"]["pairCreatedAt"].as_str().unwrap_or("");
                        let datetime: DateTime<Utc> = created_datetime_str.parse().expect("Failed to parse datetime.");
                        let unix_timestamp_milis = datetime.timestamp_millis();

                        let ath_response = get_ath(unix_timestamp_milis, address).await?;
                        let holders_response = get_holders(address).await?;

                        let chat_id = msg.clone().chat.id.to_string();
                        match db::add_call(
                            &con, 
                            user_id_str, 
                            &scanner_search["pair"]["fdv"].as_str().unwrap_or("0"), 
                            token_address, 
                            &scanner_search["pair"]["token1Symbol"].as_str().unwrap_or(""),
                            &scanner_search["pair"]["pairPrice1Usd"].as_str().unwrap_or("0"),
                            chat_id.as_str(),
                        ) {
                            Ok(_) => {
                                log::info!("Call added to database");
                            }
                            Err(e) => {
                                log::error!("Failed to add call to database: {:?}", e);
                            }
                        }
                        bot.send_message(
                            msg.chat.id,
                            call_message(
                                &ath_response,
                                &holders_response,
                                &scanner_search,
                                Some(msg.from.clone().unwrap().username.clone().unwrap_or("".to_string()))
                            )
                        )
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


pub fn is_ranking_command(message: &str) -> bool {
    message.starts_with("/ranking")
}


#[derive(Debug, Clone)]
pub struct CallWithAth {
    pub call: Call,
    pub ath_after_call: f64,
    pub multiplier: f64,
}

pub async fn time_to_timestamp(time: &str) -> i64 {
    log::info!("time: {:?}", time);
    let format = "%Y-%m-%d %H:%M:%S";
    let naive_datetime = NaiveDateTime::parse_from_str(time, format)
        .expect("Failed to parse datetime.");
    let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    datetime.timestamp_millis()
}

fn extract_days(command: &str) -> Option<u32> {
    let re = regex::Regex::new(r"/lb (\d+)d").unwrap();
    re.captures(command)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u32>().ok()))
        .flatten()
}

pub fn is_lb_command(message: &str) -> bool {
    message.starts_with("/lb")
}


pub async fn leaderboard(msg: &teloxide::types::Message, bot: &teloxide::Bot) -> Result<()> {
    let days = extract_days(msg.text().unwrap()).ok_or_else(|| anyhow::anyhow!("Failed to extract days"))?;
    let con = db::get_connection();
    let chat_id = msg.chat.id.to_string();
    let mut lb = Vec::new();
    let mut unique_tokens = std::collections::HashSet::new();

    let calls_last_x_days = db::get_channel_calls_last_x_days(&con, chat_id.as_str(), days as i32);
    for call in calls_last_x_days {
        // Check if the token is already in the unique_tokens set
        if unique_tokens.insert(call.token_address.clone()) {
            // If the token is not in the set, add it and process the call
            let ath = get_ath(time_to_timestamp(call.time.as_str()).await, call.token_address.as_str()).await?;
            log::info!("ath: {:?}", ath);
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


    bot.send_message(msg.chat.id, leaderboard_message(lb, days, msg.chat.first_name().unwrap_or(""))).parse_mode(teloxide::types::ParseMode::Html).await?;

    Ok(())
}

fn get_user_call_count_for_user(lb: &[CallWithAth], user_id: &str) -> usize {
    lb.iter()
        .filter(|call| call.call.user_tg_id == user_id)
        .count()
}

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


pub fn leaderboard_message(lb: Vec<CallWithAth>, days: u32, channel_name: &str) -> String {
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
            mvp_string.push_str(&format!("├ <code>Period:</code>         <b>{}d</b>\n", days));
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


pub async fn best_call_user(user_tg_id: &str) -> Result<Option<CallWithAth>> {
    let con = db::get_connection();
    let user_calls = db::get_all_calls_user_tg_id(&con, user_tg_id);
    let mut best_call: Option<CallWithAth> = None;
    let mut count = 0;
    for call in user_calls {
        let ath = get_ath(time_to_timestamp(call.time.as_str()).await, call.token_address.as_str()).await?;
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

pub async fn user_stats(user_tg_id: &str, bot: &teloxide::Bot, msg: &teloxide::types::Message) -> Result<()> {
    let con = db::get_connection();
    let user_calls = db::get_all_calls_user_tg_id(&con, user_tg_id);
    let best_call = match best_call_user(user_tg_id).await {
        Ok(call) => call,
        Err(e) => return Err(anyhow::Error::msg("No best call found")),
    };
    let user = match db::get_user(&con, user_tg_id) {
        Some(user) => user,
        None => return Err(anyhow::Error::msg("User not found")),
    };
    let username = user.username;
    let calls_count = user_calls.len();
    let best_call_multiplier = best_call.clone().unwrap().multiplier;
    let user_calls_string = String::new();
    let mut call_lb = Vec::new();   
    let mut seen_tokens = std::collections::HashSet::new(); // Track seen tokens

    for call in user_calls {
        if seen_tokens.contains(&call.token_symbol) {
            continue; // Skip if token has already been processed
        }
        seen_tokens.insert(call.token_symbol.clone()); // Mark token as seen

        let ath = get_ath(time_to_timestamp(call.time.as_str()).await, call.token_address.as_str()).await?;
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
    for call in call_lb {
        let multiplier = call.multiplier;
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

    bot.send_message(msg.chat.id,user_stats_message(username, calls_count, best_call_multiplier, learderboard_string)).parse_mode(teloxide::types::ParseMode::Html).await?;
    Ok(())
}

pub fn user_stats_message(username: String, calls_count: usize, best_call_multiplier: f64, learderboard_string: String) -> String {
    format!("
    🥷 @{username}\n\
    ├ Calls: <code>{calls_count}</code>\n\
    └ Return: <code>{best_call_multiplier:.2}x</code>\n\n\
    <blockquote>\
    {learderboard_string}
    </blockquote>\n\
    ")
}