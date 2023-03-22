use crate::{
    handlers::game_handler::handle_game,
    jwtoken::{decode_token, generate_token},
    models::{
        communication::{Command, Response},
        game::*,
        lobby::{Room, User},
    },
    server_messages::*,
};
use futures_channel::mpsc::UnboundedSender;
use log::{info, warn};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use std::{fs, sync::MutexGuard};
use tungstenite::protocol::Message;
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);
type ConnectionInfo<'a> = (
    &'a (SocketAddr, String),
    Option<User>,
    Option<Room>,
    Option<(String, UnboundedSender<Message>)>,
);

pub fn parse_command(msg: &Message) -> Result<Command, serde_json::Error> {
    let parsed_msg: Result<Command, serde_json::Error> = serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => return Ok(command),
        Err(error) => return Err(error),
    }
}

pub fn execute_command(command: &Command, lists: &Lists, addr_id_pair: &(SocketAddr, String)) {
    let users = lists.1.lock().unwrap();
    let current_user = match users.iter().find(|user| user.id == addr_id_pair.1) {
        Some(user) => Some(user.clone()),
        None => None,
    };

    let rooms = lists.2.lock().unwrap();
    let current_room = match current_user {
        Some(ref user) => match rooms.iter().find(|room| room.id == user.roomId) {
            Some(room) => Some(room.clone()),
            None => None,
        },
        None => None,
    };

    let games = lists.3.lock().unwrap();
    let current_game = match current_room {
        Some(ref room) => match games.iter().find(|game| game.0 == &room.id) {
            Some(game) => Some((game.0.clone(), game.1.clone())),
            None => None,
        },
        None => None,
    };

    let connection_info = (addr_id_pair, current_user, current_room, current_game);

    drop(users);
    drop(rooms);
    drop(games);

    match command {
        Command::createRoom { name } => {
            info!("Create Room request from: {}", &addr_id_pair.0);

            let create_room_result = match create_room(&connection_info, name.to_owned()) {
                Ok(res) => res,
                Err(err) => {
                    let response = Response::errorReponse {
                        errorText: err.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let response = Response::createRoomResponse {
                token: create_room_result.1,
            };

            lists.1.lock().unwrap().push(create_room_result.0);
            lists.2.lock().unwrap().push(create_room_result.2);

            send_message(response, &lists.0, &addr_id_pair.0);
            info!("Successful room creation for: {}", &addr_id_pair.0);
        }
        Command::joinRoom { name, roomId } => {
            info!("Join Room request from: {}", &addr_id_pair.0);

            let join_room_result =
                match join_room(connection_info, name.to_owned(), roomId.to_owned()) {
                    Ok(res) => res,
                    Err(err) => {
                        let response = Response::errorReponse {
                            errorText: err.to_string(),
                        };
                        send_message(response, &lists.0, &addr_id_pair.0);
                        return;
                    }
                };

            let mut rooms = lists.2.lock().unwrap();
            let target_room = match get_room_by_id(roomId, &mut rooms) {
                Ok(room) => room,
                Err(err) => {
                    let response = Response::errorReponse {
                        errorText: err.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };
            target_room.user_list.push(join_room_result.0.clone());

            let response = Response::joinRoomResponse {
                token: join_room_result.1,
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

            lists.1.lock().unwrap().push(join_room_result.0);

            info!("Successful room join for: {}", &addr_id_pair.0);
        }
        Command::heartbeat {} => {
            info!("Heartbeat from: {}", &addr_id_pair.0);
        }
        Command::startGame { token } => {
            info!("Start game command from: {}", &addr_id_pair.0);

            let token_info = match decode_token(token) {
                Ok(res) => res.claims,
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            match connection_info.1 {
                Some(_) => (),
                None => {
                    let response = Response::errorReponse {
                        errorText: "User does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            match connection_info.3 {
                Some(_) => {
                    let response = Response::errorReponse {
                        errorText: "Game already in progress".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
                None => (),
            }

            if !connection_info.1.unwrap().isHost {
                let response = Response::errorReponse {
                    errorText: "Only host can start the game".to_string(),
                };
                send_message(response, &lists.0, &addr_id_pair.0);
                return;
            }

            let mut rooms = lists.2.lock().unwrap();
            let target_room = match get_room_by_id(&token_info.roomId, &mut rooms) {
                Ok(room) => room,
                Err(err) => {
                    let response = Response::errorReponse {
                        errorText: err.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let broadcast_response = Response::startGame {};
            broadcast_message_room_all(broadcast_response, &lists.0, &target_room.user_list);

            let data = fs::read_to_string("./packs/test.json").expect("Unable to read file");
            let pack: Pack = serde_json::from_str(&data).expect("JSON wrong format");

            tokio::spawn(handle_game(
                (
                    lists.0.clone(),
                    lists.1.clone(),
                    lists.2.clone(),
                    lists.3.clone(),
                ),
                target_room.user_list.clone(),
                target_room.id.clone(),
                pack,
            ));

            info!("Loading pack success");
        }
        Command::getUserList { token } => {
            info!("Get user list request from: {}", &addr_id_pair.0);

            let token_info = match decode_token(token) {
                Ok(res) => res.claims,
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let mut rooms = lists.2.lock().unwrap();
            let target_room = match get_room_by_id(&token_info.roomId, &mut rooms) {
                Ok(room) => room,
                Err(err) => {
                    let response = Response::errorReponse {
                        errorText: err.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let user_list = target_room.user_list.clone();

            let response = Response::updateUserList {
                userList: user_list,
            };

            send_message(response, &lists.0, &addr_id_pair.0);
            info!("Successful get user list from: {}", &addr_id_pair.0);
        }
        Command::broadcastMessage { token, text } => {
            info!("Broadcast to room from: {}", &addr_id_pair.0);

            let token_info = match decode_token(token) {
                Ok(res) => res.claims,
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let response = Response::newMessage {
                text: text.to_owned(),
            };

            let mut rooms = lists.2.lock().unwrap();
            let target_room = match get_room_by_id(&token_info.roomId, &mut rooms) {
                Ok(room) => room,
                Err(err) => {
                    let response = Response::errorReponse {
                        errorText: err.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let user_list = target_room.user_list.clone();

            broadcast_message_room_all(response, &lists.0, &user_list);
            info!("Successful broadcast to room from: {}", &addr_id_pair.0);
        }
        Command::writeAnswer { token, answer } => {
            info!("Answer message from: {}", &addr_id_pair.0);

            let _token_info = match decode_token(token) {
                Ok(res) => res.claims,
                Err(error) => {
                    warn!("Error at token validation: {}", error.to_string());
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            match connection_info.1 {
                Some(_) => (),
                None => {
                    warn!("User does not exist");
                    let response = Response::errorReponse {
                        errorText: "User does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            match connection_info.2 {
                Some(_) => (),
                None => {
                    warn!("Room does not exist");
                    let response = Response::errorReponse {
                        errorText: "Room does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            match connection_info.3 {
                Some(_) => (),
                None => {
                    warn!("Game does not exist");
                    let response = Response::errorReponse {
                        errorText: "Game does not exist".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            }

            let answer = GameCommand {
                token: token.to_string(),
                answer: *answer,
            };
            connection_info
                .3
                .unwrap()
                .1
                .unbounded_send(Message::Text(serde_json::to_string(&answer).unwrap()))
                .unwrap();
        }
    }
}

fn create_room(
    connection_info: &ConnectionInfo,
    name: String,
) -> Result<(User, String, Room), String> {
    match connection_info.1 {
        Some(_) => return Err("User already exists".to_string()),
        None => (),
    }

    let mut new_user = User {
        id: connection_info.0 .1.clone(),
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
        Ok(token) => {
            return Ok((new_user, token, new_room));
        }
        Err(error) => {
            return Err(error.to_string());
        }
    }
}

fn join_room(
    connection_info: ConnectionInfo,
    name: String,
    room_id: String,
) -> Result<(User, String), String> {
    match connection_info.1 {
        Some(_) => return Err("User already exists".to_string()),
        None => (),
    }

    let new_user = User {
        id: connection_info.0 .1.clone(),
        name: name.to_string(),
        avatarPath: "".to_string(),
        roomId: room_id.to_string(),
        isHost: false,
    };

    let token = generate_token(&new_user.id, &room_id);
    match token {
        Ok(token) => {
            return Ok((new_user, token));
        }
        Err(error) => {
            return Err(error.to_string());
        }
    }
}

fn get_room_by_id<'a>(
    id: &String,
    rooms: &'a mut MutexGuard<Vec<Room>>,
) -> Result<&'a mut Room, String> {
    let room_index = rooms.iter().position(|room| &room.id == id);
    let target_room = rooms.get_mut(room_index.unwrap());

    match target_room {
        Some(room) => Ok(room),
        None => Err("Room does not exist".to_string()),
    }
}
