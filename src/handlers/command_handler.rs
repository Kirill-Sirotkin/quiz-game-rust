use crate::{
    jwtoken::{decode_token, generate_token},
    models::{
        communication::{Command, Response},
        game::*,
        lobby::{Room, User},
    },
    server_messages::*,
};
use futures_channel::mpsc::UnboundedSender;
use log::info;
use std::fs;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tungstenite::protocol::Message;
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);

pub fn parse_command(msg: &Message) -> Result<Command, serde_json::Error> {
    let parsed_msg: Result<Command, serde_json::Error> = serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => return Ok(command),
        Err(error) => return Err(error),
    }
}

pub fn execute_command(command: &Command, lists: &Lists, addr_id_pair: &(SocketAddr, String)) {
    let peers = lists.0.lock().unwrap();
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

            let mut new_user = User {
                id: current_user_id,
                name: name.to_string(),
                avatarPath: "".to_string(),
                roomId: "".to_string(),
                isHost: true,
            };

            let mut new_room = Room {
                id: Uuid::new_v4().to_string(),
                max_players: 6,
                host_id: new_user.id.clone(),
                user_list: Vec::new(),
            };
            new_user.roomId = new_room.id.clone();
            new_room.user_list.push(new_user.clone());

            let token = generate_token(&new_user.id, &new_room.id);
            match token {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            let response = Response::createRoomResponse {
                token: token.unwrap(),
            };

            match lists.1.lock() {
                Ok(mut list) => list.push(new_user),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            match lists.2.lock() {
                Ok(mut list) => list.push(new_room),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            send_message(response, &lists.0, &addr_id_pair.0);
            info!("Successful room creation for: {}", &addr_id_pair.0);
        }
        Command::joinRoom { name, roomId } => {
            info!("Join Room request from: {}", &addr_id_pair.0);

            let new_user = User {
                id: current_user_id,
                name: name.to_string(),
                avatarPath: "".to_string(),
                roomId: roomId.clone(),
                isHost: false,
            };

            let mut rooms = lists.2.lock().unwrap();
            let room_index = rooms.iter().position(|room| &room.id == roomId);
            match room_index {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "Room does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let target_room = rooms.get_mut(room_index.unwrap()).unwrap();

            let token = generate_token(&new_user.id, &target_room.id);
            match token {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            target_room.user_list.push(new_user);

            let response = Response::joinRoomResponse {
                token: token.unwrap(),
                userList: target_room.user_list.clone(),
            };

            send_message(response, &lists.0, &addr_id_pair.0);

            let broadcast_response = Response::updateUserList {
                userList: target_room.user_list.clone(),
            };

            broadcast_message_room_except(
                broadcast_response,
                &lists.0,
                &target_room.user_list,
                &addr_id_pair.0,
            );

            info!("Successful room join for: {}", &addr_id_pair.0);
        }
        Command::heartbeat {} => {
            info!("Heartbeat from: {}", &addr_id_pair.0);
        }
        Command::startGame { token } => {
            info!("Start game command from: {}", &addr_id_pair.0);

            let token_result = decode_token(token);
            match token_result {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let token_info = token_result.unwrap().claims;

            let users = lists.1.lock().unwrap();
            let user_index = users.iter().position(|user| user.id == token_info.id);
            match user_index {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "User does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let user = users.get(user_index.unwrap()).unwrap();
            if !user.isHost {
                let response = Response::errorReponse {
                    errorText: "You are not the host".to_string(),
                };
                send_message(response, &lists.0, &addr_id_pair.0);
            }

            let mut rooms = lists.2.lock().unwrap();
            let room_index = rooms.iter().position(|room| room.id == token_info.roomId);
            match room_index {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "Room does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let target_room = rooms.get_mut(room_index.unwrap()).unwrap();

            let broadcast_response = Response::startGame {};
            broadcast_message_room_all(broadcast_response, &lists.0, &target_room.user_list);

            let data = fs::read_to_string("./packs/test.json").expect("Unable to read file");
            let pack: Pack = serde_json::from_str(&data).expect("JSON wrong format");

            info!("Loading pack: {}", pack.name);
        }
        Command::getUserList { token } => {
            info!("Get user list request from: {}", &addr_id_pair.0);

            let token_result = decode_token(token);
            match token_result {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let token_info = token_result.unwrap().claims;

            let mut rooms = lists.2.lock().unwrap();
            let room_index = rooms.iter().position(|room| room.id == token_info.roomId);
            match room_index {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "Room does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let target_room = rooms.get_mut(room_index.unwrap()).unwrap();

            let user_list = target_room.user_list.clone();

            let response = Response::updateUserList {
                userList: user_list,
            };

            send_message(response, &lists.0, &addr_id_pair.0);
            info!("Successful get user list from: {}", &addr_id_pair.0);
        }
        Command::broadcastMessage { text, token } => {
            info!("Broadcast to room from: {}", &addr_id_pair.0);

            let token_result = decode_token(token);
            match token_result {
                Ok(_) => (),
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let token_info = token_result.unwrap().claims;

            let response = Response::newMessage {
                text: text.to_owned(),
            };

            let mut rooms = lists.2.lock().unwrap();
            let room_index = rooms.iter().position(|room| room.id == token_info.roomId);
            match room_index {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "Room does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }
            let target_room = rooms.get_mut(room_index.unwrap()).unwrap();

            let user_list = target_room.user_list.clone();

            broadcast_message_room_all(response, &lists.0, &user_list);
            info!("Successful broadcast to room from: {}", &addr_id_pair.0);
        }
    }
}
