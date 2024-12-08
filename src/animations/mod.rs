use animation::Animation;
use animation::AnimationType;
use easing::AnimationEasing;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_yaml_ng::Mapping;
use serde_yaml_ng::Value;
use std::any::type_name;
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
        println!("A: {}", type_name::<Value>());
        let (duration, easing) = match anim_value {
            Value::Null => (default_duration, default_easing),
            Value::Mapping(ref obj) => {
                let duration = parse_duration_from_object(obj, default_duration);
                let easing = parse_easing_from_object(obj).unwrap_or(default_easing);
                (duration, easing)
            }
            Value::String(ref s) => {
                if let Some((easing_str, duration_str)) = parse_easing_and_duration(s) {
                    println!("{}", easing_str);
                    let duration = parse_duration_from_string(duration_str, default_duration);
                    let easing = AnimationEasing::from_str(easing_str).unwrap_or(default_easing);
                    (duration, easing)
                } else {
                    return Err(D::Error::custom(format!(
                        "Invalid value for easing and duration: {}",
                        s
                    )));
                }
            }
            _ => {
                return Err(D::Error::custom(format!(
                    "Invalid value type for animation: {:?}",
                    anim_value
                )));
            }
        };

        println!("duration: {}, easing: {:?}", duration, easing);

        // Insert the deserialized animation data into the map
        deserialized.insert(
            animation_type.clone(),
            Animation {
                animation_type: animation_type.clone(),
                duration,
                easing,
            },
        );
    }

    Ok(deserialized)
}

/// Helper function to parse the `duration` from a `Value::Mapping`
fn parse_duration_from_object(obj: &Mapping, default_duration: f32) -> f32 {
    obj.get("duration")
        .and_then(|v| match v {
            Value::String(s) => Some(parse_duration_from_string(s, default_duration)),
            Value::Number(n) => n.as_f64().map(|f| f as f32),
            _ => None,
        })
        .unwrap_or(default_duration) // If no "duration" key or parsing fails, return default_duration
}

/// Helper function to parse the `duration` from a `String` value like "30ms" or "3s"
fn parse_duration_from_string(s: &str, default_duration: f32) -> f32 {
    let regex = Regex::new(r"(?i)^([\d.]+)(ms|s)$").unwrap();
    if let Some(captures) = regex.captures(s) {
        let duration_num = captures
            .get(1)
            .map(|m| m.as_str().parse::<f32>().unwrap_or(default_duration)) // Default duration if parsing fails
            .unwrap_or(default_duration);
        let unit = captures.get(2).map(|m| m.as_str()).unwrap_or("ms");

        // Convert seconds to milliseconds if necessary
        if unit == "s" {
            duration_num * 1000.0
        } else {
            duration_num
        }
    } else {
        default_duration
    }
}

fn parse_easing_and_duration(s: &str) -> Option<(&str, &str)> {
    let re = Regex::new(r"^([a-zA-Z\-]+)\s+([\d.]+(ms|s))$").unwrap();
    if let Some(captures) = re.captures(s) {
        Some((captures.get(1)?.as_str(), captures.get(2)?.as_str()))
    } else {
        // Check for cubic-bezier(...) string
        let re_cubic =
            Regex::new(r"^([Cc]ubic[-_]?[Bb]ezier\([^\)]+\))\s+([\d.]+(ms|s))$").unwrap();
        if let Some(captures) = re_cubic.captures(s) {
            Some((captures.get(1)?.as_str(), captures.get(2)?.as_str()))
        } else {
            None
        }
    }
}

fn parse_easing_from_object(obj: &Mapping) -> Option<AnimationEasing> {
    obj.get("easing")
        .and_then(|v| v.as_str())
        .map(|s| AnimationEasing::from_str(s).unwrap_or(AnimationEasing::Linear))
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
