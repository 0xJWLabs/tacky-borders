use crate::sys_tray::SystemTrayEvent;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

#[derive(Clone)]
pub struct KeybindingConfig {
    pub name: String,
    pub keybind: String,
    pub event: Option<SystemTrayEvent>,
}

impl KeybindingConfig {
    pub fn new(name: &str, keybind: &str, event: Option<SystemTrayEvent>) -> Self {
        Self {
            name: name.to_string(),
            keybind: keybind.to_string(),
            event,
        }
    }
}

impl core::fmt::Debug for KeybindingConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Display the name and keybind
        f.debug_struct("KeybindingConfig")
            .field("name", &self.name)
            .field("keybind", &self.keybind)
            .field(
                "event_callback",
                &self
                    .event
                    .as_ref()
                    .map_or("None", |action| action.as_function_name()),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
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

impl KeybindingConfig {
    pub fn from_config(config: &Keybindings) -> anyhow::Result<Vec<Self>> {
        let bindings = vec![
            KeybindingConfig::new("Reload", config.reload.as_str(), Some(SystemTrayEvent::ReloadConfig)),
            KeybindingConfig::new("Open Config", config.open_config.as_str(), Some(SystemTrayEvent::OpenConfig)),
            KeybindingConfig::new("Exit", config.exit.as_str(), Some(SystemTrayEvent::Exit)),
        ];
        debug!("[create_keybindings] Keybindings created: {bindings:#?}");
        Ok(bindings)
    }
}