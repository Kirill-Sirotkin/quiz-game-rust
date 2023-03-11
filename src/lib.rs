pub mod logger {
    use log::{Level, Metadata, Record};
    struct SimpleLogger;

    impl log::Log for SimpleLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Info
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                println!("{} - {}", record.level(), record.args());
            }
        }

        fn flush(&self) {}
    }

    use log::{LevelFilter, SetLoggerError};

    static LOGGER: SimpleLogger = SimpleLogger;

    pub fn init_logger() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
    }
}

pub mod command {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum Command {
        createRoom { name: String },
        joinRoom { name: String, roomId: String },
    }

    #[derive(Serialize, Deserialize)]
    pub struct CommandTokenPair {
        #[serde(flatten)]
        command: Command,
        token: String,
    }
}

pub mod quiz_game_backend_models {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    pub struct User {
        pub id: String,
        pub name: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Room {
        pub id: String,
        pub name: String,
        pub max_players: i32,
        pub host_id: String,
        pub user_list: Vec<User>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum Response {
        createRoomResponse { token: String, roomId: String },
        joinRoomResponse { token: String, userList: Vec<User> },
        updateUserList { userList: Vec<User> },
        errorReponse { errorText: String },
    }

    #[derive(Serialize, Deserialize)]
    pub struct Claims {
        pub id: String,
    }
}
