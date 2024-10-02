use anyhow::Result;
use tokio_tungstenite::connect_async;
use futures::stream::SplitSink;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;
use tokio::sync::broadcast;
use solana_account_decoder::UiAccountData;
use solana_program::native_token::lamports_to_sol;
use solana_sdk::native_token::sol_to_lamports;
use crate::modules::matis::get_swap_transaction;
use solana_transaction_status::UiTransactionEncoding;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_client::rpc_config::RpcTransactionConfig;
use serde::Deserialize;
use solana_sdk::signature::Signature;
use solana_sdk::pubkey::Pubkey;
use serde_json::Value as JsonValue;
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{ MaybeTlsStream, WebSocketStream};
use futures_util::stream::SplitStream;
use futures::SinkExt;
use std::env;
use solana_client::rpc_client::RpcClient;
use redis::{Commands, Connection};
use utils::helpers::{decode_signature_get_transaction, get_account_involved_in_transaction};
use tokio::time::{sleep, Duration};
mod utils;
mod modules;
use modules::{
    jupiter::{is_jupiter_swap, info_jupiter_swap},
    pump::{is_pump_swap, info_pump_swap},
    raydium::{is_raydium_swap, info_raydium_swap},
    transfer::handle_transfer_transaction
};
use serde_json::Error as JsonError;
use tokio_tungstenite::tungstenite::Message;
use std::error::Error;
use futures::lock::Mutex;

pub async fn init_rpc_client() -> Result<RpcClient> {
    // Initialize logger
    let http_url = env::var("NODE_HTTP").expect("NODE_HTTP must be set");
    pretty_env_logger::formatted_timed_builder()
        .filter(None, log::LevelFilter::Info)
        .init();

    Ok(RpcClient::new(http_url))
}

/// Return a connection to redis.
pub fn get_redis_connection() -> Connection {
    // Connect to Redis using the container hostname (default port is 6379)
    let redis_client =
        redis::Client::open("redis://telegram_app_redis:6379").expect("Couldn't create redis client.");
    let con = redis_client
        .get_connection()
        .expect("Couldn't create connection to redis.");

    return con;
}

#[derive(Debug, Deserialize, Clone)]
pub struct CopyTradeWallet {
    pub copy_trade_address: String,
    pub account_address: String,
    pub buy_amount: f64,
    pub status: bool,
}

pub fn get_copy_trade_wallets(conn: &mut redis::Connection) -> Result<Vec<CopyTradeWallet>> {
    let pattern = "user:*:copy_trade_wallet:*";
    let mut cursor = 0;
    let mut keys = Vec::new();
    let mut copy_trade_wallets: Vec<CopyTradeWallet> = Vec::new();

    loop {
        let (new_cursor, mut result): (i64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .query(conn)?;

        keys.append(&mut result);
        cursor = new_cursor;

        if cursor == 0 {
            break;
        }
    }
    for key in keys {
        let copy_trade_address: String = conn.hget(key.clone(), "copy_trade_address").unwrap();
        let status: String = conn.hget(key.clone(), "status").unwrap();
        let account_address: String = conn.hget(key.clone(), "account_address").unwrap();
        let buy_amount: f64 = conn.hget(key.clone(), "buy_amount").unwrap();
        if status == "active" {
             copy_trade_wallets.push(CopyTradeWallet { copy_trade_address: copy_trade_address.clone(), account_address: account_address.clone(), buy_amount, status: true });
        }
        if status == "inactive" {
            copy_trade_wallets.push(CopyTradeWallet { copy_trade_address: copy_trade_address.clone(), account_address: account_address.clone(), buy_amount, status: false });
        }
    }
    Ok(copy_trade_wallets)
}

pub async fn subscribe_to_account_transaction(
    write: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    addresses: Vec<CopyTradeWallet>,
) -> Result<()> {
    println!("Watching for {} addresses", addresses.len());
    for addr in addresses.iter() {
        let msg_string = format!(
            r#"{{"jsonrpc": "2.0", "id": 1, "method": "logsSubscribe", "params": [{{"mentions": ["{}"]}}, {{"commitment": "confirmed"}}]}}"#,
            addr.copy_trade_address
        );
        // creating wss message
        let msg = tokio_tungstenite::tungstenite::Message::Text(msg_string);

        // sending the message
        write.send(msg).await.map_err(|e| {
            eprintln!("Couldn't send message to WebSocket: {:?}", e);
            e
        })?;
        println!("Subscribe message sent to: {}", addr.copy_trade_address);
    }

    Ok(())
}

pub async fn handle_incoming_messages(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    write: Arc<Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    client: Arc<RpcClient>,
    tx: Arc<broadcast::Sender<String>>,
) -> Result<()> {
    println!("Listening for incoming RPC messages");

    loop {
        match read.next().await {
            Some(Ok(message)) => {
                println!("Received message: {:?}", message);
                match message {
                    Message::Ping(payload) => {
                        // Respond to Ping with a Pong
                        if let Some(write_guard) = write.lock().await.as_mut() {
                            if let Err(e) = write_guard.send(Message::Pong(payload)).await {
                                eprintln!("Error sending Pong: {:?}", e);
                            } else {
                                println!("Sent Pong response");
                            }
                        } else {
                            eprintln!("Write sink is not available");
                        }
                    }
                    Message::Close(frame) => {
                        println!("Received close frame: {:?}", frame);
                    }
                    Message::Text(text) => {
                        // Process text messages as before
                        let msg_json: JsonValue = match serde_json::from_str(&text) {
                            Ok(json) => json,
                            Err(_) => continue,
                        };
                        if msg_json["method"] == "logsNotification" {
                            match parse_logs_notification(&text, Arc::clone(&client), Arc::clone(&tx)).await {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("Error parsing logs notification: {:?}", e);
                                }
                            }
                        }
                    }
                    _ => {
                    }
                   
                }
            }
            None => {
                println!("WebSocket stream ended");
            }
            Some(Err(e)) => {
                eprintln!("WebSocket error: {:?}", e);
            }
        }
    }
}



#[derive(Debug, Deserialize)]
struct Context {
    slot: u64,
}

#[derive(Debug, Deserialize)]
struct Value {
    signature: String,
    err: Option<String>, // `err` can be `null`, so use Option
    logs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResultWrapper {
    context: Context,
    value: Value,
}

#[derive(Debug, Deserialize)]
struct Params {
    result: ResultWrapper,
    subscription: u64,
}

#[derive(Debug, Deserialize)]
struct LogsNotification {
    jsonrpc: String,
    method: String,
    params: Params,
}
pub async fn parse_logs_notification(
    msg: &str,
    client: Arc<RpcClient>,
    tx_ws: Arc<broadcast::Sender<String>>,
) -> Result<()> {
    let message: Result<LogsNotification, _> = serde_json::from_str(msg);
    match message {
        Ok(parsed_msg) => {
            // Decode the signature from base58
            let decoded_bytes = bs58::decode(parsed_msg.params.result.value.signature.trim())
                .into_vec()
                .expect("Failed to decode base58 string");
            let signature_bytes: [u8; 64] = decoded_bytes[..]
                .try_into()
                .expect("Falha ao converter o slice para um array de 64 bytes");

            // Create the signature from the bytes
            let signature = Signature::from(signature_bytes);

            // Get the decoded transaction
            let config = RpcTransactionConfig {
                commitment: Some(CommitmentConfig {
                    commitment: CommitmentLevel::Confirmed,
                }),
                encoding: Some(UiTransactionEncoding::Base58),
                max_supported_transaction_version: Some(0),
            };
            println!("");
            println!("Signature: {}", signature.to_string());
            
            // Implement retry logic
            let max_retries = 3;
            let retry_delay = Duration::from_millis(1000); // 1 second delay between retries

            for attempt in 1..=max_retries {
                match fetch_and_process_transaction(&client, &signature, &config, &tx_ws).await {
                    Ok(_) => break, // Successfully processed, exit the retry loop
                    Err(e) => {
                        if attempt == max_retries {
                            println!("Error after {} attempts: {:?}", max_retries, e);
                        } else {
                            println!("Attempt {} failed: {:?}. Retrying...", attempt, e);
                            sleep(retry_delay).await;
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error parsing logs notification: {:?}", e);
        }
    }
    Ok(())
}

async fn fetch_and_process_transaction(
    client: &Arc<RpcClient>,
    signature: &Signature,
    config: &RpcTransactionConfig,
    tx_ws: &Arc<broadcast::Sender<String>>,
) -> Result<()> {
    let tx = decode_signature_get_transaction(&signature.to_string().as_str(), client)?;
    let transaction = client.get_transaction_with_config(signature, config.clone())?;

    // Process the transaction
    match get_account_involved_in_transaction(&tx) {
        Ok(account_involved) => {
            println!("Account involved: {}", account_involved.to_string());
            if let Some(_) = &transaction.transaction.meta {
            let transaction_type = determine_transaction_type(&transaction)?;
            println!("Transaction type: {}", transaction_type);
           
            match transaction_type {
                "Transfer" => {
                    let transfer = handle_transfer_transaction(&tx)?;
                    println!("{:?}\n\n", transfer);
                }
                "Raydium Swap" => {
                    let (token_ca, side) = info_raydium_swap(&transaction.transaction, &account_involved)?;
                    handle_send_copy_trade_swap(client.clone(), token_ca, account_involved, side, tx_ws.clone()).await?;
                }
                "Jupiter Swap" => {
                    let (token_ca, side) = info_jupiter_swap(&transaction.transaction, &account_involved)?;
                    handle_send_copy_trade_swap(client.clone(), token_ca, account_involved, side, tx_ws.clone()).await?;
                }
                "Pump Swap" => {
                    let (token_ca, side) = info_pump_swap(&transaction.transaction, &account_involved)?;
                    handle_send_copy_trade_swap(client.clone(), token_ca, account_involved, side, tx_ws.clone()).await?;
                }
                _ => {}
            }


        }
    }
    Err(e) => {
        println!("Error getting account involved: {:?}", e);
    }
    }
    Ok(())
}

fn determine_transaction_type(transaction: &EncodedConfirmedTransactionWithStatusMeta) -> Result<&'static str> {
    Ok(match (
        is_raydium_swap(&transaction.transaction)?,
        is_jupiter_swap(&transaction.transaction)?,
        is_pump_swap(&transaction.transaction)?,
    ) {
        (true, false, false) => "Raydium Swap",
        (false, true, false) => "Jupiter Swap",
        (false, false, true) => "Pump Swap",
        (false, false, false) => "Transfer",
        (true, true, false) => "Jupiter Swap",
        _ => "Unknown",
    })
}

#[derive(Debug)]
pub struct TokenBalance {
    pub sol_amount: f64,
    pub lamports_amount: u64,
    pub mint: String,
    pub token_ui_amount: f64,
    pub token_amount: u64,
}

#[derive(Debug)]
pub struct TokensBalance {
    pub token_balance: Vec<TokenBalance>,
}
impl TokensBalance {
    // Método para adicionar um novo TokenBalance à lista
    pub fn add_token_balance(&mut self, token_balance: TokenBalance) {
        self.token_balance.push(token_balance);
    }
}

pub fn get_tokens_balance(
    client: Arc<RpcClient>,
    wallet_pubkey: &Pubkey,
) -> Result<TokensBalance> {
    // Fetch token accounts for the wallet
    let token_accounts = client.get_token_accounts_by_owner(
        wallet_pubkey,
        solana_client::rpc_request::TokenAccountsFilter::ProgramId(spl_token::id()),
    )?;

    let mut tokens_balance = TokensBalance {
        token_balance: Vec::new(),
    };

    for account in token_accounts {
        let lamports_amount = account.account.lamports;
        let sol_amount = lamports_to_sol(lamports_amount); // Assuming you have this function defined

        // Handle UiAccountData::Json
        if let UiAccountData::Json(parsed_account) = &account.account.data {
            let parsed_data: &JsonValue = &parsed_account.parsed;

            if let Some(token_ui_amount) = parsed_data["info"]["tokenAmount"]["uiAmount"].as_f64() {
                if let Some(token_mint) = parsed_data["info"]["mint"].as_str() {
                    let token_amount = parsed_data["info"]["tokenAmount"]["amount"]
                        .as_str()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap();

                    let token_balance = TokenBalance {
                        sol_amount,
                        lamports_amount,
                        token_amount,
                        mint: token_mint.to_string(),
                        token_ui_amount,
                    };

                    tokens_balance.token_balance.push(token_balance);
                }
            }
        }
    }

    Ok(tokens_balance)
}

async fn handle_send_copy_trade_swap(
    client: Arc<RpcClient>,
    token_ca: String,
    trader_addres: Pubkey,
    side: String,
    tx: Arc<broadcast::Sender<String>>,
) -> Result<()> {
    let mut con = get_redis_connection();
    let copy_trade_wallets = get_copy_trade_wallets(&mut con)?;
    for copy_trade in copy_trade_wallets {
        if copy_trade.copy_trade_address == trader_addres.to_string() {
            let account_pubkey = Pubkey::from_str(&copy_trade.account_address).unwrap();
            if copy_trade.status {
                if side == "buy" {
                    let result = get_and_send_buy_transaction(&account_pubkey, &token_ca, copy_trade.buy_amount, &tx).await;
                    handle_transaction_result(result, "buy", &token_ca, copy_trade.buy_amount)?;
                }
                if side == "sell" {
                    let result = get_and_send_sell_transaction(client.clone(), &account_pubkey, &token_ca, &tx).await;
                    handle_transaction_result(result, "sell", &token_ca, 0.0)?;
                }
            }
        }
    }
    Ok(())
}

async fn get_and_send_buy_transaction(
    account_pubkey: &Pubkey,
    token_ca: &str,
    buy_amount: f64,
    tx: &Arc<broadcast::Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Sending buy transaction");
    println!("Token CA: {}", token_ca);
    println!("Buy amount: {}", buy_amount);
    println!("Account pubkey: {}", account_pubkey.to_string()); 
    let transaction = get_swap_transaction(
        account_pubkey,
        sol_to_lamports(0.0015),
        "So11111111111111111111111111111111111111112".to_string(),
        token_ca.to_string(),
        sol_to_lamports(buy_amount),
    ).await?;

    let tx_string = serde_json::to_string(&transaction)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    tx.send(tx_string).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(())
}

async fn get_and_send_sell_transaction(
    client: Arc<RpcClient>,
    account_pubkey: &Pubkey,
    token_ca: &str,
    tx: &Arc<broadcast::Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let tokens_balance = get_tokens_balance(client.clone(), account_pubkey)?;
    println!("Sending buy transaction");
    println!("Token CA: {}", token_ca);
    println!("Account pubkey: {}", account_pubkey.to_string()); 
    println!("Tokens balance: {:?}", tokens_balance);
    if let Some(token_to_sell_balance) = tokens_balance.token_balance.iter().find(|token| token.mint == token_ca) {
        let transaction = get_swap_transaction(
        account_pubkey,
        sol_to_lamports(0.005),
        token_ca.to_string(),
        "So11111111111111111111111111111111111111112".to_string(),
        token_to_sell_balance.token_amount,
    ).await?;

    let tx_string = serde_json::to_string(&transaction)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        tx.send(tx_string).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    } else {
        tx.send("Token not found in balance.".to_string())?;
        Ok(())
    }
}

fn handle_transaction_result(
    result: Result<(), Box<dyn std::error::Error>>,
    action: &str,
    token_ca: &str,
    amount: f64,
) -> Result<()> {
    match result {
        Ok(_) => {
            println!("Tx to {} ({}) {} sent to telegram", action, amount, token_ca);
            Ok(())
        }
        Err(e) => {
            if let Some(json_err) = e.downcast_ref::<JsonError>() {
                if json_err.to_string().contains("missing field `inputMint`") {
                    println!("Error sending transaction: missing inputMint. This might be due to an API change or temporary issue.");
                    // Here you could implement a fallback method or retry with different parameters
                } else {
                    println!("JSON error when sending transaction: {:?}", json_err);
                }
            } else {
                println!("Error sending {} transaction: {:?}", action, e);
            }
            Err(anyhow::anyhow!("Failed to send transaction"))
        }
    }
}