use serde::Deserialize;
use serde::Deserializer;
use std::fs;
use std::sync::{LazyLock, Mutex};

use crate::utils::*;

const DEFAULT_CONFIG: &str = include_str!("resources/config.yaml");

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::new()));

#[derive(Debug, Deserialize, PartialEq, Clone)]
// Maybe support Process later.
// Getting the process name seems annoying.
pub enum MatchKind {
    Title,
    Class,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum MatchStrategy {
    Equals,
    Regex,
    Contains,
}

impl std::str::FromStr for MatchStrategy {
    type Err = String; // Or a more specific error type

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Equals" => Ok(MatchStrategy::Equals),
            "Regex" => Ok(MatchStrategy::Regex),
            "Contains" => Ok(MatchStrategy::Contains),
            _ => Err(format!("Invalid match type: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MatchDetails {
    #[serde(rename = "kind")]
    pub match_type: MatchKind,
    #[serde(rename = "value")]
    pub match_value: Option<String>,
    #[serde(rename = "strategy")]
    pub match_strategy: Option<MatchStrategy>,
    pub active_color: Option<ColorConfig>,
    pub inactive_color: Option<ColorConfig>,
    pub border_radius: Option<i32>,
    pub border_size: Option<i32>,
    pub border_offset: Option<i32>,
    pub border_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowRule {
    #[serde(rename = "match")]
    pub rule_match: MatchDetails,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobalRule {
    pub border_size: i32,
    pub border_offset: i32,
    pub border_radius: i32,
    pub active_color: Option<ColorConfig>,
    pub inactive_color: Option<ColorConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "match")]
    pub global_rule: GlobalRule,
    pub window_rules: Vec<WindowRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientDirectionPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientDirection {
    pub start: GradientDirectionPoint,
    pub end: GradientDirectionPoint,
}

impl GradientDirection {
    pub fn to_vec(&self) -> Vec<f32> {
        vec![self.start.x, self.start.y, self.end.x, self.end.y]
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientColor {
    pub colors: Vec<String>,
    pub direction: GradientDirection,
    pub animation: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum ColorConfig {
    String(String),
    Struct(GradientColor),
}

impl<'de> Deserialize<'de> for ColorConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        const FIELDS: &[&str] = &["colors", "direction", "animation"];

        struct ColorConfigVisitor;

        impl<'de> Visitor<'de> for ColorConfigVisitor {
            type Value = ColorConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a map representing a gradient color")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(ColorConfig::String(value.to_string()))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut colors = None;
                let mut direction = None;
                let mut animation = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "colors" => {
                            if colors.is_some() {
                                return Err(de::Error::duplicate_field("colors"));
                            }
                            colors = Some(map.next_value()?);
                        }
                        "direction" => {
                            if direction.is_some() {
                                return Err(de::Error::duplicate_field("direction"));
                            }
                            direction = Some(map.next_value()?);
                        }
                        "animation" => {
                            if animation.is_some() {
                                return Err(de::Error::duplicate_field("animation"));
                            }
                            animation = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(&key, FIELDS));
                        }
                    }
                }

                let colors = colors.ok_or_else(|| de::Error::missing_field("colors"))?;
                let direction = direction.ok_or_else(|| de::Error::missing_field("direction"))?;

                Ok(ColorConfig::Struct(GradientColor {
                    colors,
                    direction,
                    animation,
                }))
            }
        }

        deserializer.deserialize_any(ColorConfigVisitor)
    }
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

        let config: Config = serde_yaml::from_str(&contents).expect("error reading config.yaml");

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

impl WindowRule {
    pub fn default() -> Self {
        WindowRule {
            rule_match: MatchDetails {
                match_type: MatchKind::Title,
                match_value: None,
                match_strategy: Some(MatchStrategy::Equals),
                border_size: None,
                border_offset: None,
                border_radius: None,
                active_color: None,
                inactive_color: None,
                border_enabled: None,
            },
        }
    }
}
