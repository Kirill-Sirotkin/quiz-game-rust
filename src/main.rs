use crossbeam_queue::SegQueue;
use log::{info, warn};
use quiz_game_rust::logger::init_logger;
use quiz_game_rust::quiz_game_backend_models;
use quiz_game_rust::{command::*, quiz_game_backend_models::*};
use serde::de::Error;
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;

async fn handle_connection(
    peer_map: PeerMap,
    user_list: UserList,
    room_list: RoomList,
    raw_stream: TcpStream,
    addr: SocketAddr,
) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    let (outgoing, incoming) = ws_stream.split();

    // Read messages
    // -----------------------------------------------------------------------------------------------------
    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );
        match parse_command(&msg) {
            Ok(command) => execute_command(&command, &peer_map, &user_list, &room_list, &addr),
            Err(error) => warn!("Error parsing command!: {}", error),
        }

        future::ok(())
    });

    // Forward message to client from sink
    // -----------------------------------------------------------------------------------------------------
    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

fn parse_command(msg: &Message) -> Result<Command, serde_json::Error> {
    let parsed_msg: Result<Command, serde_json::Error> = serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => return Ok(command),
        Err(error) => return Err(error),
    }
}

fn execute_command(
    command: &Command,
    peer_map: &PeerMap,
    user_list: &UserList,
    room_list: &RoomList,
    addr: &SocketAddr,
) {
    match command {
        Command::SendMessage { text } => {
            info!("Send message: {}", text);
            let peers = peer_map.lock().unwrap();

            let broadcast_recipients = peers
                .iter()
                .filter(|(peer_addr, _)| peer_addr == &addr)
                .map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                recp.unbounded_send(Message::Text(text.to_string()))
                    .unwrap();
            }
        }
        Command::Disconnect {} => {
            info!("Disconnect message!");
        }
        Command::BroadcastMessage { text } => {
            info!("Broadcast message: {}", text);
            let peers = peer_map.lock().unwrap();

            let broadcast_recipients = peers
                .iter()
                .filter(|(peer_addr, _)| peer_addr != &addr)
                .map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                recp.unbounded_send(Message::Text(text.to_string()))
                    .unwrap();
            }
        }
        Command::AddUser { name } => {
            user_list.lock().unwrap().push(User {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                address: addr.to_string(),
            });
        }
        Command::GetUsers {} => {
            let mut user_list_json = String::new();

            let users = user_list.lock().unwrap();
            let all_users = users.iter();
            for user in all_users {
                let user_serialized = serde_json::to_string(user).unwrap();
                user_list_json.push_str(&user_serialized);
            }

            let peers = peer_map.lock().unwrap();

            let broadcast_recipients = peers
                .iter()
                .filter(|(peer_addr, _)| peer_addr == &addr)
                .map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                recp.unbounded_send(Message::Text(user_list_json.to_string()))
                    .unwrap();
            }
        }
        Command::CreateRoom { name, max_players } => {
            let users = user_list.lock().unwrap();
            let host_user = users
                .iter()
                .find(|user| user.address == addr.to_string())
                .unwrap();

            room_list.lock().unwrap().push(Room {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                max_players: *max_players,
                host_id: host_user.id.clone(),
            });
        }
        Command::GetRooms {} => {
            let mut room_list_json = String::new();

            let rooms = room_list.lock().unwrap();
            let all_rooms = rooms.iter();
            for room in all_rooms {
                let room_serialized = serde_json::to_string(room).unwrap();
                room_list_json.push_str(&room_serialized);
            }

            let peers = peer_map.lock().unwrap();

            let broadcast_recipients = peers
                .iter()
                .filter(|(peer_addr, _)| peer_addr == &addr)
                .map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                recp.unbounded_send(Message::Text(room_list_json.to_string()))
                    .unwrap();
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    init_logger().unwrap();
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:9001".to_string());

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // User list and Room list
    let users = UserList::new(Mutex::new(Vec::new()));
    let rooms = RoomList::new(Mutex::new(Vec::new()));

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            state.clone(),
            users.clone(),
            rooms.clone(),
            stream,
            addr,
        ));
    }

    Ok(())
}
