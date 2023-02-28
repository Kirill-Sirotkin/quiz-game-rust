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

    let queues_array = Arc::new(Mutex::new(Vec::new()));

    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    for stream in server.incoming() {
        let queue_stream: Arc<SegQueue<Command>> = Arc::new(SegQueue::new());
        let mut g = queues_array.lock().unwrap();
        g.push(queue_stream.clone());
        drop(g);
        make_threads(&mut queues_array.clone(), queue_stream, stream.unwrap());
        //info!("array: {}", queues_array.len());
    }
}

fn make_threads(
    v: &mut Arc<Mutex<Vec<Arc<SegQueue<Command>>>>>,
    queue_original: Arc<SegQueue<Command>>,
    stream_param: TcpStream,
) {
    // let queue_original: Arc<SegQueue<Command>> = Arc::new(SegQueue::new());
    let mut websocket = accept(stream_param).unwrap();
    // let mut g = v.lock().unwrap();
    // g.push(queue_original.clone());
    // drop(g);

    let queue = queue_original.clone();
    let queues_list = v.clone();
    spawn(move || loop {
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
                Command::BroadcastMessage { text } => {
                    let g = queues_list.lock().unwrap();
                    for i in g.iter() {
                        i.push(Command::SendMessage { text: text.clone() })
                    }
                    warn!("list: {}", g.len());
                    drop(g);
                }
            }
        }
        let msg = websocket.read_message().unwrap();

        let parsed_msg: Result<Command, serde_json::Error> = serde_json::from_str(&msg.to_string());
        match parsed_msg {
            Ok(msg) => queue.push(msg),
            Err(error) => warn!("Command parse error: {}", error),
        }
    });
}
