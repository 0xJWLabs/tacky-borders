use crate::animations::Animations;
use crate::error::LogIfErr;
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
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::DWMWCP_DEFAULT;
use windows::Win32::Graphics::Dwm::DWMWCP_DONOTROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUNDSMALL;

/// Global configuration instance, accessible throughout the application.
/// It uses `LazyLock` to initialize only when first accessed and wraps the config in an `RwLock` for thread-safe access.
pub static CONFIG: LazyLock<RwLock<Config>> =
    LazyLock::new(|| RwLock::new(Config::load_or_default()));

/// Tracks the current configuration format (e.g., YAML, JSON).
/// Useful for loading or saving configuration files in the correct format.
pub static CONFIG_TYPE: LazyLock<RwLock<ConfigType>> =
    LazyLock::new(|| RwLock::new(ConfigType::default()));

/// Default configuration content stored as a YAML string.
const DEFAULT_CONFIG: &str = include_str!("../resources/config.yaml");

/// Represents the supported configuration file formats.
#[derive(Debug, Clone, Default)]
pub enum ConfigType {
    /// YAML configuration file.
    Yaml,
    /// JSON configuration file.
    Json,
    /// JSON with comments (JSONC) configuration file.
    Jsonc,
    /// Placeholder for cases where no configuration type is detected.
    #[default]
    None,
}

/// Defines options for border radius customization.
#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
pub enum BorderRadius {
    /// Fully rounded borders.
    Round,
    /// Square borders with no rounding.
    Square,
    /// Small rounding for subtle border effects.
    SmallRound,
    /// Automatically determine the border radius based on context.
    #[default]
    Auto,
    /// Custom border radius, specified in pixels.
    #[serde(untagged)]
    Custom(f32),
}

impl BorderRadius {
    pub fn parse(&self, border_width: i32, dpi: f32, tracking_window: HWND) -> f32 {
        let base_radius = (border_width as f32) / 2.0;
        let scale_factor = dpi / 96.0;

        match self {
            BorderRadius::Custom(-1.0) | BorderRadius::Auto => {
                match WindowsApi::get_window_corner_preference(tracking_window) {
                    DWMWCP_DEFAULT | DWMWCP_ROUND => 8.0 * scale_factor + base_radius,
                    DWMWCP_ROUNDSMALL => 4.0 * scale_factor + base_radius,
                    DWMWCP_DONOTROUND => 0.0,
                    _ => base_radius, // fallback default
                }
            }
            BorderRadius::Round => 8.0 * scale_factor + base_radius,
            BorderRadius::SmallRound => 4.0 * scale_factor + base_radius,
            BorderRadius::Square => 0.0,
            BorderRadius::Custom(radius) => radius * scale_factor,
        }
    }
}

/// Specifies the type of match used for window identification.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum MatchKind {
    /// Match based on the window title.
    Title,
    /// Match based on the class name of the window.
    Class,
    /// Match based on the process name or executable associated with the window.
    Process,
}

/// Defines the strategy for matching a value against a criterion.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum MatchStrategy {
    /// Match values that are exactly equal.
    Equals,
    /// Match values using a regular expression.
    Regex,
    /// Match values that contain the specified substring.
    Contains,
}

/// Represents criteria used to match windows for applying specific configurations.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct MatchDetails {
    /// Type of match (e.g., title, class, or process).
    #[serde(rename = "kind")]
    pub match_kind: Option<MatchKind>,
    /// The value to match against (e.g., window title or class name).
    #[serde(rename = "value")]
    pub match_value: Option<String>,
    /// Strategy for matching, such as exact match or regex.
    #[serde(rename = "strategy")]
    pub match_strategy: Option<MatchStrategy>,
    /// Color for the border when the window is active.
    pub active_color: Option<GlobalColor>,
    /// Color for the border when the window is inactive.
    pub inactive_color: Option<GlobalColor>,
    /// Animation settings for the window borders.
    pub animations: Option<Animations>,
    /// Radius of the border corners.
    pub border_radius: Option<BorderRadius>,
    /// Width of the border in pixels.
    pub border_width: Option<f32>,
    /// Offset of the border relative to the window.
    pub border_offset: Option<i32>,
    /// Whether borders are enabled for this match.
    #[serde(rename = "enabled")]
    pub border_enabled: Option<bool>,
    /// Delay (in milliseconds) before applying the border after initialization.
    pub initialize_delay: Option<u64>,
    /// Delay (in milliseconds) before applying the border after unminimizing.
    pub unminimize_delay: Option<u64>,
}

/// Represents a rule for a specific window, including matching criteria and associated actions.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct WindowRule {
    /// The matching details and settings for a specific type of window.
    #[serde(rename = "match")]
    pub rule_match: MatchDetails,
}

/// Contains global configuration settings applied across all windows.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct GlobalRule {
    /// Default width of the window borders.
    pub border_width: f32,
    /// Default offset for the window borders.
    pub border_offset: i32,
    /// Default border radius settings.
    pub border_radius: BorderRadius,
    /// Default color for active window borders.
    pub active_color: GlobalColor,
    /// Default color for inactive window borders.
    pub inactive_color: GlobalColor,
    /// Animation settings for borders.
    pub animations: Option<Animations>,
    /// Delay (in milliseconds) before applying borders after initialization.
    pub initialize_delay: Option<u64>,
    /// Delay (in milliseconds) before applying borders after unminimizing.
    pub unminimize_delay: Option<u64>,
}

/// Defines the structure for the application's keybindings.
#[derive(Debug, Deserialize, Clone)]
pub struct Keybindings {
    /// Keybinding to reload the configuration.
    #[serde(default = "default_reload_keybind")]
    pub reload: String,
    /// Keybinding to open the configuration file.
    #[serde(default = "default_open_config_keybind")]
    pub open_config: String,
    /// Keybinding to exit the application.
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

/// Stores the complete configuration including global rules, window rules, and keybindings.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    /// Global settings applied across all windows.
    #[serde(rename = "global")]
    pub global_rule: GlobalRule,
    /// Specific rules for individual windows.
    pub window_rules: Vec<WindowRule>,
    /// Application keybindings.
    #[serde(default)]
    pub keybindings: Keybindings,
}

/// Methods for managing the configuration, including loading, saving, and reloading.
impl Config {
    /// Loads the configuration from a file or returns the default configuration if loading fails.
    fn load_or_default() -> Self {
        Self::new().unwrap_or_else(|e| {
            error!("could not load config: {e}");
            Self::default()
        })
    }

    /// Attempts to create a new configuration instance by reading from the config file.
    fn new() -> AnyResult<Self> {
        let config_dir = Self::get_config_dir()?;

        let config_file = Self::detect_config_file(&config_dir)?;

        // Read the contents of the chosen config file
        let contents = read_to_string(&config_file)
            .with_context(|| format!("failed to read config file: {}", config_file.display()))?;

        Self::deserialize(contents)
    }

    /// Deserializes configuration content into a `Config` instance based on the file format.
    fn deserialize(contents: String) -> AnyResult<Self> {
        let config_type = CONFIG_TYPE
            .read()
            .map_err(|e| anyhow!("failed to acquire read lock for CONFIG_TYPE: {}", e))?;

        match *config_type {
            ConfigType::Yaml => {
                serde_yml::from_str(&contents).with_context(|| "failed to deserialize YAML")
            }
            ConfigType::Json | ConfigType::Jsonc => {
                serde_jsonc2::from_str(&contents).with_context(|| "failed to deserialize JSON")
            }
            _ => Err(anyhow!("unsupported configuration format")),
        }
    }

    /// Detects the configuration file in the given directory or creates a default config file if none exists.
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

    /// Creates a default configuration file in the specified directory.
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

    /// Retrieves the configuration directory, creating it if necessary.
    pub fn get_config_dir() -> AnyResult<PathBuf> {
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

    /// Reloads the configuration by reinitializing it from the configuration file.
    ///
    /// This method replaces the current configuration with a newly loaded one.
    /// If loading fails, it falls back to the default configuration and logs an error.
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

    /// Opens the configuration file in the default editor.
    ///
    /// This method determines the configuration file's path based on the current config type
    /// (e.g., JSON, YAML, JSONC) and attempts to open it using the default file association on the system.
    pub fn open() {
        match Self::get_config_dir() {
            Ok(mut dir) => {
                let config_file = match *CONFIG_TYPE.read().unwrap() {
                    ConfigType::Json => "config.json",
                    ConfigType::Yaml => "config.yaml",
                    ConfigType::Jsonc => "config.jsonc",
                    _ => {
                        error!("Unsupported config file");
                        return;
                    }
                };

                dir.push(config_file);

                win_open::that(dir).log_if_err();
            }
            Err(err) => error!("{err}"),
        }
    }
}
