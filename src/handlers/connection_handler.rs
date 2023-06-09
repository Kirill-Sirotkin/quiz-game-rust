use crate::{
    handlers::{
        command_handler::{execute_authorized_command, execute_unauthorized_command},
        timeout_handler::handle_user_timeout,
    },
    helpers::parse_command,
    models::{
        communication::{Command, Response},
        lobby::{Room, User},
    },
    server_messages::send_message,
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
use tokio_native_tls::TlsAcceptor;
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

pub async fn handle_connection(
    lists: Lists,
    raw_stream: TcpStream,
    addr: SocketAddr,
    acceptor: TlsAcceptor,
) {
    info!("Incoming TCP connection from: {}", &addr);

    let tls_stream = match acceptor.accept(raw_stream).await {
        Ok(res) => res,
        Err(_) => return,
    };
    let ws_stream = match tokio_tungstenite::accept_async(tls_stream).await {
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
        println!("HANDLED COMMAND");

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

    match user_id {
        Some(user_id) => {
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
        }
        None => (),
    }

    println!("Wanna remove connection 1.1");
    // Remove connection from list
    lists
        .0
        .lock()
        .unwrap()
        .remove(&connection_id.lock().unwrap().clone());

    println!("Removed connection 1.2");
}
