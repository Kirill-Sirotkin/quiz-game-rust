use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};

pub trait HasId {
    fn get_id(&self) -> String;
}

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
impl HasId for User {
    fn get_id(&self) -> String {
        return self.id.clone();
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    pub id: String,
    pub max_players: i32,
    pub host_id: String,
    pub current_players: i32,
}
impl HasId for Room {
    fn get_id(&self) -> String {
        return self.id.clone();
    }
}

pub enum UserColors {
    Black,
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
            0 => UserColors::Yellow,
            1 => UserColors::Blue,
            2 => UserColors::Black,
            3 => UserColors::Green,
            4 => UserColors::Purple,
            5 => UserColors::Brown,
            6 => UserColors::Orange,
            7 => UserColors::Cyan,
            _ => UserColors::Red,
        }
    }
}
