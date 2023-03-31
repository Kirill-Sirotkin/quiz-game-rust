use crate::{
    handlers::{
        command_handler::{execute_authorized_command, execute_unauthorized_command},
        timeout_handler::handle_user_timeout,
    },
    helpers::{edit_list_element, get_list_element, get_room_user_list, parse_command},
    models::{
        communication::{Command, Response},
        lobby::{Room, User},
    },
    server_messages::send_message,
};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use log::{info, warn};
use rand::seq::SliceRandom;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpStream;
use tungstenite::Message;
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type TxTimeout = UnboundedSender<bool>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type UserTimeoutList = Arc<Mutex<HashMap<String, TxTimeout>>>;
type Lists = (PeerMap, UserList, RoomList, GameList, UserTimeoutList);
type MutexId = Arc<Mutex<String>>;

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

    let connection_id = MutexId::new(Mutex::new(Uuid::new_v4().to_string()));

    let (tx, rx) = unbounded();
    lists
        .0
        .lock()
        .unwrap()
        .insert(connection_id.lock().unwrap().clone(), tx);

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
                    connection_id.clone(),
                ),
                Command::CommandTokenPair(command) => execute_authorized_command(
                    command,
                    (
                        lists.0.clone(),
                        lists.1.clone(),
                        lists.2.clone(),
                        lists.3.clone(),
                    ),
                    connection_id.clone(),
                ),
            },
            Err(error) => {
                warn!("Error parsing command!: {}", error);
                let response = Response::errorResponse {
                    errorText: error.to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
            }
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    info!("{} disconnected", &addr);

    let room_id = match lists
        .1
        .lock()
        .unwrap()
        .iter()
        .find(|user| user.id == connection_id.lock().unwrap().clone())
    {
        Some(user) => Some(user.roomId.clone()),
        None => None,
    };

    let user_id = match lists
        .1
        .lock()
        .unwrap()
        .iter()
        .find(|user| user.id == connection_id.lock().unwrap().clone())
    {
        Some(user) => Some(user.id.clone()),
        None => None,
    };

    // Remove connection from list
    lists
        .0
        .lock()
        .unwrap()
        .remove(&connection_id.lock().unwrap().clone());

    match user_id {
        Some(user_id) => {
            let user_info = get_list_element(&user_id, lists.1.clone());

            println!("Removing user");
            // lists.1.lock().unwrap().remove(index);

            let (tx_timeout, rx_timeout) = unbounded();
            let timeout = lists.4.lock().unwrap().get(&user_id).cloned();

            match timeout {
                Some(tx) => {
                    match tx.unbounded_send(false) {
                        Ok(_) => (),
                        Err(error) => println!("Could not send: {}", error),
                    }
                    lists.4.lock().unwrap().remove(&user_id);
                }
                None => (),
            }

            lists.4.lock().unwrap().insert(user_id.clone(), tx_timeout);
            tokio::spawn(handle_user_timeout(
                user_id.clone(),
                room_id.unwrap().clone(),
                (lists.0.clone(), lists.1.clone(), lists.2.clone()),
                rx_timeout,
            ));

            match user_info {
                Some(user) => {
                    if user.isHost {
                        println!("Host disconnected!");
                        let room_users = get_room_user_list(&user.roomId, lists.1.clone());
                        let random_user = room_users.choose(&mut rand::thread_rng());

                        match random_user {
                            Some(user) => {
                                println!("Making {} host", &user.name);
                                edit_list_element(&user.id, lists.1.clone(), |user| {
                                    user.isHost = true;
                                })
                                .unwrap();
                            }
                            None => {
                                println!("Random user not found");
                            }
                        }
                    } else {
                        println!("User was not host");
                    }
                }
                None => println!("No user info found"),
            }
        }
        None => (),
    }
}
