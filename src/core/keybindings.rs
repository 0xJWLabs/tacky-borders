use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Keybindings {
    #[serde(default = "default_reload_key")]
    pub reload: String,
    #[serde(default = "default_open_config_key")]
    pub open_config: String,
    #[serde(default = "default_exit_key")]
    pub exit: String,
}

fn default_reload_key() -> String {
    "f8".to_string()
}

fn default_open_config_key() -> String {
    "f9".to_string()
}

fn default_exit_key() -> String {
    "f10".to_string()
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            reload: default_reload_key(),
            open_config: default_open_config_key(),
            exit: default_exit_key(),
        }
    }
}
