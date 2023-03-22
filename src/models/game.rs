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
    pub token: String,
    pub answer: i32,
}

pub fn test_pack() -> Pack {
    let pack = Pack {
        name: "test pack".to_string(),
        questions: vec![
            Question {
                text: "Is a1 correct?".to_string(),
                answers: vec![
                    Answer {
                        number: 1,
                        text: "A1".to_string(),
                    },
                    Answer {
                        number: 2,
                        text: "A2".to_string(),
                    },
                ],
                correct_answer: 1,
                duration_sec: 15,
            },
            Question {
                text: "Is a2 correct?".to_string(),
                answers: vec![
                    Answer {
                        number: 1,
                        text: "A1".to_string(),
                    },
                    Answer {
                        number: 2,
                        text: "A2".to_string(),
                    },
                ],
                correct_answer: 2,
                duration_sec: 15,
            },
        ],
    };
    return pack;
}
