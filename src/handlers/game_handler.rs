use crate::{
    helpers::parse_game_command,
    models::{
        communication::Response,
        game::Pack,
        lobby::{Room, User},
    },
    server_messages::broadcast_message_room_all,
};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_timer::Delay;
use futures_util::{future, pin_mut, StreamExt};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type Lists = (PeerMap, UserList, RoomList, GameList);

pub async fn handle_game(lists: Lists, user_list: Vec<User>, room_id: String, pack: Pack) {
    let (tx_room, rx_room) = unbounded();
    lists.3.lock().unwrap().insert(room_id.clone(), tx_room);

    let answers = Arc::new(Mutex::new(HashMap::<String, i32>::new()));
    user_list.iter().for_each(|user| {
        answers.lock().unwrap().insert(user.id.clone(), -1);
    });

    let scores = Arc::new(Mutex::new(HashMap::<String, i32>::new()));
    user_list.iter().for_each(|user| {
        scores.lock().unwrap().insert(user.id.clone(), 0);
    });

    let receive_future = rx_room.for_each(|msg| {
        match parse_game_command(&msg) {
            Ok(command) => match answers.lock().unwrap().get_mut(&command.0.id) {
                Some(user_answer) => *user_answer = command.1,
                None => (),
            },
            Err(_) => (),
        }
        future::ready(())
    });

    let answers_clone = answers.clone();
    let scores_clone = scores.clone();
    let game_process_future = async move {
        let mut questions_index = 0;
        while questions_index < pack.questions.len() {
            let question = pack.questions.get(questions_index).unwrap();

            let question_announcement = Response::questionResponse {
                question: question.text.clone(),
            };
            broadcast_message_room_all(question_announcement, &lists.0, &user_list);
            Delay::new(Duration::from_secs(2)).await;

            let answers_and_timer = Response::answersResponse {
                answers: question.answers.clone(),
                timer: question.duration_sec,
            };
            broadcast_message_room_all(answers_and_timer, &lists.0, &user_list);

            let mut timer_iter = question.duration_sec;
            while timer_iter >= 0 {
                let timer_response = Response::timerResponse { timer: timer_iter };
                broadcast_message_room_all(timer_response, &lists.0, &user_list);
                Delay::new(Duration::from_secs(1)).await;

                timer_iter -= 1;
            }

            let correct_answer_response = Response::correctAnswerResponse {
                answers: answers_clone.lock().unwrap().clone(),
                correct_answer: question.correct_answer,
            };
            broadcast_message_room_all(correct_answer_response, &lists.0, &user_list);

            answers_clone.lock().unwrap().iter().for_each(|answer| {
                if answer.1 == &question.correct_answer {
                    *scores_clone.lock().unwrap().get_mut(answer.0).unwrap() += 100;
                }
            });

            let scores_response = Response::scoresResponse {
                scores: scores_clone.lock().unwrap().clone(),
            };
            broadcast_message_room_all(scores_response, &lists.0, &user_list);

            answers_clone
                .lock()
                .unwrap()
                .iter_mut()
                .for_each(|answer| *answer.1 = -1);

            Delay::new(Duration::from_secs(2)).await;

            questions_index += 1;
        }
    };

    pin_mut!(receive_future, game_process_future);
    future::select(receive_future, game_process_future).await;

    lists.3.lock().unwrap().remove(&room_id);
}
