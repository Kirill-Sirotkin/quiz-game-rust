use futures_channel::mpsc::UnboundedSender;
use log::info;
use quiz_game_rust::{
    handlers::connection_handler::handle_connection,
    loggers::file_logger::init_file_logger,
    models::lobby::{Room, User},
};
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<(SocketAddr, String), Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;

#[tokio::main]
async fn main() -> Result<(), IoError> {
    init_file_logger().unwrap();
    info!("App started!");

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:9001".to_string());

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    let users = UserList::new(Mutex::new(Vec::new()));
    let rooms = RoomList::new(Mutex::new(Vec::new()));
    let games = GameList::new(Mutex::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            (state.clone(), users.clone(), rooms.clone(), games.clone()),
            stream,
            addr,
        ));
    }

    Ok(())
}
