use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    models::{
        communication::{Command, Response},
        game::GameCommand,
        lobby::{HasId, Room, User},
    },
    server_messages::broadcast_message_room_except,
};
use futures_channel::mpsc::UnboundedSender;
use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type Lists = (PeerMap, UserList, RoomList);

pub fn parse_command(msg: &Message) -> Result<Command, String> {
    let unauthorized_command = match serde_json::from_str(&msg.to_string()) {
        Ok(command) => return Ok(Command::UnauthorizedCommand(command)),
        Err(error) => error.to_string(),
    };
    match serde_json::from_str(&msg.to_string()) {
        Ok(command) => return Ok(Command::CommandTokenPair(command)),
        Err(error) => {
            return Err(format!(
                "Cannot parse authorized command: {}; Cannot parse unauthorized command: {}",
                error.to_string(),
                unauthorized_command
            ))
        }
    };
}

pub fn parse_game_command(msg: &Message) -> Result<(String, i32), String> {
    let parsed_msg: Result<GameCommand, serde_json::Error> = serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => {
            return Ok((command.user_id, command.answer));
        }
        Err(error) => return Err(error.to_string()),
    }
}

pub fn connect_user_to_room(
    room_id: &String,
    user_id: &String,
    lists: Lists,
) -> Result<Vec<User>, String> {
    match edit_list_element(room_id, lists.2.clone(), |room| {
        room.current_players += 1;
    }) {
        Ok(_) => (),
        Err(error) => {
            return Err(format!("connect_user_to_room: {}", error));
        }
    }

    let room_info = get_list_element(room_id, lists.2.clone());
    match room_info {
        Some(room) => {
            if room.current_players == 1 {
                match edit_list_element(user_id, lists.1.clone(), |user| {
                    user.isHost = true;
                }) {
                    Ok(_) => (),
                    Err(error) => {
                        return Err(format!("connect_user_to_room: {}", error));
                    }
                }
            }
        }
        None => (),
    }

    let user_list = get_room_user_list(room_id, lists.1.clone());

    let user_list_response = Response::updateUserList {
        userList: user_list.clone(),
    };
    broadcast_message_room_except(user_list_response, lists.0.clone(), &user_list, user_id);

    Ok(user_list)
}

pub fn get_room_user_list(room_id: &String, user_list: UserList) -> Vec<User> {
    let users: Vec<User> = user_list
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .filter(|user| &user.roomId == room_id)
        .collect();

    return users;
}

pub fn get_list_element<T: Clone + HasId>(id: &String, list: Arc<Mutex<Vec<T>>>) -> Option<T> {
    return match list
        .lock()
        .unwrap()
        .iter()
        .find(|element| &element.get_id() == id)
    {
        Some(element) => Some(element.clone()),
        None => None,
    };
}

pub fn edit_list_element<F, T: HasId>(
    id: &String,
    list: Arc<Mutex<Vec<T>>>,
    mut function: F,
) -> Result<(), String>
where
    F: FnMut(&mut T),
{
    let mut elements = list.lock().unwrap();
    let target_element = match elements.iter_mut().find(|element| &element.get_id() == id) {
        Some(element) => element,
        None => return Err("edit_list_element: Element does not exist in list".to_string()),
    };

    function(target_element);

    Ok(())
}
