use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatarPath: String,
    pub roomId: String,
    pub isHost: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    pub id: String,
    pub max_players: i32,
    pub host_id: String,
    pub current_players: i32,
}
