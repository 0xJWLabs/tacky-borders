use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as SerdeError;
use serde::de::Visitor;

use super::helpers::parse_duration_str;
use super::helpers::parse_length_str;

/// Enum representing a `Value` that can either be a finite number or a non-empty string.
#[derive(Clone, PartialEq, Debug, JsonSchema)]
pub enum Value {
    /// A finite number (f64).
    Number(f64),

    /// A non-empty string.
    String(String),
}

/// Visitor implementation for deserializing `Value`.
/// This handles deserializing both numbers and strings from any representation.
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

/// Validates a number and converts it into a `Value::Number`.
/// Ensures that the number is finite (not `NaN`, `infinity`, etc.).
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

macro_rules! as_type {
    ($val:expr, $target_type:ty) => {
        // Safely convert using `as` for primitive types
        $val as $target_type
    };
}

pub trait ValueConversion {
    fn as_length_f32(&self) -> Option<f32>;
    fn as_length_i32(&self) -> Option<i32>;
    fn as_length_f64(&self) -> Option<f64>;
    fn as_length_i64(&self) -> Option<i64>;
    fn as_length_u32(&self) -> Option<u32>;
    fn as_length_u64(&self) -> Option<u64>;
    fn as_duration_f32(&self) -> Option<f32>;
    fn as_duration_i32(&self) -> Option<i32>;
    fn as_duration_f64(&self) -> Option<f64>;
    fn as_duration_i64(&self) -> Option<i64>;
    fn as_duration_u32(&self) -> Option<u32>;
    fn as_duration_u64(&self) -> Option<u64>;
}

impl ValueConversion for Value {
    fn as_length_f32(&self) -> Option<f32> {
        match self {
            Value::Number(num) => Some(as_type!(*num, f32)),
            Value::String(s) => parse_length_str(s).map(|n| as_type!(n, f32)),
        }
    }

    fn as_length_i32(&self) -> Option<i32> {
        match self {
            Value::Number(num) => Some(as_type!(*num, i32)),
            Value::String(s) => parse_length_str(s).map(|n| as_type!(n, i32)),
        }
    }

    fn as_length_f64(&self) -> Option<f64> {
        match self {
            Value::Number(num) => Some(as_type!(*num, f64)),
            Value::String(s) => parse_length_str(s).map(|n| as_type!(n, f64)),
        }
    }

    fn as_length_i64(&self) -> Option<i64> {
        match self {
            Value::Number(num) => Some(as_type!(*num, i64)),
            Value::String(s) => parse_length_str(s).map(|n| as_type!(n, i64)),
        }
    }

    fn as_length_u32(&self) -> Option<u32> {
        match self {
            Value::Number(num) => Some(as_type!(*num, u32)),
            Value::String(s) => parse_length_str(s).map(|n| as_type!(n, u32)),
        }
    }

    fn as_length_u64(&self) -> Option<u64> {
        match self {
            Value::Number(num) => Some(as_type!(*num, u64)),
            Value::String(s) => parse_length_str(s).map(|n| as_type!(n, u64)),
        }
    }

    fn as_duration_f32(&self) -> Option<f32> {
        match self {
            Value::Number(num) => Some(as_type!(*num, f32)),
            Value::String(s) => parse_duration_str(s).map(|n| as_type!(n, f32)),
        }
    }

    fn as_duration_i32(&self) -> Option<i32> {
        match self {
            Value::Number(num) => Some(as_type!(*num, i32)),
            Value::String(s) => parse_duration_str(s).map(|n| as_type!(n, i32)),
        }
    }

    fn as_duration_f64(&self) -> Option<f64> {
        match self {
            Value::Number(num) => Some(as_type!(*num, f64)),
            Value::String(s) => parse_duration_str(s).map(|n| as_type!(n, f64)),
        }
    }

    fn as_duration_i64(&self) -> Option<i64> {
        match self {
            Value::Number(num) => Some(as_type!(*num, i64)),
            Value::String(s) => parse_duration_str(s).map(|n| as_type!(n, i64)),
        }
    }

    fn as_duration_u32(&self) -> Option<u32> {
        match self {
            Value::Number(num) => Some(as_type!(*num, u32)),
            Value::String(s) => parse_duration_str(s).map(|n| as_type!(n, u32)),
        }
    }

    fn as_duration_u64(&self) -> Option<u64> {
        match self {
            Value::Number(num) => Some(as_type!(*num, u64)),
            Value::String(s) => parse_duration_str(s).map(|n| as_type!(n, u64)),
        }
    }
}

impl<T: ValueConversion> ValueConversion for Option<T> {
    fn as_length_f32(&self) -> Option<f32> {
        self.as_ref()?.as_length_f32()
    }

    fn as_length_i32(&self) -> Option<i32> {
        self.as_ref()?.as_length_i32()
    }

    fn as_length_f64(&self) -> Option<f64> {
        self.as_ref()?.as_length_f64()
    }

    fn as_length_i64(&self) -> Option<i64> {
        self.as_ref()?.as_length_i64()
    }

    fn as_length_u32(&self) -> Option<u32> {
        self.as_ref()?.as_length_u32()
    }

    fn as_length_u64(&self) -> Option<u64> {
        self.as_ref()?.as_length_u64()
    }

    fn as_duration_f32(&self) -> Option<f32> {
        self.as_ref()?.as_duration_f32()
    }

    fn as_duration_i32(&self) -> Option<i32> {
        self.as_ref()?.as_duration_i32()
    }

    fn as_duration_f64(&self) -> Option<f64> {
        self.as_ref()?.as_duration_f64()
    }

    fn as_duration_i64(&self) -> Option<i64> {
        self.as_ref()?.as_duration_i64()
    }

    fn as_duration_u32(&self) -> Option<u32> {
        self.as_ref()?.as_duration_u32()
    }

    fn as_duration_u64(&self) -> Option<u64> {
        self.as_ref()?.as_duration_u64()
    }
}
