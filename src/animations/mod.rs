use crate::border_config::ConfigType;
use crate::border_config::CONFIG_TYPE;
use animation::AnimationType;
use animation::AnimationValue;
use easing::AnimationEasing;
use parser::parse_animation;
use parser::parse_animation_from_map;
use parser::parse_animation_from_str;
use parser::AnimationDataType;
use parser::AnimationParserError;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Error as JsonError;
use serde_json::Value as JsonValue;
use serde_yaml_ng::Error as YamlError;
use serde_yaml_ng::Value as YamlValue;

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

fn parse_fn_json(
    value: &JsonValue,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), AnimationParserError> {
    match value {
        JsonValue::Null => Ok((default_duration, default_easing)),
        JsonValue::String(s) => parse_animation_from_str(s, default_duration, default_easing),
        JsonValue::Object(obj) => parse_animation_from_map(
            &AnimationDataType::Json(obj.clone()),
            default_duration,
            default_easing,
        ),
        _ => Err(AnimationParserError::Json(JsonError::custom(format!(
            "Invalid value type for animation: {:?}",
            value
        )))),
    }
}

fn parse_fn_yaml(
    value: &YamlValue,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), AnimationParserError> {
    match value {
        YamlValue::Null => Ok((default_duration, default_easing)),
        YamlValue::String(s) => parse_animation_from_str(s, default_duration, default_easing),
        YamlValue::Mapping(obj) => parse_animation_from_map(
            &AnimationDataType::Yaml(obj.clone()),
            default_duration,
            default_easing,
        ),
        _ => Err(AnimationParserError::Yaml(YamlError::custom(format!(
            "Invalid value type for animation: {:?}",
            value
        )))),
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
        ConfigType::Json => parse_animation(
            deserializer,
            parse_fn_json,
            &default_durations,
            default_easing,
        )
        .map_err(|e| D::Error::custom(format!("{}", e))),
        ConfigType::Yaml => parse_animation(
            deserializer,
            parse_fn_yaml,
            &default_durations,
            default_easing,
        )
        .map_err(|e| D::Error::custom(format!("{}", e))),
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
