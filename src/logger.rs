use chrono::Local;
use lazy_static::lazy_static;
use std::fs;
use std::{io::Write, sync::Mutex};

use crate::{utils::*};

lazy_static! {
    static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::new().unwrap());
}

pub struct Logger {
    file: std::fs::File,
    last_message: Option<String>,
}

impl Logger {
    fn new() -> Result<Self, std::io::Error> {
        let config_dir = get_config();
        let log_path = config_dir.join("log.txt");

        if !fs::exists(&log_path).expect("Couldn't check if log path exists") {
            let _ = std::fs::write(&log_path, "").expect("could not generate log.txt");
        }

        let file = match fs::OpenOptions::new()
            .read(true)
            .write(true)
            .append(true)
            .open(&log_path)
        {
            Ok(file) => file,
            Err(err) => {
                Logger::log("error", &format!("Failed to open log file: {:?}", &log_path));
                Logger::log("debug", &format!("{:?}", err));
                std::process::exit(1);
            }
        };

        Ok(Logger {
            file,
            last_message: None,
        })
    }

    pub fn log(level: &str, message: &str) {
        let now = Local::now();
        let formatted_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut logger = LOGGER.lock().unwrap();

        if let Some(ref last_message) = logger.last_message {
            if last_message == message {
                return; // Don't log the same message again (can't be bothered to do it properly :3)
            }
        }

        let formatted_header = format!("[{}][{}]", level.to_uppercase(), formatted_time);

        let formatted_message = format!("{} {}\n", formatted_header, message);
        logger
            .file
            .write_all(formatted_message.as_bytes())
            .expect("Failed to write to log");
        logger.file.flush().expect("Failed to flush log");

        logger.last_message = Some(message.to_string());
    }
}
