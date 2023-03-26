use crate::{
    handlers::command_handler::execute_command,
    helpers::{get_room_user_list, parse_command},
    models::{
        communication::Response,
        lobby::{Room, User},
    },
    server_messages::{broadcast_message_room_all, send_message},
};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use log::{info, warn};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpStream;
use tungstenite::Message;
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);

pub async fn handle_connection(lists: Lists, raw_stream: TcpStream, addr: SocketAddr) {
    let addr_id_pair = (addr, Uuid::new_v4().to_string());
    info!("Incoming TCP connection from: {}", addr_id_pair.0);

    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
        Ok(stream) => stream,
        Err(error) => {
            warn!("Handshake with {} error: {}", addr, error);
            return;
        }
    };
    info!("WebSocket connection established: {}", addr_id_pair.0);

    let (tx, rx) = unbounded();
    lists.0.lock().unwrap().insert(addr_id_pair.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        match parse_command(&msg) {
            Ok(command) => execute_command(&command, &lists, &addr_id_pair),
            Err(error) => {
                warn!("Error parsing command!: {}", error);
                let response = Response::errorReponse {
                    errorText: error.to_string(),
                };
                send_message(response, &lists.0, &addr_id_pair.0);
            }
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    info!("{} disconnected", &addr_id_pair.0);

    let mut users = lists.1.lock().unwrap();
    let room_id = match users.iter().find(|user| user.id == addr_id_pair.1) {
        Some(user) => Some(user.roomId.clone()),
        None => None,
    };
    match users.iter().position(|user| user.id == addr_id_pair.1) {
        Some(index) => {
            users.remove(index);
        }
        None => (),
    };
    lists.0.lock().unwrap().remove(&addr_id_pair);

    match room_id {
        Some(id) => {
            let user_list = get_room_user_list(&id, lists.1.lock().unwrap());
            let user_disconnect_response = Response::updateUserList {
                userList: user_list.clone(),
            };
            broadcast_message_room_all(user_disconnect_response, &lists.0, &user_list)
        }
        None => (),
    }
}
