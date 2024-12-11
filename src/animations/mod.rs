use crate::border_config::ConfigType;
use crate::border_config::CONFIG_TYPE;
use animation::AnimationType;
use animation::AnimationValue;
use easing::AnimationEasing;
use parser::parse_animation;
use parser::parse_fn_json;
use parser::parse_fn_yaml;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;

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
