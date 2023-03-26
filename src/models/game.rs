use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Answer {
    pub number: i32,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct Question {
    pub text: String,
    pub answers: Vec<Answer>,
    pub correct_answer: i32,
    pub duration_sec: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Pack {
    pub name: String,
    pub questions: Vec<Question>,
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
pub struct GameCommand {
    pub user_id: String,
    pub answer: i32,
}
