use crate::bezier::bezier;
use crate::deserializer::from_str;
use crate::window_border::WindowBorder;
use crate::windows_api::WindowsApi;
use serde::Deserialize;
use serde::Deserializer;
use serde_yml::Value;
use std::collections::HashMap;
use std::time::Duration;
use windows::Foundation::Numerics::Matrix3x2;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum AnimationEasing {
    Linear,
    EaseInOut,
    EaseIn,
    EaseOut,
}

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
    pub easing: AnimationEasing,
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
                    let speed = match obj.get("speed") {
                        Some(Value::Number(n)) => n.as_f64().map(|f| f as f32),
                        _ => None,
                    };

                    let easing = match obj.get("easing") {
                        Some(Value::String(s)) => match s.as_str() {
                            "EaseInOut" => AnimationEasing::EaseInOut,
                            "EaseIn" => AnimationEasing::EaseIn,
                            "EaseOut" => AnimationEasing::EaseOut,
                            _ => AnimationEasing::Linear, // default easing if not specified
                        },
                        _ => AnimationEasing::Linear,
                    };

                    // Set the default speed based on the animation type if not provided
                    let default_speed = match animation_type {
                        AnimationType::Spiral
                        | AnimationType::ReverseSpiral
                        | AnimationType::Fade => 50.0,
                        _ => 0.0,
                    };

                    let animation = Animation {
                        animation_type: animation_type.clone(),
                        speed: speed.unwrap_or(default_speed), // use the default speed if not specified
                        easing,
                    };

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
        match self.animation_type {
            AnimationType::Spiral => {
                let delta_t = anim_elapsed.as_secs_f32() * (anim_speed / 20.0);
                border.animations.spiral_progress += delta_t;

                if border.animations.spiral_progress >= 1.0 {
                    border.animations.spiral_progress -= 1.0;
                }

                let points: Vec<f32> = match self.easing {
                    AnimationEasing::Linear => vec![0.0, 0.0, 1.0, 1.0],
                    AnimationEasing::EaseIn => vec![0.42, 0.0, 1.0, 1.0],
                    AnimationEasing::EaseOut => vec![0.0, 0.0, 0.58, 1.0],
                    AnimationEasing::EaseInOut => vec![0.42, 0.0, 0.58, 1.0],
                };

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

                let points: Vec<f32> = match self.easing {
                    AnimationEasing::Linear => vec![0.0, 0.0, 1.0, 1.0],
                    AnimationEasing::EaseIn => vec![0.42, 0.0, 1.0, 1.0],
                    AnimationEasing::EaseOut => vec![0.0, 0.0, 0.58, 1.0],
                    AnimationEasing::EaseInOut => vec![0.42, 0.0, 0.58, 1.0],
                };

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
                let easing = self.easing.clone();
                animate_fade(easing, border, anim_elapsed, anim_speed);
                let _ = self;
            }
            _ => {}
        }
    }
}

pub fn animate_fade(
    animation_easing: AnimationEasing,
    border: &mut WindowBorder,
    anim_elapsed: &Duration,
    anim_speed: f32,
) {
    let (bottom_color, top_color) = match border.is_window_active {
        true => (&mut border.inactive_color, &mut border.active_color),
        false => (&mut border.active_color, &mut border.inactive_color),
    };

    let top_opacity = top_color.get_opacity();
    let bottom_opacity = bottom_color.get_opacity();

    if border.animations.fade_progress >= 1.0 || top_opacity >= 1.0 {
        top_color.set_opacity(1.0);
        bottom_color.set_opacity(0.0);

        // Reset fade_progress so we can reuse it next time
        border.animations.fade_progress = 0.0;
        border.event_anim = ANIM_NONE;
        return;
    }

    let delta_t = anim_elapsed.as_secs_f32() * anim_speed;

    border.animations.fade_progress += delta_t;

    let points: Vec<f32> = match animation_easing {
        AnimationEasing::Linear => vec![0.0, 0.0, 1.0, 1.0],
        AnimationEasing::EaseIn => vec![0.42, 0.0, 1.0, 1.0],
        AnimationEasing::EaseOut => vec![0.0, 0.0, 0.58, 1.0],
        AnimationEasing::EaseInOut => vec![0.42, 0.0, 0.58, 1.0],
    };

    let Ok(ease) = bezier(points[0], points[1], points[2], points[3]) else {
        error!("Could not create bezier easing function!");
        return;
    };

    let new_top_opacity = ease(border.animations.fade_progress);

    // I do the following because I want this to work when a window is first opened (when only the
    // top color should be visible) without having to write a separate function for it lol.
    let new_bottom_opacity = match bottom_opacity == 0.0 {
        true => 0.0,
        false => 1.0 - new_top_opacity,
    };

    top_color.set_opacity(new_top_opacity);
    bottom_color.set_opacity(new_bottom_opacity);
}
