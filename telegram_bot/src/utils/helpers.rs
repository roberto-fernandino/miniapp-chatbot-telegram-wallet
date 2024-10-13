use chrono::{TimeDelta, TimeZone};
use sqlite::Connection;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use crate::{format_age, format_number};
use crate::db::*;
use crate::get_pair_token_pair_and_token_address;
use crate::get_scanner_search;
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
    
    if text.ends_with("d") {
        Some("Days".to_string())
    } else if text.ends_with("w") {
        Some("Weeks".to_string())
    } else if text.ends_with("m") {
        Some("Months".to_string())
    } else if text.ends_with("y") {
        Some("Years".to_string())
    } else if text.ends_with("h") {
        Some("Hours".to_string())
    } else {
        None
    }
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
/// An i64 timestamp
pub async fn time_to_timestamp(time: &str) -> i64 {
    log::info!("time: {:?}", time);
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
pub async fn get_call_info(address: &String, con: &Arc<Mutex<Connection>>, msg: &Message) -> Result<String> {
    // First call info
    let mut call_info_str = String::new();
    let is_first_call = is_first_call(&con,address.as_ref(), msg.chat.id.to_string().as_str());
    let token_pair_and_token_address  = get_pair_token_pair_and_token_address(address).await?;
    let scanner_response = get_scanner_search(token_pair_and_token_address["pairAddress"].as_str().unwrap_or(""), token_pair_and_token_address["tokenAddress"].as_str().unwrap_or("")).await?;
    let mkt_cap = scanner_response["pair"]["token1TotalSupplyFormatted"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) * scanner_response["pair"]["pairPrice1Usd"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    if !is_first_call {
        let first_call = {
            let con = con.lock().unwrap();
            get_first_call_token_chat(&con, address.as_ref(), msg.chat.id.to_string().as_str())
        };

        // Calculating the % change
        let first_call_mkt_cap = first_call.clone().expect("First call not found").mkt_cap.parse::<f64>().unwrap_or(0.0);
        let percentage_change = format_number(((mkt_cap - first_call_mkt_cap) / first_call_mkt_cap) * 100.0);


        // Calculating the age of the call
        let timestamp = time_to_timestamp(&first_call.clone().expect("First call doesn't exist.").time).await;
        let call_time = Utc.timestamp_millis_opt(timestamp).unwrap();
        let current_time = Utc::now();
        let time_delta: TimeDelta = current_time.signed_duration_since(call_time);
        if let Some(first_call) = first_call{
            let user_called_first = {
                let con = con.lock().unwrap();
                get_user(&con, first_call.user_tg_id.as_str()).expect("User not found")
            };
            call_info_str.push_str(&format!("😈 <a href=\"https://t.me/sj_copyTradebot?start=user_{}\"><i><b>{}</b></i></a> @ {} <b>[{}%]</b> ({})", first_call.user_tg_id,  user_called_first.username, format_number(first_call.mkt_cap.parse::<f64>().unwrap_or(0.0)), percentage_change, format_age(time_delta)));
        }
    } 
    Ok(call_info_str)
}
