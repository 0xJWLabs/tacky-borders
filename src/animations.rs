use crate::bezier::bezier;
use crate::deserializer::from_str;
use crate::window_border::WindowBorder;
use crate::windows_api::WindowsApi;
use regex::Regex;
use serde::Deserialize;
use serde::Deserializer;
use serde_yml::Value;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::str::FromStr;
use std::time::Duration;
use windows::Foundation::Numerics::Matrix3x2;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE: i32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub enum AnimationEasing {
    Linear,
    EaseInOut,
    EaseIn,
    EaseOut,
    CubicBezier([f32; 4]),
}

impl FromStr for AnimationEasing {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Ok(easing) = from_str::<AnimationEasing>(input) {
            return Ok(easing);
        }

        match input {
            "linear" => Ok(AnimationEasing::Linear),
            "ease-in" => Ok(AnimationEasing::EaseIn),
            "ease-out" => Ok(AnimationEasing::EaseOut),
            "ease-in-out" => Ok(AnimationEasing::EaseInOut),
            _ if input.starts_with("cubic-bezier") => {
                parse_cubic_bezier(input).map(AnimationEasing::CubicBezier).ok_or_else(|| {
                  format!("Invalid cubic-bezier format: {}", input) 
                })
            },
            _ => Err(format!("Invalid easing type: {}", input))
        }
    }
}

impl Hash for AnimationEasing {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            AnimationEasing::Linear => 0.hash(state),
            AnimationEasing::EaseInOut => 1.hash(state),
            AnimationEasing::EaseIn => 2.hash(state),
            AnimationEasing::EaseOut => 3.hash(state),
            AnimationEasing::CubicBezier(bezier) => {
                // Hash the individual elements of the CubicBezier array
                for &value in bezier.iter() {
                    value.to_bits().hash(state); // Convert f32 to bits for consistent hashing
                }
            }
        }
    }
}

impl PartialEq for AnimationEasing {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AnimationEasing::Linear, AnimationEasing::Linear)
            | (AnimationEasing::EaseInOut, AnimationEasing::EaseInOut)
            | (AnimationEasing::EaseIn, AnimationEasing::EaseIn)
            | (AnimationEasing::EaseOut, AnimationEasing::EaseOut) => true,
            (AnimationEasing::CubicBezier(a), AnimationEasing::CubicBezier(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for AnimationEasing {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum AnimationType {
    Spiral,
    Fade,
    ReverseSpiral,
    None,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Animation {
    #[serde(skip, default = "default_animation_type")]
    pub animation_type: AnimationType,
    pub speed: f32,
    pub easing_points: [f32; 4],
}

#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default)]
    pub active: HashMap<AnimationType, Animation>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: HashMap<AnimationType, Animation>,
    #[serde(skip)]
    pub current: HashMap<AnimationType, Animation>,
    #[serde(default = "default_fps")]
    pub fps: i32,
    #[serde(skip)]
    pub fade_progress: f32,
    #[serde(skip)]
    pub fade_to_visible: bool,
    #[serde(skip)]
    pub spiral_progress: f32,
    #[serde(skip)]
    pub spiral_angle: f32,
}

fn default_animation_type() -> AnimationType {
    AnimationType::None
}

fn default_fps() -> i32 {
    60
}

pub fn parse_cubic_bezier(input: &str) -> Option<[f32; 4]> {
    let re = Regex::new(r"^cubic-bezier\(([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+),\s*([-+]?[0-9]*\.?[0-9]+)\)$").unwrap();
    if let Some(caps) = re.captures(input) {
        let x1 = caps[1].parse::<f32>().ok()?;
        let y1 = caps[2].parse::<f32>().ok()?;
        let x2 = caps[3].parse::<f32>().ok()?;
        let y2 = caps[4].parse::<f32>().ok()?;
        return Some([x1, y1, x2, y2]);
    }
    None
}

pub fn animation<'de, D>(deserializer: D) -> Result<HashMap<AnimationType, Animation>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Option<HashMap<String, Value>> = Option::deserialize(deserializer)?;

    let mut result: HashMap<AnimationType, Animation> = HashMap::new();

    if let Some(entries) = map {
        for (anim_type_str, anim_value) in entries {
            // Convert the string-based animation type to an enum
            let animation_type: Result<AnimationType, _> = from_str(&anim_type_str);

            if let Ok(animation_type) = animation_type {
                if animation_type == AnimationType::None {
                    continue;
                }

                // Deserialize the remaining fields of `speed` and `easing`
                if let Value::Mapping(ref obj) = anim_value {
                    // Manually extract `speed` and provide a default if missing
                    let speed = match obj.get("speed") {
                        Some(Value::Number(n)) => n.as_f64().map(|f| f as f32),
                        _ => None, // No speed provided
                    };

                    let easing = match obj.get("easing") {
                        Some(Value::String(s)) => {
                            match AnimationEasing::from_str(&s) {
                                Ok(animation) => animation,
                                Err(_) => AnimationEasing::Linear
                            } 
                        }
                        _ => AnimationEasing::Linear,
                    };

                    let easing_points: [f32; 4] = match easing {
                        AnimationEasing::Linear => [0.0, 0.0, 1.0, 1.0],
                        AnimationEasing::EaseIn => [0.42, 0.0, 1.0, 1.0],
                        AnimationEasing::EaseOut => [0.0, 0.0, 0.58, 1.0],
                        AnimationEasing::EaseInOut => [0.42, 0.0, 0.58, 1.0],
                        AnimationEasing::CubicBezier(points) => points, // Use the cubic-bezier points
                    };

                    // Set the default speed based on the animation type if not provided
                    let default_speed = match animation_type {
                        AnimationType::Spiral
                        | AnimationType::ReverseSpiral
                        | AnimationType::Fade => 50.0,
                        _ => 0.0, // Default fallback for other types
                    };

                    // Create the animation object
                    let animation = Animation {
                        animation_type: animation_type.clone(),
                        speed: speed.unwrap_or(default_speed), // Use the default speed if not specified
                        easing_points,
                    };

                    // Insert the animation into the result map
                    result.insert(animation_type, animation);
                }
            } else {
                // Handle invalid animation type, log or return an error
                println!("Invalid animation type: {}", anim_type_str);
            }
        }
    }

    Ok(result)
}

pub trait HashMapAnimationExt {
    fn find(&self, animation_type: &AnimationType) -> Option<&Animation>;
    fn has(&self, animation_type: &AnimationType) -> bool;
    fn to_iter(&self) -> impl Iterator<Item = &Animation> + '_;
}

impl HashMapAnimationExt for HashMap<AnimationType, Animation> {
    fn find(&self, animation_type: &AnimationType) -> Option<&Animation> {
        self.get(animation_type)
    }

    fn has(&self, animation_type: &AnimationType) -> bool {
        self.contains_key(animation_type)
    }

    fn to_iter(&self) -> impl Iterator<Item = &Animation> + '_ {
        self.values()
    }
}

impl Animation {
    pub fn play(&self, border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
        let points = self.easing_points;

        match self.animation_type {
            AnimationType::Spiral => {
                let delta_t = anim_elapsed.as_secs_f32() * (anim_speed / 20.0);
                border.animations.spiral_progress += delta_t;

                if border.animations.spiral_progress >= 1.0 {
                    border.animations.spiral_progress -= 1.0;
                }

                let Ok(ease) = bezier(points[0], points[1], points[2], points[3]) else {
                    error!("Could not create bezier easing function!");
                    return;
                };

                let curve_value = ease(border.animations.spiral_progress);

                border.spiral_anim_angle += curve_value * (anim_speed / 20.0);

                if border.spiral_anim_angle >= 360.0 {
                    border.spiral_anim_angle -= 360.0;
                }

                let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
                let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

                border.brush_properties.transform =
                    Matrix3x2::rotation(border.spiral_anim_angle, center_x as f32, center_y as f32);

                let _ = self;
            }
            AnimationType::ReverseSpiral => {
                // Update spiral_progress as before
                let delta_t = anim_elapsed.as_secs_f32() * (anim_speed / 20.0); // Gradual progress update
                border.animations.spiral_progress += delta_t;

                // Wrap progress between 0 and 1
                if border.animations.spiral_progress >= 1.0 {
                    border.animations.spiral_progress -= 1.0;
                }

                let Ok(ease) = bezier(points[0], points[1], points[2], points[3]) else {
                    error!("Could not create bezier easing function!");
                    return;
                };

                let curve_value = ease(border.animations.spiral_progress);

                // Smooth angle update
                border.spiral_anim_angle += curve_value * (anim_speed / 20.0); // Smooth out the angle progression
                border.spiral_anim_angle %= 360.0; // Wrap the angle within [0, 360)

                // Apply this check for negative angles (just to be safe, though not necessary with modulus)
                if border.spiral_anim_angle < 0.0 {
                    border.spiral_anim_angle += 360.0;
                }

                // Calculate the center of the window for rotation
                let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
                let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

                // Apply the rotation transformation
                border.brush_properties.transform =
                    Matrix3x2::rotation(border.spiral_anim_angle, center_x as f32, center_y as f32);

                let _ = self;
            }
            AnimationType::Fade => {
                animate_fade(points, border, anim_elapsed, anim_speed);
                let _ = self;
            }
            _ => {}
        }
    }
}

pub fn animate_fade(
    animation_easing: [f32; 4],
    border: &mut WindowBorder,
    anim_elapsed: &Duration,
    anim_speed: f32,
) {
    if border.active_color.get_opacity() == 0.0 && border.inactive_color.get_opacity() == 0.0 {
        border.animations.fade_progress = match border.is_window_active {
            true => 0.0,
            false => 1.0,
        };
        border.animations.fade_to_visible = true;
    }

    let direction = match border.is_window_active {
        true => 1.0,
        false => -1.0,
    };

    let delta_t = anim_elapsed.as_secs_f32() * anim_speed * direction;
    border.animations.fade_progress += delta_t;

    if !(0.0..=1.0).contains(&border.animations.fade_progress) {
        let final_opacity = border.animations.fade_progress.clamp(0.0, 1.0);

        border.active_color.set_opacity(final_opacity);
        border.inactive_color.set_opacity(1.0 - final_opacity);

        border.animations.fade_progress = final_opacity;
        border.animations.fade_to_visible = false;
        border.event_anim = ANIM_NONE;
        return;
    }

    let points = animation_easing;

    let Ok(ease) = bezier(points[0], points[1], points[2], points[3]) else {
        error!("Could not create bezier easing function!");
        return;
    };

    let y_coord = ease(border.animations.fade_progress);

    let (new_active_opacity, new_inactive_opacity) = match border.animations.fade_to_visible {
        true => match border.is_window_active {
            true => (y_coord, 0.0),
            false => (0.0, 1.0 - y_coord),
        },
        false => (y_coord, 1.0 - y_coord),
    };

    border.active_color.set_opacity(new_active_opacity);
    border.inactive_color.set_opacity(new_inactive_opacity);
}
