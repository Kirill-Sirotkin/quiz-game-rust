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
        AddUser { name: String },
        GetUsers {},
        CreateRoom { name: String, max_players: i32 },
        GetRooms {},
        SendMessage { text: String },
        Disconnect {},
        BroadcastMessage { text: String },
    }

    #[derive(Serialize, Deserialize)]
    pub struct CommandTokenPair {
        #[serde(flatten)]
        command: Command,
        token: String,
    }

    pub enum MessageType {
        Broadcast,
        Target,
    }
}

pub mod quiz_game_backend_models {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct User {
        pub id: String,
        pub name: String,
        pub address: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Room {
        pub id: String,
        pub name: String,
        pub max_players: i32,
        pub host_id: String,
    }
}
