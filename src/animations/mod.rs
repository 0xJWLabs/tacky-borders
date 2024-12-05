use animation::Animation;
use animation::AnimationType;
use easing::AnimationEasing;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde::Deserializer;
use serde_plain2::from_str;
use serde_yaml_ng::Value;
use std::str::FromStr;

pub mod animation;
pub mod easing;
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
    let result = FxHashMap::<String, Value>::deserialize(deserializer);
    let hashmap = match result {
        Ok(val) => val,
        Err(err) => return Err(err),
    };

    let mut deserialized: FxHashMap<AnimationType, Animation> = FxHashMap::default();

    for (anim_type_str, anim_value) in hashmap {
        let animation_type: Result<AnimationType, _> = from_str(&anim_type_str);

        if let Ok(animation_type) = animation_type {
            if animation_type == AnimationType::None {
                continue;
            }

            let default_speed = match animation_type {
                AnimationType::Spiral | AnimationType::ReverseSpiral | AnimationType::Fade => 50.0,
                _ => 0.0, // Default fallback for other types
            };

            if let Value::Null = anim_value {
                deserialized.insert(
                    animation_type.clone(),
                    Animation {
                        animation_type: animation_type.clone(),
                        speed: default_speed,
                        easing: AnimationEasing::Linear,
                    },
                );
            } else if let Value::Mapping(ref obj) = anim_value {
                let speed = match obj.get("speed") {
                    Some(Value::Number(n)) => n.as_f64().map(|f| f as f32),
                    _ => None,
                };

                let easing = match obj.get("easing") {
                    Some(Value::String(s)) => {
                        AnimationEasing::from_str(s).unwrap_or(AnimationEasing::Linear)
                    }
                    _ => AnimationEasing::Linear,
                };

                let default_speed = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral | AnimationType::Fade => {
                        50.0
                    }
                    _ => 0.0, // Default fallback for other types
                };

                let animation = Animation {
                    animation_type: animation_type.clone(),
                    speed: speed.unwrap_or(default_speed),
                    easing,
                };

                deserialized.insert(animation_type, animation);
            } else {
                println!("Invalid value type: {:?}", anim_value);
            }
        } else {
            println!("Invalid animation type: {}", anim_type_str);
        }
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
