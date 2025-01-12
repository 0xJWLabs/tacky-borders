use schema_jsonrs::JsonSchema;
use serde::de::Error as SerdeError;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;

#[derive(Clone, PartialEq, Debug, JsonSchema)]
/// Represents a duration, which can be either a finite number (f32) or a non-empty string.
pub enum Duration {
    Number(f32),
    Text(String),
}

/// Visitor for deserializing `Duration`.
struct DurationVisitor;

impl Visitor<'_> for DurationVisitor {
    type Value = Duration;

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
        if value <= f32::MAX as u64 {
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
            Ok(Duration::Text(value.to_owned()))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        if value.trim().is_empty() {
            Err(E::custom("string cannot be empty"))
        } else {
            Ok(Duration::Text(value))
        }
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DurationVisitor)
    }
}

/// Validates a number and converts it into a `Duration::Number`.
fn validate_number<E>(value: f64) -> Result<Duration, E>
where
    E: SerdeError,
{
    if value.is_finite() {
        Ok(Duration::Number(value as f32))
    } else {
        Err(E::custom("invalid number: must be finite"))
    }
}
