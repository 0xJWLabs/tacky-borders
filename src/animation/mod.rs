use crate::core::duration::Duration;
use serde::Deserialize;

pub mod engine;
pub mod manager;
pub mod wrapper;

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct AnimationsConfig {
    pub active: Option<Vec<AnimationConfig>>,
    pub inactive: Option<Vec<AnimationConfig>>,
    pub fps: Option<i32>,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct AnimationConfig {
    pub kind: String,
    pub duration: Option<Duration>,
    pub easing: Option<String>,
}
