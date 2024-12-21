use std::{sync::Arc, time::Duration};

use crate::{
    handlers::{
        jupiter::{info_jupiter_swap, is_jupiter_swap},
        pump::{info_pump_swap, is_pump_swap},
        raydium::{info_raydium_swap, is_raydium_swap},
        swap::handle_send_copy_trade_swap,
        transfer::handle_transfer_transaction,
    },
    models::{copy_trade::CopyTradeWallet, transaction::LogsNotification},
    utils::helpers::{decode_signature_get_transaction, get_account_involved_in_transaction},
};
use anyhow::Result;
use futures::{
    lock::Mutex,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::Value as JsonValue;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    signature::Signature,
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use tokio::{net::TcpStream, sync::broadcast, time::sleep};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// Subscribe to account transactions
///
/// # Parameters
/// - `write`: &mut SplitSink<...> - Mutable reference to the WebSocket write stream
/// - `addresses`: Vec<CopyTradeWallet> - Vector of CopyTradeWallet structs to subscribe to
///
/// # Returns
/// - `Result<()>`: Ok if successful, or an error
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
                            match parse_logs_notification(
                                &text,
                                Arc::clone(&client),
                                Arc::clone(&tx),
                            )
                            .await
                            {
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

/// Determine the type of transaction
///
/// # Parameters
/// - `transaction`: &EncodedConfirmedTransactionWithStatusMeta - Reference to the transaction data
///
/// # Returns
/// - `Result<&'static str>`: The determined transaction type or an error
fn determine_transaction_type(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<&'static str> {
    Ok(
        match (
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
        },
    )
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
                        let (token_ca, side) =
                            info_raydium_swap(&transaction.transaction, &account_involved)?;
                        handle_send_copy_trade_swap(
                            client.clone(),
                            token_ca,
                            account_involved,
                            side,
                            tx_ws.clone(),
                        )
                        .await?;
                    }
                    "Jupiter Swap" => {
                        let (token_ca, side) =
                            info_jupiter_swap(&transaction.transaction, &account_involved)?;
                        handle_send_copy_trade_swap(
                            client.clone(),
                            token_ca,
                            account_involved,
                            side,
                            tx_ws.clone(),
                        )
                        .await?;
                    }
                    "Pump Swap" => {
                        let (token_ca, side) =
                            info_pump_swap(&transaction.transaction, &account_involved)?;
                        handle_send_copy_trade_swap(
                            client.clone(),
                            token_ca,
                            account_involved,
                            side,
                            tx_ws.clone(),
                        )
                        .await?;
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
