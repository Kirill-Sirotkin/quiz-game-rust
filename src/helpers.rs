use std::sync::MutexGuard;

use crate::{
    jwtoken::{decode_token, Claims},
    models::{communication::Command, game::GameCommand, lobby::User},
};
use tungstenite::Message;

pub fn parse_command(msg: &Message) -> Result<Command, serde_json::Error> {
    let parsed_msg: Result<Command, serde_json::Error> = serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => return Ok(command),
        Err(error) => return Err(error),
    }
}

pub fn parse_game_command(msg: &Message) -> Result<(Claims, i32), String> {
    let parsed_msg: Result<GameCommand, serde_json::Error> = serde_json::from_str(&msg.to_string());
    match parsed_msg {
        Ok(command) => {
            let token_info = match decode_token(&command.token) {
                Ok(res) => res,
                Err(err) => return Err(err.to_string()),
            };
            return Ok((token_info.claims, command.answer));
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
