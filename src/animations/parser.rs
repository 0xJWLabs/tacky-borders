use anyhow::anyhow;
use anyhow::Result as AnyResult;
use regex::Regex;
use std::sync::LazyLock;

const CUBIC_BEZIER_PATTERN: &str = r"(?i)^cubic[-_]?bezier\(([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+)\)$";
const DURATION_PATTERN: &str = r"(?i)^([\d.]+)(ms|s)$";
pub static CUBIC_BEZIER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(CUBIC_BEZIER_PATTERN).unwrap());

pub static DURATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(DURATION_PATTERN).unwrap());

type CubicBezierPoints = [f32; 4];

pub fn parse_cubic_bezier(input: &str) -> AnyResult<CubicBezierPoints> {
    CUBIC_BEZIER_REGEX
        .captures(input)
        .ok_or_else(|| anyhow!("Invalid cubic bezier format: {input}"))
        .and_then(|caps| {
            let x1 = caps
                .get(1)
                .ok_or_else(|| anyhow!("Missing x1 in cubic bezier: {}", input))?
                .as_str()
                .parse::<f32>()
                .map_err(|_| anyhow!("Failed to parse numeric value in: {}", input))?;

            let y1 = caps
                .get(2)
                .ok_or_else(|| anyhow!("Missing y1 in cubic bezier: {}", input))?
                .as_str()
                .parse::<f32>()
                .map_err(|_| anyhow!("Failed to parse numeric value in: {}", input))?;

            let x2 = caps
                .get(3)
                .ok_or_else(|| anyhow!("Missing x2 in cubic bezier: {}", input))?
                .as_str()
                .parse::<f32>()
                .map_err(|_| anyhow!("Failed to parse numeric value in: {}", input))?;

            let y2 = caps
                .get(4)
                .ok_or_else(|| anyhow!("Missing y2 in cubic bezier: {}", input))?
                .as_str()
                .parse::<f32>()
                .map_err(|_| anyhow!("Failed to parse numeric value in: {}", input))?;

            Ok([x1, y1, x2, y2])
        })
}

pub fn parse_duration_str(input: &str) -> AnyResult<f32> {
    DURATION_REGEX
        .captures(input)
        .ok_or_else(|| anyhow!("Invalid duration format: {}", input))
        .and_then(|caps| {
            let value = caps
                .get(1)
                .ok_or_else(|| anyhow!("Missing value in duration: {}", input))?
                .as_str()
                .parse::<f32>()
                .map_err(|_| anyhow!("Failed to parse numeric value in: {}", input))?;
            let unit = caps
                .get(2)
                .ok_or_else(|| anyhow!("Missing unit in duration: {}", input))?
                .as_str();
            Ok(if unit.eq_ignore_ascii_case("s") {
                value * 1000.0
            } else {
                value
            })
        })
}
