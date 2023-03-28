use std::sync::{Arc, Mutex};

use crate::models::{
    communication::Command,
    game::GameCommand,
    lobby::{HasId, User},
};
use tungstenite::Message;

type UserList = Arc<Mutex<Vec<User>>>;

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
        None => return Err("Element does not exist in list".to_string()),
    };

    function(target_element);

    Ok(())
}
