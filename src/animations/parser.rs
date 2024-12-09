use super::easing::AnimationEasing;
use regex::Regex;
use std::str::FromStr;

pub fn parse_cubic_bezier(input: &str) -> Option<[f32; 4]> {
    let re = Regex::new(r"^[Cc]ubic[-_]?[Bb]ezier\(([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+)\)$").unwrap();

    if let Some(caps) = re.captures(input) {
        let x1 = caps[1].parse::<f32>().ok()?;
        let y1 = caps[2].parse::<f32>().ok()?;
        let x2 = caps[3].parse::<f32>().ok()?;
        let y2 = caps[4].parse::<f32>().ok()?;
        return Some([x1, y1, x2, y2]);
    }
    None
}

pub fn parse_easing_and_duration(
    s: &str,
    default_duration: f32,
    default_easing: AnimationEasing,
) -> Result<(f32, AnimationEasing), String> {
    let re =
        Regex::new(r"^([a-zA-Z\-]+|[Cc]ubic[-_]?[Bb]ezier\([^\)]+\))\s+([\d.]+(ms|s))$").unwrap();

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
