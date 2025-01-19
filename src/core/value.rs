use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as SerdeError;
use serde::de::Visitor;

use super::helpers::parse_duration_str;
use super::helpers::parse_length_str;

#[derive(Clone, PartialEq, Debug, JsonSchema)]
/// Represents a number, which can be either a finite number (f32) or a non-empty string.
pub enum Value {
    Number(f64),
    String(String),
}

/// Visitor for deserializing `Value`.
struct ValueVisitor;

impl Visitor<'_> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a finite number or a non-empty string")
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        validate_number(value)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        validate_number(value as f64)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        if value <= f64::MAX as u64 {
            validate_number(value as f64)
        } else {
            Err(E::custom(format!(
                "invalid number: {} exceeds f32::MAX {}",
                value,
                f32::MAX
            )))
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        if value.trim().is_empty() {
            Err(E::custom("string cannot be empty"))
        } else {
            Ok(Value::String(value.to_owned()))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        if value.trim().is_empty() {
            Err(E::custom("string cannot be empty"))
        } else {
            Ok(Value::String(value))
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Validates a number and converts it into a `Value::Value`.
fn validate_number<E>(value: f64) -> Result<Value, E>
where
    E: SerdeError,
{
    if value.is_finite() {
        Ok(Value::Number(value))
    } else {
        Err(E::custom("invalid number: must be finite"))
    }
}

/// Trait for converting `Value` into various types.
pub trait ValueConversion {
    fn as_i64(&self) -> Option<i64>;
    fn as_i32(&self) -> Option<i32>;
    fn as_f32(&self) -> Option<f32>;
    fn as_f64(&self) -> Option<f64>;
    fn as_duration(&self) -> Option<f64>;
}

impl ValueConversion for Value {
    fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(num) => Some(*num as i64),
            Value::String(s) => parse_length_str(s).map(|n| n as i64),
        }
    }

    fn as_i32(&self) -> Option<i32> {
        self.as_i64().map(|n| n as i32)
    }

    fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|n| n as f32)
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(num) => Some(*num),
            Value::String(s) => parse_length_str(s),
        }
    }

    fn as_duration(&self) -> Option<f64> {
        match self {
            Value::Number(num) => Some(*num),
            Value::String(s) => parse_duration_str(s),
        }
    }
}

/// Blanket implementation for `Option<Value>`
impl<T: ValueConversion> ValueConversion for Option<T> {
    fn as_i64(&self) -> Option<i64> {
        self.as_ref()?.as_i64()
    }

    fn as_i32(&self) -> Option<i32> {
        self.as_ref()?.as_i32()
    }

    fn as_f32(&self) -> Option<f32> {
        self.as_ref()?.as_f32()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_ref()?.as_f64()
    }

    fn as_duration(&self) -> Option<f64> {
        self.as_ref()?.as_duration()
    }
}