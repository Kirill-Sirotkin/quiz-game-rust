use crate::{
    handlers::{
        command_handler::{execute_authorized_command, execute_unauthorized_command},
        room_handler::handle_room_timeout,
    },
    helpers::{edit_list_element, get_list_element, get_room_user_list, parse_command},
    models::{
        communication::{Command, Response},
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
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);

pub async fn handle_connection(lists: Lists, raw_stream: TcpStream, addr: SocketAddr) {
    info!("Incoming TCP connection from: {}", &addr);

    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
        Ok(stream) => stream,
        Err(error) => {
            warn!("Handshake with {} error: {}", addr, error);
            return;
        }
    };
    info!("WebSocket connection established: {}", &addr);

    let connection_id = Uuid::new_v4().to_string();
    let (tx, rx) = unbounded();
    lists.0.lock().unwrap().insert(connection_id.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        match parse_command(&msg) {
            Ok(command) => match command {
                Command::UnauthorizedCommand(command) => execute_unauthorized_command(
                    command,
                    (
                        lists.0.clone(),
                        lists.1.clone(),
                        lists.2.clone(),
                        lists.3.clone(),
                    ),
                    &connection_id,
                ),
                Command::CommandTokenPair(command) => execute_authorized_command(
                    command,
                    (
                        lists.0.clone(),
                        lists.1.clone(),
                        lists.2.clone(),
                        lists.3.clone(),
                    ),
                    &connection_id,
                ),
            },
            Err(error) => {
                warn!("Error parsing command!: {}", error);
                let response = Response::errorReponse {
                    errorText: error.to_string(),
                };
                send_message(response, lists.0.clone(), &connection_id);
            }
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    info!("{} disconnected", &addr);

    let mut users = lists.1.lock().unwrap();
    let room_id = match users.iter().find(|user| user.id == connection_id) {
        Some(user) => Some(user.roomId.clone()),
        None => None,
    };
    match users.iter().position(|user| user.id == connection_id) {
        Some(index) => {
            users.remove(index);
        }
        None => (),
    };
    lists.0.lock().unwrap().remove(&connection_id);

    drop(users);

    match room_id {
        Some(room_id) => {
            edit_list_element(&room_id, lists.2.clone(), |room| {
                room.current_players -= 1;
            })
            .unwrap();

            let user_list = get_room_user_list(&room_id, lists.1.clone());
            let update_user_list_response = Response::updateUserList {
                userList: user_list.clone(),
            };
            broadcast_message_room_all(update_user_list_response, lists.0.clone(), &user_list);

            let room_info = get_list_element(&room_id, lists.2.clone()).unwrap();
            if room_info.current_players <= 0 {
                tokio::spawn(handle_room_timeout(room_id.clone(), lists.2.clone()));
            }
        }
        None => (),
    }

    // match room_id {
    //     Some(id) => {
    //         let user_list = get_room_user_list(&id, lists.1.lock().unwrap());
    //         let user_disconnect_response = Response::updateUserList {
    //             userList: user_list.clone(),
    //         };
    //         match edit_list_element(&id, lists.2.clone(), |room| {
    //             room.current_players -= 1;
    //         }) {
    //             Ok(_) => (),
    //             Err(error) => warn!("Could not remove user from room: {}", error),
    //         };
    //         broadcast_message_room_all(user_disconnect_response, &lists.0, &user_list);

    //         let room_players = match lists.2.lock().unwrap().iter().find(|room| room.id == id) {
    //             Some(room) => Some(room.current_players),
    //             None => None,
    //         };

    //         let room_index = match lists
    //             .2
    //             .lock()
    //             .unwrap()
    //             .iter()
    //             .position(|room| room.id == id)
    //         {
    //             Some(index) => Some(index),
    //             None => None,
    //         };
    //         match room_players {
    //             Some(players) => {
    //                 if players <= 0 {
    //                     lists.2.lock().unwrap().remove(room_index.unwrap());
    //                 }
    //             }
    //             None => (),
    //         };
    //     }
    //     None => (),
    // }
}
