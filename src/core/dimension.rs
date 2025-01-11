#![allow(dead_code)]
use crate::user_config::ConfigFormat;
use crate::user_config::CONFIG_FORMAT;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
#[cfg(feature = "json")]
use serde_jsonc2::Number as JsonNumber;
#[cfg(feature = "json")]
use serde_jsonc2::Value as JsonValue;
#[cfg(feature = "yml")]
use serde_yml::Number as YamlNumber;
#[cfg(feature = "yml")]
use serde_yml::Value as YamlValue;

pub enum Dimension {
    #[cfg(feature = "json")]
    JsonNumber(JsonNumber),
    #[cfg(feature = "yml")]
    YamlNumber(YamlNumber),
    String(String),
}

pub trait AsI64 {
    fn as_i64(&self) -> Option<i64>;
}

#[cfg(feature = "json")]
impl AsI64 for JsonNumber {
    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }
}

#[cfg(feature = "yml")]
impl AsI64 for YamlNumber {
    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }
}

fn parse_value<E>(value: Dimension) -> Result<i32, E>
where
    E: Error,
{
    match value {
        #[cfg(feature = "json")]
        Dimension::JsonNumber(num) => num.as_i64(),
        #[cfg(feature = "yml")]
        Dimension::YamlNumber(num) => num.as_i64(),
        Dimension::String(s) => {
            let trimmed = s.strip_suffix("px").unwrap_or(&s);
            trimmed.parse::<i64>().ok()
        }
    }
    .map(|n| n as i32)
    .ok_or_else(|| E::custom("Invalid value"))
}

fn parse_optional_value<E>(value: Dimension) -> Result<Option<i32>, E>
where
    E: Error,
{
    let val = match value {
        #[cfg(feature = "json")]
        Dimension::JsonNumber(num) => num.as_i64(),
        #[cfg(feature = "yml")]
        Dimension::YamlNumber(num) => num.as_i64(),
        Dimension::String(s) => {
            let trimmed = s.strip_suffix("px").unwrap_or(&s);
            trimmed.parse::<i64>().ok()
        }
    }
    .map(|n| n as i32);

    Ok(val)
}

pub fn deserialize_dimension<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let config_format = &*CONFIG_FORMAT
        .read()
        .map_err(|_| D::Error::custom("config format lock poisoned"))?;

    #[cfg(feature = "json")]
    if matches!(config_format, ConfigFormat::Json | ConfigFormat::Jsonc) {
        let value: JsonValue = Deserialize::deserialize(deserializer)?;
        return match value {
            JsonValue::Number(num) => parse_value(Dimension::JsonNumber(num)),
            JsonValue::String(s) => parse_value(Dimension::String(s)),
            _ => Err(D::Error::custom("Expected a number or a string")),
        };
    }

    #[cfg(feature = "yml")]
    if matches!(config_format, ConfigFormat::Yaml) {
        let value: YamlValue = Deserialize::deserialize(deserializer)?;
        return match value {
            YamlValue::Number(num) => parse_value(Dimension::YamlNumber(num)),
            YamlValue::String(s) => parse_value(Dimension::String(s)),
            _ => Err(D::Error::custom("Expected a number or a string")),
        };
    }

    Err(D::Error::custom("Invalid file type"))
}

pub fn deserialize_optional_dimension<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let config_format = &*CONFIG_FORMAT
        .read()
        .map_err(|_| D::Error::custom("config format lock poisoned"))?;

    #[cfg(feature = "json")]
    if matches!(config_format, ConfigFormat::Json | ConfigFormat::Jsonc) {
        let value: Option<JsonValue> = Option::deserialize(deserializer)?;
        return match value {
            Some(value) => match value {
                JsonValue::Number(num) => parse_optional_value(Dimension::JsonNumber(num)),
                JsonValue::String(s) => parse_optional_value(Dimension::String(s)),
                JsonValue::Null => Ok(None),
                _ => Err(D::Error::custom("Expected a number or string")),
            },
            None => Ok(None), // Handle the case where the value is missing
        };
    }

    #[cfg(feature = "yml")]
    if matches!(config_format, ConfigFormat::Yaml) {
        let value: Option<YamlValue> = Option::deserialize(deserializer)?;
        return match value {
            Some(value) => match value {
                YamlValue::Number(num) => parse_optional_value(Dimension::YamlNumber(num)),
                YamlValue::String(s) => parse_optional_value(Dimension::String(s)),
                YamlValue::Null => Ok(None),
                _ => Err(D::Error::custom("Expected a number or string")),
            },
            None => Ok(None), // Handle the case where the value is missing
        };
    }

    Err(D::Error::custom("Invalid file type")) // Handle invalid file types
}
