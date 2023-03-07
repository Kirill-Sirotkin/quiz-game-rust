use log::{info, warn};
use quiz_game_rust::command::Command;
use quiz_game_rust::logger::init_logger;
use serde::{Deserialize, Serialize};
use tokio::*;
use tokio_tungstenite::*;

fn make_threads(
    v: &mut Arc<Mutex<Vec<Arc<SegQueue<Command>>>>>,
    queue_original: Arc<SegQueue<Command>>,
    stream_param: TcpStream,
) {
    let mut websocket_result = accept(stream_param);
    match websocket_result {
        Ok(_) => (),
        Err(err) => {
            warn!("WS Err: {}", err);
            return;
        }
    };

    let queue = queue_original.clone();
    let queues_list = v.clone();
    spawn(move || loop {
        let websocket = websocket_result.as_mut().unwrap();
        if !websocket.can_read() {
            warn!("socket closed!");
            break;
        }

        websocket.get_ref().set_nonblocking(true).expect("fail");
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
        let msg = websocket.read_message();
        match msg {
            Ok(msg) => {
                let parsed_msg: Result<Command, serde_json::Error> =
                    serde_json::from_str(&msg.to_string());
                match parsed_msg {
                    Ok(msg) => queue.push(msg),
                    Err(error) => warn!("Command parse error: {}", error),
                }
            }
            Err(_) => (),
        }
    });
}
