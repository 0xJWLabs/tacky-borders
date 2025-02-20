use crate::animation::AnimationsConfig;
use crate::app_manager::AppManager;
use crate::border_manager::reload_borders;
use crate::colors::GlobalColor;
use crate::core::helpers::parse_length_str;
use crate::core::helpers::serde_default_i32;
use crate::core::helpers::serde_default_u32;
use crate::core::keybindings::KeybindingConfig;
use crate::core::keybindings::Keybindings;
use crate::core::value::Value;
use crate::core::value::ValueConversion;
use crate::effect::EffectsConfig;
use crate::error::LogIfErr;
use crate::keyboard_hook::KEYBOARD_HOOK;
use crate::theme_manager::ThemeManager;
use crate::theme_manager::deserialize_theme;
use crate::windows_api::WindowsApi;
use anyhow::Context;
use anyhow::anyhow;
use regex::Regex;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de;
use std::fs::DirBuilder;
use std::fs::exists;
use std::fs::read_to_string;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::RwLock;
use windows::Win32::Graphics::Dwm::DWMWCP_DEFAULT;
use windows::Win32::Graphics::Dwm::DWMWCP_DONOTROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUNDSMALL;

#[cfg(feature = "yml")]
const DEFAULT_CONFIG: &str = include_str!("../resources/config.yaml");

#[cfg(not(feature = "yml"))]
#[cfg(feature = "json")]
const DEFAULT_CONFIG: &str = include_str!("../resources/config.jsonc");

pub static CONFIG_FORMAT: LazyLock<RwLock<ConfigFormat>> =
    LazyLock::new(|| RwLock::new(ConfigFormat::default()));

/// Represents the supported configuration file formats.
#[derive(Debug, Clone, Default)]
pub enum ConfigFormat {
    #[cfg(feature = "json")]
    /// JSON configuration file.
    Json,
    #[cfg(feature = "json")]
    /// JSON with comments (JSONC) configuration file.
    Jsonc,
    #[cfg(feature = "yml")]
    /// YAML configuration file.
    Yaml,
    /// Placeholder for cases where no configuration type is detected.
    #[default]
    None,
}

/// Defines options for border radius customization.
#[derive(Debug, PartialEq, Clone, Default, JsonSchema)]
pub enum BorderStyle {
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
    Radius(f32),
}

impl<'de> Deserialize<'de> for BorderStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let upper = s.to_ascii_uppercase();

        match upper.as_str() {
            "ROUND" => Ok(BorderStyle::Round),
            "SQUARE" => Ok(BorderStyle::Square),
            "SMALLROUND" => Ok(BorderStyle::SmallRound),
            "AUTO" => Ok(BorderStyle::Auto),
            _ if upper.starts_with("RADIUS(") && upper.ends_with(')') => {
                let inner = &s[7..s.len() - 1];
                parse_length_str(inner)
                    .map(|value| BorderStyle::Radius(value as f32))
                    .ok_or_else(|| de::Error::custom("Invalid radius value"))
            }
            _ => Err(de::Error::custom("Invalid border style")),
        }
    }
}

impl BorderStyle {
    pub fn to_radius(&self, border_width: i32, dpi: f32, tracking_window: isize) -> f32 {
        let base_radius = (border_width as f32) / 2.0;
        let scale_factor = dpi / 96.0;

        match self {
            BorderStyle::Radius(-1.0) | BorderStyle::Auto => {
                match WindowsApi::get_window_corner_preference(tracking_window) {
                    DWMWCP_DEFAULT | DWMWCP_ROUND => 8.0 * scale_factor + base_radius,
                    DWMWCP_ROUNDSMALL => 4.0 * scale_factor + base_radius,
                    DWMWCP_DONOTROUND => 0.0,
                    _ => base_radius, // fallback default
                }
            }
            BorderStyle::Round => 8.0 * scale_factor + base_radius,
            BorderStyle::SmallRound => 4.0 * scale_factor + base_radius,
            BorderStyle::Square => 0.0,
            BorderStyle::Radius(radius) => radius * scale_factor,
        }
    }
}

/// Specifies the type of match used for window identification.
#[derive(Debug, Serialize, PartialEq, Clone, JsonSchema)]
pub enum MatchKind {
    /// Match based on the window title.
    Title,
    /// Match based on the class name of the window.
    Class,
    /// Match based on the process name or executable associated with the window.
    Process,
}

impl FromStr for MatchKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "title" => Ok(MatchKind::Title),
            "class" => Ok(MatchKind::Class),
            "process" => Ok(MatchKind::Process),
            _ => Err(anyhow!("MatchKind {s} does not exist")),
        }
    }
}

impl<'de> Deserialize<'de> for MatchKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Defines the strategy for matching a value against a criterion.
#[derive(Debug, PartialEq, Clone, JsonSchema)]
pub enum MatchStrategy {
    /// Match values that are exactly equal.
    Equals,
    /// Match values using a regular expression.
    Regex,
    /// Match values that contain the specified substring.
    Contains,
}

impl FromStr for MatchStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "contains" => Ok(MatchStrategy::Contains),
            "equals" => Ok(MatchStrategy::Equals),
            "regex" => Ok(MatchStrategy::Regex),
            _ => Err(anyhow!("MatchStrategy {s} does not exist")),
        }
    }
}

impl<'de> Deserialize<'de> for MatchStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl MatchStrategy {
    #[must_use]
    pub fn is_match(&self, value_1: &str, value_2: &str) -> bool {
        match self {
            MatchStrategy::Equals => value_1
                .to_ascii_lowercase()
                .eq(value_2.to_ascii_lowercase().as_str()),
            MatchStrategy::Contains => value_1
                .to_ascii_lowercase()
                .contains(value_2.to_ascii_lowercase().as_str()),
            MatchStrategy::Regex => Regex::new(value_2)
                .map(|re| re.captures(value_1).is_some())
                .unwrap_or(false),
        }
    }
}

/// Represents criteria used to match windows for applying specific configurations.
#[derive(Debug, Deserialize, Clone, Default, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WindowMatchConfig {
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
    pub animations: Option<AnimationsConfig>,
    /// Effect settings for the window borders.
    pub effects: Option<EffectsConfig>,
    /// Radius of the border corners.
    pub border_style: Option<BorderStyle>,
    /// Width of the border in pixels.
    #[serde(deserialize_with = "deserialize_optional_dimension", default)]
    pub border_width: Option<i32>,
    /// Offset of the border relative to the window.
    #[serde(deserialize_with = "deserialize_optional_dimension", default)]
    pub border_offset: Option<i32>,
    /// Whether borders are enabled for this match.
    #[serde(rename = "enabled")]
    pub enabled: Option<bool>,
    /// Delay (in milliseconds) before applying the border after initialization.
    pub initialize_delay: Option<u32>,
    /// Delay (in milliseconds) before applying the border after unminimizing.
    #[serde(alias = "restore_delay")]
    pub unminimize_delay: Option<u32>,
}

/// Represents a rule for a specific window, including matching criteria and associated actions.
#[derive(Debug, Deserialize, Clone, Default, PartialEq, JsonSchema)]
pub struct WindowRuleConfig {
    /// The matching details and settings for a specific type of window.
    #[serde(rename = "match")]
    pub match_window: WindowMatchConfig,
}

fn serde_default_global() -> GlobalRuleConfig {
    GlobalRuleConfig {
        border_width: serde_default_i32::<2>(),
        border_offset: serde_default_i32::<-1>(),
        ..Default::default()
    }
}

/// Contains global configuration settings applied across all windows.
#[derive(Debug, Deserialize, Clone, Default, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlobalRuleConfig {
    /// Default width of the window borders.
    #[serde(
        deserialize_with = "deserialize_dimension",
        default = "serde_default_i32::<1>"
    )]
    pub border_width: i32,
    /// Default offset for the window borders.
    #[serde(
        deserialize_with = "deserialize_dimension",
        default = "serde_default_i32::<-1>"
    )]
    pub border_offset: i32,
    /// Default border radius settings.
    #[serde(default)]
    pub border_style: BorderStyle,
    /// Default color for active window borders.
    #[serde(default)]
    pub active_color: GlobalColor,
    /// Default color for inactive window borders.
    #[serde(default)]
    pub inactive_color: GlobalColor,
    /// Animation settings for borders.
    #[serde(default)]
    pub animations: AnimationsConfig,
    /// Effect settings for borders.
    #[serde(default)]
    pub effects: EffectsConfig,
    /// Delay (in milliseconds) before applying borders after initialization.
    #[serde(alias = "init_delay", default = "serde_default_u32::<250>")]
    pub initialize_delay: u32,
    /// Delay (in milliseconds) before applying borders after unminimizing.
    #[serde(alias = "restore_delay", default = "serde_default_u32::<200>")]
    pub unminimize_delay: u32,
}

/// Stores the complete configuration including global rules, window rules, and keybindings.
#[derive(Debug, Deserialize, Clone, Default, PartialEq, JsonSchema)]
#[serde(default)]
pub struct UserConfig {
    /// Global settings applied across all windows.
    #[serde(rename = "global", default = "serde_default_global")]
    pub global_rule: GlobalRuleConfig,
    /// Specific rules for individual windows.
    #[serde(default)]
    pub window_rules: Vec<WindowRuleConfig>,
    /// Application keybindings.
    #[serde(default)]
    pub keybindings: Keybindings,
    /// Enables monitoring for changes in the configuration file.
    #[serde(default)]
    pub monitor_config_changes: bool,
    /// Enable custom predefined theme
    #[serde(deserialize_with = "deserialize_theme")]
    pub theme: ThemeManager,
}

/// Methods for managing the configuration, including loading, saving, and reloading.
impl UserConfig {
    /// Attempts to create a new configuration instance by reading from the config file.
    pub fn create() -> anyhow::Result<Self> {
        let config_file = Self::detect_config_file().unwrap_or_else(|_| {
            debug!("Creating default config file");
            Self::create_default_config().unwrap_or_default()
        });

        let config_format = UserConfig::detect_config_format()?;
        *CONFIG_FORMAT.write().unwrap() = config_format.clone();

        Self::read(&config_file)
    }

    fn read(path: &PathBuf) -> anyhow::Result<Self> {
        let content = read_to_string(path).inspect_err(|e| {
            WindowsApi::show_error_dialog(
                "UserConfig",
                &format!(
                    "Failed to read config file: {}\n(Error: {:?}",
                    path.display(),
                    e
                ),
            );
        })?;

        let config_format = &*CONFIG_FORMAT
            .read()
            .map_err(|_| anyhow!("config format lock poisoned"))?;

        let config: anyhow::Result<Self> = match config_format {
            #[cfg(feature = "json")]
            ConfigFormat::Json | ConfigFormat::Jsonc => {
                serde_jsonc2::from_str(&content).with_context(|| "failed to deserialize JSON")
            }
            #[cfg(feature = "yml")]
            ConfigFormat::Yaml => {
                serde_yml::from_str(&content).with_context(|| "failed to deserialize YAML")
            }
            _ => Err(anyhow!("Unsupported config format")),
        };

        config.map_err(|err| {
            WindowsApi::show_error_dialog("UserConfig", &format!("{}", err));
            err
        })
    }

    /// Find the configuration file in config directory
    fn find_config_file(config_dir: &Path) -> Option<PathBuf> {
        let candidates = [
            #[cfg(feature = "yml")]
            "yaml",
            #[cfg(feature = "yml")]
            "yml",
            #[cfg(feature = "json")]
            "json",
            #[cfg(feature = "json")]
            "jsonc",
        ];

        for ext in candidates {
            let file_path = config_dir.join("config").with_extension(ext);
            if exists(file_path.clone()).unwrap_or(false) {
                return Some(file_path);
            }
        }
        None
    }

    /// Detects the configuration file in the given directory or creates a default config file if none exists.
    pub fn detect_config_file() -> anyhow::Result<PathBuf> {
        let config_dir = Self::get_config_dir()?;
        Self::find_config_file(&config_dir).ok_or_else(|| {
            anyhow!(
                "[detect_config_file] Config: Not Found (Directory: {})",
                config_dir.display()
            )
        })
    }

    /// Creates a default configuration file in the specified directory.
    pub fn create_default_config() -> anyhow::Result<PathBuf> {
        let config_dir = Self::get_config_dir()?;

        #[cfg(all(feature = "yml", feature = "json"))]
        {
            let path = if WindowsApi::show_yes_no_dialog(
                "Create Default Config",
                "No configuration file found. Would you like to create a default config from YAML instead of JSONC?",
            ) {
                config_dir.join("config.yaml")
            } else {
                config_dir.join("config.jsonc")
            };

            Self::write_default_config(&path)?;
            Ok(path)
        }

        #[cfg(not(feature = "yml"))]
        #[cfg(feature = "json")]
        {
            let path = config_dir.join("config.jsonc");
            Self::write_default_config(&path)?;
            Ok(path)
        }

        #[cfg(not(feature = "json"))]
        #[cfg(feature = "yml")]
        {
            let path = config_dir.join("config.yaml");
            Self::write_default_config(&path)?;
            Ok(path)
        }
    }

    fn write_default_config(path: &Path) -> anyhow::Result<()> {
        write(path, DEFAULT_CONFIG.as_bytes())
            .with_context(|| format!("Failed to write default config to {}", path.display()))?;
        Ok(())
    }

    pub fn detect_config_format() -> anyhow::Result<ConfigFormat> {
        let config_dir = Self::get_config_dir()?;
        let file_path = Self::find_config_file(&config_dir)
            .ok_or_else(|| anyhow!("No supported config file found"))?;

        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();

        match extension {
            #[cfg(feature = "yml")]
            "yaml" | "yml" => Ok(ConfigFormat::Yaml),
            #[cfg(feature = "json")]
            "json" => Ok(ConfigFormat::Json),
            #[cfg(feature = "json")]
            "jsonc" => Ok(ConfigFormat::Jsonc),
            _ => Err(anyhow!("Unsupported config file extension: {}", extension)),
        }
    }

    /// Retrieves the configuration directory, creating it if necessary.
    pub fn get_config_dir() -> anyhow::Result<PathBuf> {
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

    /// Update the configuration by reinitializing it from the configuration file.
    ///
    /// This method replaces the current configuration with a newly loaded one.
    /// If loading fails, it falls back to the default configuration and logs an error.
    pub fn update() {
        let app_manager = AppManager::get();
        let new_config = match Self::create() {
            Ok(config) => {
                let config_watcher_is_running = app_manager.config_watcher_is_running();

                if config.monitor_config_changes && !config_watcher_is_running {
                    app_manager.start_config_watcher();
                } else if !config.monitor_config_changes && config_watcher_is_running {
                    app_manager.stop_config_watcher();
                }
                config
            }
            Err(e) => {
                error!("could not reload config: {e}");
                UserConfig::default()
            }
        };

        app_manager.set_config(new_config);
    }

    /// Reloads the application configuration and restarts the borders.
    ///
    /// This method calls the `update()` function to reload the configuration and refresh the
    /// application state. After updating the configuration, it restarts the borders and updates
    /// the keyboard bindings if they are available.
    ///
    /// # Side Effects
    /// - The configuration is reloaded from the file and written to the shared configuration store.
    /// - The borders are reloaded, which may involve reinitializing UI components.
    /// - If a keyboard hook is available, the keybindings are refreshed and applied.
    pub fn reload() -> bool {
        let app_manager = AppManager::get();
        debug!("[reload] UserConfig: Reloading and restarting borders");
        let old_config = app_manager.config().clone();
        Self::update();
        let new_config = app_manager.config();

        if old_config != *new_config {
            reload_borders();
            if let Some(hook) = KEYBOARD_HOOK.get() {
                let bindings = Vec::<KeybindingConfig>::from(&new_config.keybindings);
                hook.update(&bindings);
            }
            return true;
        }
        false
    }

    /// Opens the configuration file in the default editor.
    ///
    /// This method determines the configuration file's path based on the current config type
    /// (e.g., JSON, YAML, JSONC) and attempts to open it using the default file association on the system.
    pub fn open() {
        match Self::detect_config_file() {
            Ok(config_file) => win_open::that(config_file).log_if_err(),
            Err(err) => error!("{err}"),
        }
    }
}

// Deserializer

/// Deserializes a dimension value.
pub fn deserialize_dimension<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    value
        .as_length_i32()
        .ok_or_else(|| de::Error::custom("Invalid Value"))
}

/// Deserializes an optional dimension value.
pub fn deserialize_optional_dimension<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|v| v.as_length_i32()))
}
