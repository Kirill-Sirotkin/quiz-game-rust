use std::sync::MutexGuard;

use crate::models::{communication::CommandTokenPair, game::GameCommand, lobby::User};
use tungstenite::Message;

pub fn parse_command(msg: &Message) -> Result<CommandTokenPair, String> {
    let parsed_msg: Result<CommandTokenPair, serde_json::Error> =
        serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => return Ok(command),
        Err(error) => return Err(error.to_string()),
    }
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

pub fn get_room_user_list(room_id: &String, user_list: MutexGuard<Vec<User>>) -> Vec<User> {
    let users = user_list.iter().filter(|user| &user.roomId == room_id);
    let mut room_users = Vec::<User>::new();

    for user in users {
        room_users.push(user.clone());
    }

    return room_users;
}
