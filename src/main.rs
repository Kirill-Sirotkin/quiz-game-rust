use futures_channel::mpsc::UnboundedSender;
use log::info;
use native_tls::Identity;
use quiz_game_rust::{
    handlers::connection_handler::handle_connection,
    loggers::file_logger::init_file_logger,
    models::lobby::{Room, User},
};
use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type TxTimeout = UnboundedSender<bool>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type GameList = Arc<Mutex<HashMap<String, Tx>>>;
type UserTimeoutList = Arc<Mutex<HashMap<String, TxTimeout>>>;

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
    info!("Listening on: {}", addr);

    let users = UserList::new(Mutex::new(Vec::new()));
    let rooms = RoomList::new(Mutex::new(Vec::new()));
    let games = GameList::new(Mutex::new(HashMap::new()));
    let user_timeouts = UserTimeoutList::new(Mutex::new(HashMap::new()));

    let der: [u8; 4] = [1, 2, 3, 4];
    let cert = Identity::from_pkcs12(&der, "quiz").expect("can't certify");
    let tls_acceptor = tokio_native_tls::TlsAcceptor::from(
        native_tls::TlsAcceptor::builder(cert)
            .build()
            .expect("can't make tls acceptor"),
    );

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            (
                state.clone(),
                users.clone(),
                rooms.clone(),
                games.clone(),
                user_timeouts.clone(),
            ),
            stream,
            addr,
            tls_acceptor.clone(),
        ));
    }

    Ok(())
}
