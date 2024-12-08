use super::easing::AnimationEasing;
use super::ANIM_NONE;
use crate::window_border::WindowBorder;
use crate::windows_api::WindowsApi;
use serde::Deserialize;
use simple_bezier_easing::bezier;
use std::time::Duration;
use win_color::ColorImpl;
use windows::Foundation::Numerics::Matrix3x2;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum AnimationType {
    #[serde(alias = "spiral")]
    Spiral,
    #[serde(alias = "fade")]
    Fade,
    #[serde(
        alias = "reverse_spiral",
        alias = "reverseSpiral",
        alias = "reverse-spiral"
    )]
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
            AnimationType::Spiral | AnimationType::ReverseSpiral => {
                let anim_speed = if self.animation_type == AnimationType::ReverseSpiral {
                    -anim_speed
                } else {
                    anim_speed
                };
                animate_spiral(points, border, anim_elapsed, anim_speed)
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
) {
    // Update spiral_progress
    let delta_t = anim_elapsed.as_secs_f32() * (anim_speed / 20.0);
    border.animations.spiral_progress =
        (border.animations.spiral_progress + delta_t).rem_euclid(1.0); // Wrap progress to [0, 1)

    // Create the easing function
    let Ok(ease) = bezier(points[0], points[1], points[2], points[3]) else {
        error!("Could not create bezier easing function!");
        return;
    };

    // Calculate the eased curve value
    let curve_value = match ease(border.animations.spiral_progress) {
        Ok(val) => val,
        Err(err) => {
            error!("{err}");
            border.event_anim = ANIM_NONE;
            return;
        }
    };

    // Adjust the spiral angle based on the direction
    let angle_delta = curve_value * (anim_speed / 20.0);
    border.spiral_anim_angle = (border.spiral_anim_angle + angle_delta).rem_euclid(360.0); // Wrap to [0, 360)

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

    let y_coord = match ease(border.animations.fade_progress) {
        Ok(coord) => coord,
        Err(err) => {
            error!("{err}");
            border.event_anim = ANIM_NONE;
            return;
        }
    };

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
