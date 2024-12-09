use animation::Animation;
use animation::AnimationType;
use easing::AnimationEasing;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_yaml_ng::Value;
use std::str::FromStr;

pub mod animation;
pub mod easing;
pub mod timer;
pub mod utils;

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

fn parse_easing_and_duration(
    s: &str,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), String> {
    let re =
        Regex::new(r"^([a-zA-Z\-]+|[Cc]ubic[-_]?[Bb]ezier\([^\)]+\))\s+([\d.]+(ms|s))$").unwrap();

    re.captures(s)
        .ok_or_else(|| format!("Invalid value for easing and duration: {}", s))
        .map(|caps| {
            let easing = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let duration = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            (
                parse_duration_str(duration).unwrap_or(default_duration),
                AnimationEasing::from_str(easing).unwrap_or(default_easing),
            )
        })
}

fn parse_duration_str(s: &str) -> Option<f32> {
    let regex = Regex::new(r"(?i)^([\d.]+)(ms|s)$").unwrap();
    regex.captures(s).and_then(|caps| {
        let value = caps.get(1)?.as_str().parse::<f32>().ok()?;
        Some(if caps.get(2)?.as_str() == "s" {
            value * 1000.0
        } else {
            value
        })
    })
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
