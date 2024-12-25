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
        f.debug_struct("AnimationParameters")
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

    let delta_x = anim_elapsed.as_millis_f32() / anim_params.duration * direction;
    border.animations.progress.spiral += delta_x;

    if !(0.0..=1.0).contains(&border.animations.progress.spiral) {
        border.animations.progress.spiral = border.animations.progress.spiral.rem_euclid(1.0);
    }

    let y_coord = match anim_params.easing_fn.as_ref()(border.animations.progress.spiral) {
        Ok(val) => val,
        Err(err) => {
            error!("could not create bezier easing function: {err}");
            return;
        }
    };

    border.animations.progress.angle = 360.0 * y_coord;

    // Calculate the center point of the window
    let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
    let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

    let transform = Matrix3x2::rotation(
        border.animations.progress.angle,
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
        // Set progress.fade here so we start from 0 opacity for the visible color
        border.animations.progress.fade = match border.is_window_active {
            true => 0.0,
            false => 1.0,
        };

        border.animations.flags.fade_to_visible = true;
    }

    // Determine which direction we should move progress.fade
    let direction = match border.is_window_active {
        true => 1.0,
        false => -1.0,
    };

    let delta_x = anim_elapsed.as_millis_f32() / anim_params.duration * direction;
    border.animations.progress.fade += delta_x;

    if !(0.0..=1.0).contains(&border.animations.progress.fade) {
        let final_opacity = border.animations.progress.fade.clamp(0.0, 1.0);

        border.active_color.set_opacity(final_opacity);
        border.inactive_color.set_opacity(1.0 - final_opacity);

        border.animations.progress.fade = final_opacity;
        border.animations.flags.fade_to_visible = false;
        border.animations.flags.should_fade = false;
        return;
    }

    let y_coord = match anim_params.easing_fn.as_ref()(border.animations.progress.fade) {
        Ok(val) => val,
        Err(err) => {
            error!("could not create bezier easing function: {err}");
            border.animations.flags.should_fade = false;
            return;
        }
    };

    let (new_active_opacity, new_inactive_opacity) = match border.animations.flags.fade_to_visible {
        true => match border.is_window_active {
            true => (y_coord, 0.0),
            false => (0.0, 1.0 - y_coord),
        },
        false => (y_coord, 1.0 - y_coord),
    };

    border.active_color.set_opacity(new_active_opacity);
    border.inactive_color.set_opacity(new_inactive_opacity);
}
