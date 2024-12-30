use serde::{de, Deserialize, Deserializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Keybindings {
    pub reload: String,
    pub open_config: String,
    pub exit: String,
}

impl<'de> Deserialize<'de> for Keybindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeybindingsVisitor;

        impl<'de> de::Visitor<'de> for KeybindingsVisitor {
            type Value = Keybindings;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with keys 'reload', 'open_config', and 'exit'")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut reload: Option<String> = None;
                let mut open_config: Option<String> = None;
                let mut exit: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "reload" => reload = map.next_value()?,
                        "open_config" => open_config = map.next_value()?,
                        "exit" => exit = map.next_value()?,
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?; // Ignore unknown fields
                        }
                    }
                }

                Ok(Keybindings {
                    reload: reload.unwrap_or_else(|| "f8".to_string()),
                    open_config: open_config.unwrap_or_else(|| "f9".to_string()),
                    exit: exit.unwrap_or_else(|| "f10".to_string()),
                })
            }
        }

        deserializer.deserialize_map(KeybindingsVisitor)
    }
}
