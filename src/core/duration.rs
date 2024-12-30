use serde::{de, Deserialize, Deserializer};

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

        impl de::Visitor<'_> for DurationVisitor {
            type Value = Duration;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number (integer or float) or a string")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.is_finite() {
                    Ok(Duration::Number(value as f32))
                } else {
                    Err(E::custom("Invalid number: must be finite"))
                }
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Duration::Number(value as f32))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value <= f32::MAX as u64 {
                    Ok(Duration::Number(value as f32))
                } else {
                    Err(E::custom("Invalid number: out of range for f32"))
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
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
