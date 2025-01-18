use crate::core::{helpers::serde_default_f32, value::Value};
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

pub mod engine;
pub mod manager;

/// Configuration for multiple effects, including active and inactive effects.
#[derive(Debug, Default, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct EffectsConfig {
    /// A list of active effects.
    pub active: Vec<EffectConfig>,

    /// A list of inactive effects.
    pub inactive: Vec<EffectConfig>,

    /// Indicates whether effects are enabled or not.
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

/// Configuration for the translation of an effect, including both x and y coordinates.
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
#[serde(default)] // Apply the default values provided in `default_translation` to both x and y.
pub struct EffectTranslationConfig {
    /// The translation on the x-axis.
    /// Default is a `Value::Number(0.0)` if no value is provided.
    #[serde(default = "default_translation")]
    pub x: Value,

    /// The translation on the y-axis.
    /// Default is a `Value::Number(0.0)` if no value is provided.
    #[serde(default = "default_translation")]
    pub y: Value,
}

/// Default value for `EffectTranslationConfig::x` and `EffectTranslationConfig::y`.
/// Both x and y will default to `Value::Number(0.0)` if not specified.
fn default_translation() -> Value {
    Value::Number(0.0)
}

/// Default implementation for `EffectTranslationConfig`. This will ensure that both x and y have the default translation.
impl Default for EffectTranslationConfig {
    fn default() -> Self {
        EffectTranslationConfig {
            x: default_translation(),
            y: default_translation(),
        }
    }
}
