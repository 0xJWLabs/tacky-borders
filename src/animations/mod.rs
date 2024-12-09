use animation::Animation;
use animation::AnimationType;
use easing::AnimationEasing;
use parser::parse_duration_str;
use parser::parse_easing_and_duration;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_yaml_ng::Value;
use std::str::FromStr;

pub mod animation;
mod easing;
mod parser;
pub mod timer;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE: i32 = 1;

#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default)]
    pub active: FxHashMap<AnimationType, Animation>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: FxHashMap<AnimationType, Animation>,
    #[serde(skip)]
    pub current: FxHashMap<AnimationType, Animation>,
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

fn animation<'de, D>(deserializer: D) -> Result<FxHashMap<AnimationType, Animation>, D::Error>
where
    D: Deserializer<'de>,
{
    let result = FxHashMap::<AnimationType, Value>::deserialize(deserializer);

    // If deserialize returns an error, it's possible that an invalid AnimationType was listed
    let map = match result {
        Ok(val) => val
            .into_iter() // Convert into iterator
            .collect::<FxHashMap<_, _>>(), // Collect back into a map
        Err(err) => return Err(err),
    };

    let mut deserialized: FxHashMap<AnimationType, Animation> = FxHashMap::default();

    for (animation_type, anim_value) in map {
        let default_duration = match animation_type {
            AnimationType::Spiral | AnimationType::ReverseSpiral => 1800.0,
            AnimationType::Fade => 200.0,
        };
        let default_easing = AnimationEasing::Linear;
        let (duration, easing) = match anim_value {
            Value::Null => (default_duration, default_easing),
            Value::Mapping(ref obj) => (
                obj.get("duration")
                    .and_then(|v| match v {
                        Value::String(s) => parse_duration_str(s),
                        Value::Number(n) => n.as_f64().map(|f| f as f32),
                        _ => None,
                    })
                    .unwrap_or(default_duration),
                obj.get("easing")
                    .and_then(|v| v.as_str().and_then(|s| AnimationEasing::from_str(s).ok()))
                    .unwrap_or(default_easing),
            ),
            Value::String(s) => parse_easing_and_duration(&s, default_duration, default_easing)
                .map_err(D::Error::custom)?, // Explicit conversion
            _ => {
                return Err(D::Error::custom(format!(
                    "Invalid value type for animation: {:?}",
                    anim_value
                )));
            }
        };

        let anim = Animation {
            animation_type: animation_type.clone(),
            duration,
            easing,
        };

        // Insert the deserialized animation data into the map
        deserialized.insert(animation_type.clone(), anim);
    }

    Ok(deserialized)
}

pub trait HashMapAnimationExt {
    fn find(&self, animation_type: &AnimationType) -> Option<&Animation>;
    fn has(&self, animation_type: &AnimationType) -> bool;
    fn to_iter(&self) -> impl Iterator<Item = &Animation> + '_;
}

impl HashMapAnimationExt for FxHashMap<AnimationType, Animation> {
    fn find(&self, animation_type: &AnimationType) -> Option<&Animation> {
        self.get(animation_type)
    }

    fn has(&self, animation_type: &AnimationType) -> bool {
        self.contains_key(animation_type)
    }

    fn to_iter(&self) -> impl Iterator<Item = &Animation> + '_ {
        self.values()
    }
}
