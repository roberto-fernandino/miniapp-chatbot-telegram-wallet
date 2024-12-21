use futures::StreamExt;
use futures_util::SinkExt;
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_tungstenite::tungstenite::Message as tungsMessage;

pub async fn start_websocket_server(tx: Arc<broadcast::Sender<String>>) {
    // Define the address to listen on
    let addr = "0.0.0.0:4040";

    // Create a TCP listener
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");
    println!("Websocket server listening on {}", addr);

    // Continuously accept incoming connections
    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from {}", addr);
        let tx = Arc::clone(&tx);
        tokio::spawn(handle_connection(stream, tx));
    }
}

pub async fn handle_connection(stream: TcpStream, tx: Arc<broadcast::Sender<String>>) {
    // Establish the WebSocket handshake
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during WS handshake");
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
                    if write.send(tungsMessage::Text(msg)).await.is_err() {
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
