use crate::animations::Animations;
use crate::colors::color::ColorConfig;
use anyhow::Context;
use anyhow::Result as AnyResult;
use dirs::home_dir;
use serde::Deserialize;
use serde::Serialize;
use std::fs::exists;
use std::fs::read_to_string;
use std::fs::write;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::Mutex;
use thiserror::Error;

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    Mutex::new(match Config::new() {
        Ok(config) => config,
        Err(err) => {
            error!("Error: {}", err);
            Config::default()
        }
    })
});

const DEFAULT_CONFIG: &str = include_str!("../resources/config.yaml");

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum BorderRadiusOption {
    Round,
    Square,
    SmallRound,
    Auto,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum BorderRadius {
    String(BorderRadiusOption),
    Float(f32),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum MatchKind {
    Title,
    Class,
    Process,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum MatchStrategy {
    Equals,
    Regex,
    Contains,
}

impl Default for BorderRadius {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MatchDetails {
    #[serde(rename = "kind")]
    pub match_kind: Option<MatchKind>,
    #[serde(rename = "value")]
    pub match_value: Option<String>,
    #[serde(rename = "strategy")]
    pub match_strategy: Option<MatchStrategy>,
    pub active_color: Option<ColorConfig>,
    pub inactive_color: Option<ColorConfig>,
    pub animations: Option<Animations>,
    pub border_radius: Option<BorderRadius>,
    pub border_width: Option<f32>,
    pub border_offset: Option<i32>,
    #[serde(rename = "enabled")]
    pub border_enabled: Option<bool>,
    pub initialize_delay: Option<u64>,
    pub unminimize_delay: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WindowRule {
    #[serde(rename = "match")]
    pub rule_match: MatchDetails,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct GlobalRule {
    pub border_width: f32,
    pub border_offset: i32,
    pub border_radius: BorderRadius,
    pub active_color: ColorConfig,
    pub inactive_color: ColorConfig,
    pub animations: Option<Animations>,
    pub initialize_delay: Option<u64>,
    pub unminimize_delay: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(rename = "global")]
    pub global_rule: GlobalRule,
    pub window_rules: Vec<WindowRule>,
}

impl Config {
    fn new() -> AnyResult<Self> {
        let config_dir = Self::get_config_dir()?;
        let config_path = config_dir.join("config.yaml");
        if !exists(&config_path).context("Could not find config file in path")? {
            write(&config_path, DEFAULT_CONFIG.as_bytes())
                .context("could not generate default config.yaml")?;
        }

        let contents = read_to_string(&config_path).context("Could not read config.yaml")?;

        let config = serde_yaml_ng::from_str(&contents)?;
        Ok(config)
    }

    pub fn get_config_dir() -> AnyResult<PathBuf, ConfigError> {
        let Some(home_dir) = home_dir() else {
            return Err(ConfigError::HomeDir);
        };

        let config_dir = home_dir.join(".config").join("tacky-borders");
        let fallback_dir = home_dir.join(".tacky-borders");

        if config_dir.exists() {
            return Ok(config_dir);
        }

        if fallback_dir.exists() {
            return Ok(fallback_dir);
        }

        DirBuilder::new()
            .recursive(true)
            .create(&config_dir)
            .map_err(|_| ConfigError::ConfigDir)?;

        Ok(config_dir)
    }

    pub fn reload() {
        let new_config = match Self::new() {
            Ok(config) => config,
            Err(err) => {
                error!("Error: {}", err);
                Config::default()
            }
        };
        *CONFIG.lock().unwrap() = new_config;
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Could not find home directory")]
    HomeDir,
    #[error("Could not create config directory")]
    ConfigDir,
}
