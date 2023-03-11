use jsonwebtoken::{encode, EncodingKey, Header};
use log::{info, warn};
use quiz_game_rust::logger::init_logger;
use quiz_game_rust::{command::*, quiz_game_backend_models::*};
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
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;

async fn handle_connection(
    peer_map: PeerMap,
    user_list: UserList,
    room_list: RoomList,
    raw_stream: TcpStream,
    addr: SocketAddr,
) {
    let addr_id_pair = (addr, Uuid::new_v4().to_string());
    println!("Incoming TCP connection from: {}", addr_id_pair.0);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr_id_pair.0);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr_id_pair.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    // Read messages
    // -----------------------------------------------------------------------------------------------------
    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr_id_pair.0,
            msg.to_text().unwrap()
        );
        match parse_command(&msg) {
            Ok(command) => {
                execute_command(&command, &peer_map, &user_list, &room_list, &addr_id_pair)
            }
            Err(error) => warn!("Error parsing command!: {}", error),
        }

        future::ok(())
    });

    // Forward message to client from sink
    // -----------------------------------------------------------------------------------------------------
    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr_id_pair.0);
    let mut users = user_list.lock().unwrap();
    match users.iter().position(|user| user.id == addr_id_pair.1) {
        Some(index) => {
            users.remove(index);
            return;
        }
        None => (),
    }
    peer_map.lock().unwrap().remove(&addr_id_pair);
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
    addr_id_pair: &(SocketAddr, String),
) {
    let peers = peer_map.lock().unwrap();
    let current_user_id = peers
        .iter()
        .find(|(peer_addr, _)| peer_addr == &addr_id_pair)
        .unwrap()
        .0
         .1
        .clone();
    drop(peers);

    match command {
        Command::createRoom { name } => {
            info!("Create Room request from: {}", &addr_id_pair.0);

            let new_user = User {
                id: current_user_id,
                name: name.to_string(),
            };

            let token = generate_token(&new_user.id);
            match token {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &peer_map, &addr_id_pair.0);
                    return;
                }
            }

            let mut new_room = Room {
                id: Uuid::new_v4().to_string(),
                name: name.to_string(),
                max_players: 6,
                host_id: new_user.id.clone(),
                user_list: Vec::new(),
            };
            new_room.user_list.push(new_user.clone());

            let response = Response::createRoomResponse {
                token: token.unwrap(),
                roomId: new_room.id.clone(),
            };

            match user_list.lock() {
                Ok(mut list) => list.push(new_user),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &peer_map, &addr_id_pair.0);
                    return;
                }
            }

            match room_list.lock() {
                Ok(mut list) => list.push(new_room),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &peer_map, &addr_id_pair.0);
                    return;
                }
            }

            send_message(response, &peer_map, &addr_id_pair.0);
            info!("Successful room creation for: {}", &addr_id_pair.0);
        }
        Command::joinRoom { name, roomId } => {
            let new_user = User {
                id: current_user_id,
                name: name.to_string(),
            };

            let token = generate_token(&new_user.id);
            match token {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &peer_map, &addr_id_pair.0);
                    return;
                }
            }

            let mut rooms = room_list.lock().unwrap();
            let room_index = rooms.iter().position(|room| &room.id == roomId);
            match room_index {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "Room does not exist".to_string(),
                    };
                    send_message(response, &peer_map, &addr_id_pair.0);
                    return;
                }
            }
            let target_room = rooms.get_mut(room_index.unwrap()).unwrap();

            target_room.user_list.push(new_user);

            let user_list = target_room.user_list.clone();

            let response = Response::joinRoomResponse {
                token: token.unwrap(),
                userList: user_list.clone(),
            };

            send_message(response, &peer_map, &addr_id_pair.0);

            let broadcast_response = Response::updateUserList {
                userList: user_list.clone(),
            };

            broadcast_message_room_except(
                broadcast_response,
                &peer_map,
                &user_list,
                &addr_id_pair.0,
            );
        }
    }
}

fn generate_token(id: &String) -> Result<String, jsonwebtoken::errors::Error> {
    let new_claims = Claims { id: id.clone() };
    let token = encode(
        &Header::default(),
        &new_claims,
        &EncodingKey::from_secret("secret".as_ref()),
    );
    return token;
}
fn send_message(response: Response, peer_map: &PeerMap, addr: &SocketAddr) {
    info!("Sending msg to: {}", &addr);

    let peers = peer_map.lock().unwrap();
    info!("Peers locked");
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| &peer_addr.0 == addr)
        .map(|(_, ws_sink)| ws_sink);

    info!("Recipients found, sending...");
    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }
    info!("Message sent successfully to: {}", &addr);
}
fn broadcast_message_all(response: Response, peer_map: &PeerMap) {
    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }
}
fn broadcast_message_except(response: Response, peer_map: &PeerMap, addr: &SocketAddr) {
    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| &peer_addr.0 != addr)
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }
}
fn broadcast_message_room_all(response: Response, peer_map: &PeerMap, user_list: &Vec<User>) {
    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| {
            user_list
                .iter()
                .map(|user| &user.id)
                .any(|id| id == &peer_addr.1)
        })
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }
}
fn broadcast_message_room_except(
    response: Response,
    peer_map: &PeerMap,
    user_list: &Vec<User>,
    addr: &SocketAddr,
) {
    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| {
            user_list
                .iter()
                .map(|user| &user.id)
                .any(|id| id == &peer_addr.1)
        } && &peer_addr.0 != addr)
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
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
