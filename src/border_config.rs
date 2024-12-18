use crate::animations::Animations;
use crate::windows_api::WindowsApi;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result as AnyResult;
use serde::Deserialize;
use serde::Serialize;
use std::fs::exists;
use std::fs::read_to_string;
use std::fs::write;
use std::fs::DirBuilder;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::RwLock;
use win_color::GlobalColor;

pub static CONFIG: LazyLock<RwLock<Config>> =
    LazyLock::new(|| RwLock::new(Config::load_or_default()));

pub static CONFIG_TYPE: LazyLock<RwLock<ConfigType>> =
    LazyLock::new(|| RwLock::new(ConfigType::default()));

const DEFAULT_CONFIG: &str = include_str!("../resources/config.yaml");

#[derive(Debug, Clone, Default)]
pub enum ConfigType {
    Yaml,
    Json,
    Jsonc,
    #[default]
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

#[derive(Debug, Deserialize, Clone)]
pub struct Keybindings {
    #[serde(default = "default_reload_keybind")]
    pub reload: String,
    #[serde(default = "default_open_config_keybind")]
    pub open_config: String,
    #[serde(default = "default_exit_keybind")]
    pub exit: String,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            reload: "f8".to_string(),
            open_config: "f9".to_string(),
            exit: "f10".to_string(),
        }
    }
}

fn default_reload_keybind() -> String {
    "f8".to_string()
}

fn default_open_config_keybind() -> String {
    "f9".to_string()
}

fn default_exit_keybind() -> String {
    "f10".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(rename = "global")]
    pub global_rule: GlobalRule,
    pub window_rules: Vec<WindowRule>,
    #[serde(default)]
    pub keybindings: Keybindings,
}

pub trait ConfigImpl {
    fn reload();
    fn get_config_dir() -> AnyResult<PathBuf>;
}

impl Config {
    fn load_or_default() -> Self {
        Self::new().unwrap_or_else(|e| {
            error!("could not load config: {e}");
            Self::default()
        })
    }

    fn new() -> AnyResult<Self> {
        let config_dir = Self::get_config_dir()?;

        let config_file = Self::detect_config_file(&config_dir)?;

        // Read the contents of the chosen config file
        let contents = read_to_string(&config_file)
            .with_context(|| format!("failed to read config file: {}", config_file.display()))?;

        Self::deserialize(contents)
    }

    fn deserialize(contents: String) -> AnyResult<Self> {
        let config_type_lock = CONFIG_TYPE
            .read()
            .map_err(|e| anyhow!("failed to acquire read lock for CONFIG_TYPE: {}", e))?;

        match *config_type_lock {
            ConfigType::Yaml => {
                serde_yml::from_str(&contents).with_context(|| "failed to deserialize YAML")
            }
            ConfigType::Json | ConfigType::Jsonc => {
                serde_jsonc2::from_str(&contents).with_context(|| "failed to deserialize JSON")
            }
            _ => Err(anyhow!("unsupported configuration format")),
        }
    }

    fn detect_config_file(config_dir: &Path) -> AnyResult<PathBuf> {
        let candidates = [
            ("yaml", ConfigType::Yaml),
            ("json", ConfigType::Json),
            ("jsonc", ConfigType::Jsonc),
        ];

        let mut config_type_lock = CONFIG_TYPE
            .write()
            .map_err(|e| anyhow!("failed to acquire write lock for CONFIG_TYPE: {}", e))?;

        for (ext, config_type) in candidates {
            let file_path = config_dir.join("config").with_extension(ext);
            if exists(file_path.clone())? {
                *config_type_lock = config_type;
                return Ok(file_path);
            }
        }

        // Create default config if none exist
        Self::create_default_config(config_dir)
    }

    fn create_default_config(config_dir: &Path) -> AnyResult<PathBuf> {
        let path = config_dir.join("config.yaml");
        write(path.clone(), DEFAULT_CONFIG.as_bytes())
            .with_context(|| format!("failed to write default config to {}", path.display()))?;
        let mut config_type_lock = CONFIG_TYPE
            .write()
            .map_err(|e| anyhow!("failed to acquire write lock for CONFIG_TYPE: {}", e))?;

        *config_type_lock = ConfigType::Yaml;
        Ok(path.clone())
    }
}

impl ConfigImpl for Config {
    fn reload() {
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

    fn get_config_dir() -> AnyResult<PathBuf> {
        let home_dir = WindowsApi::home_dir()?;

        let config_dir = home_dir.join(".config").join("tacky-borders");
        let fallback_dir = home_dir.join(".tacky-borders");

        if exists(config_dir.clone())
            .with_context(|| format!("could not find directory: {}", config_dir.display()))?
        {
            return Ok(config_dir);
        } else if exists(fallback_dir.clone())
            .with_context(|| format!("could not find directory: {}", fallback_dir.display()))?
        {
            return Ok(fallback_dir);
        }

        DirBuilder::new()
            .recursive(true)
            .create(&config_dir)
            .map_err(|_| anyhow!("could not create config directory"))?;

        Ok(config_dir)
    }
}
