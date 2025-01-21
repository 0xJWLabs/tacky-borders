use super::{EffectConfig, EffectTranslationConfig};
use crate::core::helpers::parse_length_str;
use crate::core::value::{Value, ValueConversion};
use anyhow::anyhow;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct EffectEngine {
    pub kind: EffectKind,
    pub standard_deviation: f32,
    pub opacity: f32,
    pub translation: EffectTranslation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectKind {
    Glow,
    Shadow,
}

impl FromStr for EffectKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "glow" => Ok(EffectKind::Glow),
            "shadow" => Ok(EffectKind::Shadow),
            _ => Err("Unknown effect type"),
        }
    }
}

impl TryFrom<EffectConfig> for EffectEngine {
    type Error = anyhow::Error;

    fn try_from(value: EffectConfig) -> Result<Self, Self::Error> {
        let kind = EffectKind::from_str(value.kind.as_str())
            .map_err(|_| anyhow!("invalid or missing animation kind"))?;

        let standard_deviation = value.standard_deviation.as_length_f32().unwrap_or(8.0);

        let translation = match value.translation {
            EffectTranslationConfig::String(translation) => {
                // Split the string by whitespace into components for x and y.
                let data = translation.split_ascii_whitespace().collect::<Vec<&str>>();

                // Ensure there are at least two elements (x and y).
                if data.len() >= 2 {
                    let x_str = data[0];
                    let y_str = data[1];

                    EffectTranslation {
                        x: parse_length_str(x_str).unwrap_or_default() as f32,
                        y: parse_length_str(y_str).unwrap_or_default() as f32,
                    }
                } else {
                    // Handle case where translation string doesn't have both x and y values.
                    EffectTranslation {
                        x: 0.0, // Default value if parsing fails or if there are not enough components.
                        y: 0.0,
                    }
                }
            }
            EffectTranslationConfig::Struct(ref translation) => {
                // Extract x and y from the EffectTranslationStruct
                EffectTranslation {
                    x: translation.x.as_length_f32().unwrap_or_default(),
                    y: translation.y.as_length_f32().unwrap_or_default(),
                }
            }
        };

        Ok(Self {
            kind,
            standard_deviation,
            opacity: value.opacity,
            translation,
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EffectTranslation {
    pub x: f32,
    pub y: f32,
}

/// Configuration for the translation of an effect, including both x and y coordinates.
/// This struct is used when the translation is more complex, involving both `x` and `y` axes.
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
#[serde(default)] // Apply the default values provided in `default_translation` to both x and y.
pub struct EffectTranslationStruct {
    /// The translation along the x-axis.
    /// If no value is provided during deserialization, it defaults to `0.0`.
    #[serde(default = "default_translation")]
    pub x: Value,

    /// The translation along the y-axis.
    /// If no value is provided during deserialization, it defaults to `0.0`.
    #[serde(default = "default_translation")]
    pub y: Value,
}

/// Default value for `EffectTranslationConfig::x` and `EffectTranslationConfig::y`.
/// Both x and y will default to `Value::Number(0.0)` if not specified.
fn default_translation() -> Value {
    Value::Number(0.0)
}

/// Default implementation for `EffectTranslationConfig`. This will ensure that both x and y have the default translation.
impl Default for EffectTranslationStruct {
    fn default() -> Self {
        EffectTranslationStruct {
            x: default_translation(),
            y: default_translation(),
        }
    }
}