use super::EffectConfig;
use crate::core::helpers::parse_length_str;
use crate::core::{
    effect::{EffectKind, EffectTranslation},
    value::Value,
};
use anyhow::anyhow;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct EffectEngine {
    pub kind: EffectKind,
    pub standard_deviation: f32,
    pub opacity: f32,
    pub translation: EffectTranslation,
}

impl TryFrom<EffectConfig> for EffectEngine {
    type Error = anyhow::Error;

    fn try_from(value: EffectConfig) -> Result<Self, Self::Error> {
        let kind = EffectKind::from_str(value.kind.as_str())
            .map_err(|_| anyhow!("invalid or missing animation kind"))?;

        let standard_deviation = match value.standard_deviation {
            Some(Value::Number(val)) => val,
            Some(Value::Text(val)) => parse_length_str(&val)
                .ok_or_else(|| anyhow!("invalid length string for standard_deviation"))?,
            None => 8.0,
        };

        let translation = EffectTranslation {
            x: match value.translation.x {
                Value::Number(val) => val,
                Value::Text(ref val) => parse_length_str(val).unwrap_or_default(),
            },
            y: match value.translation.y {
                Value::Number(val) => val,
                Value::Text(ref val) => parse_length_str(val).unwrap_or_default(),
            },
        };

        Ok(Self {
            kind,
            standard_deviation,
            opacity: value.opacity,
            translation,
        })
    }
}
