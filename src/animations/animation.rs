use super::{easing::AnimationEasing, ANIM_NONE};
use crate::{bezier::bezier, window_border::WindowBorder, windows_api::WindowsApi};
use serde::Deserialize;
use std::time::Duration;
use windows::Foundation::Numerics::Matrix3x2;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Default)]
pub enum AnimationType {
    Spiral,
    Fade,
    ReverseSpiral,
    #[default]
    None,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct Animation {
    #[serde(skip)]
    pub animation_type: AnimationType,
    pub speed: f32,
    pub easing: AnimationEasing,
}

impl Animation {
    pub fn play(&self, border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
        let points = self.easing.to_points();

        match self.animation_type {
            AnimationType::Spiral => {
                animate_spiral(points, border, anim_elapsed, anim_speed, false)
            }
            AnimationType::ReverseSpiral => {
                animate_spiral(points, border, anim_elapsed, anim_speed, true)
            }
            AnimationType::Fade => {
                animate_fade(points, border, anim_elapsed, anim_speed);
            }
            _ => {}
        }
    }
}

fn animate_spiral(
    points: [f32; 4],
    border: &mut WindowBorder,
    anim_elapsed: &Duration,
    anim_speed: f32,
    reverse: bool, // Determines direction of the spiral
) {
    // Update spiral_progress
    let delta_t = anim_elapsed.as_secs_f32() * (anim_speed / 20.0);
    border.animations.spiral_progress += delta_t;

    // Wrap progress between 0 and 1
    if border.animations.spiral_progress >= 1.0 {
        border.animations.spiral_progress -= 1.0;
    }

    // Create the easing function
    let Ok(ease) = bezier(points[0], points[1], points[2], points[3]) else {
        error!("Could not create bezier easing function!");
        return;
    };

    // Calculate the eased curve value
    let curve_value = ease(border.animations.spiral_progress);

    // Adjust the spiral angle based on the direction
    let angle_delta = curve_value * (anim_speed / 20.0);
    if reverse {
        border.spiral_anim_angle -= angle_delta;
        border.spiral_anim_angle %= 360.0; // Wrap the angle within [0, 360)
        if border.spiral_anim_angle < 0.0 {
            border.spiral_anim_angle += 360.0;
        }
    } else {
        border.spiral_anim_angle += angle_delta;
        if border.spiral_anim_angle >= 360.0 {
            border.spiral_anim_angle -= 360.0;
        }
    }

    // Calculate the center of the window
    let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
    let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

    // Apply the rotation transformation
    border.brush_properties.transform =
        Matrix3x2::rotation(border.spiral_anim_angle, center_x as f32, center_y as f32);
}

fn animate_fade(
    points: [f32; 4],
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
