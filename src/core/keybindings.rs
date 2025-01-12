use crate::core::app_state::APP_STATE;
use crate::keyboard_hook::KeybindingConfig;
use crate::sys_tray::SystemTrayEvent;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

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

pub fn create_keybindings() -> anyhow::Result<Vec<KeybindingConfig>> {
    let config = (*APP_STATE.config.read().unwrap()).clone();

    let bindings = vec![
        KeybindingConfig::new(
            SystemTrayEvent::OpenConfig.into(),
            config.keybindings.open_config.clone().as_str(),
            Some(SystemTrayEvent::OpenConfig),
        ),
        KeybindingConfig::new(
            SystemTrayEvent::ReloadConfig.into(),
            config.keybindings.reload.clone().as_str(),
            Some(SystemTrayEvent::ReloadConfig),
        ),
        KeybindingConfig::new(
            SystemTrayEvent::Exit.into(),
            config.keybindings.exit.clone().as_str(),
            Some(SystemTrayEvent::Exit),
        ),
    ];

    debug!("keybindings created: {bindings:#?}");

    Ok(bindings)
}
