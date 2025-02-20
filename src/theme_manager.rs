use core::fmt;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::de;
use std::ffi::OsStr;
use std::fs::DirBuilder;
use std::path::Path;
use std::path::PathBuf;

use crate::env;
use crate::env::OStringExt;
use crate::env::PathBufExt;
use crate::user_config::UserConfig;

#[derive(PartialEq, Clone, Default, JsonSchema)]
pub struct ThemeManager(Option<String>);

impl fmt::Debug for ThemeManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(path) => {
                // Use theme_name to get the theme name
                if let Some(theme_name) = self.theme_name() {
                    f.debug_struct("ThemeManager")
                        .field("theme_name", &theme_name)
                        .field("path", &path)
                        .finish()
                } else {
                    f.debug_struct("ThemeManager")
                        .field("theme_name", &"Unknown")
                        .field("path", &path)
                        .finish()
                }
            }
            None => f
                .debug_struct("ThemeManager")
                .field("theme_name", &"None")
                .field("path", &"None")
                .finish(),
        }
    }
}

impl fmt::Display for ThemeManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(path) => {
                // Use the theme_name function
                if let Some(theme_name) = self.theme_name() {
                    write!(f, "ThemeManager({}, {})", theme_name, path)
                } else {
                    write!(f, "ThemeManager(unknown, {})", path)
                }
            }
            None => write!(f, "ThemeManager(None, None)"),
        }
    }
}

impl ThemeManager {
    pub fn path(&self) -> Option<String> {
        self.0.clone()
    }

    pub fn theme_name(&self) -> Option<String> {
        self.0.as_ref().and_then(|path| {
            path.as_path()
                .file_stem()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
    }
}

fn get_theme_path(theme_dir: &std::path::Path, theme_name: &str) -> Option<std::path::PathBuf> {
    let extensions = ["json", "jsonc"];
    #[cfg(feature = "yml")]
    {
        let yaml_path = theme_dir.join(format!("{}.yaml", theme_name));
        if yaml_path.exists() {
            return Some(yaml_path);
        }
    }

    for ext in extensions.iter() {
        let path = theme_dir.join(format!("{}.{}", theme_name, ext));
        if path.exists() {
            return Some(path);
        }
    }

    None
}

pub fn deserialize_theme<'de, D>(deserializer: D) -> Result<ThemeManager, D::Error>
where
    D: Deserializer<'de>,
{
    let theme_name: Option<String> = Option::deserialize(deserializer)?;

    match theme_name {
        Some(theme_name) => {
            if let Some(theme_path) = fix_absolute_path(&theme_name) {
                if is_valid_theme(&theme_path) {
                    match theme_path.clone().try_into_string() {
                        Some(theme_path_str) => Ok(ThemeManager(Some(theme_path_str))),
                        None => Err(de::Error::custom(format!(
                            "theme '{}' is not valid",
                            theme_path.to_string_lossy()
                        ))),
                    }
                } else {
                    Err(de::Error::custom(format!(
                        "theme '{}' is not valid",
                        theme_path.to_string_lossy()
                    )))
                }
            } else {
                let config_dir = UserConfig::get_config_dir().map_err(|e| {
                    de::Error::custom(format!("failed to retrieve the config directory: {}", e))
                })?;

                let theme_dir = config_dir.join("themes");

                // Ensure theme directory exists, creating it if necessary.
                if !theme_dir.exists() {
                    create_theme_directory(&theme_dir).map_err(|e| {
                        de::Error::custom(format!("failed to create themes directory: {}", e))
                    })?;

                    return Err(de::Error::custom(format!(
                        "theme '{}' is not found in the newly created themes directory",
                        theme_name
                    )));
                }

                // Try to find the theme file with any valid extension.
                if let Some(theme_path) = get_theme_path(&theme_dir, &theme_name) {
                    match theme_path.clone().try_into_string() {
                        Some(theme_path_str) => Ok(ThemeManager(Some(theme_path_str))),
                        None => Err(de::Error::custom(format!(
                            "theme '{}' is not found in the themes directory",
                            theme_path.to_string_lossy()
                        ))),
                    }
                } else {
                    Err(de::Error::custom(format!(
                        "theme '{}' is not found in the themes directory",
                        theme_name
                    )))
                }
            }
        }
        None => Ok(ThemeManager(None)),
    }
}

fn create_theme_directory(path: &Path) -> anyhow::Result<()> {
    DirBuilder::new().recursive(true).create(path)?;
    info!("created theme directory at {:?}", path);
    Ok(())
}

fn fix_absolute_path(path: &str) -> Option<PathBuf> {
    env::full_path(path).ok().and_then(|p| {
        let p = Path::new(p.as_ref());
        if p.is_absolute() {
            Some(p.to_path_buf())
        } else {
            None
        }
    })
}

fn is_valid_theme(path: &Path) -> bool {
    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => matches!(ext, "jsonc" | "json" | "yaml"),
        None => false,
    }
}
