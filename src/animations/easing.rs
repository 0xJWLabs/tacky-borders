use super::utils::parse_cubic_bezier;
use serde::Deserialize;
use std::hash::Hash;
use std::hash::Hasher;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum AnimationEasing {
    // Linear
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

    CubicBezier([f32; 4]),
}

impl Hash for AnimationEasing {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            // Linear
            AnimationEasing::Linear => 0.hash(state),

            // EaseIn and its variants
            AnimationEasing::EaseIn => 1.hash(state),
            AnimationEasing::EaseInSine => 2.hash(state),
            AnimationEasing::EaseInQuad => 3.hash(state),
            AnimationEasing::EaseInCubic => 4.hash(state),
            AnimationEasing::EaseInQuart => 5.hash(state),
            AnimationEasing::EaseInQuint => 6.hash(state),
            AnimationEasing::EaseInExpo => 7.hash(state),
            AnimationEasing::EaseInCirc => 8.hash(state),
            AnimationEasing::EaseInBack => 9.hash(state),

            // EaseOut and its variants
            AnimationEasing::EaseOut => 10.hash(state),
            AnimationEasing::EaseOutSine => 11.hash(state),
            AnimationEasing::EaseOutQuad => 12.hash(state),
            AnimationEasing::EaseOutCubic => 13.hash(state),
            AnimationEasing::EaseOutQuart => 14.hash(state),
            AnimationEasing::EaseOutQuint => 15.hash(state),
            AnimationEasing::EaseOutExpo => 16.hash(state),
            AnimationEasing::EaseOutCirc => 17.hash(state),
            AnimationEasing::EaseOutBack => 18.hash(state),

            // EaseInOut and its variants
            AnimationEasing::EaseInOut => 19.hash(state),
            AnimationEasing::EaseInOutSine => 20.hash(state),
            AnimationEasing::EaseInOutQuad => 21.hash(state),
            AnimationEasing::EaseInOutCubic => 22.hash(state),
            AnimationEasing::EaseInOutQuart => 23.hash(state),
            AnimationEasing::EaseInOutQuint => 24.hash(state),
            AnimationEasing::EaseInOutExpo => 25.hash(state),
            AnimationEasing::EaseInOutCirc => 26.hash(state),
            AnimationEasing::EaseInOutBack => 27.hash(state),

            // CubicBezier variant
            AnimationEasing::CubicBezier(bezier) => {
                28.hash(state); // Unique prefix for the CubicBezier variant
                for &value in bezier.iter() {
                    value.to_bits().hash(state); // Hash each float consistently
                }
            }
        }
    }
}

impl Eq for AnimationEasing {}

impl FromStr for AnimationEasing {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            // Pascal Case
            "Linear" | "linear" => Ok(AnimationEasing::Linear),

            // EaseIn variants (PascalCase | camelCase | snake_case | kebab-case)
            "EaseIn" | "easeIn" | "ease_in" | "ease-in" => Ok(AnimationEasing::EaseIn),
            "EaseInSine" | "easeInSine" | "ease_in_sine" | "ease-in-sine" => {
                Ok(AnimationEasing::EaseInSine)
            }
            "EaseInQuad" | "easeInQuad" | "ease_in_quad" | "ease-in-quad" => {
                Ok(AnimationEasing::EaseInQuad)
            }
            "EaseInCubic" | "easeInCubic" | "ease_in_cubic" | "ease-in-cubic" => {
                Ok(AnimationEasing::EaseInCubic)
            }
            "EaseInQuart" | "easeInQuart" | "ease_in_quart" | "ease-in-quart" => {
                Ok(AnimationEasing::EaseInQuart)
            }
            "EaseInQuint" | "easeInQuint" | "ease_in_quint" | "ease-in-quint" => {
                Ok(AnimationEasing::EaseInQuint)
            }
            "EaseInExpo" | "easeInExpo" | "ease_in_expo" | "ease-in-expo" => {
                Ok(AnimationEasing::EaseInExpo)
            }
            "EaseInCirc" | "easeInCirc" | "ease_in_circ" | "ease-in-circ" => {
                Ok(AnimationEasing::EaseInCirc)
            }
            "EaseInBack" | "easeInBack" | "ease_in_back" | "ease-in-back" => {
                Ok(AnimationEasing::EaseInBack)
            }

            // EaseOut variants
            "EaseOut" | "easeOut" | "ease_out" | "ease-out" => Ok(AnimationEasing::EaseOut),
            "EaseOutSine" | "easeOutSine" | "ease_out_sine" | "ease-out-sine" => {
                Ok(AnimationEasing::EaseOutSine)
            }
            "EaseOutQuad" | "easeOutQuad" | "ease_out_quad" | "ease-out-quad" => {
                Ok(AnimationEasing::EaseOutQuad)
            }
            "EaseOutCubic" | "easeOutCubic" | "ease_out_cubic" | "ease-out-cubic" => {
                Ok(AnimationEasing::EaseOutCubic)
            }
            "EaseOutQuart" | "easeOutQuart" | "ease_out_quart" | "ease-out-quart" => {
                Ok(AnimationEasing::EaseOutQuart)
            }
            "EaseOutQuint" | "easeOutQuint" | "ease_out_quint" | "ease-out-quint" => {
                Ok(AnimationEasing::EaseOutQuint)
            }
            "EaseOutExpo" | "easeOutExpo" | "ease_out_expo" | "ease-out-expo" => {
                Ok(AnimationEasing::EaseOutExpo)
            }
            "EaseOutCirc" | "easeOutCirc" | "ease_out_circ" | "ease-out-circ" => {
                Ok(AnimationEasing::EaseOutCirc)
            }
            "EaseOutBack" | "easeOutBack" | "ease_out_back" | "ease-out-back" => {
                Ok(AnimationEasing::EaseOutBack)
            }

            // EaseInOut variants
            "EaseInOut" | "easeInOut" | "ease_in_out" | "ease-in-out" => {
                Ok(AnimationEasing::EaseInOut)
            }
            "EaseInOutSine" | "easeInOutSine" | "ease_in_out_sine" | "ease-in-out-sine" => {
                Ok(AnimationEasing::EaseInOutSine)
            }
            "EaseInOutQuad" | "easeInOutQuad" | "ease_in_out_quad" | "ease-in-out-quad" => {
                Ok(AnimationEasing::EaseInOutQuad)
            }
            "EaseInOutCubic" | "easeInOutCubic" | "ease_in_out_cubic" | "ease-in-out-cubic" => {
                Ok(AnimationEasing::EaseInOutCubic)
            }
            "EaseInOutQuart" | "easeInOutQuart" | "ease_in_out_quart" | "ease-in-out-quart" => {
                Ok(AnimationEasing::EaseInOutQuart)
            }
            "EaseInOutQuint" | "easeInOutQuint" | "ease_in_out_quint" | "ease-in-out-quint" => {
                Ok(AnimationEasing::EaseInOutQuint)
            }
            "EaseInOutExpo" | "easeInOutExpo" | "ease_in_out_expo" | "ease-in-out-expo" => {
                Ok(AnimationEasing::EaseInOutExpo)
            }
            "EaseInOutCirc" | "easeInOutCirc" | "ease_in_out_circ" | "ease-in-out-circ" => {
                Ok(AnimationEasing::EaseInOutCirc)
            }
            "EaseInOutBack" | "easeInOutBack" | "ease_in_out_back" | "ease-in-out-back" => {
                Ok(AnimationEasing::EaseInOutBack)
            }

            // Cubic-bezier parsing
            _ if input.starts_with("cubic-bezier")
                || input.starts_with("CubicBezier")
                || input.starts_with("cubicBezier")
                || input.starts_with("cubic_bezier") =>
            {
                parse_cubic_bezier(input)
                    .map(AnimationEasing::CubicBezier)
                    .ok_or_else(|| format!("Invalid cubic-bezier format: {}", input))
            }

            // Default case for invalid input
            _ => Err(format!("Invalid easing type: {}", input)),
        }
    }
}

impl AnimationEasing {
    /// Converts the easing to a corresponding array of points.
    /// Linear and named easing variants will return predefined control points,
    /// while CubicBezier returns its own array.
    pub fn to_points(&self) -> [f32; 4] {
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
}
