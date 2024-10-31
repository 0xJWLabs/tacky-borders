use serde::Deserializer;
use serde::Deserialize;
use serde::Serialize;
use dirs::home_dir;
use std::fs;
use std::fs::DirBuilder;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};

use crate::utils::*;

const DEFAULT_CONFIG: &str = include_str!("resources/config.yaml");

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::new()));

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// Maybe support Process later.
// Getting the process name seems annoying.
pub enum RuleMatch {
  Global,
  Title,
  Class,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowRule {
  #[serde(rename = "match")]
  pub rule_match: RuleMatch,
  pub contains: Option<String>,
  pub active_color: Option<String>,
  pub inactive_color: Option<String>,
  pub enabled: Option<bool>
}


#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub border_size: i32,
    pub border_offset: i32,
    pub border_radius: BorderRadius,
    pub window_rules: Vec<WindowRule>,
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
        let home_dir = home_dir().expect("can't find home path");
        let config_dir = get_config();
        let config_path = config_dir.join("config.yaml");

        if !fs::exists(&config_path).expect("Couldn't check if config path exists") {
            let _ = std::fs::write(&config_path, &DEFAULT_CONFIG.as_bytes()).expect("could not generate default config.yaml");
        }

        let contents = match fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(err) => panic!("could not read config.yaml in: {}", config_path.display()),
        }; 

        let config: Config = serde_yaml::from_str(&contents).expect("error reading config.yaml");

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