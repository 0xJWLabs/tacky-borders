use std::sync::Arc;

use crate::border_config::ConfigType;
use crate::border_config::CONFIG_TYPE;
use animation::AnimationParams;
use animation::AnimationType;
use animation::AnimationValue;
use easing::AnimationEasing;
use easing::AnimationEasingImpl;
use parser::parse_animation_from_json;
use parser::parse_animation_from_yaml;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value as YamlValue;
use simple_bezier_easing::bezier;

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
    let default_durations: FxHashMap<AnimationType, f32> = FxHashMap::from_iter([
        (AnimationType::Spiral, 1800.0),
        (AnimationType::ReverseSpiral, 1800.0),
        (AnimationType::Fade, 200.0),
    ]);
    let default_easing = AnimationEasing::Linear;

    match *CONFIG_TYPE.read().unwrap() {
        ConfigType::Json => {
            FxHashMap::<AnimationType, JsonValue>::deserialize(deserializer)
                .map_err(D::Error::custom)
                .and_then(|map| {
                    // Iterate over the map and handle the deserialized values
                    let mut result = FxHashMap::default();
                    for (animation_type, animation_value) in map.iter() {
                        let (duration, easing) = parse_animation_from_json(
                            animation_value,
                            default_durations[animation_type],
                            default_easing.clone(),
                        )
                        .map_err(|e| D::Error::custom(format!("{}", e)))?;

                        let easing_points = easing.to_points();

                        let easing_fn = bezier(
                            easing_points[0],
                            easing_points[1],
                            easing_points[2],
                            easing_points[3],
                        )
                        .map_err(|e| D::Error::custom(format!("{}", e)))?;

                        result.insert(
                            animation_type.clone(),
                            AnimationValue {
                                animation_type: animation_type.clone(),
                                animation_params: AnimationParams {
                                    duration,
                                    easing_fn: Arc::new(easing_fn),
                                },
                            },
                        );
                    }
                    Ok(result)
                })
        }
        ConfigType::Yaml => {
            FxHashMap::<AnimationType, YamlValue>::deserialize(deserializer)
                .map_err(D::Error::custom)
                .and_then(|map| {
                    // Iterate over the map and handle the deserialized values
                    let mut result = FxHashMap::default();
                    for (animation_type, animation_value) in map.iter() {
                        let (duration, easing) = parse_animation_from_yaml(
                            animation_value,
                            default_durations[animation_type],
                            default_easing.clone(),
                        )
                        .map_err(|e| D::Error::custom(format!("{}", e)))?;

                        let easing_points = easing.to_points();

                        let easing_fn = bezier(
                            easing_points[0],
                            easing_points[1],
                            easing_points[2],
                            easing_points[3],
                        )
                        .map_err(|e| D::Error::custom(format!("{}", e)))?;

                        result.insert(
                            animation_type.clone(),
                            AnimationValue {
                                animation_type: animation_type.clone(),
                                animation_params: AnimationParams {
                                    duration,
                                    easing_fn: Arc::new(easing_fn),
                                },
                            },
                        );
                    }
                    Ok(result)
                })
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
