use super::easing::AnimationEasing;
use regex::Regex;
use serde_json::Map;
use serde_json::Value as JsonValue;
use serde_yaml_ng::Mapping;
use serde_yaml_ng::Value as YamlValue;
use std::str::FromStr;

pub enum AnimationDataType {
    Yaml(Mapping),
    Json(Map<String, JsonValue>),
}

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

pub fn parse_animation_from_str(
    s: &str,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), String> {
    let re =
        Regex::new(r"(?i)^([a-zA-Z\-]+|cubic[-_]?bezier\([^\)]+\))\s+([\d.]+(ms|s))$").unwrap();

    re.captures(s)
        .ok_or_else(|| format!("Invalid value for easing and duration: {}", s))
        .map(|caps| {
            let easing = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let duration = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            (
                parse_duration_str(duration).unwrap_or(default_duration),
                AnimationEasing::from_str(easing).unwrap_or(default_easing),
            )
        })
}

pub fn parse_animation_from_object(
    anim_value: &AnimationDataType,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> (f32, AnimationEasing) {
    match anim_value {
        AnimationDataType::Json(obj) => (
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
        ),
        AnimationDataType::Yaml(obj) => (
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
        ),
    }
}
