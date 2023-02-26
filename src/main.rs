use crossbeam_queue::SegQueue;
use log::{info, warn};
use quiz_game_rust::command::Command;
use quiz_game_rust::logger::init_logger;
use serde::{Deserialize, Serialize};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;
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
        make_threads(&mut queues_array, stream.unwrap());
        info!("array: {}", queues_array.len());
    }
}

fn make_threads(v: &mut Vec<Arc<SegQueue<Command>>>, stream_param: TcpStream) {
    let queue_original: Arc<SegQueue<Command>> = Arc::new(SegQueue::new());
    let queue_msgs_original: Arc<SegQueue<tungstenite::Message>> = Arc::new(SegQueue::new());
    let mut websocket = accept(stream_param).unwrap();
    v.push(queue_original.clone());

    let queue = queue_original.clone();
    let queue_msgs = queue_msgs_original.clone();
    let queues_list = v.clone();
    // spawn(move || {
    //     warn!("t1 start");
    //     loop {
    //         while let Some(i) = queue.pop() {
    //             // info!("{}", i);
    //             match i {
    //                 Command::SendMessage { text } => {
    //                     queue_msgs.push(tungstenite::Message::Text(String::from(text)));
    //                 }
    //                 Command::Disconnect {} => {
    //                     queue_msgs.push(tungstenite::Message::Text(String::from("Disconnected")))
    //                 }
    //             }
    //         }
    //     }
    // });
    let queue = queue_original.clone();
    let queue_msgs = queue_msgs_original.clone();
    spawn(move || loop {
        // while let Some(i) = queue_msgs.pop() {
        //     websocket.write_message(i).unwrap();
        // }
        while let Some(i) = queue.pop() {
            match i {
                Command::SendMessage { text } => {
                    websocket
                        .write_message(tungstenite::Message::Text(text.to_string()))
                        .unwrap();
                }
                Command::Disconnect {} => {
                    websocket
                        .write_message(tungstenite::Message::Text("Disconnect!".to_string()))
                        .unwrap();
                }
            }
        }
        let msg = websocket.read_message().unwrap();

        let parsed_msg: Result<Command, serde_json::Error> = serde_json::from_str(&msg.to_string());
        match parsed_msg {
            Ok(msg) => queue.push(msg),
            Err(error) => warn!("Command parse error: {}", error),
        }
        //std::thread::sleep(Duration::from_millis(50));
    });
}
