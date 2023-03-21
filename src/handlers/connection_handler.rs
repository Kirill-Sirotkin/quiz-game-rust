use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use log::warn;
use tokio::net::TcpStream;
use tungstenite::Message;
use uuid::Uuid;

use crate::{
    handlers::command_handler::{execute_command, parse_command},
    models::lobby::{Room, User},
};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);

pub async fn handle_connection(lists: Lists, raw_stream: TcpStream, addr: SocketAddr) {
    let addr_id_pair = (addr, Uuid::new_v4().to_string());
    println!("Incoming TCP connection from: {}", addr_id_pair.0);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr_id_pair.0);

    let (tx, rx) = unbounded();
    lists.0.lock().unwrap().insert(addr_id_pair.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr_id_pair.0,
            msg.to_text().unwrap()
        );
        match parse_command(&msg) {
            Ok(command) => execute_command(&command, &lists, &addr_id_pair),
            Err(error) => warn!("Error parsing command!: {}", error),
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr_id_pair.0);

    let mut users = lists.1.lock().unwrap();
    match users.iter().position(|user| user.id == addr_id_pair.1) {
        Some(index) => {
            users.remove(index);
            ();
        }
        None => (),
    }
    lists.0.lock().unwrap().remove(&addr_id_pair);
}
