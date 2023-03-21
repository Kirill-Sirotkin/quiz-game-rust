use serde::{Deserialize, Serialize};

use super::lobby::User;

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
pub enum Response {
    createRoomResponse { token: String },
    joinRoomResponse { token: String, userList: Vec<User> },
    updateUserList { userList: Vec<User> },
    newMessage { text: String },
    startGame {},
    errorReponse { errorText: String },
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
pub enum Command {
    createRoom { name: String },
    joinRoom { name: String, roomId: String },
    heartbeat {},
    startGame { token: String },
    getUserList { token: String },
    broadcastMessage { token: String, text: String },
}
