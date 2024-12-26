use regex::Regex;
use serde::de::Error as SerdeError;
use serde_jsonc2::Error as JsonError;
use serde_jsonc2::Map;
use serde_jsonc2::Value as JsonValue;
use serde_yml::Error as YamlError;
use serde_yml::Mapping;
use serde_yml::Value as YamlValue;
use simple_bezier_easing::bezier;
use std::error::Error as StdError;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use super::animation::Animation;
use super::animation::AnimationType;
use super::easing::AnimationEasing;
use super::easing::AnimationEasingCallback;
use super::easing::AnimationEasingImpl;

const MISSING_KIND_FIELD: &str = "Missing 'kind' field";
const INVALID_KIND_FIELD_TYPE: &str = "Invalid 'kind' field type";

pub fn parse_cubic_bezier(input: &str) -> Option<[f32; 4]> {
    let re = Regex::new(r"(?i)^cubic[-_]?bezier\(([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+)\)$").unwrap();

    if let Some(caps) = re.captures(input) {
        let x1 = caps[1].parse::<f32>().ok()?;
        let y1 = caps[2].parse::<f32>().ok()?;
        let x2 = caps[3].parse::<f32>().ok()?;
        let y2 = caps[4].parse::<f32>().ok()?;
        return Some([x1, y1, x2, y2]);
    }
    None
}

pub fn parse_duration_str(s: &str) -> Option<f32> {
    let regex = Regex::new(r"(?i)^([\d.]+)(ms|s)$").unwrap();
    regex.captures(s).and_then(|caps| {
        let value = caps.get(1)?.as_str().parse::<f32>().ok()?;
        Some(if caps.get(2)?.as_str() == "s" {
            value * 1000.0
        } else {
            value
        })
    })
}

#[derive(Debug)]
pub enum AnimationParserError {
    Json(JsonError),
    Yaml(YamlError),
    Custom(String),
}

impl SerdeError for AnimationParserError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        AnimationParserError::Custom(msg.to_string())
    }
}

impl StdError for AnimationParserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AnimationParserError::Json(err) => Some(err),
            AnimationParserError::Yaml(err) => Some(err),
            AnimationParserError::Custom(_) => None, // No underlying error for Custom variant
        }
    }
}

impl fmt::Display for AnimationParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnimationParserError::Json(err) => write!(f, "[ERROR] JSON: {}", err),
            AnimationParserError::Yaml(err) => write!(f, "[ERROR] YAML: {}", err),
            AnimationParserError::Custom(msg) => write!(f, "[ERROR] Custom: {}", msg),
        }
    }
}

pub enum AnimationDataType {
    Yaml(Mapping),
    Json(Map<String, JsonValue>),
}

fn parse_animation_from_map(
    anim_value: &AnimationDataType,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, Arc<AnimationEasingCallback>), AnimationParserError> {
    let easing = match anim_value {
        AnimationDataType::Json(obj) => obj
            .get("easing")
            .and_then(|v| v.as_str().and_then(|s| AnimationEasing::from_str(s).ok()))
            .unwrap_or(default_easing),
        AnimationDataType::Yaml(obj) => obj
            .get("easing")
            .and_then(|v| v.as_str().and_then(|s| AnimationEasing::from_str(s).ok()))
            .unwrap_or(default_easing),
    };

    let duration = match anim_value {
        AnimationDataType::Json(obj) => obj
            .get("duration")
            .and_then(|v| match v {
                JsonValue::String(s) => parse_duration_str(s),
                JsonValue::Number(n) => n.as_f64().map(|f| f as f32),
                _ => None,
            })
            .unwrap_or(default_duration),
        AnimationDataType::Yaml(obj) => obj
            .get("duration")
            .and_then(|v| match v {
                YamlValue::String(s) => parse_duration_str(s),
                YamlValue::Number(n) => n.as_f64().map(|f| f as f32),
                _ => None,
            })
            .unwrap_or(default_duration),
    };

    let easing_points = easing.evaluate();

    let easing_fn = bezier(
        easing_points[0],
        easing_points[1],
        easing_points[2],
        easing_points[3],
    )
    .map_err(|e| match anim_value {
        AnimationDataType::Json(_) => AnimationParserError::Json(JsonError::custom(e.to_string())),
        AnimationDataType::Yaml(_) => AnimationParserError::Yaml(YamlError::custom(e.to_string())),
    })?;

    Ok((duration, Arc::new(easing_fn)))
}

impl From<JsonError> for AnimationParserError {
    fn from(err: JsonError) -> Self {
        AnimationParserError::Json(err)
    }
}

impl From<YamlError> for AnimationParserError {
    fn from(err: YamlError) -> Self {
        AnimationParserError::Yaml(err)
    }
}

pub trait IdentifiableAnimationValue {
    const TYPE_NAME: &'static str;

    fn parse(&self) -> Result<Animation, AnimationParserError>;
}

impl IdentifiableAnimationValue for JsonValue {
    const TYPE_NAME: &'static str = "serde_jsonc2::Value";

    fn parse(&self) -> Result<Animation, AnimationParserError> {
        match self {
            JsonValue::Object(obj) => {
                let kind = obj.get("kind").ok_or_else(|| {
                    AnimationParserError::Json(JsonError::custom(MISSING_KIND_FIELD))
                })?;

                let animation_type = kind
                    .as_str()
                    .ok_or_else(|| {
                        AnimationParserError::Json(JsonError::custom(INVALID_KIND_FIELD_TYPE))
                    })?
                    .parse::<AnimationType>()
                    .map_err(|e| AnimationParserError::Json(JsonError::custom(e)))?;

                let default_duration = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral => 1800.0,
                    AnimationType::Fade => 200.0,
                };

                let (duration, easing_fn) = parse_animation_from_map(
                    &AnimationDataType::Json(obj.clone()),
                    default_duration,
                    AnimationEasing::Linear,
                )?;

                // Return the constructed AnimationConfig
                Ok(Animation {
                    kind: animation_type,
                    duration,
                    easing_fn,
                })
            }
            _ => Err(AnimationParserError::Json(JsonError::custom(
                "Expected JSON object for animation config",
            ))),
        }
    }
}

impl IdentifiableAnimationValue for YamlValue {
    const TYPE_NAME: &'static str = "serde_yml::Value";

    fn parse(&self) -> Result<Animation, AnimationParserError> {
        match self {
            YamlValue::Mapping(obj) => {
                let kind = obj.get("kind").ok_or_else(|| {
                    AnimationParserError::Yaml(YamlError::custom("Missing `kind` field"))
                })?;

                let animation_type = kind
                    .as_str()
                    .ok_or_else(|| {
                        AnimationParserError::Yaml(YamlError::custom("Invalid `kind` field type"))
                    })?
                    .parse::<AnimationType>()
                    .map_err(|e| AnimationParserError::Yaml(YamlError::custom(e)))?;

                let default_duration = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral => 1800.0,
                    AnimationType::Fade => 200.0,
                };

                let (duration, easing_fn) = parse_animation_from_map(
                    &AnimationDataType::Yaml(obj.clone()),
                    default_duration,
                    AnimationEasing::Linear,
                )?;

                // Return the constructed AnimationConfig
                Ok(Animation {
                    kind: animation_type,
                    duration,
                    easing_fn,
                })
            }
            _ => Err(AnimationParserError::Yaml(YamlError::custom(
                "Expected YAML mapping for animation config",
            ))),
        }
    }
}
