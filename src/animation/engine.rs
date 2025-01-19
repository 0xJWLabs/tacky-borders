use super::AnimationConfig;
use crate::border_manager::Border;
use crate::colors::ColorImpl;
use crate::core::animation::AnimationEasing;
use crate::core::animation::AnimationEasingImpl;
use crate::core::animation::AnimationKind;
use crate::core::value::ValueConversion;
use anyhow::anyhow;
use std::str::FromStr;
use std::time::Duration as StdDuration;
use windows::Foundation::Numerics::Matrix3x2;

#[derive(Clone, PartialEq, Debug)]
pub struct AnimationEngine {
    pub kind: AnimationKind,
    pub duration: f32,
    pub easing: AnimationEasing,
}

impl AnimationEngine {
    const MINIMUM_PROGRESS: f32 = 0.0;
    const MAXIMUM_PROGRESS: f32 = 1.0;

    /// Plays the animation, updating the border state based on elapsed time.
    pub fn play(&self, border: &mut Border, elapsed_time: &StdDuration) {
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
    fn animate_spiral(&self, border: &mut Border, elapsed_time: &StdDuration, reverse: bool) {
        let direction: f32 = if reverse { -1.0 } else { 1.0 };
        let delta_x = elapsed_time.as_millis_f32() / self.duration * direction;
        border.animation_manager.progress.spiral += delta_x;

        if !(Self::MINIMUM_PROGRESS..=Self::MAXIMUM_PROGRESS)
            .contains(&border.animation_manager.progress.spiral)
        {
            border.animation_manager.progress.spiral =
                border.animation_manager.progress.spiral.rem_euclid(1.0);
        }

        let easing_fn = match self.easing.to_fn() {
            Ok(func) => func,
            Err(err) => {
                error!("could not transform easing to function: {err}");
                return;
            }
        };

        let y_coord = match (easing_fn)(border.animation_manager.progress.spiral) {
            Ok(val) => val,
            Err(err) => {
                error!("could not create bezier easing function: {err}");
                return;
            }
        };

        border.animation_manager.progress.angle = 360.0 * y_coord;

        // Calculate the center point of the window
        let center_x = border.window_rect.width() / 2;
        let center_y = border.window_rect.height() / 2;

        let transform = Matrix3x2::rotation(
            border.animation_manager.progress.angle,
            center_x as f32,
            center_y as f32,
        );

        border.active_color.set_transform(&transform);
        border.inactive_color.set_transform(&transform);
    }

    fn animate_fade(&self, border: &mut Border, elapsed_time: &StdDuration) {
        // If both are 0, that means the window has been opened for the first time or has been
        // unminimized. If that is the case, only one of the colors should be visible while fading.
        if border.active_color.get_opacity() == Some(0.0)
            && border.inactive_color.get_opacity() == Some(0.0)
        {
            // Set progress.fade here so we start from 0 opacity for the visible color
            border.animation_manager.progress.fade = if border.is_window_active {
                Self::MINIMUM_PROGRESS
            } else {
                Self::MAXIMUM_PROGRESS
            };
            border.animation_manager.flags.fade_to_visible = true;
        }

        let direction = if border.is_window_active { 1.0 } else { -1.0 };

        let delta_x = elapsed_time.as_millis_f32() / self.duration * direction;
        border.animation_manager.progress.fade += delta_x;

        if !(Self::MINIMUM_PROGRESS..=Self::MAXIMUM_PROGRESS)
            .contains(&border.animation_manager.progress.fade)
        {
            let final_opacity = border
                .animation_manager
                .progress
                .fade
                .clamp(Self::MINIMUM_PROGRESS, Self::MAXIMUM_PROGRESS);

            border.active_color.set_opacity(final_opacity);
            border
                .inactive_color
                .set_opacity(Self::MAXIMUM_PROGRESS - final_opacity);

            border.animation_manager.progress.fade = final_opacity;
            border.animation_manager.flags.fade_to_visible = false;
            border.animation_manager.flags.should_fade = false;
            return;
        }

        let easing_fn = match self.easing.to_fn() {
            Ok(func) => func,
            Err(err) => {
                error!("could not transform easing to function: {err}");
                return;
            }
        };

        let y_coord = match (easing_fn)(border.animation_manager.progress.fade) {
            Ok(val) => val,
            Err(err) => {
                error!("could not create bezier easing function: {err}");
                border.animation_manager.flags.should_fade = false;
                return;
            }
        };

        let (new_active_opacity, new_inactive_opacity) =
            if border.animation_manager.flags.fade_to_visible {
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

impl TryFrom<AnimationConfig> for AnimationEngine {
    type Error = anyhow::Error;
    fn try_from(value: AnimationConfig) -> Result<AnimationEngine, Self::Error> {
        // Try to parse the kind. If it's invalid, return an error.
        let kind = AnimationKind::from_str(value.kind.as_str())
            .map_err(|_| anyhow!("invalid or missing animation kind"))?;

        let default_duration = match kind {
            AnimationKind::Spiral | AnimationKind::ReverseSpiral => 1800.0,
            AnimationKind::Fade => 200.0,
        };

        // Parse easing, using a default value if not provided or invalid.
        let easing = AnimationEasing::from_str(value.easing.clone().unwrap_or_default().as_str())
            .unwrap_or_default();

        let duration = value.duration.as_duration().unwrap_or(default_duration) as f32;

        // Return the constructed Animation struct.
        Ok(AnimationEngine {
            kind,
            duration,
            easing,
        })
    }
}
