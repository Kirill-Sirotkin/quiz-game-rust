use crate::{
    handlers::game_handler::handle_game,
    helpers::{connect_user_to_room, get_list_element, get_room_user_list},
    jwtoken::{decode_token, generate_token},
    models::{
        communication::{AuthorizedCommand, CommandTokenPair, Response, UnauthorizedCommand},
        game::*,
        lobby::{Room, User, UserColors},
    },
    server_messages::*,
};
use futures_channel::mpsc::UnboundedSender;
use log::info;
use std::fs;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tungstenite::protocol::Message;
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);
type MutexId = Arc<Mutex<String>>;

// pub fn execute_command(command: &CommandTokenPair, lists: Lists, addr: &SocketAddr) {
//     let token_info = decode_token(&command.token);

//     let users = lists.1.lock().unwrap();
//     let current_user = match users.iter().find(|user| user.id == addr.1) {
//         Some(user) => Some(user.clone()),
//         None => None,
//     };

//     let rooms = lists.2.lock().unwrap();
//     let current_room = match current_user {
//         Some(ref user) => match rooms.iter().find(|room| room.id == user.roomId) {
//             Some(room) => Some(room.clone()),
//             None => None,
//         },
//         None => None,
//     };

//     let games = lists.3.lock().unwrap();
//     let current_game = match current_room {
//         Some(ref room) => match games.iter().find(|game| game.0 == &room.id) {
//             Some(game) => Some((game.0.clone(), game.1.clone())),
//             None => None,
//         },
//         None => None,
//     };

//     let connection_info = (addr, current_user, current_room, current_game, token_info);

//     drop(users);
//     drop(rooms);
//     drop(games);

//     match &command.command {
//         Command::createRoom { name, avatarPath } => {
//             info!("Create Room request from: {}", &addr.0);

//             match connection_info.4 {
//                 Ok(_) => {
//                     let response = Response::errorReponse {
//                         errorText: "Token valid".to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//                 Err(_) => (),
//             };

//             let create_room_result =
//                 match create_room(&connection_info, name.to_owned(), avatarPath.to_owned()) {
//                     Ok(res) => res,
//                     Err(err) => {
//                         let response = Response::errorReponse {
//                             errorText: err.to_string(),
//                         };
//                         send_message(response, &lists.0, &addr.0);
//                         return;
//                     }
//                 };

//             let response = Response::createRoomResponse {
//                 token: create_room_result.1,
//             };

//             lists.1.lock().unwrap().push(create_room_result.0);
//             lists.2.lock().unwrap().push(create_room_result.2);

//             send_message(response, &lists.0, &addr.0);
//             info!("Successful room creation for: {}", &addr.0);
//         }
//         Command::joinRoom {
//             name,
//             avatarPath,
//             roomId,
//         } => {
//             info!("Join Room request from: {}", &addr.0);

//             match connection_info.4 {
//                 Ok(ref token_info) => {
//                     match lists
//                         .1
//                         .lock()
//                         .unwrap()
//                         .iter()
//                         .find(|user| user.id == token_info.claims.id)
//                     {
//                         Some(_) => {
//                             let response = Response::errorReponse {
//                                 errorText: "User already active".to_string(),
//                             };
//                             send_message(response, &lists.0, &addr.0);
//                             return;
//                         }
//                         None => (),
//                     }
//                 }
//                 Err(_) => (),
//             };

//             let target_room_player_max = match lists
//                 .2
//                 .lock()
//                 .unwrap()
//                 .iter()
//                 .find(|room| &room.id == roomId)
//             {
//                 Some(room) => Some(room.max_players),
//                 None => None,
//             };

//             match lists
//                 .2
//                 .lock()
//                 .unwrap()
//                 .iter()
//                 .find(|room| &room.id == roomId)
//             {
//                 Some(room) => {
//                     if room.current_players >= target_room_player_max.unwrap() {
//                         let response = Response::errorReponse {
//                             errorText: "Max players reached".to_string(),
//                         };
//                         send_message(response, &lists.0, &addr.0);
//                         return;
//                     }
//                 }
//                 None => (),
//             };

//             let join_room_result = match join_room(
//                 connection_info,
//                 name.to_owned(),
//                 avatarPath.to_owned(),
//                 roomId.to_owned(),
//             ) {
//                 Ok(res) => res,
//                 Err(err) => {
//                     let response = Response::errorReponse {
//                         errorText: err.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             lists.1.lock().unwrap().push(join_room_result.0);

//             match edit_list_element(roomId, lists.2.clone(), |room| {
//                 room.current_players += 1;
//             }) {
//                 Ok(_) => (),
//                 Err(error) => warn!("Could not ad user to room: {}", error),
//             };

//             let response = Response::joinRoomResponse {
//                 token: join_room_result.1,
//                 userList: get_room_user_list(&roomId, lists.1.lock().unwrap()),
//             };

//             send_message(response, &lists.0, &addr.0);

//             let broadcast_response = Response::updateUserList {
//                 userList: get_room_user_list(&roomId, lists.1.lock().unwrap()),
//             };

//             broadcast_message_room_except(
//                 broadcast_response,
//                 &lists.0,
//                 &get_room_user_list(&roomId, lists.1.lock().unwrap()),
//                 &addr.0,
//             );

//             info!("Successful room join for: {}", &addr.0);
//         }
//         Command::heartbeat {} => {
//             info!("Heartbeat from: {}", &addr.0);
//         }
//         Command::startGame { packPath } => {
//             info!("Start game command from: {}", &addr.0);

//             let token_info = match connection_info.4 {
//                 Ok(res) => res.claims,
//                 Err(error) => {
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             match connection_info.1 {
//                 Some(_) => (),
//                 None => {
//                     let response = Response::errorReponse {
//                         errorText: "User does not exist".to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             }

//             match connection_info.3 {
//                 Some(_) => {
//                     let response = Response::errorReponse {
//                         errorText: "Game already in progress".to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//                 None => (),
//             }

//             if !connection_info.1.unwrap().isHost {
//                 let response = Response::errorReponse {
//                     errorText: "Only host can start the game".to_string(),
//                 };
//                 send_message(response, &lists.0, &addr.0);
//                 return;
//             }

//             let broadcast_response = Response::startGame {};
//             broadcast_message_room_all(
//                 broadcast_response,
//                 &lists.0,
//                 &get_room_user_list(&token_info.roomId.clone(), lists.1.lock().unwrap()),
//             );

//             // previously: fs::read_to_string("./packs/test.json")
//             let data = match fs::read_to_string(packPath) {
//                 Ok(data) => data,
//                 Err(error) => {
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };
//             let pack: Pack = match serde_json::from_str(&data) {
//                 Ok(pack) => pack,
//                 Err(error) => {
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             tokio::spawn(handle_game(
//                 (
//                     lists.0.clone(),
//                     lists.1.clone(),
//                     lists.2.clone(),
//                     lists.3.clone(),
//                 ),
//                 get_room_user_list(&token_info.roomId.clone(), lists.1.lock().unwrap()),
//                 connection_info.2.unwrap().id.clone(),
//                 pack,
//             ));

//             info!("Loading pack success");
//         }
//         Command::getUserList {} => {
//             info!("Get user list request from: {}", &addr.0);

//             let token_info = match connection_info.4 {
//                 Ok(res) => res.claims,
//                 Err(error) => {
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             let user_list = get_room_user_list(&token_info.roomId, lists.1.lock().unwrap());

//             let response = Response::updateUserList {
//                 userList: user_list,
//             };

//             send_message(response, &lists.0, &addr.0);
//             info!("Successful get user list from: {}", &addr.0);
//         }
//         Command::broadcastMessage { text } => {
//             info!("Broadcast to room from: {}", &addr.0);

//             let token_info = match connection_info.4 {
//                 Ok(res) => res.claims,
//                 Err(error) => {
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             let response = Response::newMessage {
//                 text: text.to_owned(),
//                 author: token_info.id,
//             };

//             let user_list = get_room_user_list(&token_info.roomId, lists.1.lock().unwrap());

//             broadcast_message_room_all(response, &lists.0, &user_list);
//             info!("Successful broadcast to room from: {}", &addr.0);
//         }
//         Command::writeAnswer { answer } => {
//             info!("Answer message from: {}", &addr.0);

//             let token_info = match connection_info.4 {
//                 Ok(res) => res.claims,
//                 Err(error) => {
//                     warn!("Error at token validation: {}", error.to_string());
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             match connection_info.1 {
//                 Some(_) => (),
//                 None => {
//                     warn!("User does not exist");
//                     let response = Response::errorReponse {
//                         errorText: "User does not exist".to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             }

//             match connection_info.2 {
//                 Some(_) => (),
//                 None => {
//                     warn!("Room does not exist");
//                     let response = Response::errorReponse {
//                         errorText: "Room does not exist".to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             }

//             match connection_info.3 {
//                 Some(_) => (),
//                 None => {
//                     warn!("Game does not exist");
//                     let response = Response::errorReponse {
//                         errorText: "Game does not exist".to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             }

//             let answer = GameCommand {
//                 user_id: token_info.id.to_string(),
//                 answer: *answer,
//             };
//             connection_info
//                 .3
//                 .unwrap()
//                 .1
//                 .unbounded_send(Message::Text(serde_json::to_string(&answer).unwrap()))
//                 .unwrap();

//             info!("Successful answer message from: {}", &addr.0);
//         }
//         Command::changeUsername { newName } => {
//             let token_info = match connection_info.4 {
//                 Ok(res) => res.claims,
//                 Err(error) => {
//                     warn!("Error at token validation: {}", error.to_string());
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             edit_list_element(&token_info.id, lists.1.clone(), |user| {
//                 user.name = newName.clone();
//             })
//             .unwrap();
//         }
//         Command::changeAvatar { newAvatarPath } => {
//             let token_info = match connection_info.4 {
//                 Ok(res) => res.claims,
//                 Err(error) => {
//                     warn!("Error at token validation: {}", error.to_string());
//                     let response = Response::errorReponse {
//                         errorText: error.to_string(),
//                     };
//                     send_message(response, &lists.0, &addr.0);
//                     return;
//                 }
//             };

//             edit_list_element(&token_info.id, lists.1.clone(), |user| {
//                 user.avatarPath = newAvatarPath.clone();
//             })
//             .unwrap();
//         }
//     }
// }

pub fn execute_unauthorized_command(
    command: UnauthorizedCommand,
    lists: Lists,
    connection_id: MutexId,
) {
    match command {
        UnauthorizedCommand::createRoom { name, avatarPath } => {
            // Return error if user exists
            match get_list_element(&connection_id.lock().unwrap().clone(), lists.1.clone()) {
                Some(_) => {
                    let response = Response::errorResponse {
                        errorText: "User already exists".to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
                None => (),
            }

            // Try create user, token and room and handle it
            match create_room(connection_id.clone(), name, avatarPath) {
                Ok(create_room) => {
                    let room_id = create_room.2.id.clone();

                    lists.1.lock().unwrap().push(create_room.0);
                    lists.2.lock().unwrap().push(create_room.2);

                    let user_list = get_room_user_list(&room_id, lists.1.clone());

                    let response = Response::createRoomResponse {
                        token: create_room.1,
                        userList: user_list.clone(),
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );

                    let user_list_response = Response::updateUserList {
                        userList: user_list.clone(),
                    };
                    broadcast_message_room_except(
                        user_list_response,
                        lists.0.clone(),
                        &user_list,
                        &connection_id.lock().unwrap().clone(),
                    );
                }
                Err(error) => {
                    let response = Response::errorResponse {
                        errorText: error,
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            }
        }
        UnauthorizedCommand::joinRoom {
            name,
            avatarPath,
            roomId,
        } => {
            // Return error if user exists
            match get_list_element(&connection_id.lock().unwrap().clone(), lists.1.clone()) {
                Some(_) => {
                    let response = Response::errorResponse {
                        errorText: "User already exists".to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
                None => (),
            }

            // Return if room does not exist
            let room = match get_list_element(&roomId, lists.2.clone()) {
                Some(room) => room,
                None => {
                    let response = Response::errorResponse {
                        errorText: "Room does not exist".to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            };

            // Return if already max players in room
            if room.current_players >= room.max_players {
                let response = Response::errorResponse {
                    errorText: "Room is full".to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
                return;
            }

            // Return if game in progress
            if lists.3.lock().unwrap().contains_key(&roomId) {
                let response = Response::errorResponse {
                    errorText: "Game is in progress".to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
                return;
            }

            // Try create user and token and handle it
            match join_room(connection_id.clone(), name, avatarPath, &roomId) {
                Ok(join_room) => {
                    lists.1.lock().unwrap().push(join_room.0);

                    let user_list = match connect_user_to_room(
                        &roomId,
                        &connection_id.lock().unwrap().clone(),
                        (lists.0.clone(), lists.1.clone(), lists.2.clone()),
                    ) {
                        Ok(list) => list,
                        Err(error) => {
                            let response = Response::errorResponse {
                                errorText: error,
                                errorCode: 0,
                            };
                            send_message(
                                response,
                                lists.0.clone(),
                                &connection_id.lock().unwrap().clone(),
                            );
                            return;
                        }
                    };

                    let token_response = Response::joinRoomResponse {
                        token: join_room.1,
                        userList: user_list.clone(),
                    };
                    send_message(
                        token_response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );

                    let user_list_response = Response::updateUserList {
                        userList: user_list.clone(),
                    };
                    broadcast_message_room_except(
                        user_list_response,
                        lists.0.clone(),
                        &user_list,
                        &connection_id.lock().unwrap().clone(),
                    );
                }
                Err(error) => {
                    let response = Response::errorResponse {
                        errorText: error,
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            }
        }
        UnauthorizedCommand::heartbeat {} => {
            info!("Heartbeat from: {}", &connection_id.lock().unwrap().clone());
        }
    }
}

pub fn execute_authorized_command(
    command_token_pair: CommandTokenPair,
    lists: Lists,
    connection_id: MutexId,
) {
    // Validate token
    let token_info = match decode_token(&command_token_pair.token) {
        Ok(info) => info.claims,
        Err(error) => {
            let response = Response::errorResponse {
                errorText: error,
                errorCode: 2,
            };
            send_message(
                response,
                lists.0.clone(),
                &connection_id.lock().unwrap().clone(),
            );
            return;
        }
    };

    match command_token_pair.command {
        AuthorizedCommand::reconnectRoom {} => {
            println!("Started RECONNECT 2.1");
            // let mut peer_map_lock = lists.0.lock().unwrap();

            println!("Locked list of connections 2.1.5");
            // Return if connection is active
            let peer_map_active = lists.0.lock().unwrap().contains_key(&token_info.id).clone();
            println!("Got bool for connection active 2.1.5");
            if peer_map_active {
                let response = Response::errorResponse {
                    errorText: "User already active".to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
                println!("Returned RECONNECT user active 2.3");
                return;
            }
            println!("Done user active check 2.1.5");

            // Return if this connection has no tx channel
            let connection_channel = match lists
                .0
                .lock()
                .unwrap()
                .get(&connection_id.lock().unwrap().clone())
            {
                Some(tx) => tx.clone(),
                None => {
                    let response = Response::errorResponse {
                        errorText: "Cannot find connection channel".to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    println!("Returned RECONNECT connction channel no longer exists 2.3");
                    return;
                }
            };
            println!("Done connection remove check 2.1.5");

            // Check if user has been removed in the meantime
            let existing_user = get_list_element(&token_info.id, lists.1.clone());
            match existing_user {
                Some(_) => (),
                None => {
                    let response = Response::errorResponse {
                        errorText: "User has been removed".to_string(),
                        errorCode: 2,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    println!("Returned RECONNECT user no longer exists 2.3");
                    return;
                }
            }
            println!("Done user remove check 2.1.5");

            // Remove and re-insert tx channel with id from token
            lists
                .0
                .lock()
                .unwrap()
                .remove(&connection_id.lock().unwrap().clone());
            lists
                .0
                .lock()
                .unwrap()
                .insert(token_info.id.clone(), connection_channel);

            // Change connection ID for connection handler
            *connection_id.lock().unwrap() = token_info.id.clone();

            // drop(peer_map_lock);

            println!("Insert-reinsert 2.1.5");
            // Respond with room user list
            let user_list_response = Response::updateUserList {
                userList: get_room_user_list(&token_info.roomId, lists.1.clone()).clone(),
            };
            send_message(user_list_response, lists.0.clone(), &token_info.id);
            println!("Finished RECONNECT 2.2");
        }
        AuthorizedCommand::startGame { packPath } => {
            info!(
                "Start game command from: {}",
                &connection_id.lock().unwrap().clone()
            );

            // Return error if user doesn't exist
            match get_list_element(&connection_id.lock().unwrap().clone(), lists.1.clone()) {
                Some(_) => (),
                None => {
                    let response = Response::errorResponse {
                        errorText: "User does not exist".to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            }

            // Return error if game exists (i.e. is in progress)
            if lists.3.lock().unwrap().contains_key(&token_info.roomId) {
                let response = Response::errorResponse {
                    errorText: "Game in progress".to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
                return;
            }

            // Return error if user is not host
            let is_user_host = get_list_element(&token_info.id, lists.1.clone())
                .unwrap()
                .isHost;
            if !is_user_host {
                let response = Response::errorResponse {
                    errorText: "Only host can start game".to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
                return;
            }
            drop(is_user_host);

            // Broadcast to the room that the game has started
            let broadcast_response = Response::startGame {};
            broadcast_message_room_all(
                broadcast_response,
                lists.0.clone(),
                &get_room_user_list(&token_info.roomId.clone(), lists.1.clone()),
            );

            // Read pack data from file to string, return error if did not work
            // previously: fs::read_to_string("./packs/test.json")
            let data = match fs::read_to_string(packPath) {
                Ok(data) => data,
                Err(error) => {
                    let response = Response::errorResponse {
                        errorText: error.to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            };

            // Transform pack data to Pack object, return error if did not work
            let pack: Pack = match serde_json::from_str(&data) {
                Ok(pack) => pack,
                Err(error) => {
                    let response = Response::errorResponse {
                        errorText: error.to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            };

            // Spawn a thread to handle game
            tokio::spawn(handle_game(
                (
                    lists.0.clone(),
                    lists.1.clone(),
                    lists.2.clone(),
                    lists.3.clone(),
                ),
                get_room_user_list(&token_info.roomId.clone(), lists.1.clone()),
                token_info.roomId.clone(),
                pack,
            ));

            info!("Loading pack success");
        }
        AuthorizedCommand::getUserList {} => (),
        AuthorizedCommand::broadcastMessage { text } => {
            // Broadcast to everybody in the room
            let response = Response::newMessage {
                author: token_info.id,
                text: text,
            };
            broadcast_message_room_all(
                response,
                lists.0.clone(),
                &get_room_user_list(&token_info.roomId, lists.1.clone()),
            );
        }
        AuthorizedCommand::writeAnswer { answer } => {
            info!(
                "Answer message from: {}",
                &connection_id.lock().unwrap().clone()
            );

            // Return error if user doesn't exist
            match get_list_element(&connection_id.lock().unwrap().clone(), lists.1.clone()) {
                Some(_) => (),
                None => {
                    let response = Response::errorResponse {
                        errorText: "User does not exist".to_string(),
                        errorCode: 0,
                    };
                    send_message(
                        response,
                        lists.0.clone(),
                        &connection_id.lock().unwrap().clone(),
                    );
                    return;
                }
            }

            // Return error if game doesn't exist
            if !lists.3.lock().unwrap().contains_key(&token_info.roomId) {
                let response = Response::errorResponse {
                    errorText: "Game is not started".to_string(),
                    errorCode: 0,
                };
                send_message(
                    response,
                    lists.0.clone(),
                    &connection_id.lock().unwrap().clone(),
                );
                return;
            }

            let answer = GameCommand {
                user_id: token_info.id.to_string(),
                answer: answer,
            };
            lists
                .3
                .lock()
                .unwrap()
                .get(&token_info.roomId)
                .unwrap()
                .unbounded_send(Message::Text(serde_json::to_string(&answer).unwrap()))
                .unwrap();

            info!(
                "Successful answer message from: {}",
                &connection_id.lock().unwrap().clone()
            );
        }
        AuthorizedCommand::changeUsername { newName } => {
            info!("{}", newName);
        }
        AuthorizedCommand::changeAvatar { newAvatarPath } => {
            info!("{}", newAvatarPath);
        }
    }
}

fn create_room(
    id: MutexId,
    name: String,
    avatar_path: String,
) -> Result<(User, String, Room), String> {
    let new_room = Room {
        id: Uuid::new_v4().to_string(),
        max_players: 6,
        host_id: id.lock().unwrap().clone(),
        current_players: 1,
    };

    let color: UserColors = rand::random();
    let new_user = User {
        id: id.lock().unwrap().clone(),
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
    id: MutexId,
    name: String,
    avatar_path: String,
    room_id: &String,
) -> Result<(User, String), String> {
    let color: UserColors = rand::random();
    let new_user = User {
        id: id.lock().unwrap().clone(),
        name: name.to_string(),
        avatarPath: avatar_path.to_string(),
        roomId: room_id.clone(),
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
