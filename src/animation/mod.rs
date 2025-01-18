use crate::core::value::Value;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

pub mod engine;
pub mod manager;
pub mod wrapper;

#[derive(Debug, Deserialize, Clone, Default, PartialEq, JsonSchema)]
pub struct AnimationsConfig {
    pub active: Option<Vec<AnimationConfig>>,
    pub inactive: Option<Vec<AnimationConfig>>,
    pub fps: Option<i32>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, JsonSchema)]
pub struct AnimationConfig {
    pub kind: String,
    pub duration: Option<Value>,
    pub easing: Option<String>,
}
