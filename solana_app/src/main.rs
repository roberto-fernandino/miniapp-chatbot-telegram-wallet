use anyhow::Result;
use axum::http::StatusCode;
use axum::{
    extract::{Path, State as AxumState},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_app::client::redis::{get_copy_trade_wallets, get_redis_connection};
use solana_app::client::rpc::{get_sol_balance, init_rpc_client};
use solana_app::client::websocket::{handle_incoming_messages, subscribe_to_account_transaction};
use solana_app::handlers::matis::get_legacy_swap_transaction;
use solana_app::handlers::swap::{sign_and_send_swap_transaction, User};
use solana_app::handlers::transfer::sign_and_send_transaction;
use solana_app::models::token::get_tokens_balance;
use solana_app::server::start_websocket_server;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_instruction;
use solana_sdk::{message::Message, transaction::Transaction};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::WebSocketStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize RPC client
    let client: Arc<RpcClient> = Arc::new(init_rpc_client().await.unwrap());

    let client_arc = Arc::clone(&client);
    let mut con = get_redis_connection();

    // Create a channel for broadcasting messages
    let (tx, _rx) = broadcast::channel::<String>(100);
    let tx = Arc::new(tx);
    let tx_clone1 = Arc::clone(&tx);
    let _ws_server = tokio::spawn(async move {
        start_websocket_server(Arc::clone(&tx_clone1)).await;
    });

    // Get all addresses to watch from Redis
    let copy_trade_wallets = Arc::new(Mutex::new(get_copy_trade_wallets(&mut con)?));

    let (ws_stream, _) =
        connect_async(env::var("NODE_WSS").expect("NODE_WSS must be set.")).await?;
    println!("Connected to Solana WebSocket");

    // Splitting the stream
    let (mut write1, mut read1) = ws_stream.split();

    // Initial addresses subscription
    {
        let wallets = copy_trade_wallets.lock().await.clone();
        subscribe_to_account_transaction(&mut write1, wallets)
            .await
            .map_err(|e| {
                eprintln!("Couldn't subscribe to account transactions: {:?}", e);
                e
            })?;
    }

    // Shared state for the Tide server
    let write1_arc = Arc::new(Mutex::new(Some(write1)));

    // Clone necessary variables for the server
    let server_write1 = Arc::clone(&write1_arc);

    // Spawn the Tide server that listens for resubscribe requests
    let server = tokio::spawn(async move {
        let state = State {
            write1: Arc::clone(&server_write1),
            client: Arc::clone(&client),
        };

        let app = Router::new()
            .route("/resubscribe", get(resubscribe))
            .route(
                "/get_wallet_sol_balance/:address",
                get(get_wallet_sol_balance),
            )
            .route("/get_positions/:address", get(tokens_balance))
            .route("/sol/swap", post(sol_swap))
            .route("/sol/transfer", post(transfer_sol))
            .with_state(state);

        let listener = TcpListener::bind("0.0.0.0:3030")
            .await
            .expect("Failed to bind to address");
        println!("Solana app Listening on port 3030");
        axum::serve(listener, app.into_make_service())
            .await
            .expect("Failed to serve");
    });

    let tx_clone2 = Arc::clone(&tx);
    // Spawn task to handle incoming messages from the Solana node
    let handle_messages = tokio::spawn(async move {
        loop {
            let write1 = Arc::clone(&write1_arc);
            match handle_incoming_messages(
                &mut read1,
                write1.clone(),
                client_arc.clone(),
                Arc::clone(&tx_clone2),
            )
            .await
            {
                Ok(_) => {
                    println!("Successfully handled incoming messages");
                }
                Err(e) => {
                    eprintln!("Error handling incoming messages: {:?}", e);
                }
            }
        }
    });

    // Wait for both the TIDE server and sol message handler to complete
    let _ = tokio::join!(server, handle_messages);

    Ok(())
}

#[derive(Clone)]
pub struct State {
    write1: Arc<
        Mutex<
            Option<
                SplitSink<
                    WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
                    tokio_tungstenite::tungstenite::Message,
                >,
            >,
        >,
    >,
    client: Arc<RpcClient>,
}

async fn reconnect_websocket() -> Result<
    (
        SplitSink<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
            tokio_tungstenite::tungstenite::Message,
        >,
        futures::stream::SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>,
    ),
    Box<dyn std::error::Error>,
> {
    let (ws_stream, _) =
        connect_async(env::var("NODE_WSS").expect("NODE_WSS must be set.")).await?;
    println!("Reconnected to Solana WebSocket");
    Ok(ws_stream.split())
}

/// Get the SOL balance of a wallet
///
/// # Arguments
///
/// * `req` - The request
///
/// # Returns
///
/// A `Result` containing a `Response` or a `tide::Error`
pub async fn get_wallet_sol_balance(
    AxumState(_state): AxumState<State>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match get_sol_balance(address.as_str()) {
        Ok(balance) => Ok(Json(json!({ "balance": balance }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Resubscribes to the Solana node
///
/// # Arguments
///
/// * `req` - The request
///
/// # Returns
///
/// A `Result` containing a `Response` or a `tide::Error`
pub async fn resubscribe(AxumState(state): AxumState<State>) -> impl IntoResponse {
    let mut con = get_redis_connection();
    let wallets = get_copy_trade_wallets(&mut con).unwrap();
    let mut write_lock = state.write1.lock().await;

    let mut retry_count = 0;
    let max_retries = 3;
    let retry_delay = Duration::from_secs(5);

    while retry_count < max_retries {
        match write_lock.as_mut() {
            Some(write) => {
                match subscribe_to_account_transaction(write, wallets.clone()).await {
                    Ok(_) => {
                        println!("Successfully resubscribed");
                        return Ok((StatusCode::OK, "Resubscribed"));
                    }
                    Err(e) => {
                        eprintln!("Resubscribe attempt {} failed: {:?}", retry_count + 1, e);
                        // Connection might be closed, attempt to reconnect
                        *write_lock = None;
                    }
                }
            }
            None => {
                // Attempt to reconnect
                match reconnect_websocket().await {
                    Ok((new_write, _new_read)) => {
                        *write_lock = Some(new_write);
                        println!("Reconnected to WebSocket");
                    }
                    Err(e) => {
                        eprintln!("Failed to reconnect: {:?}", e);
                    }
                }
            }
        }

        retry_count += 1;
        if retry_count < max_retries {
            tokio::time::sleep(retry_delay).await;
        }
    }

    eprintln!("Resubscribe failed after {} attempts", max_retries);
    Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Resubscribe failed after {} attempts", max_retries),
    ))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwapRequest {
    user: User,
    priorization_fee_lamports: u64,
    jito_tip_amount: u64,
    input_mint: String,
    output_mint: String,
    amount: u64,
    slippage: f64,
}

/// @sol_swap /sol/swap
///
/// @POST
///
/// @body [SwapRequest]
///
/// # Description
///
/// Sign and send sol swap transaction
///
/// # ArgumentsSolana swap route
///
/// * `req` - The request
///
/// # Returns
///
/// A `Result` containing a `Response` or a `tide::Error`
pub async fn sol_swap(
    AxumState(_state): AxumState<State>,
    Json(swap_request): Json<SwapRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    println!("@sol_swap /sol/swap received request");
    let SwapRequest {
        user,
        priorization_fee_lamports,
        jito_tip_amount,
        input_mint,
        output_mint,
        amount,
        slippage,
    } = swap_request.clone();
    println!("@sol_swap /sol/swap parsed request");

    println!("@sol_swap /sol/swap request: {:?}", swap_request.clone());
    let pubkey = Pubkey::from_str(&user.public_key).expect("Invalid pubkey");
    println!("@sol_swap /sol/swap getting transaction");
    let swap_transacation = get_legacy_swap_transaction(
        &pubkey,
        priorization_fee_lamports,
        input_mint,
        output_mint,
        amount,
        slippage,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    println!("@sol_swap /sol/swap got transaction");

    println!("@sol_swap /sol/swap signing and sending transaction");
    match sign_and_send_swap_transaction(swap_transacation, user, jito_tip_amount).await {
        Ok(sig) => {
            println!("@sol_swap /sol/swap transaction sent: {:?}", sig);
            println!(
                "@sol_swap /sol/swap response: {:?}",
                json!({ "transaction": sig.to_string() })
            );
            Ok((
                StatusCode::OK,
                Json(json!({ "transaction": sig.to_string() })),
            ))
        }
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
}

/// Get the positions of a wallet
///
/// @GET
///
/// @path /sol/get_positions/{address}
///
/// # Arguments
///
/// * `address` - The address of the wallet
///
/// # Returns
///
/// A `Result` containing a `Response` or a `tide::Error`
pub async fn tokens_balance(
    AxumState(state): AxumState<State>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    println!("@get_positions /sol/get_positions/{address} received request");

    let pubkey = Pubkey::from_str(&address).expect("Invalid pubkey");
    println!(
        "@get_positions /sol/get_positions/{address} pubkey: {:?}",
        pubkey
    );

    let rpc_client = Arc::clone(&state.client);

    let positions = get_tokens_balance(rpc_client, &pubkey)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    println!(
        "@get_positions /sol/get_positions/{address} positions: {:?}",
        positions
    );

    // Calculate total SOL balance
    let total_sol_balance: f64 = positions
        .token_balance
        .iter()
        .map(|token| token.sol_amount)
        .sum();

    // Create a JSON response including total SOL balance and UI amount for each token
    let response = json!({
        "total_sol_balance": total_sol_balance,
        "tokens": positions.token_balance.iter().map(|token| {
            json!({
                "mint": token.mint,
                "sol_amount": token.sol_amount,
                "token_ui_amount": token.token_ui_amount,
                "lamports_amount": token.lamports_amount,
                "token_amount": token.token_amount
            })
        }).collect::<Vec<_>>()
    });

    Ok(Json(response))
}

/// Transfer payload struct
///
/// # Fields
///
/// * `amount` - The amount to transfer
/// * `sender_pubkey` - The sender's public key
/// * `receiver_pubkey` - The receiver's public key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPayload {
    pub amount: u64,
    pub sender_pubkey: Pubkey,
    pub receiver_pubkey: Pubkey,
    pub user: User,
}

/// Transfer SOL
///
/// # Arguments
///
/// * `payload` - The transfer payload
///
/// # Returns
///
/// A `Result` indicating the success of the operation
pub async fn transfer_sol(
    AxumState(_state): AxumState<State>,
    Json(payload): Json<TransferPayload>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    // Create the transfer instruction
    println!("@transfer_sol/ transfer_sol received request");
    println!("@transfer_sol/ transfer_sol payload: {:?}", payload);
    let ix = system_instruction::transfer(
        &payload.sender_pubkey,
        &payload.receiver_pubkey,
        payload.amount,
    );
    println!("@transfer_sol/ transfer_sol ix: {:?}", ix);
    let rpc_client = RpcClient::new(env::var("NODE_HTTP").expect("NODE_HTTP must be set"));
    let latest_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // Create unsigned transaction

    let tx = Transaction::new_unsigned(Message::new_with_blockhash(
        &[ix],
        Some(&payload.sender_pubkey), // Payer
        &latest_blockhash,
    ));
    println!("@transfer_sol/ transfer_sol tx: {:?}", tx);

    match sign_and_send_transaction(tx, payload.user).await {
        Ok(sig) => Ok((
            StatusCode::OK,
            Json(json!({ "transaction": sig.to_string() })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
