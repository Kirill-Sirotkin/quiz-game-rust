use crate::{
    handlers::game_handler::handle_game,
    helpers::get_room_user_list,
    jwtoken::{decode_token, generate_token, Claims},
    models::{
        communication::{Command, CommandTokenPair, Response},
        game::*,
        lobby::{HasId, Room, User, UserColors},
    },
    server_messages::*,
};
use futures_channel::mpsc::UnboundedSender;
use jsonwebtoken::TokenData;
use log::{info, warn};
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
type ConnectionInfo<'a> = (
    &'a (SocketAddr, String),
    Option<User>,
    Option<Room>,
    Option<(String, UnboundedSender<Message>)>,
    Result<TokenData<Claims>, String>,
);

pub fn execute_command(
    command: &CommandTokenPair,
    lists: &Lists,
    addr_id_pair: &(SocketAddr, String),
) {
    let token_info = decode_token(&command.token);

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

    let connection_info = (
        addr_id_pair,
        current_user,
        current_room,
        current_game,
        token_info,
    );

    drop(users);
    drop(rooms);
    drop(games);

    match &command.command {
        Command::createRoom { name, avatarPath } => {
            info!("Create Room request from: {}", &addr_id_pair.0);

            match connection_info.4 {
                Ok(_) => {
                    let response = Response::errorReponse {
                        errorText: "Token valid".to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
                Err(_) => (),
            };

            let create_room_result =
                match create_room(&connection_info, name.to_owned(), avatarPath.to_owned()) {
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
        Command::joinRoom {
            name,
            avatarPath,
            roomId,
        } => {
            info!("Join Room request from: {}", &addr_id_pair.0);

            match connection_info.4 {
                Ok(ref token_info) => {
                    match lists
                        .1
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|user| user.id == token_info.claims.id)
                    {
                        Some(_) => {
                            let response = Response::errorReponse {
                                errorText: "User already active".to_string(),
                            };
                            send_message(response, &lists.0, &addr_id_pair.0);
                            return;
                        }
                        None => (),
                    }
                }
                Err(_) => (),
            };

            let target_room_player_max = match lists
                .2
                .lock()
                .unwrap()
                .iter()
                .find(|room| &room.id == roomId)
            {
                Some(room) => Some(room.max_players),
                None => None,
            };

            match lists
                .2
                .lock()
                .unwrap()
                .iter()
                .find(|room| &room.id == roomId)
            {
                Some(room) => {
                    if room.current_players >= target_room_player_max.unwrap() {
                        let response = Response::errorReponse {
                            errorText: "Max players reached".to_string(),
                        };
                        send_message(response, &lists.0, &addr_id_pair.0);
                        return;
                    }
                }
                None => (),
            };

            let join_room_result = match join_room(
                connection_info,
                name.to_owned(),
                avatarPath.to_owned(),
                roomId.to_owned(),
            ) {
                Ok(res) => res,
                Err(err) => {
                    let response = Response::errorReponse {
                        errorText: err.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            lists.1.lock().unwrap().push(join_room_result.0);

            match edit_list_element(roomId, lists.2.clone(), |room| {
                room.current_players += 1;
            }) {
                Ok(_) => (),
                Err(error) => warn!("Could not ad user to room: {}", error),
            };

            let response = Response::joinRoomResponse {
                token: join_room_result.1,
                userList: get_room_user_list(&roomId, lists.1.lock().unwrap()),
            };

            send_message(response, &lists.0, &addr_id_pair.0);

            let broadcast_response = Response::updateUserList {
                userList: get_room_user_list(&roomId, lists.1.lock().unwrap()),
            };

            broadcast_message_room_except(
                broadcast_response,
                &lists.0,
                &get_room_user_list(&roomId, lists.1.lock().unwrap()),
                &addr_id_pair.0,
            );

            info!("Successful room join for: {}", &addr_id_pair.0);
        }
        Command::heartbeat {} => {
            info!("Heartbeat from: {}", &addr_id_pair.0);
        }
        Command::startGame { packPath } => {
            info!("Start game command from: {}", &addr_id_pair.0);

            let token_info = match connection_info.4 {
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

            let broadcast_response = Response::startGame {};
            broadcast_message_room_all(
                broadcast_response,
                &lists.0,
                &get_room_user_list(&token_info.roomId.clone(), lists.1.lock().unwrap()),
            );

            // previously: fs::read_to_string("./packs/test.json")
            let data = match fs::read_to_string(packPath) {
                Ok(data) => data,
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };
            let pack: Pack = match serde_json::from_str(&data) {
                Ok(pack) => pack,
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            tokio::spawn(handle_game(
                (
                    lists.0.clone(),
                    lists.1.clone(),
                    lists.2.clone(),
                    lists.3.clone(),
                ),
                get_room_user_list(&token_info.roomId.clone(), lists.1.lock().unwrap()),
                connection_info.2.unwrap().id.clone(),
                pack,
            ));

            info!("Loading pack success");
        }
        Command::getUserList {} => {
            info!("Get user list request from: {}", &addr_id_pair.0);

            let token_info = match connection_info.4 {
                Ok(res) => res.claims,
                Err(error) => {
                    let response = Response::errorReponse {
                        errorText: error.to_string(),
                    };
                    send_message(response, &lists.0, &addr_id_pair.0);
                    return;
                }
            };

            let user_list = get_room_user_list(&token_info.roomId, lists.1.lock().unwrap());

            let response = Response::updateUserList {
                userList: user_list,
            };

            send_message(response, &lists.0, &addr_id_pair.0);
            info!("Successful get user list from: {}", &addr_id_pair.0);
        }
        Command::broadcastMessage { text } => {
            info!("Broadcast to room from: {}", &addr_id_pair.0);

            let token_info = match connection_info.4 {
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
                author: token_info.id,
            };

            let user_list = get_room_user_list(&token_info.roomId, lists.1.lock().unwrap());

            broadcast_message_room_all(response, &lists.0, &user_list);
            info!("Successful broadcast to room from: {}", &addr_id_pair.0);
        }
        Command::writeAnswer { answer } => {
            info!("Answer message from: {}", &addr_id_pair.0);

            let token_info = match connection_info.4 {
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
                user_id: token_info.id.to_string(),
                answer: *answer,
            };
            connection_info
                .3
                .unwrap()
                .1
                .unbounded_send(Message::Text(serde_json::to_string(&answer).unwrap()))
                .unwrap();

            info!("Successful answer message from: {}", &addr_id_pair.0);
        }
        Command::changeUsername { newName } => {
            let token_info = match connection_info.4 {
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

            edit_list_element(&token_info.id, lists.1.clone(), |user| {
                user.name = newName.clone();
            })
            .unwrap();
        }
        Command::changeAvatar { newAvatarPath } => {
            let token_info = match connection_info.4 {
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

            edit_list_element(&token_info.id, lists.1.clone(), |user| {
                user.avatarPath = newAvatarPath.clone();
            })
            .unwrap();
        }
    }
}

fn create_room(
    connection_info: &ConnectionInfo,
    name: String,
    avatar_path: String,
) -> Result<(User, String, Room), String> {
    match connection_info.1 {
        Some(_) => return Err("User already exists".to_string()),
        None => (),
    }

    let new_room = Room {
        id: Uuid::new_v4().to_string(),
        max_players: 6,
        host_id: connection_info.0 .1.clone(),
        current_players: 1,
    };

    let color: UserColors = rand::random();
    let new_user = User {
        id: connection_info.0 .1.clone(),
        name: name.to_string(),
        avatarPath: avatar_path.to_string(),
        roomId: new_room.id.clone(),
        isHost: true,
        userColor: color.value(),
    };

    let token = generate_token(&new_user);
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
    avatar_path: String,
    room_id: String,
) -> Result<(User, String), String> {
    match connection_info.1 {
        Some(_) => return Err("User already exists".to_string()),
        None => (),
    }

    match connection_info.3 {
        Some(_) => return Err("Game in progress".to_string()),
        None => (),
    }

    let color: UserColors = rand::random();
    let new_user = User {
        id: connection_info.0 .1.clone(),
        name: name.to_string(),
        avatarPath: avatar_path.to_string(),
        roomId: room_id.to_string(),
        isHost: false,
        userColor: color.value(),
    };

    let token = generate_token(&new_user);
    match token {
        Ok(token) => {
            return Ok((new_user, token));
        }
        Err(error) => {
            return Err(error.to_string());
        }
    }
}

pub fn edit_list_element<F, T: HasId>(
    id: &String,
    list: Arc<Mutex<Vec<T>>>,
    mut function: F,
) -> Result<(), String>
where
    F: FnMut(&mut T),
{
    let mut users = list.lock().unwrap();
    let target_user = match users.iter_mut().find(|user| &user.get_id() == id) {
        Some(user) => user,
        None => return Err("Item does not exist in list".to_string()),
    };

    function(target_user);

    Ok(())
}
