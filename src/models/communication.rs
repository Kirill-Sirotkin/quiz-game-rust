#![allow(non_camel_case_types, non_snake_case)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{game::Answer, lobby::User};

#[derive(Serialize, Deserialize)]
#[serde(tag = "response", content = "data")]
pub enum Response {
    createRoomResponse {
        token: String,
    },
    joinRoomResponse {
        token: String,
        userList: Vec<User>,
    },
    updateUserList {
        userList: Vec<User>,
    },
    newMessage {
        author: String,
        text: String,
    },
    startGame,
    errorReponse {
        errorText: String,
    },
    questionResponse {
        question: String,
    },
    answersResponse {
        answers: Vec<Answer>,
        timer: i32,
    },
    timerResponse {
        timer: i32,
    },
    correctAnswerResponse {
        answers: HashMap<String, i32>,
        correct_answer: i32,
    },
    scoresResponse {
        scores: HashMap<String, i32>,
    },
}

pub enum Command {
    UnauthorizedCommand(UnauthorizedCommand),
    CommandTokenPair(CommandTokenPair),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UnauthorizedCommand {
    createRoom {
        name: String,
        avatarPath: String,
    },
    joinRoom {
        name: String,
        avatarPath: String,
        roomId: String,
    },
    heartbeat,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AuthorizedCommand {
    reconnectRoom {},
    startGame { packPath: String },
    getUserList {},
    broadcastMessage { text: String },
    writeAnswer { answer: i32 },
    changeUsername { newName: String },
    changeAvatar { newAvatarPath: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandTokenPair {
    #[serde(flatten)]
    pub command: AuthorizedCommand,
    pub token: String,
}
