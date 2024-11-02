use dirs::home_dir;
use serde::de::{self, Visitor};
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use std::fmt;
use std::fs;
use std::fs::DirBuilder;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};

use crate::utils::*;

const DEFAULT_CONFIG: &str = include_str!("resources/config.yaml");

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::new()));

#[derive(Debug, Deserialize, PartialEq, Clone)]
// Maybe support Process later.
// Getting the process name seems annoying.
pub enum RuleMatch {
    Global,
    Title,
    Class,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum MatchType {
    Equals,
    Regex,
    Contains,
}

impl std::str::FromStr for MatchType {
    type Err = String; // Or a more specific error type

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Equals" => Ok(MatchType::Equals),
            "Regex" => Ok(MatchType::Regex),
            "Contains" => Ok(MatchType::Contains),
            _ => Err(format!("Invalid match type: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MatchDetails {
    #[serde(rename = "type")]
    pub match_type: RuleMatch,
    #[serde(rename = "value")]
    pub match_value: Option<String>,
    #[serde(rename = "strategy")]
    pub match_strategy: Option<MatchType>,
    pub active_color: Option<ColorConfig>,
    pub inactive_color: Option<ColorConfig>,
    pub border_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowRule {
    #[serde(rename = "match")]
    pub rule_match: MatchDetails,
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

        const FIELDS: &'static [&'static str] = &["colors", "direction", "animation"];

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
        let home_dir = home_dir().expect("can't find home path");
        let config_dir = get_config();
        let config_path = config_dir.join("config.yaml");

        if !fs::exists(&config_path).expect("Couldn't check if config path exists") {
            let _ = std::fs::write(&config_path, &DEFAULT_CONFIG.as_bytes())
                .expect("could not generate default config.yaml");
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
