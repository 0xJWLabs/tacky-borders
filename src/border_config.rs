use std::{io::Read, sync::Mutex};

use crate::{logger::Logger, utils::get_file};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG: &str = include_str!("resources/config.yaml");

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::new());
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub border_size: i32,
    pub border_offset: i32,
    pub border_radius: f32,
    pub active_color: String,
    pub inactive_color: String,
}

impl Config {
    fn new() -> Self {
        let mut file = get_file("config.yaml", DEFAULT_CONFIG);
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(..) => {}
            Err(err) => {
                Logger::log("error", "Failed to read config file");
                Logger::log("debug",&format!("{:?}", err));
                std::process::exit(1);
            }
        }

        let config: Config = match serde_yaml::from_str(contents.as_str()) {
            Ok(config) => config,
            Err(err) => {
                Logger::log("error", "Failed to parse config file");
                Logger::log("debug", &format!("{:?}", err));
                std::process::exit(1);
            }
        };

        config
    }
    pub fn reload() {
        let mut config = CONFIG.lock().unwrap();
        *config = Self::new();
    }
    pub fn get() -> Self {
        CONFIG.lock().unwrap().clone()
    }
}