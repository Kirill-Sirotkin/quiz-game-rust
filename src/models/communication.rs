use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{game::Answer, lobby::User};

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
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
    startGame {},
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

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
pub enum Command {
    createRoom {
        name: String,
        avatarPath: String,
    },
    joinRoom {
        name: String,
        avatarPath: String,
        roomId: String,
    },
    heartbeat {},
    startGame {
        token: String,
        packPath: String,
    },
    getUserList {
        token: String,
    },
    broadcastMessage {
        token: String,
        text: String,
    },
    writeAnswer {
        token: String,
        answer: i32,
    },
    changeUsername {
        token: String,
        newName: String,
    },
    changeAvatar {
        token: String,
        newAvatarPath: String,
    },
}
