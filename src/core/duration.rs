use serde::{de, Deserialize, Deserializer};

#[derive(Clone, PartialEq, Debug)]
pub enum DurationValue {
    Number(f32),
    Text(String),
}

impl<'de> Deserialize<'de> for DurationValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DurationVisitor;

        impl de::Visitor<'_> for DurationVisitor {
            type Value = DurationValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number (integer or float) or a string")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DurationValue::Number(value as f32))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DurationValue::Number(value as f32))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DurationValue::Number(value as f32))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DurationValue::Text(value.to_string()))
            }
        }

        deserializer.deserialize_any(DurationVisitor)
    }
}
