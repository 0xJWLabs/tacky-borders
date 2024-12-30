use serde::de::Error as SerdeError;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;

#[derive(Clone, PartialEq, Debug)]
pub enum Duration {
    Number(f32),
    Text(String),
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DurationVisitor;

        impl DurationVisitor {
            fn validate_number<E>(self, value: f64) -> Result<Duration, E>
            where
                E: SerdeError,
            {
                if value.is_finite() {
                    Ok(Duration::Number(value as f32))
                } else {
                    Err(E::custom("Invalid number: must be finite"))
                }
            }
        }

        impl Visitor<'_> for DurationVisitor {
            type Value = Duration;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a finite number or a non-empty string")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                self.validate_number(value)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                self.validate_number(value as f64)
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                if value <= f32::MAX as u64 {
                    self.validate_number(value as f64)
                } else {
                    Err(E::custom("Invalid number: out of range for f32"))
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                if value.trim().is_empty() {
                    Err(E::custom("String cannot be empty"))
                } else {
                    Ok(Duration::Text(value.to_string()))
                }
            }
        }

        deserializer.deserialize_any(DurationVisitor)
    }
}
