use std::fs::DirBuilder;

use schema_jsonrs::JsonSchema;
use serde::{de, Deserialize, Deserializer};
use std::path::Path;

use crate::user_config::UserConfig;

#[derive(PartialEq, Clone, Default, Debug, JsonSchema)]
pub struct Theme(pub Option<String>);

impl Theme {
    pub fn get(&self) -> Option<String> {
        self.0.clone()
    }
}

pub fn deserialize_theme<'de, D>(deserializer: D) -> Result<Theme, D::Error>
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
                Ok(Theme(Some(theme_json_path.to_string_lossy().into_owned())))
            } else if theme_jsonc_path.exists() {
                Ok(Theme(Some(theme_jsonc_path.to_string_lossy().into_owned())))
            } else {
                Err(de::Error::custom(format!(
                    "theme '{}' is not found in the themes directory",
                    theme_name
                )))
            }
        }
        None => Ok(Theme(None)),
    }
}

fn create_theme_directory(path: &Path) -> anyhow::Result<()> {
    DirBuilder::new().recursive(true).create(path)?;
    info!("created theme directory at {:?}", path);
    Ok(())
}
