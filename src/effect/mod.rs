use crate::core::{helpers::serde_default_bool, helpers::serde_default_f32, value::Value};
use engine::EffectTranslationStruct;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

pub mod engine;
pub mod manager;
pub mod wrapper;

/// Configuration for multiple effects, including effects for the active and inactive windows.
#[derive(Debug, Default, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct EffectsConfig {
    /// A list of effects to apply to the active window or element.
    ///
    /// This field contains a list of `EffectConfig` objects that represent the effects applied
    /// to the currently active window or element. These effects will take precedence over inactive ones
    /// when the window is in focus or active.
    #[serde(default)]
    pub active: Vec<EffectConfig>,

    /// A list of effects to apply to inactive windows or elements.
    ///
    /// This field contains a list of `EffectConfig` objects that represent the effects applied
    /// to windows or elements that are not currently in focus or are inactive. These effects are applied
    /// to create visual differentiation between active and inactive states.
    #[serde(default)]
    pub inactive: Vec<EffectConfig>,

    /// Indicates whether effects are enabled or not.
    ///
    /// This boolean flag determines whether any effects in `active` or `inactive` should be applied.
    /// It defaults to `true`, indicating that effects are enabled by default.
    #[serde(default = "serde_default_bool::<true>")]
    pub enabled: bool,
}

/// Configuration for a single effect, including its kind, opacity, and translation.
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
pub struct EffectConfig {
    /// The type or kind of effect (e.g., "glow", "shadow").
    ///
    /// This field specifies the type of effect to apply. It can be values like `"glow"`, `"shadow"`, etc.
    /// The effect will be applied based on the specified kind and related configuration.
    pub kind: String,

    /// Optional field representing the standard deviation (or radius) of the effect.
    /// It can be either a number or a string representing length (e.g., "5px").
    ///
    /// This field represents the size or radius of the effect, often used for blur or spread-like effects.
    /// It allows flexible formats like numeric values or strings (e.g., "5px", "0.5pt").
    #[serde(alias = "radius")]
    pub standard_deviation: Option<Value>,

    /// The opacity of the effect, with a maximum value of `f32::MAX` (fully opaque).
    ///
    /// This field controls the transparency of the effect. A value of `1.0` represents fully opaque,
    /// while values between `0.0` and `1.0` represent varying degrees of transparency.
    /// When the opacity is greater than `1.0`, the effect will be duplicated:
    /// - The integer part of the opacity will determine how many fully opaque effects (with `opacity = 1.0`) are applied.
    /// - The fractional part will create a final effect with reduced opacity (e.g., `opacity = 0.5` for `2.5`).
    ///
    /// For example, if the opacity is `2.5`:
    /// - Two fully opaque effects will be created (opacity = 1.0),
    /// - One effect will have `opacity = 0.5` (the remainder).
    ///
    /// - For values between `0.0` and `1.0`, the effect will be semi-transparent.
    #[serde(default = "serde_default_f32::<1>")]
    pub opacity: f32,

    /// The translation applied to the effect, with default values set to (0.0, 0.0).
    ///
    /// This field defines the positional offset applied to the effect, typically used for shifting effects
    /// in various directions (e.g., translating a shadow effect).
    #[serde(default)]
    pub translation: EffectTranslationConfig,
}

/// Enum representing the configuration for translating an effect.
/// This enum can store either a structured translation configuration (`EffectTranslationStruct`),
/// or a simple string-based translation.
///
/// # Examples
///
/// ## Structured Translation:
/// You can specify the translation with explicit `x` and `y` values, either as numbers (e.g., `0.0`) or
/// as strings with units (e.g., `"0px"`). For example:
///
/// - `{ x: 0.0, y: 0.0 }`
/// - `{ x: "0px", y: "0px" }`
///
/// ## String Translation:
/// Alternatively, you can represent the translation as a single string, where the `x` and `y` values are
/// separated by a space. For example:
///
/// - `"0px 0px"`
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum EffectTranslationConfig {
    /// A structured translation configuration.
    ///
    /// This variant holds an `EffectTranslationStruct` which specifies how to translate the effect using
    /// structured `x` and `y` values. The values can either be numbers (e.g., `0.0`) or strings with units (e.g., `"10px"`).
    Struct(EffectTranslationStruct),

    /// A simple string translation.
    ///
    /// This variant holds a simple string representing the translation, where the `x` and `y` values are
    /// separated by a space (e.g., `"10px 5px"`).
    String(String),
}

impl Default for EffectTranslationConfig {
    fn default() -> Self {
        EffectTranslationConfig::Struct(Default::default())
    }
}
