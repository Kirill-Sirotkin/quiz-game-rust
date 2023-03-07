use crossbeam_queue::SegQueue;
use futures_util::{
    future::{self, ok},
    stream::Forward,
    StreamExt, TryStreamExt,
};
use log::{info, warn};
use quiz_game_rust::command::Command;
use quiz_game_rust::logger::init_logger;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::*;

#[tokio::main]
async fn main() {
    init_logger().unwrap();
    info!("Server started");

    //let queues_array = Arc::new(Mutex::new(Vec::new()));

    let try_socket = TcpListener::bind("127.0.0.1:9001").await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: 127.0.0.1:9001");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(make_threads(stream));
    }
}

async fn make_threads(stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, mut read) = ws_stream.split();
    // We should not forward messages other than text or binary.\
    loop {
        let msg = read.next().await.unwrap();
        info!("New message!: {}", msg.unwrap());
    }
}
