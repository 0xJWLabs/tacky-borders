#![allow(dead_code)]
use crate::user_config::{ConfigFormat, CONFIG_FORMAT};
use serde_jsonc2::Number as JsonNumber;
use serde_jsonc2::Value as JsonValue;
use serde_yml::Number as YamlNumber;
use serde_yml::Value as YamlValue;

use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;

pub enum Dimension {
    JsonNumber(JsonNumber),
    YamlNumber(YamlNumber),
    String(String),
}

pub trait AsI64 {
    fn as_i64(&self) -> Option<i64>;
}

impl AsI64 for JsonNumber {
    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }
}

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
        Dimension::JsonNumber(num) => num.as_i64(),
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
        Dimension::JsonNumber(num) => num.as_i64(),
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
    match *CONFIG_FORMAT.read().unwrap() {
        ConfigFormat::Json | ConfigFormat::Jsonc => {
            let value: JsonValue = Deserialize::deserialize(deserializer)?;
            match value {
                JsonValue::Number(num) => parse_value(Dimension::JsonNumber(num)),
                JsonValue::String(s) => parse_value(Dimension::String(s)),
                _ => Err(D::Error::custom("Expected a number or a string")),
            }
        }
        ConfigFormat::Yaml => {
            let value: YamlValue = Deserialize::deserialize(deserializer)?;
            match value {
                YamlValue::Number(num) => parse_value(Dimension::YamlNumber(num)),
                YamlValue::String(s) => parse_value(Dimension::String(s)),
                _ => Err(D::Error::custom("Expected a number or a string")),
            }
        }
        _ => Err(D::Error::custom("Invalid file type")),
    }
}

pub fn deserialize_optional_dimension<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize into Option<JsonValue> (for Json) or Option<YamlValue> (for Yaml)
    match *CONFIG_FORMAT.read().unwrap() {
        ConfigFormat::Json | ConfigFormat::Jsonc => {
            let value: Option<JsonValue> = Option::deserialize(deserializer)?;
            match value {
                Some(value) => match value {
                    JsonValue::Number(num) => parse_optional_value(Dimension::JsonNumber(num)),
                    JsonValue::String(s) => parse_optional_value(Dimension::String(s)),
                    JsonValue::Null => Ok(None),
                    _ => Err(D::Error::custom("Expected a number or string")),
                },
                None => Ok(None), // Handle the case where the value is missing
            }
        }
        ConfigFormat::Yaml => {
            let value: Option<YamlValue> = Option::deserialize(deserializer)?;
            match value {
                Some(value) => match value {
                    YamlValue::Number(num) => parse_optional_value(Dimension::YamlNumber(num)),
                    YamlValue::String(s) => parse_optional_value(Dimension::String(s)),
                    YamlValue::Null => Ok(None),
                    _ => Err(D::Error::custom("Expected a number or string")),
                },
                None => Ok(None), // Handle the case where the value is missing
            }
        }
        _ => Err(D::Error::custom("Invalid file type")), // Handle invalid file types
    }
}
