use super::{EffectConfig, EffectTranslationConfig};
use crate::core::helpers::parse_length_str;
use crate::core::value::ValueConversion;
use anyhow::anyhow;
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EffectTranslation {
    pub x: f32,
    pub y: f32,
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
