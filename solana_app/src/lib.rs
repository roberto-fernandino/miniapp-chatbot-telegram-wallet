use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use solana_account_decoder::UiAccountData;
use solana_program::native_token::{lamports_to_sol, sol_to_lamports};
use solana_sdk::{
    commitment_config::{CommitmentLevel, CommitmentConfig},
    signature::Signature,
    pubkey::Pubkey,
};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use serde_json::Value as JsonValue;
use tokio::{net::TcpStream, sync::broadcast, time::{sleep, Duration}};
use std::{env, str::FromStr, sync::Arc, error::Error};
use redis::{Commands, Connection};

// Import local modules
mod utils;
mod modules;
use modules::{
    jupiter, pump, raydium, transfer,
    matis::{SwapTransaction, get_swap_transaction},
};

/// Initialize the RPC client
///
/// # Returns
/// - `Result<RpcClient>`: The initialized RPC client or an error
pub async fn init_rpc_client() -> Result<RpcClient> {
    // Initialize logger
    let http_url = env::var("NODE_HTTP").expect("NODE_HTTP must be set");
    pretty_env_logger::formatted_timed_builder()
        .filter(None, log::LevelFilter::Info)
        .init();

    // Create and return the RPC client
    Ok(RpcClient::new(http_url))
}

/// Get a connection to Redis
///
/// # Returns
/// - `Connection`: The Redis connection
pub fn get_redis_connection() -> Connection {
    // Connect to Redis using the container hostname (default port is 6379)
    let redis_client = redis::Client::open("redis://telegram_app_redis:6379")
        .expect("Couldn't create redis client.");
    let con = redis_client.get_connection()
        .expect("Couldn't create connection to redis.");

    con
}

/// Struct representing a copy trade wallet
#[derive(Debug, Deserialize, Clone)]
pub struct CopyTradeWallet {
    pub copy_trade_address: String,
    pub account_address: String,
    pub buy_amount: f64,
    pub status: bool,
}

/// Fetch copy trade wallets from Redis
///
/// # Parameters
/// - `conn`: &mut redis::Connection - Mutable reference to the Redis connection
///
/// # Returns
/// - `Result<Vec<CopyTradeWallet>>`: A vector of CopyTradeWallet structs or an error
pub fn get_copy_trade_wallets(conn: &mut redis::Connection) -> Result<Vec<CopyTradeWallet>> {
    let pattern = "user:*:copy_trade_wallet:*";
    let mut cursor = 0;
    let mut keys = Vec::new();
    let mut copy_trade_wallets: Vec<CopyTradeWallet> = Vec::new();

    // Scan Redis for matching keys
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

    // Process each key and create CopyTradeWallet structs
    for key in keys {
        let copy_trade_address: String = conn.hget(key.clone(), "copy_trade_address").unwrap();
        let status: String = conn.hget(key.clone(), "status").unwrap();
        let account_address: String = conn.hget(key.clone(), "account_address").unwrap();
        let buy_amount: f64 = conn.hget(key.clone(), "buy_amount").unwrap();

        let wallet = CopyTradeWallet {
            copy_trade_address: copy_trade_address.clone(),
            account_address: account_address.clone(),
            buy_amount,
            status: status == "active",
        };
        copy_trade_wallets.push(wallet);
    }

    Ok(copy_trade_wallets)
}

/// Subscribe to account transactions
///
/// # Parameters
/// - `write`: &mut SplitSink<...> - Mutable reference to the WebSocket write stream
/// - `addresses`: Vec<CopyTradeWallet> - Vector of CopyTradeWallet structs to subscribe to
///
/// # Returns
/// - `Result<()>`: Ok if successful, or an error
pub async fn subscribe_to_account_transaction(
    write: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    addresses: Vec<CopyTradeWallet>,
) -> Result<()> {
    println!("Watching for {} addresses", addresses.len());

    // Subscribe to each address
    for addr in addresses.iter() {
        let msg_string = format!(
            r#"{{"jsonrpc": "2.0", "id": 1, "method": "logsSubscribe", "params": [{{"mentions": ["{}"]}}, {{"commitment": "confirmed"}}]}}"#,
            addr.copy_trade_address
        );
        let msg = tokio_tungstenite::tungstenite::Message::Text(msg_string);

        // Send the subscription message
        write.send(msg).await.map_err(|e| {
            eprintln!("Couldn't send message to WebSocket: {:?}", e);
            e
        })?;
        println!("Subscribe message sent to: {}", addr.copy_trade_address);
    }

    Ok(())
}

/// Handle incoming WebSocket messages
///
/// # Parameters
/// - `read`: &mut SplitStream<...> - Mutable reference to the WebSocket read stream
/// - `write`: Arc<Mutex<Option<SplitSink<...>>>> - Thread-safe reference to the WebSocket write stream
/// - `client`: Arc<RpcClient> - Thread-safe reference to the RPC client
/// - `tx`: Arc<broadcast::Sender<String>> - Thread-safe reference to the broadcast sender
///
/// # Returns
/// - `Result<()>`: Ok if successful, or an error
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
                match message {
                    Message::Ping(payload) => {
                        // Respond to Ping with a Pong
                        if let Some(write_guard) = write.lock().await.as_mut() {
                            if let Err(e) = write_guard.send(Message::Pong(payload)).await {
                                eprintln!("Error sending Pong: {:?}", e);
                            }
                        } else {
                            eprintln!("Write sink is not available");
                        }
                    }
                    Message::Close(frame) => {
                        println!("Received close frame: {:?}", frame);
                    }
                    Message::Text(text) => {
                        // Process text messages
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
                    _ => {}
                }
            }
            None => {
                println!("Node WebSocket stream ended");
            }
            Some(Err(e)) => {
                eprintln!("WebSocket error: {:?}", e);
            }
        }
    }
}

// Struct definitions for parsing log notifications
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

/// Parse and process log notifications
///
/// # Parameters
/// - `msg`: &str - The raw message string
/// - `client`: Arc<RpcClient> - Thread-safe reference to the RPC client
/// - `tx_ws`: Arc<broadcast::Sender<String>> - Thread-safe reference to the broadcast sender
///
/// # Returns
/// - `Result<()>`: Ok if successful, or an error
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
                .expect("Failed to convert slice to a 64-byte array");

            // Create the signature from the bytes
            let signature = Signature::from(signature_bytes);

            // Set up the RPC configuration
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

/// Fetch and process a transaction with retry logic
///
/// # Parameters
/// - `client`: &Arc<RpcClient> - Reference to the thread-safe RPC client
/// - `signature`: &Signature - Reference to the transaction signature
/// - `config`: &RpcTransactionConfig - Reference to the RPC configuration
/// - `tx_ws`: &Arc<broadcast::Sender<String>> - Reference to the thread-safe broadcast sender
///
/// # Returns
/// - `Result<()>`: Ok if successful, or an error
async fn fetch_and_process_transaction(
    client: &Arc<RpcClient>,
    signature: &Signature,
    config: &RpcTransactionConfig,
    tx_ws: &Arc<broadcast::Sender<String>>,
) -> Result<()> {
    // Fetch the transaction
    let tx = decode_signature_get_transaction(&signature.to_string().as_str(), client)?;
    let transaction = client.get_transaction_with_config(signature, config.clone())?;

    // Process the transaction
    match get_account_involved_in_transaction(&tx) {
        Ok(account_involved) => {
            if let Some(_) = &transaction.transaction.meta {
                let transaction_type = determine_transaction_type(&transaction)?;
                println!("Transaction type: {}", transaction_type);
               
                // Handle different transaction types
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

/// Determine the type of transaction
///
/// # Parameters
/// - `transaction`: &EncodedConfirmedTransactionWithStatusMeta - Reference to the transaction data
///
/// # Returns
/// - `Result<&'static str>`: The determined transaction type or an error
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

/// Struct representing a token balance
#[derive(Debug)]
pub struct TokenBalance {
    pub sol_amount: f64,
    pub lamports_amount: u64,
    pub mint: String,
    pub token_ui_amount: f64,
    pub token_amount: u64,
}

/// Struct representing multiple token balances
#[derive(Debug)]
pub struct TokensBalance {
    pub token_balance: Vec<TokenBalance>,
}

impl TokensBalance {
    /// Add a new TokenBalance to the list
    pub fn add_token_balance(&mut self, token_balance: TokenBalance) {
        self.token_balance.push(token_balance);
    }
}

/// Get token balances for a wallet
///
/// # Parameters
/// - `client`: Arc<RpcClient> - Thread-safe reference to the RPC client
/// - `wallet_pubkey`: &Pubkey - Reference to the wallet's public key
///
/// # Returns
/// - `Result<TokensBalance>`: The token balances or an error
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

    // Process each token account
    for account in token_accounts {
        let lamports_amount = account.account.lamports;
        let sol_amount = lamports_to_sol(lamports_amount);

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

/// Handle sending copy trade swap transactions
///
/// # Parameters
/// - `client`: Arc<RpcClient> - Thread-safe reference to the RPC client
/// - `token_ca`: String - The token's contract address
/// - `trader_address`: Pubkey - The trader's public key
/// - `side`: String - The side of the trade (buy/sell)
/// - `tx`: Arc<broadcast::Sender<String>> - Thread-safe reference to the broadcast sender
///
/// # Returns
/// - `Result<()>`: Ok if successful, or an error
async fn handle_send_copy_trade_swap(
    client: Arc<RpcClient>,
    token_ca: String,
    trader_address: Pubkey,
    side: String,
    tx: Arc<broadcast::Sender<String>>,
) -> Result<()> {
    let mut con = get_redis_connection();
    let copy_trade_wallets = get_copy_trade_wallets(&mut con)?;

    // Process each copy trade wallet
    for copy_trade in copy_trade_wallets {
        if copy_trade.copy_trade_address == trader_address.to_string() {
            let account_pubkey = Pubkey::from_str(&copy_trade.account_address).unwrap();
            if copy_trade.status {
                if side == "buy" {
                    match get_and_send_buy_transaction(&account_pubkey, &token_ca, copy_trade.buy_amount, &tx).await {
                        Ok(_) => println!("Buy transaction sent successfully"),
                        Err(e) => println!("Error sending buy transaction: {:?}", e),
                    }
                }
                if side == "sell" {
                    match get_and_send_sell_transaction(client.clone(), &account_pubkey, &token_ca, &tx).await {
                        Ok(_) => println!("Sell transaction sent successfully"),
                        Err(e) => println!("Error sending sell transaction: {:?}", e),
                    }
                }
            }
        }
    }
    Ok(())
}

/// Struct representing the payload for swap transactions
#[derive(Debug, Serialize)]
pub struct Payload {
    pub event_type: String,
    pub data: SwapTransaction,
}

/// Get and send a buy transaction
///
/// # Parameters
/// - `account_pubkey`: &Pubkey - Reference to the account's public key
/// - `token_ca`: &str - The token's contract address
/// - `buy_amount`: f64 - The amount to buy
/// - `tx`: &Arc<broadcast::Sender<String>> - Reference to the thread-safe broadcast sender
///
/// # Returns
/// - `Result<(), Box<dyn std::error::Error>>`: Ok if successful, or an error
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

    // Get swap transaction
    let transaction = get_swap_transaction(
        account_pubkey,
        sol_to_lamports(0.0015),
        "So11111111111111111111111111111111111111112".to_string(),
        token_ca.to_string(),
        sol_to_lamports(buy_amount),
    ).await?;
    let payload = Payload {
        event_type: "copy_trade".to_string(),
        data: transaction,
    };
    println!("Payload: {:?}", payload);
    let tx_string = serde_json::to_string(&payload)?;
    tx.send(tx_string)?;
    println!("Buy transaction sent to telegram successfully");
    Ok(())
}

/// Get and send a sell transaction
///
/// # Parameters
/// - `client`: Arc<RpcClient> - Thread-safe reference to the RPC client
/// - `account_pubkey`: &Pubkey - Reference to the account's public key
/// - `token_ca`: &str - The token's contract address
/// - `tx`: &Arc<broadcast::Sender<String>> - Reference to the thread-safe broadcast sender
///
/// # Returns
/// - `Result<(), Box<dyn std::error::Error>>`: Ok if successful, or an error
async fn get_and_send_sell_transaction(
    client: Arc<RpcClient>,
    account_pubkey: &Pubkey,
    token_ca: &str,
    tx: &Arc<broadcast::Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let tokens_balance = get_tokens_balance(client.clone(), account_pubkey)?;
    if let Some(token_to_sell_balance) = tokens_balance.token_balance.iter().find(|token| token.mint == token_ca) {
        println!("Sending sell transaction");
        println!("Token CA: {}", token_ca);
        println!("Account pubkey: {}", account_pubkey.to_string()); 
        println!("Token amount: {:?}", token_to_sell_balance.token_ui_amount);

        // Get swap transaction
        let transaction = get_swap_transaction(
            account_pubkey,
            sol_to_lamports(0.005),
            token_ca.to_string(),
            "So11111111111111111111111111111111111111112".to_string(),
            token_to_sell_balance.token_amount,
        ).await?;

        // Send transaction
        let tx_string = serde_json::to_string(&transaction)?;
        tx.send(tx_string)?;
        Ok(())
    } else {
        tx.send("Token not found in balance.".to_string())?;
        Ok(())
    }
}