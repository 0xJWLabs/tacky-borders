use crate::animations::Animations;
use crate::utils::home_dir;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result as AnyResult;
use serde::Deserialize;
use serde::Serialize;
use std::fs::exists;
use std::fs::read_to_string;
use std::fs::write;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::RwLock;
use win_color::GlobalColor;

pub static CONFIG: LazyLock<RwLock<Config>> = LazyLock::new(|| {
    RwLock::new(match Config::new() {
        Ok(config) => config,
        Err(e) => {
            error!("could not read config.yaml: {e:#}");
            Config::default()
        }
    })
});

pub static CONFIG_TYPE: RwLock<ConfigType> = RwLock::new(ConfigType::None);

const DEFAULT_CONFIG: &str = include_str!("../resources/config.yaml");

#[derive(Debug)]
pub enum ConfigType {
    Yaml,
    Json,
    Jsonc,
    None,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
pub enum BorderRadius {
    Round,
    Square,
    SmallRound,
    #[default]
    Auto,
    #[serde(untagged)]
    Custom(f32),
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
    pub active_color: Option<GlobalColor>,
    pub inactive_color: Option<GlobalColor>,
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
    pub active_color: GlobalColor,
    pub inactive_color: GlobalColor,
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
        let config_path = config_dir.join("config");

        let json_path = config_path.with_extension("json");
        let jsonc_path = config_path.with_extension("jsonc");
        let yaml_path = config_path.with_extension("yaml");

        let config_file = {
            // Lock the CONFIG_TYPE for setting its value
            let mut config_type_lock = CONFIG_TYPE
                .write()
                .map_err(|e| anyhow!("failed to acquire write lock for CONFIG_TYPE: {}", e))?;

            // Decide which file to use based on existence
            if exists(yaml_path.clone())? {
                *config_type_lock = ConfigType::Yaml;
                yaml_path.clone()
            } else if exists(json_path.clone())? {
                *config_type_lock = ConfigType::Json;
                json_path.clone()
            } else if exists(jsonc_path.clone())? {
                *config_type_lock = ConfigType::Jsonc;
                jsonc_path.clone()
            } else {
                *config_type_lock = ConfigType::Yaml;
                Self::create_default_config(&yaml_path.clone())?;
                info!(r"generating default config in {}", yaml_path.display());
                yaml_path.clone()
            }
        };

        // Read the contents of the chosen config file
        let contents = read_to_string(&config_file)
            .with_context(|| format!("failed to read config file: {}", config_file.display()))?;

        // Deserialize the config file based on the configuration type
        let config: Config = {
            let config_type_lock = CONFIG_TYPE
                .read()
                .map_err(|e| anyhow!("failed to acquire read lock for CONFIG_TYPE: {}", e))?;

            match *config_type_lock {
                ConfigType::Json | ConfigType::Jsonc => serde_jsonc2::from_str(&contents)
                    .with_context(|| "Failed to deserialize config.json")?,
                ConfigType::Yaml => serde_yml::from_str(&contents)
                    .with_context(|| "Failed to deserialize config.yaml")?,
                _ => return Err(anyhow!("Unsupported config file format")),
            }
        };

        Ok(config)
    }

    fn create_default_config(path: &PathBuf) -> AnyResult<()> {
        write(path, DEFAULT_CONFIG.as_bytes())
            .with_context(|| format!("Failed to write default config to {}", path.display()))?;
        Ok(())
    }

    pub fn get_config_dir() -> AnyResult<PathBuf> {
        let home_dir = home_dir()?;

        let config_dir = home_dir.join(".config").join("tacky-borders");
        let fallback_dir = home_dir.join(".tacky-borders");

        if exists(config_dir.clone())
            .with_context(|| format!("Could not find {}", config_dir.display()))?
        {
            return Ok(config_dir);
        } else if exists(fallback_dir.clone())
            .with_context(|| format!("Could not find {}", fallback_dir.display()))?
        {
            return Ok(fallback_dir);
        }

        DirBuilder::new()
            .recursive(true)
            .create(&config_dir)
            .map_err(|_| anyhow!("Could not create config directory"))?;

        Ok(config_dir)
    }

    pub fn reload() {
        let new_config = match Self::new() {
            Ok(config) => config,
            Err(e) => {
                error!("could not reload config: {e}");
                Config::default() // Consider whether this default state is acceptable
            }
        };

        match CONFIG.write() {
            Ok(mut config_lock) => {
                *config_lock = new_config;
            }
            Err(e) => {
                error!("RwLock poisoned: {e:#}");
                // Optionally, handle the failure here
            }
        }
    }
}
