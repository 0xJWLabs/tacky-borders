use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::sync::LazyLock;
use std::sync::Mutex;

use crate::animations::Animations;
use crate::colors::ColorConfig;
use crate::utils::get_config;

const DEFAULT_CONFIG: &str = include_str!("resources/config.yaml");

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::new()));

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum BorderRadiusOption {
    Round,
    Square,
    SmallRound,
    Auto
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum BorderRadius {
    String(BorderRadiusOption),
    Float(f32)
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
    pub init_delay: Option<u64>,
    pub unminimize_delay: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WindowRule {
    #[serde(rename = "match")]
    pub rule_match: MatchDetails,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobalRule {
    pub border_width: f32,
    pub border_offset: i32,
    pub border_radius: BorderRadius,
    pub active_color: ColorConfig,
    pub inactive_color: ColorConfig,
    pub animations: Option<Animations>,
    pub init_delay: Option<u64>,
    pub unminimize_delay: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "global")]
    pub global_rule: GlobalRule,
    pub window_rules: Vec<WindowRule>,
}

impl Config {
    fn new() -> Self {
        let config_dir = get_config();
        let config_path = config_dir.join("config.yaml");

        if !fs::exists(&config_path).expect("Couldn't check if config path exists") {
            std::fs::write(&config_path, DEFAULT_CONFIG.as_bytes())
                .expect("could not generate default config.yaml");
        }

        let contents = match fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(_err) => panic!("could not read config.yaml in: {}", config_path.display()),
        };

        let config: Config = serde_yml::from_str(&contents).expect("error reading config.yaml");

        config
    }
    pub fn reload() {
        let mut config = CONFIG.lock().unwrap();
        *config = Self::new();
        drop(config);
    }
    pub fn _get() -> Self {
        CONFIG.lock().unwrap().clone()
    }
}