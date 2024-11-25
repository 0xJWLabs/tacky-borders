use crate::colors::color::Color;
use crate::colors::color::Solid;
use crate::colors::gradient::Gradient;
use crate::colors::utils::adjust_gradient_stops;
use crate::colors::utils::interpolate_d2d1_to_visible;
use crate::deserializer::from_str;
use crate::window_border::WindowBorder;
use crate::windows_api::WindowsApi;
use serde::Deserialize;
use serde::Deserializer;
use serde_yml::Value;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Graphics::Direct2D::Common::D2D1_GRADIENT_STOP;

pub const ANIM_NONE: i32 = 0;
pub const ANIM_FADE_TO_ACTIVE: i32 = 1;
pub const ANIM_FADE_TO_INACTIVE: i32 = 2;
pub const ANIM_FADE_TO_VISIBLE: i32 = 3;

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
    pub active: Vec<Animation>,
    #[serde(deserialize_with = "animation", default)]
    pub inactive: Vec<Animation>,
    #[serde(default = "default_fps")]
    pub fps: i32,
}

fn default_fps() -> i32 {
    60
}

pub fn animation<'de, D>(deserializer: D) -> Result<Vec<Animation>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Option<HashMap<String, Value>> = Option::deserialize(deserializer)?;

    let mut result: Vec<Animation> = Vec::new();

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

                result.add(Animation {
                    animation_type,
                    speed: speed.unwrap_or(default_speed),
                });
            }
        }
    }

    Ok(result)
}

#[allow(unused)]
pub trait VecAnimation {
    fn add(&mut self, animation: Animation);
    fn fetch(&self, animation_type: &AnimationType) -> Option<&Animation>;
    fn remove(&mut self, animation_type: &AnimationType) -> Option<Animation>;
    fn has(&self, animation_type: &AnimationType) -> bool;
}

impl VecAnimation for Vec<Animation> {
    fn add(&mut self, animation: Animation) {
        // Check if the animation type already exists, and replace it if found
        if let Some(existing_index) = self
            .iter()
            .position(|a| a.animation_type == animation.animation_type)
        {
            self[existing_index] = animation;
        } else {
            self.push(animation);
        }
    }

    fn fetch(&self, animation_type: &AnimationType) -> Option<&Animation> {
        self.iter().find(|a| &a.animation_type == animation_type)
    }

    fn remove(&mut self, animation_type: &AnimationType) -> Option<Animation> {
        self.iter()
            .position(|a| &a.animation_type == animation_type)
            .map(|index| self.swap_remove(index))
    }

    fn has(&self, animation_type: &AnimationType) -> bool {
        self.iter().any(|a| &a.animation_type == animation_type)
    }
}

pub fn animate_fade_setup(border: &mut WindowBorder) {
    // Reset last_anim_time here because otherwise, anim_elapsed will be
    // too large due to being paused and interpolation won't work correctly
    border.last_animation_time = Some(Instant::now());

    border.current_color = if border.is_window_active {
        border.active_color.clone()
    } else {
        border.inactive_color.clone()
    };

    // Set the alpha of the current color to 0 so we can animate from invisible to visible
    if let Color::Gradient(mut current_gradient) = border.current_color.clone() {
        let mut gradient_stops: Vec<D2D1_GRADIENT_STOP> = Vec::new();
        for i in 0..current_gradient.gradient_stops.len() {
            current_gradient.gradient_stops[i].color.a = 0.0;
            let color = current_gradient.gradient_stops[i].color;
            let position = current_gradient.gradient_stops[i].position;
            gradient_stops.push(D2D1_GRADIENT_STOP { color, position });
        }

        let direction = current_gradient.direction;

        border.current_color = Color::Gradient(Gradient {
            gradient_stops,
            direction,
        })
    } else if let Color::Solid(mut current_solid) = border.current_color.clone() {
        current_solid.color.a = 0.0;
        let color = current_solid.color;

        border.current_color = Color::Solid(Solid { color });
    }
    border.event_anim = ANIM_FADE_TO_VISIBLE;
}

impl Animation {
    pub fn play(
        &self,
        border: &mut WindowBorder,
        anim_elapsed: Option<&Duration>,
        anim_speed: Option<f32>,
    ) {
        match self.animation_type {
            AnimationType::Spiral => {
                if let (Some(anim_elapsed), Some(anim_speed)) = (anim_elapsed, anim_speed) {
                    if border.spiral_anim_angle >= 360.0 {
                        border.spiral_anim_angle -= 360.0;
                    }
                    border.spiral_anim_angle +=
                        (anim_elapsed.as_secs_f32() * anim_speed).min(359.0);

                    let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
                    let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

                    border.brush_properties.transform = Matrix3x2::rotation(
                        border.spiral_anim_angle,
                        center_x as f32,
                        center_y as f32,
                    );
                }
            }
            AnimationType::ReverseSpiral => {
                if let (Some(anim_elapsed), Some(anim_speed)) = (anim_elapsed, anim_speed) {
                    border.spiral_anim_angle %= 360.0;
                    if border.spiral_anim_angle < 0.0 {
                        border.spiral_anim_angle += 360.0;
                    }
                    border.spiral_anim_angle -=
                        (anim_elapsed.as_secs_f32() * anim_speed).min(359.0);

                    let center_x = WindowsApi::get_rect_width(border.window_rect) / 2;
                    let center_y = WindowsApi::get_rect_height(border.window_rect) / 2;

                    border.brush_properties.transform = Matrix3x2::rotation(
                        border.spiral_anim_angle,
                        center_x as f32,
                        center_y as f32,
                    );
                }
            }
            AnimationType::Fade => match (anim_elapsed, anim_speed) {
                (Some(anim_elapsed), Some(anim_speed)) => {
                    animate_fade_colors(border, anim_elapsed, anim_speed);
                }
                _ => {
                    animate_fade_setup(border);
                }
            },
        }
    }
}

fn animate_fade_colors(border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
    if let Color::Solid(_) = border.active_color {
        if let Color::Solid(_) = border.inactive_color {
            // If both active and inactive color are solids, use interpolate_solids
            interpolate_solids(border, anim_elapsed, anim_speed);
        }
    } else {
        interpolate_gradients(border, anim_elapsed, anim_speed);
    }
}

pub fn interpolate_solids(border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
    //let before = std::time::Instant::now();
    let Color::Solid(current_solid) = border.current_color.clone() else {
        error!("Could not convert current_color for interpolation");
        return;
    };
    let end_solid = match border.is_window_active {
        true => {
            let Color::Solid(active_solid) = border.active_color.clone() else {
                error!("Could not convet active_color for interpolation");
                return;
            };
            active_solid
        }
        false => {
            let Color::Solid(inactive_solid) = border.inactive_color.clone() else {
                error!("Could not convet active_color for interpolation");
                return;
            };
            inactive_solid
        }
    };

    let mut finished = false;
    let color = match border.event_anim {
        ANIM_FADE_TO_VISIBLE | ANIM_FADE_TO_ACTIVE | ANIM_FADE_TO_INACTIVE => {
            interpolate_d2d1_to_visible(
                &current_solid.color,
                &end_solid.color,
                anim_elapsed.as_secs_f32(),
                anim_speed,
                &mut finished,
            )
        }
        _ => return,
    };

    if finished {
        border.event_anim = ANIM_NONE;
    } else {
        border.current_color = Color::Solid(Solid { color });
    }
}

pub fn interpolate_gradients(border: &mut WindowBorder, anim_elapsed: &Duration, anim_speed: f32) {
    //let before = time::Instant::now();
    let current_gradient = match &border.current_color {
        Color::Gradient(gradient) => gradient.clone(),
        Color::Solid(solid) => {
            // If current_color is not a gradient, that means at least one of active or inactive
            // color must be solid, so only one of these if let statements should evaluate true
            let reference_gradient = match (&border.active_color, &border.inactive_color) {
                (Color::Gradient(active), _) => active,
                (_, Color::Gradient(inactive)) => inactive,
                _ => {
                    debug!("an interpolation function failed pattern matching");
                    return;
                }
            };

            // Convert current_color to a gradient
            let mut solid_as_gradient = reference_gradient.clone();
            for stop in &mut solid_as_gradient.gradient_stops {
                stop.color = solid.color;
            }
            solid_as_gradient
        }
    };
    //debug!("time elapsed: {:?}", before.elapsed());

    let target_stops_len = match border.event_anim {
        ANIM_FADE_TO_ACTIVE => border.active_color.gradient_stops_len(),
        ANIM_FADE_TO_INACTIVE => border.inactive_color.gradient_stops_len(),
        _ => current_gradient.gradient_stops.len(),
    };

    let mut gradient_stops: Vec<D2D1_GRADIENT_STOP> = Vec::with_capacity(target_stops_len);

    let mut active_colors: Color = border.active_color.clone();
    let mut inactive_colors: Color = border.inactive_color.clone();
    let mut current_gradient_stops = current_gradient.gradient_stops.clone();

    let mut all_finished = true;

    if target_stops_len != 0 {
        current_gradient_stops = adjust_gradient_stops(current_gradient_stops, target_stops_len);
        active_colors = match active_colors {
            Color::Gradient(gradient) => {
                let gradient_stops =
                    adjust_gradient_stops(gradient.gradient_stops, target_stops_len);
                Color::Gradient(Gradient {
                    gradient_stops,
                    direction: gradient.direction,
                })
            }
            Color::Solid(color) => Color::Solid(color),
        };

        inactive_colors = match inactive_colors {
            Color::Gradient(gradient) => {
                let gradient_stops =
                    adjust_gradient_stops(gradient.gradient_stops, target_stops_len);
                Color::Gradient(Gradient {
                    gradient_stops,
                    direction: gradient.direction,
                })
            }
            Color::Solid(color) => Color::Solid(color),
        };
    };

    println!("Interpolate Gradients: {}", border.event_anim);

    for (i, _) in current_gradient_stops.iter().enumerate() {
        let mut current_finished = false;

        let end_color = match border.is_window_active {
            true => match &active_colors {
                Color::Gradient(gradient) => gradient.gradient_stops[i].color,
                Color::Solid(solid) => solid.color,
            },
            false => match &inactive_colors {
                Color::Gradient(gradient) => gradient.gradient_stops[i].color,
                Color::Solid(solid) => solid.color,
            },
        };

        let color = match border.event_anim {
            ANIM_FADE_TO_VISIBLE | ANIM_FADE_TO_ACTIVE | ANIM_FADE_TO_INACTIVE => {
                interpolate_d2d1_to_visible(
                    &current_gradient_stops[i].color,
                    &end_color,
                    anim_elapsed.as_secs_f32(),
                    anim_speed,
                    &mut current_finished,
                )
            }
            _ => return,
        };

        if !current_finished {
            all_finished = false;
        }

        // TODO currently this works well because users cannot adjust the positions of the
        // gradient stops, so both inactive and active gradients will have the same positions,
        // but this might need to be interpolated if we add position configuration.
        let position = current_gradient_stops[i].position;

        let stop = D2D1_GRADIENT_STOP { color, position };
        gradient_stops.push(stop);
    }

    let direction = current_gradient.direction;

    if all_finished {
        match border.event_anim {
            ANIM_FADE_TO_ACTIVE => border.current_color = border.active_color.clone(),
            ANIM_FADE_TO_INACTIVE => border.current_color = border.inactive_color.clone(),
            ANIM_FADE_TO_VISIBLE => {
                border.current_color = match border.is_window_active {
                    true => border.active_color.clone(),
                    false => border.inactive_color.clone(),
                }
            }
            _ => {}
        }
        border.event_anim = ANIM_NONE;
    } else {
        border.current_color = Color::Gradient(Gradient {
            gradient_stops,
            direction,
        });
    }
}
