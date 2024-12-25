use crate::user_config::ConfigFormat;
use crate::user_config::CONFIG_FORMAT;
use animation::Animation;
use animation::AnimationType;
use parser::AnimationParserError;
use parser::IdentifiableAnimationValue;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use serde_jsonc2::Value as JsonValue;
use serde_yml::Value as YamlValue;
use timer::AnimationTimer;

pub mod animation;
mod easing;
mod parser;
pub mod timer;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AnimationProgress {
    pub fade: f32,
    pub spiral: f32,
    pub angle: f32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AnimationFlags {
    pub fade_to_visible: bool,
    pub should_fade: bool,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default)]
    pub active: Vec<Animation>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: Vec<Animation>,
    #[serde(default = "default_fps")]
    pub fps: i32,
    #[serde(skip)]
    pub progress: AnimationProgress,
    #[serde(skip)]
    pub flags: AnimationFlags,
    #[serde(skip)]
    pub timer: Option<AnimationTimer>,
}

pub trait AnimationsImpl {
    fn contains_kind(&self, kind: AnimationType) -> bool;
}

// Implement the trait for Vec<Animation>
impl AnimationsImpl for Vec<Animation> {
    fn contains_kind(&self, kind: AnimationType) -> bool {
        self.iter().any(|a| a.kind == kind)
    }
}

fn handle_vec<T>(vec: Vec<T>) -> Result<Vec<Animation>, AnimationParserError>
where
    T: IdentifiableAnimationValue,
{
    vec.into_iter()
        .map(|animation_value| animation_value.parse())
        .collect()
}

fn animation<'de, D>(deserializer: D) -> Result<Vec<Animation>, D::Error>
where
    D: Deserializer<'de>,
{
    match *CONFIG_FORMAT.read().unwrap() {
        ConfigFormat::Json | ConfigFormat::Jsonc => {
            let vec: Vec<JsonValue> = Vec::deserialize(deserializer).map_err(D::Error::custom)?;

            handle_vec(vec).map_err(D::Error::custom)
        }
        ConfigFormat::Yaml => {
            let vec: Vec<YamlValue> = Vec::deserialize(deserializer).map_err(D::Error::custom)?;
            handle_vec(vec).map_err(D::Error::custom)
        }
        _ => Err(D::Error::custom("Invalid file type")),
    }
}

fn default_fps() -> i32 {
    60
}
