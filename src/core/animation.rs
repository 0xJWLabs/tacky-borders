use anyhow::anyhow;
use regex::Regex;
use serde::Deserialize;
use simple_bezier_easing::bezier;
use std::{
    str::FromStr,
    sync::{Arc, LazyLock},
};

const CUBIC_BEZIER_PATTERN: &str = r"(?i)^cubic[-_]?bezier\(([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+)\)$";
const DURATION_PATTERN: &str = r"(?i)^([\d.]+)(ms|s)$";
pub static CUBIC_BEZIER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(CUBIC_BEZIER_PATTERN).unwrap());

pub static DURATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(DURATION_PATTERN).unwrap());

type CubicBezierPoints = [f32; 4];

pub fn parse_cubic_bezier(input: &str) -> anyhow::Result<CubicBezierPoints> {
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

pub fn parse_duration_str(input: &str) -> anyhow::Result<f32> {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum AnimationKind {
    Spiral,
    Fade,
    ReverseSpiral,
}

impl FromStr for AnimationKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spiral" => Ok(AnimationKind::Spiral),
            "fade" => Ok(AnimationKind::Fade),
            "reverse_spiral" | "reversespiral" | "reverse-spiral" => {
                Ok(AnimationKind::ReverseSpiral)
            }
            _ => Err("Unknown animation type"),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq)]
// #[derive(Debug, Default, Clone, Deserialize, PartialEq)]
pub enum AnimationEasing {
    // Linear
    #[default]
    Linear,

    // EaseIn variants
    EaseIn,
    EaseInSine,
    EaseInQuad,
    EaseInCubic,
    EaseInQuart,
    EaseInQuint,
    EaseInExpo,
    EaseInCirc,
    EaseInBack,

    // EaseOut variants
    EaseOut,
    EaseOutSine,
    EaseOutQuad,
    EaseOutCubic,
    EaseOutQuart,
    EaseOutQuint,
    EaseOutExpo,
    EaseOutCirc,
    EaseOutBack,

    // EaseInOut variants
    EaseInOut,
    EaseInOutSine,
    EaseInOutQuad,
    EaseInOutCubic,
    EaseInOutQuart,
    EaseInOutQuint,
    EaseInOutExpo,
    EaseInOutCirc,
    EaseInOutBack,

    #[serde(untagged)]
    CubicBezier([f32; 4]),
}

impl Eq for AnimationEasing {}

pub type AnimationEasingCallback =
    dyn Fn(f32) -> Result<f32, simple_bezier_easing::BezierError> + Send + Sync;

impl FromStr for AnimationEasing {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_ascii_lowercase().as_str() {
            // Pascal Case
            "linear" => Ok(AnimationEasing::Linear),

            // EaseIn variants
            "easein" | "ease_in" | "ease-in" => Ok(AnimationEasing::EaseIn),
            "easeinsine" | "ease_in_sine" | "ease-in-sine" => Ok(AnimationEasing::EaseInSine),
            "easeinquad" | "ease_in_quad" | "ease-in-quad" => Ok(AnimationEasing::EaseInQuad),
            "easeincubic" | "ease_in_cubic" | "ease-in-cubic" => Ok(AnimationEasing::EaseInCubic),
            "easeinquart" | "ease_in_quart" | "ease-in-quart" => Ok(AnimationEasing::EaseInQuart),
            "easeinquint" | "ease_in_quint" | "ease-in-quint" => Ok(AnimationEasing::EaseInQuint),
            "easeinexpo" | "ease_in_expo" | "ease-in-expo" => Ok(AnimationEasing::EaseInExpo),
            "easeincirc" | "ease_in_circ" | "ease-in-circ" => Ok(AnimationEasing::EaseInCirc),
            "easeinback" | "ease_in_back" | "ease-in-back" => Ok(AnimationEasing::EaseInBack),

            // EaseOut variants
            "easeout" | "ease_out" | "ease-out" => Ok(AnimationEasing::EaseOut),
            "easeoutsine" | "ease_out_sine" | "ease-out-sine" => Ok(AnimationEasing::EaseOutSine),
            "easeoutquad" | "ease_out_quad" | "ease-out-quad" => Ok(AnimationEasing::EaseOutQuad),
            "easeoutcubic" | "ease_out_cubic" | "ease-out-cubic" => {
                Ok(AnimationEasing::EaseOutCubic)
            }
            "easeoutquart" | "ease_out_quart" | "ease-out-quart" => {
                Ok(AnimationEasing::EaseOutQuart)
            }
            "easeoutquint" | "ease_out_quint" | "ease-out-quint" => {
                Ok(AnimationEasing::EaseOutQuint)
            }
            "easeoutexpo" | "ease_out_expo" | "ease-out-expo" => Ok(AnimationEasing::EaseOutExpo),
            "easeoutcirc" | "ease_out_circ" | "ease-out-circ" => Ok(AnimationEasing::EaseOutCirc),
            "easeoutback" | "ease_out_back" | "ease-out-back" => Ok(AnimationEasing::EaseOutBack),

            // EaseInOut variants
            "easeinout" | "ease_in_out" | "ease-in-out" => Ok(AnimationEasing::EaseInOut),
            "easeinoutsine" | "ease_in_out_sine" | "ease-in-out-sine" => {
                Ok(AnimationEasing::EaseInOutSine)
            }
            "easeinoutquad" | "ease_in_out_quad" | "ease-in-out-quad" => {
                Ok(AnimationEasing::EaseInOutQuad)
            }
            "easeinoutcubic" | "ease_in_out_cubic" | "ease-in-out-cubic" => {
                Ok(AnimationEasing::EaseInOutCubic)
            }
            "easeinoutquart" | "ease_in_out_quart" | "ease-in-out-quart" => {
                Ok(AnimationEasing::EaseInOutQuart)
            }
            "easeinoutquint" | "ease_in_out_quint" | "ease-in-out-quint" => {
                Ok(AnimationEasing::EaseInOutQuint)
            }
            "easeinoutexpo" | "ease_in_out_expo" | "ease-in-out-expo" => {
                Ok(AnimationEasing::EaseInOutExpo)
            }
            "easeinoutcirc" | "ease_in_out_circ" | "ease-in-out-circ" => {
                Ok(AnimationEasing::EaseInOutCirc)
            }
            "easeinoutback" | "ease_in_out_back" | "ease-in-out-back" => {
                Ok(AnimationEasing::EaseInOutBack)
            }

            _ if input.to_ascii_lowercase().starts_with("cubic-bezier")
                || input.to_ascii_lowercase().starts_with("cubicbezier")
                || input.to_lowercase().starts_with("cubic_bezier") =>
            {
                parse_cubic_bezier(input)
                    .map(AnimationEasing::CubicBezier)
                    .map_err(|err| format!("invalid cubic-bezier format: {}: {}", input, err))
            }

            _ => Ok(AnimationEasing::default()),
        }
    }
}

pub trait AnimationEasingImpl {
    fn evaluate(&self) -> [f32; 4];
    fn to_fn(&self) -> anyhow::Result<Arc<AnimationEasingCallback>>;
}

impl AnimationEasingImpl for AnimationEasing {
    /// Converts the easing to a corresponding array of points.
    /// Linear and named easing variants will return predefined control points,
    /// while CubicBezier returns its own array.
    fn evaluate(&self) -> [f32; 4] {
        match self {
            // Linear
            AnimationEasing::Linear => [0.0, 0.0, 1.0, 1.0],

            // EaseIn variants
            AnimationEasing::EaseIn => [0.42, 0.0, 1.0, 1.0],
            AnimationEasing::EaseInSine => [0.12, 0.0, 0.39, 0.0],
            AnimationEasing::EaseInQuad => [0.11, 0.0, 0.5, 0.0],
            AnimationEasing::EaseInCubic => [0.32, 0.0, 0.67, 0.0],
            AnimationEasing::EaseInQuart => [0.5, 0.0, 0.75, 0.0],
            AnimationEasing::EaseInQuint => [0.64, 0.0, 0.78, 0.0],
            AnimationEasing::EaseInExpo => [0.7, 0.0, 0.84, 0.0],
            AnimationEasing::EaseInCirc => [0.55, 0.0, 1.0, 0.45],
            AnimationEasing::EaseInBack => [0.36, 0.0, 0.66, -0.56],

            // EaseOut variants
            AnimationEasing::EaseOut => [0.0, 0.0, 0.58, 1.0],
            AnimationEasing::EaseOutSine => [0.61, 1.0, 0.88, 1.0],
            AnimationEasing::EaseOutQuad => [0.5, 1.0, 0.89, 1.0],
            AnimationEasing::EaseOutCubic => [0.33, 1.0, 0.68, 1.0],
            AnimationEasing::EaseOutQuart => [0.25, 1.0, 0.5, 1.0],
            AnimationEasing::EaseOutQuint => [0.22, 1.0, 0.36, 1.0],
            AnimationEasing::EaseOutExpo => [0.16, 1.0, 0.3, 1.0],
            AnimationEasing::EaseOutCirc => [0.0, 0.55, 0.45, 1.0],
            AnimationEasing::EaseOutBack => [0.34, 1.56, 0.64, 1.0],

            // EaseInOut variants
            AnimationEasing::EaseInOut => [0.42, 0.0, 0.58, 1.0],
            AnimationEasing::EaseInOutSine => [0.37, 0.0, 0.63, 1.0],
            AnimationEasing::EaseInOutQuad => [0.45, 0.0, 0.55, 1.0],
            AnimationEasing::EaseInOutCubic => [0.65, 0.0, 0.35, 1.0],
            AnimationEasing::EaseInOutQuart => [0.76, 0.0, 0.24, 1.0],
            AnimationEasing::EaseInOutQuint => [0.83, 0.0, 0.17, 1.0],
            AnimationEasing::EaseInOutExpo => [0.87, 0.0, 0.13, 1.0],
            AnimationEasing::EaseInOutCirc => [0.85, 0.0, 0.15, 1.0],
            AnimationEasing::EaseInOutBack => [0.68, -0.6, 0.32, 1.6],

            // CubicBezier variant returns its own points.
            AnimationEasing::CubicBezier(bezier) => *bezier,
        }
    }

    fn to_fn(&self) -> anyhow::Result<Arc<AnimationEasingCallback>> {
        let easing_points = self.evaluate();

        let easing_fn = bezier(
            easing_points[0],
            easing_points[1],
            easing_points[2],
            easing_points[3],
        )?;

        Ok(Arc::new(easing_fn))
    }
}
