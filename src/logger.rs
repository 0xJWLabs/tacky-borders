use crate::utils::get_file;
use chrono::Local;
use lazy_static::lazy_static;
use std::{io::Write, sync::Mutex};

lazy_static! {
  static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::new().unwrap());
}

pub struct Logger {
  file: std::fs::File,
  last_message: Option<String>,
}

impl Logger {
  fn new() -> Result<Self, std::io::Error> {
    let file = get_file("log.txt", "");
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