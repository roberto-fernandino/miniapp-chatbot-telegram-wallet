use anyhow::Result;
use crate::modules::swap::sign_and_send_swap_transaction;
use std::str::FromStr;
use crate::modules::matis::get_swap_transaction;
use solana_sdk::pubkey::Pubkey;
use serde::{Serialize, Deserialize};
use futures_util::SinkExt;
use tide::Request;
use tide::Response;
use tide::StatusCode;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use futures::stream::SplitSink;
use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use solana_app::{init_rpc_client, get_redis_connection, get_copy_trade_wallets, subscribe_to_account_transaction, handle_incoming_messages, get_sol_balance};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use std::env;
use futures_util::stream::StreamExt;
mod modules;
mod utils;
use tokio::sync::broadcast;
use futures::lock::Mutex;
use std::time::Duration;
mod turnkey;

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
        subscribe_to_account_transaction(&mut write1, wallets).await.map_err(|e| {
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
        };

        let mut app = tide::with_state(state);

        // Define the resubscribe endpoint
        app.at("/resubscribe").get(|req: Request<State>| async move { resubscribe(req).await });
        app.at("/get_wallet_sol_balance/:address").get(|req: Request<State>| async move { get_wallet_sol_balance(req).await }); 
        app.at("/sol/swap").post(|req: Request<State>| async move { sol_swap(req).await });

        // Start the Tide server on port 3030
        println!("Solana app Listening on port 3030");
        app.listen("0.0.0.0:3030").await?;
        Ok::<(), tide::Error>(())
    });

    let tx_clone2 = Arc::clone(&tx);
    // Spawn task to handle incoming messages from the Solana node
    let handle_messages = tokio::spawn(async move {
            loop {
                let write1 = Arc::clone(&write1_arc);
                match handle_incoming_messages(&mut read1, write1.clone(), client_arc.clone(), Arc::clone(&tx_clone2)).await {
                    Ok(_) => {
                        println!("Successfully handled incoming messages");
                    },
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


async fn start_websocket_server(tx: Arc<broadcast::Sender<String>>) {
    // Define the address to listen on
    let addr = "0.0.0.0:4040";

    // Create a TCP listener
    let listener = TcpListener::bind(addr).await.expect("Failed to bind to address");
    println!("Websocket server listening on {}", addr);

    // Continuously accept incoming connections
    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from {}", addr);
        let tx = Arc::clone(&tx);
        tokio::spawn(handle_connection(stream, tx));
    }
}

async fn handle_connection(stream: TcpStream, tx: Arc<broadcast::Sender<String>>) {
    // Establish the WebSocket handshake
    let ws_stream = tokio_tungstenite::accept_async(stream).await.expect("Error during WS handshake");
    let addr = ws_stream.get_ref().peer_addr().unwrap();
    println!("Telegram Connection opened from {}", addr);


    // Split the WebSocket stream into a write and read half
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to the broadcast channel
    let mut rx = tx.subscribe();

    // Continuously read messages from the WebSocket and broadcast them
    loop {
        tokio::select! {
            // Broadcast messages to the WebSocket
            msg = rx.recv() => {
                 if let Ok(msg) = msg {
                    if write.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
            // Read messages from the WebSocket
            msg = read.next() => {
                 // Check if there is a message
                 if let Some(Ok(msg)) = msg {
                    // Check if the message is a close message
                    if msg.is_close() {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }
    println!("Telegram Connection closed from {}", addr);
}

// State for the Tide server to subscribe changes to the Solana node
#[derive(Clone)]
pub struct State {
    write1: Arc<Mutex<Option<SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>>>>,
}

async fn reconnect_websocket() -> Result<(SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>, 
                                          futures::stream::SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>), 
                                         Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async(env::var("NODE_WSS").expect("NODE_WSS must be set.")).await?;
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
pub async fn get_wallet_sol_balance(req: Request<State>) -> Result<Response, tide::Error> {
    let address = req.param("address").unwrap();
    let balance = get_sol_balance(address)?;
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(format!("{}", balance));
    Ok(res)
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
pub async fn resubscribe(req: Request<State>) -> Result<Response, tide::Error> {
    let mut con = get_redis_connection();
    let wallets = get_copy_trade_wallets(&mut con).unwrap();
    let state = req.state().clone();
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
                        return Ok(Response::new(StatusCode::Ok));
                    },
                    Err(e) => {
                        eprintln!("Resubscribe attempt {} failed: {:?}", retry_count + 1, e);
                        // Connection might be closed, attempt to reconnect
                        *write_lock = None;
                    }
                }
            },
            None => {
                // Attempt to reconnect
                match reconnect_websocket().await {
                    Ok((new_write, _new_read)) => {
                        *write_lock = Some(new_write);
                        println!("Reconnected to WebSocket");
                    },
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
    let mut res = Response::new(StatusCode::InternalServerError);
    res.set_body(format!("Resubscribe failed after {} attempts", max_retries));
    Ok(res)
}


#[derive(Serialize, Deserialize)]
struct SwapRequest {
    user: crate::modules::swap::User,
    priorization_fee_lamports: u64,
    input_mint: String,
    output_mint: String,
    amount: u64,
    slippage: f64
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
pub async fn sol_swap(mut req: Request<State>) -> tide::Result {
    println!("@sol_swap /sol/swap received request");
    let SwapRequest {
        user,
        priorization_fee_lamports,
        input_mint,
        output_mint,
        amount,
        slippage
    } = req.body_json().await.expect("Failed to parse swap request");
    println!("@sol_swap /sol/swap parsed request");
    let pubkey = Pubkey::from_str(&user.public_key).expect("Invalid pubkey");
    println!("@sol_swap /sol/swap getting transaction");
    let swap_transacation = get_swap_transaction(&pubkey, priorization_fee_lamports, input_mint, output_mint, amount, slippage).await?;
    println!("@sol_swap /sol/swap got transaction");
    println!("@sol_swap /sol/swap signing and sending transaction");
    let tx = sign_and_send_swap_transaction(swap_transacation, user).await.expect("Failed to sign swap transaction");
    println!("@sol_swap /sol/swap transaction sent: {:?}", tx);
    Ok(Response::new(StatusCode::Ok))
}