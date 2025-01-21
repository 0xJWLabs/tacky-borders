use crate::core::value::Value;
use schema_jsonrs::JsonSchema;
use serde::Deserialize;

pub mod engine;
pub mod manager;
pub mod wrapper;

/// Configuration for animations applied to custom tacky borders on Windows,
/// including both active and inactive states of the borders, as well as the frame rate (FPS).
///
/// This configuration is used to control the animations of custom borders, which can have effects like fading,
/// sliding, or zooming for the border's appearance or transition.
///
/// # Fields:
/// - `active`: An optional list of animations applied to the active border state.
/// - `inactive`: An optional list of animations applied to the inactive border state.
/// - `fps`: An optional frame rate for the animations, in frames per second (FPS).
#[derive(Debug, Deserialize, Clone, Default, PartialEq, JsonSchema)]
pub struct AnimationsConfig {
    /// A list of configurations for animations applied to the active state of the custom window borders.
    /// These animations are triggered when the window is active (focused).
    pub active: Option<Vec<AnimationConfig>>,

    /// A list of configurations for animations applied to the inactive state of the custom window borders.
    /// These animations are triggered when the window is inactive (not focused).
    pub inactive: Option<Vec<AnimationConfig>>,

    /// The frame rate for the animations, specified in frames per second (FPS).
    /// This controls how smoothly the animations are rendered during transitions of custom window borders.
    pub fps: Option<i32>,
}

/// Configuration for a single animation applied to custom window borders, including its type, duration, and easing function.
///
/// # Fields:
/// - `kind`: The type of animation (e.g., "fade", "slide", "zoom") applied to the custom border.
/// - `duration`: The duration of the animation, either as a string (e.g., "100ms") or a number (e.g., 100).
/// - `easing`: The easing function for the animation (e.g., "ease-in", "linear") to control the timing of the transition.
#[derive(Clone, PartialEq, Debug, Deserialize, JsonSchema)]
pub struct AnimationConfig {
    /// The type or kind of animation (e.g., "fade", "spiral", "reverse-spiral") to apply to the custom border.
    /// This defines the visual effect when transitioning the border.
    pub kind: String,

    /// The duration of the animation, specified either as a string (e.g., "100ms") or a number (e.g., 100).
    /// If a string is used, it can include units such as "ms" (milliseconds) or "s" (seconds).
    pub duration: Option<Value>,

    /// The easing function for the animation, specified as a string (e.g., "ease-in", "linear", "cubic-bezier(0.42, 0.0, 0.58, 1.0)").
    /// This defines the pacing of the animation's progress over time.
    pub easing: Option<String>,
}
