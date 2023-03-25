use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
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
    pub userColor: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    pub id: String,
    pub max_players: i32,
    pub host_id: String,
    pub current_players: i32,
}

pub enum UserColors {
    Black,
    White,
    Yellow,
    Blue,
    Red,
    Green,
    Purple,
    Brown,
    Orange,
    Cyan,
}

impl UserColors {
    pub fn value(&self) -> String {
        match *self {
            UserColors::Black => "#000000".to_string(),
            UserColors::White => "#FFFFFF".to_string(),
            UserColors::Yellow => "#FFFF00".to_string(),
            UserColors::Blue => "#00FFFF".to_string(),
            UserColors::Red => "#FF0000".to_string(),
            UserColors::Green => "#00FF00".to_string(),
            UserColors::Purple => "#A020F0".to_string(),
            UserColors::Brown => "#964B00".to_string(),
            UserColors::Orange => "#FFA500".to_string(),
            UserColors::Cyan => "#00FFFF".to_string(),
        }
    }
}

impl Distribution<UserColors> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UserColors {
        match rng.gen_range(0..=9) {
            0 => UserColors::White,
            1 => UserColors::Yellow,
            2 => UserColors::Blue,
            3 => UserColors::Black,
            4 => UserColors::Green,
            5 => UserColors::Purple,
            6 => UserColors::Brown,
            7 => UserColors::Orange,
            8 => UserColors::Cyan,
            _ => UserColors::Red,
        }
    }
}
