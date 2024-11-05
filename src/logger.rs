use chrono::Local;
use std::fs::*;
use std::io::Write;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use crate::utils::*;

pub static LOGGER: LazyLock<Mutex<Logger>> =
    LazyLock::new(|| Mutex::new(Logger::new().expect("Failed to initialize logger")));

pub struct Logger {
    file: std::fs::File,
    last_message: Option<String>,
}

impl Logger {
    fn new() -> Result<Self, std::io::Error> {
        let config_dir = get_config();
        let log_path = config_dir.join("log.txt");

        if exists(&log_path).expect("Couldn't check if log path exists") {
            // Overwrite the file with an empty string
            write(&log_path, "").map_err(|e| {
                eprintln!("Failed to reset log file: {:?}", e);
                e
            })?;
        }

        if !exists(&log_path).expect("Couldn't check if log path exists") {
            write(&log_path, "").expect("could not generate log.txt");
        }

        let file = OpenOptions::new()
            .append(true) // Allow appending to the file
            .create(true) // Create the file if it doesn't exist
            .open(&log_path)
            .map_err(|err| {
                Logger::file_log(
                    "error",
                    &format!("Failed to open log file: {:?}", &log_path),
                    std::panic::Location::caller().file(),
                    std::panic::Location::caller().line(),
                );
                Logger::file_log(
                    "debug",
                    &format!("{:?}", err),
                    std::panic::Location::caller().file(),
                    std::panic::Location::caller().line(),
                );
                err
            })?;

        Ok(Logger {
            file,
            last_message: None,
        })
    }

    pub fn file_log(level: &str, message: &str, file: &'static str, line: u32) {
        let logger_mutex = &*LOGGER;
        let now = Local::now();
        let formatted_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut logger = logger_mutex.lock().unwrap();

        if let Some(ref last_message) = logger.last_message {
            if last_message == message {
                return; // Don't log the same message again
            }
        }

        let relative_file_path = normalize_path(file).unwrap_or_else(|e| {
            Logger::file_log(
                "error",
                format!("Error normalizing path: {}", e).as_str(),
                std::panic::Location::caller().file(),
                std::panic::Location::caller().line(),
            );
            Path::new(file).display().to_string()
        });
        let formatted_header = format!(
            "[{}][{}][{}:{}]",
            level.to_uppercase(),
            formatted_time,
            relative_file_path,
            line
        );

        let formatted_message = format!("{} {}\n", formatted_header, message);

        logger
            .file
            .write_all(formatted_message.as_bytes())
            .expect("Failed to write to log");
        logger.file.flush().expect("Failed to flush log");

        logger.last_message = Some(message.to_string());
        drop(logger);
    }
}

fn normalize_path<P: AsRef<Path>>(path: P) -> Result<String, String> {
    if let Some(s) = path.as_ref().to_str() {
        // Replace backslashes with forward slashes
        Ok(s.replace('\\', "/"))
    } else {
        // Log a warning if the path is invalid Unicode
        eprintln!(
            "Warning: Invalid Unicode path encountered: {:?}",
            path.as_ref()
        );
        // Fallback to display representation
        Ok(path.as_ref().display().to_string())
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $message:expr) => {
        Logger::file_log(
            $level,
            $message,
            std::panic::Location::caller().file(),
            std::panic::Location::caller().line(),
        );
    };
}
