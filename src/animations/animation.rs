use super::ANIM_NONE;
use crate::border_manager::Border;
use crate::windows_api::WindowsApi;
use core::fmt;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use win_color::ColorImpl;
use windows::Foundation::Numerics::Matrix3x2;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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
}

#[derive(Clone)]
pub struct AnimationParameters {
    pub duration: f32,
    pub easing_fn: Arc<dyn Fn(f32) -> Result<f32, simple_bezier_easing::BezierError> + Send + Sync>,
}

impl fmt::Debug for AnimationParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationParams")
            .field("duration", &self.duration)
            .field("easing_fn", &Arc::as_ptr(&self.easing_fn))
            .finish()
    }
}

impl AnimationParameters {
    pub fn play(
        &self,
        animation_type: &AnimationType,
        border: &mut Border,
        anim_elapsed: &Duration,
    ) {
        match animation_type {
            AnimationType::Spiral | AnimationType::ReverseSpiral => {
                let reverse = *animation_type == AnimationType::ReverseSpiral;
                animate_spiral(border, anim_elapsed, self, reverse)
            }
            AnimationType::Fade => {
                animate_fade(border, anim_elapsed, self);
            }
        }
    }
}

fn animate_spiral(
    border: &mut Border,
    anim_elapsed: &Duration,
    anim_params: &AnimationParameters,
    reverse: bool,
) {
    let direction = match reverse {
        true => -1.0,
        false => 1.0,
    };

    let delta_x = anim_elapsed.as_secs_f32() * 1000.0 / anim_params.duration * direction;
    border.animations.spiral_progress += delta_x;

    if !(0.0..=1.0).contains(&border.animations.spiral_progress) {
        border.animations.spiral_progress = border.animations.spiral_progress.rem_euclid(1.0);
    }

    let y_coord = match anim_params.easing_fn.as_ref()(border.animations.spiral_progress) {
        Ok(val) => val,
        Err(err) => {
            error!("could not create bezier easing function: {err}");
            border.animations.event = ANIM_NONE;
            return;
        }
    };

    border.animations.spiral_angle = 360.0 * y_coord;

    // Calculate the center point of the window
    let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
    let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

    let transform = Matrix3x2::rotation(
        border.animations.spiral_angle,
        center_x as f32,
        center_y as f32,
    );

    border.active_color.set_transform(&transform);
    border.inactive_color.set_transform(&transform);
}

fn animate_fade(border: &mut Border, anim_elapsed: &Duration, anim_params: &AnimationParameters) {
    // If both are 0, that means the window has been opened for the first time or has been
    // unminimized. If that is the case, only one of the colors should be visible while fading.
    if border.active_color.get_opacity() == Some(0.0)
        && border.inactive_color.get_opacity() == Some(0.0)
    {
        // Set fade_progress here so we start from 0 opacity for the visible color
        border.animations.fade_progress = match border.is_window_active {
            true => 0.0,
            false => 1.0,
        };

        border.animations.fade_to_visible = true;
    }

    // Determine which direction we should move fade_progress
    let direction = match border.is_window_active {
        true => 1.0,
        false => -1.0,
    };

    let delta_x = anim_elapsed.as_secs_f32() * 1000.0 / anim_params.duration * direction;
    border.animations.fade_progress += delta_x;

    if !(0.0..=1.0).contains(&border.animations.fade_progress) {
        let final_opacity = border.animations.fade_progress.clamp(0.0, 1.0);

        border.active_color.set_opacity(final_opacity);
        border.inactive_color.set_opacity(1.0 - final_opacity);

        border.animations.fade_progress = final_opacity;
        border.animations.fade_to_visible = false;
        border.animations.event = ANIM_NONE;
        return;
    }

    let y_coord = match anim_params.easing_fn.as_ref()(border.animations.fade_progress) {
        Ok(val) => val,
        Err(err) => {
            error!("could not create bezier easing function: {err}");
            border.animations.event = ANIM_NONE;

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
