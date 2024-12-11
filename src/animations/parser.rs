use super::animation::AnimationParams;
use super::animation::AnimationType;
use super::animation::AnimationValue;
use super::easing::AnimationEasing;
use super::easing::AnimationEasingImpl;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::de::Deserialize as DeDeserialize;
use serde::de::Error as SerdeError;
use serde::Deserializer;
use serde_json::Error as JsonError;
use serde_json::Map;
use serde_json::Value as JsonValue;
use serde_yaml_ng::Error as YamlError;
use serde_yaml_ng::Mapping;
use serde_yaml_ng::Value as YamlValue;
use simple_bezier_easing::bezier;
use std::error::Error as StdError;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

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

pub enum AnimationDataType {
    Yaml(Mapping),
    Json(Map<String, JsonValue>),
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

fn parse_duration_str(s: &str) -> Option<f32> {
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

fn handle_animation_map<T, E>(
    map: FxHashMap<AnimationType, T>,
    parse_fn: impl Fn(&T, f32, AnimationEasing) -> Result<(f32, AnimationEasing), E>,
    default_durations: &FxHashMap<AnimationType, f32>,
    default_easing: AnimationEasing,
) -> Result<FxHashMap<AnimationType, AnimationValue>, E>
where
    E: SerdeError,
{
    let mut deserialized: FxHashMap<AnimationType, AnimationValue> = FxHashMap::default();

    for (animation_type, anim_value) in map {
        let default_duration = *default_durations.get(&animation_type).unwrap_or(&200.0);
        let (duration, easing) = parse_fn(&anim_value, default_duration, default_easing.clone())?;

        let points = easing.to_points();
        let easing_fn = bezier(points[0], points[1], points[2], points[3])
            .map_err(|e| E::custom(e.to_string()))?; // Generic error mapping

        deserialized.insert(
            animation_type.clone(),
            AnimationValue {
                animation_type: animation_type.clone(),
                animation_params: AnimationParams {
                    duration,
                    easing_fn: Arc::new(easing_fn),
                },
            },
        );
    }

    Ok(deserialized)
}

fn parse_animation_from_str(
    s: &str,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), AnimationParserError> {
    let re =
        Regex::new(r"(?i)^([a-zA-Z\-]+|cubic[-_]?bezier\([^\)]+\))\s+([\d.]+(ms|s))$").unwrap();

    re.captures(s)
        .ok_or_else(|| {
            AnimationParserError::Custom(format!("Invalid value for easing and duration: {}", s))
        })
        .map(|caps| {
            let easing = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let duration = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            (
                parse_duration_str(duration).unwrap_or(default_duration),
                AnimationEasing::from_str(easing).unwrap_or(default_easing),
            )
        })
}

fn parse_animation_from_map(
    anim_value: &AnimationDataType,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), AnimationParserError> {
    match anim_value {
        AnimationDataType::Json(obj) => Ok((
            obj.get("duration")
                .and_then(|v| match v {
                    JsonValue::String(s) => parse_duration_str(s),
                    JsonValue::Number(n) => n.as_f64().map(|f| f as f32),
                    _ => None,
                })
                .unwrap_or(default_duration),
            obj.get("easing")
                .and_then(|v| v.as_str().and_then(|s| AnimationEasing::from_str(s).ok()))
                .unwrap_or(default_easing),
        )),
        AnimationDataType::Yaml(obj) => Ok((
            obj.get("duration")
                .and_then(|v| match v {
                    YamlValue::String(s) => parse_duration_str(s),
                    YamlValue::Number(n) => n.as_f64().map(|f| f as f32),
                    _ => None,
                })
                .unwrap_or(default_duration),
            obj.get("easing")
                .and_then(|v| v.as_str().and_then(|s| AnimationEasing::from_str(s).ok()))
                .unwrap_or(default_easing),
        )),
    }
}

pub fn parse_animation<'de, T, D>(
    deserializer: D,
    parse_fn: impl Fn(&T, f32, AnimationEasing) -> Result<(f32, AnimationEasing), AnimationParserError>,
    default_durations: &FxHashMap<AnimationType, f32>,
    default_easing: AnimationEasing,
) -> Result<FxHashMap<AnimationType, AnimationValue>, AnimationParserError>
where
    D: Deserializer<'de>,
    T: DeDeserialize<'de>,
{
    let result = FxHashMap::<AnimationType, T>::deserialize(deserializer);

    match result {
        Ok(map) => handle_animation_map(map, parse_fn, default_durations, default_easing),
        Err(err) => Err(AnimationParserError::Custom(format!("{}", err))),
    }
}

pub fn parse_fn_json(
    value: &JsonValue,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), AnimationParserError> {
    match value {
        JsonValue::Null => Ok((default_duration, default_easing)),
        JsonValue::String(s) => parse_animation_from_str(s, default_duration, default_easing),
        JsonValue::Object(obj) => parse_animation_from_map(
            &AnimationDataType::Json(obj.clone()),
            default_duration,
            default_easing,
        ),
        _ => Err(AnimationParserError::Json(JsonError::custom(format!(
            "Invalid value type for animation: {:?}",
            value
        )))),
    }
}

pub fn parse_fn_yaml(
    value: &YamlValue,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), AnimationParserError> {
    match value {
        YamlValue::Null => Ok((default_duration, default_easing)),
        YamlValue::String(s) => parse_animation_from_str(s, default_duration, default_easing),
        YamlValue::Mapping(obj) => parse_animation_from_map(
            &AnimationDataType::Yaml(obj.clone()),
            default_duration,
            default_easing,
        ),
        _ => Err(AnimationParserError::Yaml(YamlError::custom(format!(
            "Invalid value type for animation: {:?}",
            value
        )))),
    }
}
