use crate::border_config::ConfigType;
use crate::border_config::CONFIG_TYPE;
use animation::AnimationParams;
use animation::AnimationType;
use animation::AnimationValue;
use easing::AnimationEasing;
use easing::AnimationEasingImpl;
use parser::parse_animation_from_object;
use parser::parse_animation_from_str;
use parser::AnimationDataType;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value as YamlValue;
use simple_bezier_easing::bezier;
use std::sync::Arc;

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

fn handle_animation_map<T, E>(
    map: FxHashMap<AnimationType, T>,
    parse_fn: impl Fn(&T, f32, AnimationEasing) -> Result<(f32, AnimationEasing), E>,
    default_durations: &FxHashMap<AnimationType, f32>,
    default_easing: AnimationEasing,
) -> Result<FxHashMap<AnimationType, AnimationValue>, E>
where
    E: serde::de::Error,
{
    let mut deserialized: FxHashMap<AnimationType, AnimationValue> = FxHashMap::default();

    for (animation_type, anim_value) in map {
        let default_duration = *default_durations.get(&animation_type).unwrap_or(&200.0);
        let (duration, easing) = parse_fn(&anim_value, default_duration, default_easing.clone())?;

        let points = easing.to_points();
        let easing_fn = bezier(points[0], points[1], points[2], points[3])
            .map_err(|e| E::custom(e.to_string()))?; // Generic error mapping

        deserialized.insert(
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

    Ok(deserialized)
}

fn parse_map<'de, T, E, D>(
    deserializer: D,
    parse_fn: impl Fn(&T, f32, AnimationEasing) -> Result<(f32, AnimationEasing), E>,
    default_durations: &FxHashMap<AnimationType, f32>,
    default_easing: AnimationEasing,
) -> Result<FxHashMap<AnimationType, AnimationValue>, E>
where
    E: serde::de::Error,
    D: Deserializer<'de, Error = E>,
    T: serde::de::Deserialize<'de>,
{
    let result = FxHashMap::<AnimationType, T>::deserialize(deserializer);

    match result {
        Ok(map) => handle_animation_map(map, parse_fn, default_durations, default_easing),
        Err(err) => Err(err),
    }
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
            let parse_fn = |value: &JsonValue,
                            default_duration: f32,
                            default_easing: AnimationEasing|
             -> Result<(f32, AnimationEasing), D::Error> {
                match value {
                    JsonValue::Null => Ok((default_duration, default_easing)),
                    JsonValue::String(s) => {
                        parse_animation_from_str(s, default_duration, default_easing)
                            .map_err(serde::de::Error::custom)
                    }
                    JsonValue::Object(obj) => Ok(parse_animation_from_object(
                        &AnimationDataType::Json(obj.clone()),
                        default_duration,
                        default_easing,
                    )),
                    _ => Err(serde::de::Error::custom(format!(
                        "Invalid value type for animation: {:?}",
                        value
                    ))),
                }
            };

            parse_map(deserializer, parse_fn, &default_durations, default_easing)
        }
        ConfigType::Yaml => {
            let parse_fn = |value: &YamlValue,
                            default_duration: f32,
                            default_easing: AnimationEasing|
             -> Result<(f32, AnimationEasing), D::Error> {
                match value {
                    YamlValue::Null => Ok((default_duration, default_easing)),
                    YamlValue::String(s) => {
                        parse_animation_from_str(s, default_duration, default_easing)
                            .map_err(serde::de::Error::custom)
                    }
                    YamlValue::Mapping(obj) => Ok(parse_animation_from_object(
                        &AnimationDataType::Yaml(obj.clone()),
                        default_duration,
                        default_easing,
                    )),
                    _ => Err(serde::de::Error::custom(format!(
                        "Invalid value type for animation: {:?}",
                        value
                    ))),
                }
            };

            parse_map(deserializer, parse_fn, &default_durations, default_easing)
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
