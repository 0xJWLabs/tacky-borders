use animation::AnimationParams;
use animation::AnimationType;
use animation::AnimationValue;
use easing::AnimationEasing;
use parser::parse_duration_str;
use parser::parse_easing_and_duration;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value as YamlValue;
use simple_bezier_easing::bezier;
use std::str::FromStr;
use std::sync::Arc;
use toml::Value as TomlValue;

use crate::border_config::CONFIG_TYPE;

pub mod animation;
mod easing;
mod parser;
pub mod timer;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE: i32 = 1;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default)]
    pub active: FxHashMap<AnimationType, AnimationValue>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: FxHashMap<AnimationType, AnimationValue>,
    #[serde(skip)]
    pub current: FxHashMap<AnimationType, AnimationValue>,
    #[serde(default = "default_fps")]
    pub fps: i32,
    #[serde(skip)]
    pub fade_progress: f32,
    #[serde(skip)]
    pub fade_to_visible: bool,
    #[serde(skip)]
    pub spiral_progress: f32,
    #[serde(skip)]
    pub spiral_angle: f32,
}

fn default_fps() -> i32 {
    60
}

fn animation<'de, D>(deserializer: D) -> Result<FxHashMap<AnimationType, AnimationValue>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut deserialized: FxHashMap<AnimationType, AnimationValue> = FxHashMap::default();
    match *CONFIG_TYPE.lock().unwrap() {
        "json" => {
            let result = FxHashMap::<AnimationType, JsonValue>::deserialize(deserializer);

            // If deserialize returns an error, it's possible that an invalid AnimationType was listed
            let map = match result {
                Ok(val) => val
                    .into_iter() // Convert into iterator
                    .collect::<FxHashMap<_, _>>(), // Collect back into a map
                Err(err) => return Err(err),
            };

            for (animation_type, anim_value) in map {
                let default_duration = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral => 1800.0,
                    AnimationType::Fade => 200.0,
                };
                let default_easing = AnimationEasing::Linear;
                let (duration, easing) = match anim_value {
                    JsonValue::Null => (default_duration, default_easing),
                    JsonValue::Object(ref obj) => (
                        obj.get("duration")
                            .and_then(|v| match v {
                                JsonValue::String(s) => parse_duration_str(s),
                                JsonValue::Number(n) => n.as_f64().map(|f| f as f32),
                                _ => None,
                            })
                            .unwrap_or(default_duration),
                        obj.get("easing")
                            .and_then(|v| {
                                v.as_str().and_then(|s| AnimationEasing::from_str(s).ok())
                            })
                            .unwrap_or(default_easing),
                    ),
                    JsonValue::String(s) => {
                        parse_easing_and_duration(&s, default_duration, default_easing)
                            .map_err(D::Error::custom)?
                    } // Explicit conversion
                    _ => {
                        return Err(D::Error::custom(format!(
                            "Invalid value type for animation: {:?}",
                            anim_value
                        )));
                    }
                };

                let points = easing.to_points();

                let easing_fn = bezier(points[0], points[1], points[2], points[3])
                    .map_err(serde::de::Error::custom)?;

                let animation_params = AnimationParams {
                    duration,
                    easing_fn: Arc::new(easing_fn),
                };

                let anim = AnimationValue {
                    animation_type: animation_type.clone(),
                    animation_params,
                };

                // Insert the deserialized animation data into the map
                deserialized.insert(animation_type.clone(), anim);
            }

            Ok(deserialized)
        }
        "yaml" => {
            let result = FxHashMap::<AnimationType, YamlValue>::deserialize(deserializer);

            // If deserialize returns an error, it's possible that an invalid AnimationType was listed
            let map = match result {
                Ok(val) => val
                    .into_iter() // Convert into iterator
                    .collect::<FxHashMap<_, _>>(), // Collect back into a map
                Err(err) => return Err(err),
            };

            for (animation_type, anim_value) in map {
                let default_duration = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral => 1800.0,
                    AnimationType::Fade => 200.0,
                };
                let default_easing = AnimationEasing::Linear;
                let (duration, easing) = match anim_value {
                    YamlValue::Null => (default_duration, default_easing),
                    YamlValue::Mapping(ref obj) => (
                        obj.get("duration")
                            .and_then(|v| match v {
                                YamlValue::String(s) => parse_duration_str(s),
                                YamlValue::Number(n) => n.as_f64().map(|f| f as f32),
                                _ => None,
                            })
                            .unwrap_or(default_duration),
                        obj.get("easing")
                            .and_then(|v| {
                                v.as_str().and_then(|s| AnimationEasing::from_str(s).ok())
                            })
                            .unwrap_or(default_easing),
                    ),
                    YamlValue::String(s) => {
                        parse_easing_and_duration(&s, default_duration, default_easing)
                            .map_err(D::Error::custom)?
                    } // Explicit conversion
                    _ => {
                        return Err(D::Error::custom(format!(
                            "Invalid value type for animation: {:?}",
                            anim_value
                        )));
                    }
                };

                let points = easing.to_points();

                let easing_fn = bezier(points[0], points[1], points[2], points[3])
                    .map_err(serde::de::Error::custom)?;

                let animation_params = AnimationParams {
                    duration,
                    easing_fn: Arc::new(easing_fn),
                };

                let anim = AnimationValue {
                    animation_type: animation_type.clone(),
                    animation_params,
                };

                // Insert the deserialized animation data into the map
                deserialized.insert(animation_type.clone(), anim);
            }

            Ok(deserialized)
        }
        "toml" => {
            let result = FxHashMap::<AnimationType, TomlValue>::deserialize(deserializer);

            // If deserialize returns an error, it's possible that an invalid AnimationType was listed
            let map = match result {
                Ok(val) => val
                    .into_iter() // Convert into iterator
                    .collect::<FxHashMap<_, _>>(), // Collect back into a map
                Err(err) => return Err(err),
            };

            for (animation_type, anim_value) in map {
                let default_duration = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral => 1800.0,
                    AnimationType::Fade => 200.0,
                };
                let default_easing = AnimationEasing::Linear;
                let (duration, easing) = match anim_value {
                    TomlValue::Table(ref obj) => (
                        obj.get("duration")
                            .and_then(|v| match v {
                                TomlValue::String(s) => parse_duration_str(s),
                                TomlValue::Float(n) => Some(*n as f32),
                                _ => None,
                            })
                            .unwrap_or(default_duration),
                        obj.get("easing")
                            .and_then(|v| {
                                v.as_str().and_then(|s| AnimationEasing::from_str(s).ok())
                            })
                            .unwrap_or(default_easing),
                    ),
                    TomlValue::String(s) => {
                        parse_easing_and_duration(&s, default_duration, default_easing)
                            .map_err(D::Error::custom)?
                    } // Explicit conversion
                    _ => {
                        return Err(D::Error::custom(format!(
                            "Invalid value type for animation: {:?}",
                            anim_value
                        )));
                    }
                };

                let points = easing.to_points();

                let easing_fn = bezier(points[0], points[1], points[2], points[3])
                    .map_err(serde::de::Error::custom)?;

                let animation_params = AnimationParams {
                    duration,
                    easing_fn: Arc::new(easing_fn),
                };

                let anim = AnimationValue {
                    animation_type: animation_type.clone(),
                    animation_params,
                };

                // Insert the deserialized animation data into the map
                deserialized.insert(animation_type.clone(), anim);
            }

            Ok(deserialized)
        }
        _ => Err(D::Error::custom("invalid file type".to_string())),
    }
}

pub trait HashMapAnimationExt {
    fn find(&self, animation_type: &AnimationType) -> Option<&AnimationValue>;
    fn has(&self, animation_type: &AnimationType) -> bool;
    fn to_iter(&self) -> impl Iterator<Item = &AnimationValue> + '_;
}

impl HashMapAnimationExt for FxHashMap<AnimationType, AnimationValue> {
    fn find(&self, animation_type: &AnimationType) -> Option<&AnimationValue> {
        self.get(animation_type)
    }

    fn has(&self, animation_type: &AnimationType) -> bool {
        self.contains_key(animation_type)
    }

    fn to_iter(&self) -> impl Iterator<Item = &AnimationValue> + '_ {
        self.values()
    }
}
