use regex::Regex;
use std::sync::LazyLock;

const CUBIC_BEZIER_PATTERN: &str = r"(?i)^cubic[-_]?bezier\(([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+)\)$";
const DURATION_PATTERN: &str = r"(?i)^([\d.]+)(ms|s)$";
pub static CUBIC_BEZIER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(CUBIC_BEZIER_PATTERN).unwrap());

pub static DURATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(DURATION_PATTERN).unwrap());

pub fn parse_cubic_bezier(input: &str) -> Option<[f32; 4]> {
    if let Some(caps) = CUBIC_BEZIER_REGEX.captures(input) {
        let x1 = caps[1].parse::<f32>().ok()?;
        let y1 = caps[2].parse::<f32>().ok()?;
        let x2 = caps[3].parse::<f32>().ok()?;
        let y2 = caps[4].parse::<f32>().ok()?;
        return Some([x1, y1, x2, y2]);
    }
    None
}

pub fn parse_duration_str(s: &str) -> Option<f32> {
    DURATION_REGEX.captures(s).and_then(|caps| {
        let value = caps.get(1)?.as_str().parse::<f32>().ok()?;
        Some(if caps.get(2)?.as_str() == "s" {
            value * 1000.0
        } else {
            value
        })
    })
}
