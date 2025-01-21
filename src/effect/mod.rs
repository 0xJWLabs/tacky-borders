use crate::core::{helpers::serde_default_bool, helpers::serde_default_f32, value::Value};
use engine::EffectTranslationStruct;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

pub mod engine;
pub mod manager;
pub mod wrapper;

/// Configuration for multiple effects, including active and inactive effects.
#[derive(Debug, Default, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct EffectsConfig {
    /// A list of active effects.
    #[serde(default)]
    pub active: Vec<EffectConfig>,

    /// A list of inactive effects.
    #[serde(default)]
    pub inactive: Vec<EffectConfig>,

    /// Indicates whether effects are enabled or not.
    #[serde(default = "serde_default_bool::<true>")]
    pub enabled: bool,
}

/// Configuration for a single effect, including its kind, opacity, and translation.
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
pub struct EffectConfig {
    /// The type or kind of effect (e.g., "fade", "zoom").
    pub kind: String,

    /// Optional field representing the standard deviation (or radius) of the effect.
    /// It can be either a number or a string representing length (e.g., "5px").
    #[serde(alias = "radius")]
    // Allows aliasing the "radius" field to be parsed as "standard_deviation".
    pub standard_deviation: Option<Value>,

    /// The opacity of the effect, with a default value of 1.0 (fully opaque).
    #[serde(default = "serde_default_f32::<1>")] // Default opacity is set to 1.0.
    pub opacity: f32,

    /// The translation applied to the effect, with default values set to (0.0, 0.0).
    #[serde(default)] // Defaults to (0.0, 0.0) via the EffectTranslationConfig's Default impl.
    pub translation: EffectTranslationConfig,
}

/// Enum representing the configuration for effect translation.
/// This enum can either hold a structured translation configuration (`EffectTranslationStruct`),
/// or a simple string value.
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum EffectTranslationConfig {
    Struct(EffectTranslationStruct),
    String(String),
}

impl Default for EffectTranslationConfig {
    fn default() -> Self {
        EffectTranslationConfig::Struct(Default::default())
    }
}
