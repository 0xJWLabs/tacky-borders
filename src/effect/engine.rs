use super::{EffectConfig, EffectTranslationConfig};
use crate::core::helpers::parse_length_str;
use crate::core::value::{Value, ValueConversion};
use anyhow::anyhow;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;
use std::str::FromStr;

/// Represents an effect applied to an object, such as a custom window border, including the effect's type,
/// standard deviation (for blur or spread effects), opacity, and translation in 2D space.
///
/// The `EffectEngine` struct is used to define the characteristics of an effect that can be applied to the custom
/// window borders. This may include glow effects, shadow effects, and their respective attributes like how much
/// blur they should have, how transparent they are, and how far the effect is translated from its original position.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectEngine {
    /// The type of effect, such as glow or shadow.
    ///
    /// This defines what kind of effect to apply to the border. For instance, a `Glow` effect can create a halo
    /// around the border, while a `Shadow` effect gives depth or emphasis to the border.
    pub kind: EffectKind,

    /// The standard deviation for the effect, such as the blur intensity for shadow or glow.
    ///
    /// This value determines how far the effect spreads or blurs. For instance, a high standard deviation in a glow
    /// effect would make the glow spread further out from the object.
    pub standard_deviation: f32,

    /// The opacity of the effect, with values between 0.0 (completely transparent) and 1.0 (completely opaque).
    ///
    /// This controls the transparency of the effect itself. For example, a shadow with lower opacity will look
    /// lighter and more transparent, while a higher opacity will make the shadow more intense and visible.
    pub opacity: f32,

    /// The translation of the effect in 2D space, defined by `x` and `y` offsets.
    ///
    /// This specifies how much the effect should be shifted in the x and y directions relative to the object.
    /// For example, a shadow might be translated downward to create the illusion that it is cast beneath the border.
    pub translation: EffectTranslation,
}

/// Defines the different kinds of effects that can be applied to custom window borders, such as a glow effect or a shadow effect.
///
/// This enum allows you to specify what kind of visual effect should be applied. The available options are:
/// `Glow` for a halo-like effect and `Shadow` for a depth-enhancing effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectKind {
    /// A glowing effect, often used to create a halo or highlight around the object.
    ///
    /// A `Glow` effect is typically used to create an illuminated look for the border, often used in modern designs.
    Glow,

    /// A shadow effect, typically used to create depth or emphasis for the object.
    ///
    /// The `Shadow` effect creates the illusion of the object floating above the surface, with a darker or more
    /// diffused shadow based on the chosen parameters.
    Shadow,
}

impl FromStr for EffectKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "glow" => Ok(EffectKind::Glow),
            "shadow" => Ok(EffectKind::Shadow),
            _ => Err("Unknown effect type"),
        }
    }
}

impl TryFrom<EffectConfig> for EffectEngine {
    type Error = anyhow::Error;

    fn try_from(value: EffectConfig) -> Result<Self, Self::Error> {
        let kind = EffectKind::from_str(value.kind.as_str())
            .map_err(|_| anyhow!("invalid or missing animation kind"))?;

        let standard_deviation = value.standard_deviation.as_length_f32().unwrap_or(8.0);

        let translation = match value.translation {
            EffectTranslationConfig::String(translation) => {
                // Split the string by whitespace into components for x and y.
                let data = translation.split_ascii_whitespace().collect::<Vec<&str>>();

                // Ensure there are at least two elements (x and y).
                if data.len() >= 2 {
                    let x_str = data[0];
                    let y_str = data[1];

                    EffectTranslation {
                        x: parse_length_str(x_str).unwrap_or_default() as f32,
                        y: parse_length_str(y_str).unwrap_or_default() as f32,
                    }
                } else {
                    // Handle case where translation string doesn't have both x and y values.
                    EffectTranslation {
                        x: 0.0, // Default value if parsing fails or if there are not enough components.
                        y: 0.0,
                    }
                }
            }
            EffectTranslationConfig::Struct(ref translation) => {
                // Extract x and y from the EffectTranslationStruct
                EffectTranslation {
                    x: translation.x.as_length_f32().unwrap_or_default(),
                    y: translation.y.as_length_f32().unwrap_or_default(),
                }
            }
        };

        Ok(Self {
            kind,
            standard_deviation,
            opacity: value.opacity,
            translation,
        })
    }
}

/// Represents the translation of an effect in 2D space, defined by `x` and `y` coordinates.
///
/// This struct is used for simpler translation cases, where both coordinates are of type `f32`.
/// The default translation is `(x = 0.0, y = 0.0)` when not explicitly set, indicating no translation of the effect.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EffectTranslation {
    /// The translation along the x-axis.
    ///
    /// This value specifies how much the effect should be moved horizontally from its original position.
    /// For example, for a shadow, a positive value on the x-axis might translate the shadow to the right.
    pub x: f32,

    /// The translation along the y-axis.
    ///
    /// This value specifies how much the effect should be moved vertically from its original position.
    /// For example, a positive y translation on a shadow might shift the shadow downward.
    pub y: f32,
}

/// Represents a more flexible configuration for translating an effect in 2D space using dynamic values.
///
/// The `EffectTranslationStruct` allows for more customizable translations, where both the `x` and `y` values
/// are represented as `Value`. This can accommodate dynamic translations that may change over time or need to be
/// evaluated from external sources (e.g., user inputs or system configurations).
/// Default values for both `x` and `y` are `0.0` if not specified.
#[derive(Debug, Clone, Deserialize, PartialEq, JsonSchema)]
#[serde(default)] // Apply the default values provided in `default_translation` to both x and y.
pub struct EffectTranslationStruct {
    /// The translation along the x-axis, represented as a `Value` to allow flexibility in the type.
    ///
    /// The `Value` type allows for more complex translations, including dynamic or deferred evaluation.
    /// By default, this is set to `0.0` if not provided.
    #[serde(default = "default_translation")]
    pub x: Value,

    /// The translation along the y-axis, represented as a `Value` to allow flexibility in the type.
    ///
    /// Like `x`, the `y` translation can be dynamically evaluated, making it useful for cases where translations
    /// depend on variables or expressions. It defaults to `0.0` if not provided.
    #[serde(default = "default_translation")]
    pub y: Value,
}

/// Default value for the translation (`x` and `y`), which returns a `Value::Number` set to `0.0`.
fn default_translation() -> Value {
    Value::Number(0.0)
}

/// Default implementation for `EffectTranslationStruct`. This ensures that both `x` and `y` have the default translation (`0.0`).
impl Default for EffectTranslationStruct {
    fn default() -> Self {
        EffectTranslationStruct {
            x: default_translation(),
            y: default_translation(),
        }
    }
}

