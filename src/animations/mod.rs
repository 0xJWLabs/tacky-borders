use std::collections::HashMap;
use std::str::FromStr;

use animation::Animation;
use animation::AnimationType;
use easing::AnimationEasing;
use serde::Deserialize;
use serde::Deserializer;
use serde_yml::Value;

use crate::deserializer::from_str;

pub mod animation;
pub mod easing;
pub mod utils;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE: i32 = 1;

#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default)]
    pub active: HashMap<AnimationType, Animation>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: HashMap<AnimationType, Animation>,
    #[serde(skip)]
    pub current: HashMap<AnimationType, Animation>,
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

fn animation<'de, D>(deserializer: D) -> Result<HashMap<AnimationType, Animation>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Option<HashMap<String, Value>> = Option::deserialize(deserializer)?;
    let mut result: HashMap<AnimationType, Animation> = HashMap::new();

    if let Some(entries) = map {
        for (anim_type_str, anim_value) in entries {
            let animation_type: Result<AnimationType, _> = from_str(&anim_type_str);

            if let Ok(animation_type) = animation_type {
                if animation_type == AnimationType::None {
                    continue;
                }

                if let Value::Mapping(ref obj) = anim_value {
                    let speed = match obj.get("speed") {
                        Some(Value::Number(n)) => n.as_f64().map(|f| f as f32),
                        _ => None,
                    };

                    let easing = match obj.get("easing") {
                        Some(Value::String(s)) => match AnimationEasing::from_str(&s) {
                            Ok(animation) => animation,
                            Err(_) => AnimationEasing::Linear,
                        },
                        _ => AnimationEasing::Linear,
                    };

                    let default_speed = match animation_type {
                        AnimationType::Spiral
                        | AnimationType::ReverseSpiral
                        | AnimationType::Fade => 50.0,
                        _ => 0.0, // Default fallback for other types
                    };

                    let animation = Animation {
                        animation_type: animation_type.clone(),
                        speed: speed.unwrap_or(default_speed),
                        easing,
                    };

                    println!("{:?}", animation);

                    result.insert(animation_type, animation);
                }
            } else {
                println!("Invalid animation type: {}", anim_type_str);
            }
        }
    }

    Ok(result)
}

pub trait HashMapAnimationExt {
    fn find(&self, animation_type: &AnimationType) -> Option<&Animation>;
    fn has(&self, animation_type: &AnimationType) -> bool;
    fn to_iter(&self) -> impl Iterator<Item = &Animation> + '_;
}

impl HashMapAnimationExt for HashMap<AnimationType, Animation> {
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
