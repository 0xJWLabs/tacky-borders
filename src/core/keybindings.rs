use crate::sys_tray::SystemTrayEvent;
#[cfg(feature = "fast-hash")]
use fx_hash::FxHashMap as HashMap;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
#[cfg(not(feature = "fast-hash"))]
use std::collections::HashMap;

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

pub trait KeybindingExt<V> {
    fn get_value(&self, key: &str) -> V;
}

impl KeybindingExt<String> for HashMap<String, String> {
    fn get_value(&self, key: &str) -> String {
        self.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

macro_rules! zoom_and_enhance {
    ($(#[$meta:meta])* pub struct $name:ident { $($(#[$fmeta:meta])* pub $fname:ident : $ftype:ty),* $(,)? }) => {
        $(#[$meta])*
        pub struct $name {
            $($(#[$fmeta])* pub $fname : $ftype),*
        }

        impl $name {
            pub fn field_titles() -> HashMap<String, String> {
                static NAMES: &'static [&'static str] = &[$(stringify!($fname)),*];
                NAMES
                    .iter()
                    .map(|&name| {
                        let formatted_name = name.split('_')
                            .map(|word| {
                                let mut chars = word.chars();
                                match chars.next() {
                                    Some(first) => {
                                        let uppercase_first = first.to_uppercase().collect::<String>();
                                        let rest = chars.as_str();
                                        format!("{}{}", uppercase_first, rest)
                                    },
                                    None => "".to_string(),
                                }
                            })
                            .collect::<Vec<String>>()
                            .join(" ");
                        (name.to_string(), formatted_name)
                    })
                    .collect::<HashMap<String, String>>()
            }
        }
    };
}

zoom_and_enhance! {
    #[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
    #[serde(default)]
    pub struct Keybindings {
        #[serde(default = "default_reload_key")]
        pub reload: String,
        #[serde(default = "default_open_config_key")]
        pub open_config: String,
        #[serde(default = "default_exit_key")]
        pub exit: String,
    }
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

fn create_keybindings(value: &Keybindings) -> Vec<KeybindingConfig> {
    let field_names = Keybindings::field_titles();
    let bindings = vec![
        KeybindingConfig::new(
            field_names.get_value("reload").as_str(),
            value.reload.as_str(),
            Some(SystemTrayEvent::ReloadConfig),
        ),
        KeybindingConfig::new(
            field_names.get_value("open_config").as_str(),
            value.open_config.as_str(),
            Some(SystemTrayEvent::OpenConfig),
        ),
        KeybindingConfig::new(
            field_names.get_value("exit").as_str(),
            value.exit.as_str(),
            Some(SystemTrayEvent::Exit),
        ),
    ];
    debug!("Keybindings: Created ({bindings:#?})");
    bindings
}

impl From<Keybindings> for Vec<KeybindingConfig> {
    fn from(value: Keybindings) -> Self {
        create_keybindings(&value)
    }
}

impl From<&Keybindings> for Vec<KeybindingConfig> {
    fn from(value: &Keybindings) -> Self {
        create_keybindings(value)
    }
}
