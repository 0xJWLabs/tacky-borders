use animation::AnimationConfig;
use serde::Deserialize;
use timer::AnimationTimer;
pub use wrapper::AnimationsVec;

pub mod animation;
mod easing;
mod parser;
pub mod timer;
pub mod wrapper;

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct AnimationsConfig {
    pub active: Option<Vec<AnimationConfig>>,
    pub inactive: Option<Vec<AnimationConfig>>,
    pub fps: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Animations {
    pub active: AnimationsVec,
    pub inactive: AnimationsVec,
    pub fps: i32,
    pub progress: AnimationsProgress,
    pub flags: AnimationsFlags,
    pub timer: Option<AnimationTimer>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct AnimationsProgress {
    pub fade: f32,
    pub spiral: f32,
    pub angle: f32,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct AnimationsFlags {
    pub fade_to_visible: bool,
    pub should_fade: bool,
}

impl TryFrom<AnimationsConfig> for Animations {
    type Error = anyhow::Error;
    fn try_from(value: AnimationsConfig) -> Result<Animations, Self::Error> {
        let active = AnimationsVec::try_from(value.active.clone().unwrap_or_default())?;
        let inactive = AnimationsVec::try_from(value.inactive.clone().unwrap_or_default())?;
        Ok(Animations {
            active,
            inactive,
            fps: value.fps.unwrap_or(60),
            ..Default::default()
        })
    }
}
