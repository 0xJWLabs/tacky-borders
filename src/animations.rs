use serde::Deserialize;
use serde::Deserializer;
use serde_yml::Value;
use std::collections::HashMap;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE_TO_ACTIVE: i32 = 1;
pub const ANIM_FADE_TO_INACTIVE: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum AnimationType {
    Spiral,
    Fade,
}

fn animation<'de, D>(deserializer: D) -> Result<HashMap<AnimationType, f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Option<HashMap<String, Value>> = Option::deserialize(deserializer)?;

    let mut result = HashMap::new();
    if let Some(entries) = map {
        for (key, value) in entries {
            let animation_type = match key.as_str() {
                "Spiral" => AnimationType::Spiral,
                "Fade" => AnimationType::Fade,
                _ => continue, // Ignore unknown animation types
            };

            // Default speed is 100 if the value is missing or null
            let speed = match value {
                Value::Number(n) => {
                    if n.is_f64() {
                        n.as_f64().map(|f| f as f32)
                    } else if n.is_i64() {
                        n.as_i64().map(|i| i as f32)
                    } else {
                        None
                    }
                }
                Value::Null => None, // If the value is null, we will assign default speeds later
                _ => None, // Handle invalid formats
            };

            // Apply the default speed for each animation type if it's null or missing
            let default_speed = match animation_type {
                AnimationType::Spiral => 100.0,
                AnimationType::Fade => 100.0,
            };

            // If the speed is None (either null or missing), assign the default speed
            result.insert(animation_type, speed.unwrap_or(default_speed));
        }
    }

    // Return the populated HashMap (or an empty one if None)
    Ok(result)
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default = "default_anim")]
    pub active: HashMap<AnimationType, f32>,
    #[serde(deserialize_with = "animation", default = "default_anim")]
    pub inactive: HashMap<AnimationType, f32>,
    #[serde(default = "default_fps")]
    pub fps: i32,
}

fn default_fps() -> i32 {
    30
}

fn default_anim() -> HashMap<AnimationType, f32> {
    HashMap::new()
}

impl Default for Animations {
    fn default() -> Self {
        Self {
            active: HashMap::new(),
            inactive: HashMap::new(),
            fps: 30,
        }
    }
}