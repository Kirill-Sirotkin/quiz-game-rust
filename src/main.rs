use crossbeam_queue::SegQueue;
use log::{info, warn};
use quiz_game_rust::command::{Command, TextCommand};
use quiz_game_rust::logger::init_logger;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::spawn;
use tungstenite::accept;

#[derive(Serialize, Deserialize, Debug)]
struct User {
    username: String,
    age: i32,
    email: String,
}

fn main() {
    init_logger().unwrap();
    warn!("success");

    let mut queues_array = Vec::new();

    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    for stream in server.incoming() {
        make_threads(&mut queues_array, stream);
    }
}

fn make_threads(v: &mut Vec<Arc<SegQueue<Box<&impl Command>>>>, stream: Result<TcpStream, Error>) {
    let queue_original: Arc<SegQueue<Box<&impl Command>>> = Arc::new(SegQueue::new());
    v.push(queue_original.clone());

    let queue = queue_original.clone();
    spawn(move || {
        let mut websocket = accept(stream.unwrap()).unwrap();
        warn!("t1 start");
        loop {
            while let Some(i) = queue.pop() {
                // info!("{}", i);
                i.default_action(websocket);
            }
        }
    });
    let queue = queue_original.clone();
    spawn(move || {
        warn!("t2 start");
        for i in 1..100 {
            //queue.push(i);
        }
    });
    let mut queue_list = v.clone();
    spawn(move || {
        let mut websocket = accept(stream.unwrap()).unwrap();

        loop {
            let msg = websocket.read_message().unwrap();

            let user_info: Result<User, _> = serde_json::from_str(&msg.to_string());
            let text_command: Result<Option<TextCommand>, _> =
                serde_json::from_str(&msg.to_string());

            match user_info {
                Ok(res) => {
                    info!("user OK");
                }
                Err(error) => {
                    warn!("User none! {}", error);
                }
            };

            match text_command {
                Ok(res) => {
                    info!("text command OK");
                    // let text = res.as_ref().unwrap().command_text.parse().unwrap();
                    for i in queue_list.iter_mut() {
                        i.push(Box::new(res.unwrap()));
                    }
                    res
                }
                Err(error) => {
                    warn!("text command none! {}", error);
                    None
                }
            };

            // if msg.is_binary() || msg.is_text() {
            //     websocket.write_message(msg).unwrap();
            // }
        }
    });
}
