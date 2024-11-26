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
pub enum AnimationType {
    Spiral,
    Fade,
    ReverseSpiral,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Animation {
    pub animation_type: AnimationType,
    pub speed: f32,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Default)]
pub struct Animations {
    #[serde(deserialize_with = "animation", default)]
    pub active: HashMap<AnimationType, f32>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: HashMap<AnimationType, f32>,
    #[serde(default = "default_fps")]
    pub fps: i32,
}

fn default_fps() -> i32 {
    60
}

pub fn animation<'de, D>(deserializer: D) -> Result<HashMap<AnimationType, f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Option<HashMap<String, Value>> = Option::deserialize(deserializer)?;

    let mut result: HashMap<AnimationType, f32> = HashMap::new();

    if let Some(entries) = map {
        for (anim_type, anim_value) in entries {
            let animation_type: Result<AnimationType, _> = from_str(&anim_type);

            if let Ok(animation_type) = animation_type {
                let speed = match anim_value {
                    Value::Number(n) => n.as_f64().map(|f| f as f32),
                    _ => None,
                };

                let default_speed = match animation_type {
                    AnimationType::Spiral | AnimationType::ReverseSpiral => 100.0,
                    AnimationType::Fade => 200.0,
                };

                result.insert(animation_type, speed.unwrap_or(default_speed));
            }
        }
    }

    Ok(result)
}

pub trait HashMapAnimationExt {
    fn find(&self, animation_type: &AnimationType) -> Option<Animation>;
    fn has(&self, animation_type: &AnimationType) -> bool;
    fn to_iter(&self) -> impl Iterator<Item = Animation> + '_;
}

impl HashMapAnimationExt for HashMap<AnimationType, f32> {
    fn find(&self, animation_type: &AnimationType) -> Option<Animation> {
        self.get(animation_type).map(|&speed| Animation {
            animation_type: animation_type.clone(),
            speed,
        })
    }

    fn has(&self, animation_type: &AnimationType) -> bool {
        self.contains_key(animation_type)
    }

    fn to_iter(&self) -> impl Iterator<Item = Animation> + '_ {
        self.iter().map(|(animation_type, &speed)| Animation {
            animation_type: animation_type.clone(),
            speed,
        })
    }
}

impl Animation {
    pub fn play(&self, border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
        match self.animation_type {
            AnimationType::Spiral => {
                if border.spiral_anim_angle >= 360.0 {
                    border.spiral_anim_angle -= 360.0;
                }
                border.spiral_anim_angle += (anim_elapsed.as_secs_f32() * anim_speed).min(359.0);

                let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
                let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

                border.brush_properties.transform =
                    Matrix3x2::rotation(border.spiral_anim_angle, center_x as f32, center_y as f32);
            }
            AnimationType::ReverseSpiral => {
                border.spiral_anim_angle %= 360.0;
                if border.spiral_anim_angle < 0.0 {
                    border.spiral_anim_angle += 360.0;
                }
                border.spiral_anim_angle -= (anim_elapsed.as_secs_f32() * anim_speed).min(359.0);

                let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
                let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

                border.brush_properties.transform =
                    Matrix3x2::rotation(border.spiral_anim_angle, center_x as f32, center_y as f32);
            }
            AnimationType::Fade => {
                animate_fade(border, anim_elapsed, anim_speed);
            }
        }
    }
}

fn animate_fade(border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
    let (bottom_color, top_color) = match border.is_window_active {
        true => (&mut border.inactive_color, &mut border.active_color),
        false => (&mut border.active_color, &mut border.inactive_color),
    };

    let top_opacity = top_color.get_opacity();
    let bottom_opacity = bottom_color.get_opacity();

    let anim_step = anim_elapsed.as_secs_f32() * anim_speed * (0.75 + (top_opacity / 4.0));

    let mut new_top_opacity = top_opacity + anim_step;
    let mut new_bottom_opacity = bottom_opacity - anim_step;

    if new_top_opacity >= 1.0 {
        new_top_opacity = 1.0;
        new_bottom_opacity = 0.0;
        border.event_anim = ANIM_NONE;
    }

    top_color.set_opacity(new_top_opacity);
    bottom_color.set_opacity(new_bottom_opacity);
}
