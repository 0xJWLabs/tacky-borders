use std::{io::Read, sync::Mutex, str::FromStr};

use crate::{logger::Logger, utils::get_file};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize, Deserializer};

const DEFAULT_CONFIG: &str = include_str!("resources/config.yaml");

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::new());
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub border_size: i32,
    pub border_offset: i32,
    pub border_radius: BorderRadius,
    pub active_color: String,
    pub inactive_color: String,
}

#[derive(Debug, Clone)]
pub enum BorderRadius {
    Value(f32),
    CssString(String),
}

impl<'de> Deserialize<'de> for BorderRadius {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Ok(value) = f32::from_str(&s) {
            Ok(BorderRadius::Value(value))
        } else {
            Ok(BorderRadius::CssString(s))
        }
    }
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
        drop(config);
    }
    pub fn get() -> Self {
        CONFIG.lock().unwrap().clone()
    }

    pub fn get_border_radius(&self) -> f32 {
        match &self.border_radius {
            BorderRadius::Value(radius) => *radius,
            BorderRadius::CssString(css) => {
                css.trim_end_matches("px").parse::<f32>().unwrap_or(0.0)
            }
        }
    }
}