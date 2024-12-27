use crate::border_manager::Border;
use crate::windows_api::WindowsApi;
use serde::Deserialize;
use std::str::FromStr;
use std::time::Duration;
use win_color::ColorImpl;
use windows::Foundation::Numerics::Matrix3x2;

use super::easing::{AnimationEasing, AnimationEasingImpl};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum AnimationKind {
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

#[derive(Clone, PartialEq, Debug)]
pub struct Animation {
    pub kind: AnimationKind,
    pub duration: f32,
    pub easing: AnimationEasing,
}

impl Animation {
    const MINIMUM_PROGRESS: f32 = 0.0;
    const MAXIMUM_PROGRESS: f32 = 1.0;

    /// Plays the animation, updating the border state based on elapsed time.
    pub fn play(&self, border: &mut Border, elapsed_time: &Duration) {
        if self.duration <= 0.0 {
            warn!("animation duration can't be zero or negative.");
            return;
        }
        match self.kind {
            AnimationKind::Spiral | AnimationKind::ReverseSpiral => {
                let reverse = self.kind == AnimationKind::ReverseSpiral;
                self.animate_spiral(border, elapsed_time, reverse);
            }
            AnimationKind::Fade => self.animate_fade(border, elapsed_time),
        }
    }

    /// Animates a spiral effect on the border.
    fn animate_spiral(&self, border: &mut Border, elapsed_time: &Duration, reverse: bool) {
        let direction: f32 = if reverse { -1.0 } else { 1.0 };
        let delta_x = elapsed_time.as_millis_f32() / self.duration * direction;
        border.animations.progress.spiral += delta_x;

        if !(Self::MINIMUM_PROGRESS..=Self::MAXIMUM_PROGRESS)
            .contains(&border.animations.progress.spiral)
        {
            border.animations.progress.spiral = border.animations.progress.spiral.rem_euclid(1.0);
        }

        let easing_fn = match self.easing.to_fn() {
            Ok(func) => func,
            Err(err) => {
                error!("could not transform easing to function: {err}");
                return;
            }
        };

        let y_coord = match (easing_fn)(border.animations.progress.spiral) {
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

    fn animate_fade(&self, border: &mut Border, elapsed_time: &Duration) {
        // If both are 0, that means the window has been opened for the first time or has been
        // unminimized. If that is the case, only one of the colors should be visible while fading.
        if border.active_color.get_opacity() == Some(0.0)
            && border.inactive_color.get_opacity() == Some(0.0)
        {
            // Set progress.fade here so we start from 0 opacity for the visible color
            border.animations.progress.fade = if border.is_window_active {
                Self::MINIMUM_PROGRESS
            } else {
                Self::MAXIMUM_PROGRESS
            };
            border.animations.flags.fade_to_visible = true;
        }

        let direction = if border.is_window_active { 1.0 } else { -1.0 };

        let delta_x = elapsed_time.as_millis_f32() / self.duration * direction;
        border.animations.progress.fade += delta_x;

        if !(Self::MINIMUM_PROGRESS..=Self::MAXIMUM_PROGRESS)
            .contains(&border.animations.progress.fade)
        {
            let final_opacity = border
                .animations
                .progress
                .fade
                .clamp(Self::MINIMUM_PROGRESS, Self::MAXIMUM_PROGRESS);

            border.active_color.set_opacity(final_opacity);
            border
                .inactive_color
                .set_opacity(Self::MAXIMUM_PROGRESS - final_opacity);

            border.animations.progress.fade = final_opacity;
            border.animations.flags.fade_to_visible = false;
            border.animations.flags.should_fade = false;
            return;
        }

        let easing_fn = match self.easing.to_fn() {
            Ok(func) => func,
            Err(err) => {
                error!("could not transform easing to function: {err}");
                return;
            }
        };

        let y_coord = match (easing_fn)(border.animations.progress.fade) {
            Ok(val) => val,
            Err(err) => {
                error!("could not create bezier easing function: {err}");
                border.animations.flags.should_fade = false;
                return;
            }
        };

        let (new_active_opacity, new_inactive_opacity) = if border.animations.flags.fade_to_visible
        {
            if border.is_window_active {
                (y_coord, Self::MINIMUM_PROGRESS)
            } else {
                (Self::MINIMUM_PROGRESS, Self::MAXIMUM_PROGRESS - y_coord)
            }
        } else {
            (y_coord, Self::MAXIMUM_PROGRESS - y_coord)
        };

        border.active_color.set_opacity(new_active_opacity);
        border.inactive_color.set_opacity(new_inactive_opacity);
    }
}
