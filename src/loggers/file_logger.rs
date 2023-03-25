use chrono;
use log::{info, LevelFilter, SetLoggerError};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn init_file_logger() -> Result<(), SetLoggerError> {
    let current_date = chrono::offset::Utc::now().date_naive().to_string();
    let path = format!("log/{}.log", current_date);

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%H:%M:%S)(utc)} {l} - {m}\n",
        )))
        .build(path);

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile.unwrap())))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info));

    log4rs::init_config(config.unwrap())?;
    info!("File logger initialized");

    Ok(())
}
