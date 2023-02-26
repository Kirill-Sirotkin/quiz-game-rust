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
    use std::net::TcpStream;

    use serde::{Deserialize, Serialize};
    use tungstenite::{Message, WebSocket};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TextCommand {
        pub command_text: String,
    }
    impl Command for TextCommand {
        fn default_action(&self, mut websocket: WebSocket<TcpStream>) {
            let message = Message::text(self.command_text.to_owned());
            websocket.write_message(message).unwrap();
            //info!("{}", logtext);
        }
    }

    pub trait Command {
        fn default_action(&self, websocket: WebSocket<TcpStream>);
    }
}
