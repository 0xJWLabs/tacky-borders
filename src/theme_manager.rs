use core::fmt;
use std::fs::DirBuilder;
use serde::de;
use serde::Deserializer;
use serde::Deserialize;
use std::path::Path;
use schema_jsonrs::JsonSchema;

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
            None => f.debug_struct("ThemeManager")
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
            Path::new(path)
                .file_stem()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
    }
}

pub fn deserialize_theme<'de, D>(deserializer: D) -> Result<ThemeManager, D::Error>
where
    D: Deserializer<'de>,
{
    // First, attempt to deserialize the theme as a String.
    let theme_name: Option<String> = Option::deserialize(deserializer)?;

    match theme_name {
        Some(theme_name) => {
            let config_dir = match UserConfig::get_config_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    return Err(de::Error::custom(format!(
                        "failed to retrieve the config directory: {}",
                        e
                    )))
                }
            };

            let theme_dir = config_dir.join("themes");

            // If the themes directory does not exist, create it.
            if !theme_dir.exists() {
                if let Err(e) = create_theme_directory(&theme_dir) {
                    return Err(de::Error::custom(format!(
                        "failed to create themes directory: {}",
                        e
                    )));
                }

                // If the directory is created, no theme file exists yet.
                return Err(de::Error::custom(format!(
                    "theme '{}' is not found in the newly created themes directory",
                    theme_name
                )));
            }

            // Check if the theme file (with .json or .jsonc extension) exists.
            let theme_json_path = theme_dir.join(format!("{}.json", theme_name));
            let theme_jsonc_path = theme_dir.join(format!("{}.jsonc", theme_name));

            if theme_json_path.exists() {
                Ok(ThemeManager(Some(theme_json_path.to_string_lossy().into_owned())))
            } else if theme_jsonc_path.exists() {
                Ok(ThemeManager(Some(theme_jsonc_path.to_string_lossy().into_owned())))
            } else {
                Err(de::Error::custom(format!(
                    "theme '{}' is not found in the themes directory",
                    theme_name
                )))
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