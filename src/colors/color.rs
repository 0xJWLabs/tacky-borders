use super::gradient::Gradient;
use super::gradient::GradientConfig;
use super::gradient::GradientCoordinates;
use super::gradient::GradientDirection;
use super::utils::darken;
use super::utils::lighten;
use super::ToColor;
use super::ANSI_COLORS;
use super::COLOR_REGEX;
use super::DARKEN_LIGHTEN_REGEX;
use crate::utils::strip_string;
use crate::windows_api::WindowsApi;
use serde::Deserialize;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::Common::D2D1_GRADIENT_STOP;
use windows::Win32::Graphics::Direct2D::Common::D2D_POINT_2F;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::Direct2D::ID2D1HwndRenderTarget;
use windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_EXTEND_MODE_CLAMP;
use windows::Win32::Graphics::Direct2D::D2D1_GAMMA_2_2;
use windows::Win32::Graphics::Direct2D::D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;

use super::utils::is_valid_direction;

#[derive(Debug, Clone, PartialEq)]
pub struct Solid {
    pub color: D2D1_COLOR_F,
    pub opacity: f32
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColorConfig {
    String(String),
    Mapping(GradientConfig),
}

#[derive(Debug, Clone)]
pub enum Color {
    Solid(Solid),
    Gradient(Gradient),
}

impl Default for Color {
    fn default() -> Self {
        Color::Solid(Solid {
            color: D2D1_COLOR_F::default(),
            opacity: 0.0
        })
    }
}

impl ToColor for String {
    fn to_d2d1_color(self, is_active_color: Option<bool>) -> D2D1_COLOR_F {
        if self == "accent" {
            let mut pcr_colorization: u32 = 0;
            let mut pf_opaqueblend: BOOL = FALSE;
            let result =
                unsafe { DwmGetColorizationColor(&mut pcr_colorization, &mut pf_opaqueblend) };

            if result.is_err() {
                error!("could not retrieve Windows accent color!");
            }

            let r = ((pcr_colorization & 0x00FF0000) >> 16) as f32 / 255.0;
            let g = ((pcr_colorization & 0x0000FF00) >> 8) as f32 / 255.0;
            let b = (pcr_colorization & 0x000000FF) as f32 / 255.0;
            let avg = (r + g + b) / 3.0;

            return match is_active_color {
                Some(true) => D2D1_COLOR_F { r, g, b, a: 1.0 },
                _ => D2D1_COLOR_F {
                    r: avg / 1.5 + r / 10.0,
                    g: avg / 1.5 + g / 10.0,
                    b: avg / 1.5 + b / 10.0,
                    a: 1.0,
                },
            };
        } else if self.starts_with("#") {
            if self.len() != 7 && self.len() != 9 && self.len() != 4 && self.len() != 5 {
                error!("{}", format!("Invalid hex color format: {}", self).as_str());
                return D2D1_COLOR_F::default();
            }

            let hex = match self.len() {
                4 | 5 => format!(
                    "#{}{}{}{}",
                    self.get(1..2).unwrap_or("").repeat(2),
                    self.get(2..3).unwrap_or("").repeat(2),
                    self.get(3..4).unwrap_or("").repeat(2),
                    self.get(4..5).unwrap_or("").repeat(2)
                ),
                _ => self.to_string(),
            };

            // Parse RGB and Alpha
            let (r, g, b, a) = (
                u8::from_str_radix(&hex[1..3], 16).unwrap_or(0) as f32 / 255.0,
                u8::from_str_radix(&hex[3..5], 16).unwrap_or(0) as f32 / 255.0,
                u8::from_str_radix(&hex[5..7], 16).unwrap_or(0) as f32 / 255.0,
                if hex.len() == 9 {
                    u8::from_str_radix(&hex[7..9], 16).unwrap_or(0) as f32 / 255.0
                } else {
                    1.0
                },
            );

            return D2D1_COLOR_F { r, g, b, a };
        } else if self.starts_with("rgb(") || self.starts_with("rgba(") {
            let rgba = strip_string(self, &["rgb(", "rgba("], ')');
            let components: Vec<&str> = rgba.split(',').map(|s| s.trim()).collect();
            if components.len() == 3 || components.len() == 4 {
                let r: f32 = components[0].parse::<u32>().unwrap_or(0) as f32 / 255.0;
                let g: f32 = components[1].parse::<u32>().unwrap_or(0) as f32 / 255.0;
                let b: f32 = components[2].parse::<u32>().unwrap_or(0) as f32 / 255.0;
                let a = components
                    .get(3)
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.0)
                    .clamp(0.0, 1.0);

                return D2D1_COLOR_F { r, g, b, a };
            }

            return D2D1_COLOR_F::default();
        } else if self.starts_with("darken(") || self.starts_with("lighten(") {
            let darken_lighten_re = &DARKEN_LIGHTEN_REGEX;

            if let Some(caps) = darken_lighten_re.captures(self.as_str()) {
                if caps.len() != 4 {
                    return D2D1_COLOR_F::default();
                }
                let dark_or_lighten = &caps[1];
                let color_str = &caps[2];
                let percentage = &caps[3].parse::<f32>().unwrap_or(10.0);
                let color = color_str.to_string().to_d2d1_color(is_active_color);
                let color_res = match dark_or_lighten {
                    "darken" => darken(color, *percentage),
                    "lighten" => lighten(color, *percentage),
                    _ => color,
                };

                return color_res;
            }

            return D2D1_COLOR_F::default();
        } else if let Some(&(_, color_value)) =
            ANSI_COLORS.iter().find(|&&(key, _)| key == self.as_str())
        {
            return color_value;
        }

        D2D1_COLOR_F::default()
    }
}

impl Color {
    fn from_string(color: String, is_active: Option<bool>) -> Self {
        if color.starts_with("gradient(") && color.ends_with(")") {
            return Self::from_string(strip_string(color, &["gradient("], ')'), is_active);
        }

        let color_re = &COLOR_REGEX;

        // Collect valid colors using regex
        let color_matches: Vec<&str> = color_re
            .captures_iter(&color)
            .filter_map(|cap| cap.get(0).map(|m| m.as_str()))
            .collect();

        if color_matches.len() == 1 {
            return Self::Solid(Solid {
                color: color_matches[0].to_string().to_d2d1_color(is_active),
                opacity: 0.0
            });
        }

        let remaining_input = color[color.rfind(color_matches.last().unwrap()).unwrap()
            + color_matches.last().unwrap().len()..]
            .trim_start();

        let remaining_input_arr: Vec<&str> = remaining_input
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then_some(trimmed)
            })
            .collect();

        let direction = remaining_input_arr
            .iter()
            .find(|&&input| is_valid_direction(input))
            .map(|&s| s.to_string())
            .unwrap_or_else(|| "to_right".to_string());
        let colors: Vec<D2D1_COLOR_F> = color_matches
            .iter()
            .map(|&color| color.to_string().to_d2d1_color(is_active))
            .collect();

        let num_colors = colors.len();
        let step = 1.0 / (num_colors - 1) as f32;

        let gradient_stops = colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| D2D1_GRADIENT_STOP {
                position: i as f32 * step,
                color,
            })
            .collect();

        let direction = GradientCoordinates::from(&direction);

        Self::Gradient(Gradient {
            gradient_stops,
            direction,
            opacity: 0.0
        })
    }

    fn from_mapping(color: GradientConfig, is_active: Option<bool>) -> Self {
        match color.colors.len() {
            0 => Color::Solid(Solid {
                color: D2D1_COLOR_F::default(),
                opacity: 0.0
            }),
            1 => Color::Solid(Solid {
                color: color.colors[0].clone().to_d2d1_color(is_active),
                opacity: 0.0
            }),
            _ => {
                let num_colors = color.colors.len();
                let step = 1.0 / (num_colors - 1) as f32;
                let gradient_stops: Vec<D2D1_GRADIENT_STOP> = color
                    .colors
                    .iter()
                    .enumerate()
                    .map(|(i, hex)| D2D1_GRADIENT_STOP {
                        position: i as f32 * step,
                        color: hex.to_string().to_d2d1_color(is_active),
                    })
                    .collect();

                let direction = match color.direction {
                    GradientDirection::Direction(direction) => {
                        GradientCoordinates::from(&direction)
                    }
                    GradientDirection::Coordinates(direction) => direction,
                };

                Color::Gradient(Gradient {
                    gradient_stops,
                    direction,
                    opacity: 0.0
                })
            }
        }
    }

    pub fn from(color_definition: &ColorConfig, is_active: Option<bool>) -> Self {
        match color_definition {
            ColorConfig::String(s) => Self::from_string(s.clone(), is_active),
            ColorConfig::Mapping(gradient_def) => {
                Self::from_mapping(gradient_def.clone(), is_active)
            }
        }
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        match self {
            Color::Gradient(gradient) => gradient.opacity = opacity,
            Color::Solid(solid) => solid.opacity = opacity,
        }
    }

    pub fn get_opacity(&self) -> f32 {
        match self {
            Color::Gradient(gradient) => gradient.opacity,
            Color::Solid(solid) => solid.opacity,
        }
    }

    pub fn to_brush(
        &self,
        render_target: &ID2D1HwndRenderTarget, //&ID2D1HwndRenderTarget,
        window_rect: &RECT,
        brush_properties: &D2D1_BRUSH_PROPERTIES,
    ) -> Option<ID2D1Brush> {
        match self {
            Color::Solid(solid) => unsafe {
                let Ok(brush) =
                    render_target.CreateSolidColorBrush(&solid.color, Some(brush_properties))
                else {
                    return None;
                };

                brush.SetOpacity(solid.opacity);

                Some(brush.into())
            },
            Color::Gradient(gradient) => unsafe {
                let width = WindowsApi::get_rect_width(*window_rect) as f32;
                let height = WindowsApi::get_rect_height(*window_rect) as f32;

                let gradient_properties = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                    startPoint: D2D_POINT_2F {
                        x: gradient.direction.start[0] * width,
                        y: gradient.direction.start[1] * height,
                    },
                    endPoint: D2D_POINT_2F {
                        x: gradient.direction.end[0] * width,
                        y: gradient.direction.end[1] * height,
                    },
                };

                let Ok(gradient_stop_collection) = render_target.CreateGradientStopCollection(
                    &gradient.gradient_stops,
                    D2D1_GAMMA_2_2,
                    D2D1_EXTEND_MODE_CLAMP,
                ) else {
                    // TODO instead of panicking, I should just return a default value
                    panic!("could not create gradient_stop_collection!");
                };

                let Ok(brush) = render_target.CreateLinearGradientBrush(
                    &gradient_properties,
                    Some(brush_properties),
                    &gradient_stop_collection,
                ) else {
                    return None;
                };

                brush.SetOpacity(gradient.opacity);

                Some(brush.into())
            },
        }
    }
}
