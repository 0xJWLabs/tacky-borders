use regex::Regex;
use serde::Deserialize;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::Direct2D::ID2D1GradientStopCollection;
use windows::Win32::Graphics::Direct2D::ID2D1HwndRenderTarget;
use windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_EXTEND_MODE_CLAMP;
use windows::Win32::Graphics::Direct2D::D2D1_GAMMA_2_2;
use windows::Win32::Graphics::Direct2D::D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES;
use windows::{
    Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*, Win32::Graphics::Dwm::*,
};

use crate::utils::*;
use log::*;

// Constants
const COLOR_PATTERN: &str = r"(?i)#[0-9A-F]{3,8}|rgba?\([0-9]{1,3},\s*[0-9]{1,3},\s*[0-9]{1,3}(?:,\s*[0-9]*(?:\.[0-9]+)?)?\)|accent|transparent";

// Enums
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GradientDirection {
    String(String),
    Map(GradientDirectionCoordinates),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColorDefinition {
    String(String),
    Map(GradientDefinition),
}

#[derive(Debug, Clone)]
pub enum Color {
    Solid(D2D1_COLOR_F),
    Gradient(Gradient),
}
// Structs
#[derive(Debug, Clone, Deserialize)]
pub struct GradientDirectionCoordinates {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientDefinition {
    pub colors: Vec<String>,
    pub direction: GradientDirection,
    pub animation: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Gradient {
    pub direction: Option<Vec<f32>>,
    pub gradient_stops: Vec<D2D1_GRADIENT_STOP>, // Array of gradient stops
    pub animation: Option<bool>,
}

impl GradientDirection {
    pub fn to_vec(&self) -> Vec<f32> {
        match self {
            GradientDirection::String(direction) => Self::parse_direction(direction),
            GradientDirection::Map(gradient_struct) => {
                let start_slice: &[f32] = &gradient_struct.start;
                let end_slice: &[f32] = &gradient_struct.end;

                [start_slice, end_slice].concat()
            }
        }
    }

    fn parse_direction(direction: &str) -> Vec<f32> {
        if let Some(degree) = direction
            .strip_suffix("deg")
            .and_then(|d| d.trim().parse::<f32>().ok())
        {
            let rad = (degree - 90.0) * std::f32::consts::PI / 180.0;
            // let rad = degree * PI / 180.0; // Left to Right
            let (cos, sin) = (rad.cos(), rad.sin());

            // Adjusting calculations based on the origin being (0.5, 0.5)
            return vec![
                0.5 - 0.5 * cos,
                0.5 - 0.5 * sin, // Start point (x1, y1) adjusted
                0.5 + 0.5 * cos,
                0.5 + 0.5 * sin, // End point (x2, y2) adjusted
            ];
        }

        match direction {
            "to right" => vec![0.0, 0.5, 1.0, 0.5],     // Left to right
            "to left" => vec![1.0, 0.5, 0.0, 0.5],      // Right to left
            "to top" => vec![0.5, 1.0, 0.5, 0.0],       // Bottom to top
            "to bottom" => vec![0.5, 0.0, 0.5, 1.0],    // Top to bottom
            "to top right" => vec![0.0, 1.0, 1.0, 0.0], // Bottom-left to top-right
            "to top left" => vec![1.0, 1.0, 0.0, 0.0],  // Bottom-right to top-left
            "to bottom right" => vec![0.0, 0.0, 1.0, 1.0], // Top-left to bottom-right
            "to bottom left" => vec![1.0, 0.0, 0.0, 1.0], // Top-right to bottom-left
            _ => vec![0.5, 1.0, 0.5, 0.0],              // Default to "to top"
        }
    }
}

impl From<String> for Color {
    fn from(color: String) -> Self {
        if color.starts_with("gradient(") && color.ends_with(")") {
            return Color::from(
                color
                    .strip_prefix("gradient(")
                    .unwrap_or(&color)
                    .strip_suffix(")")
                    .unwrap_or(&color)
                    .to_string(),
            );
        }

        let color_re = Regex::new(COLOR_PATTERN).unwrap();

        // Collect valid colors using regex
        let colors_vec: Vec<&str> = color_re
            .captures_iter(&color)
            .filter_map(|cap| cap.get(0).map(|m| m.as_str()))
            .collect();

        if colors_vec.len() == 1 {
            return Color::Solid(get_color(colors_vec[0]));
        }

        let rest_of_input = color
            [color.rfind(colors_vec.last().unwrap()).unwrap() + colors_vec.last().unwrap().len()..]
            .trim_start();

        let rest_of_input_array: Vec<&str> = rest_of_input
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then_some(trimmed)
            })
            .collect();

        let (mut animation, mut direction) = (false, Some("to right".to_string()));
        let colors: Vec<D2D1_COLOR_F> = colors_vec.iter().map(|&part| get_color(part)).collect();

        for part in rest_of_input_array {
            match part.to_lowercase().as_str() {
                "true" => animation = true,
                "false" => animation = false,
                _ if is_direction(part) && direction.is_none() => {
                    direction = Some(part.to_string())
                }
                _ => {}
            }
        }

        if colors.is_empty() {
            return Color::Gradient(Gradient::default());
        }

        let num_colors = colors.len();
        let gradient_stops: Vec<D2D1_GRADIENT_STOP> = colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| D2D1_GRADIENT_STOP {
                position: i as f32 / (num_colors - 1) as f32,
                color,
            })
            .collect();

        // Return the GradientColor
        Color::Gradient(Gradient {
            gradient_stops,
            direction: Some(GradientDirection::String(direction.unwrap()).to_vec()),
            animation: Some(animation),
        })
    }
}

impl From<GradientDefinition> for Color {
    fn from(color: GradientDefinition) -> Self {
        match color.colors.len() {
            0 => Color::Gradient(Gradient::default()),
            1 => Color::Solid(get_color(&color.colors[0])),
            _ => {
                let gradient_stops: Vec<_> = color
                    .colors
                    .iter()
                    .enumerate()
                    .map(|(i, hex)| D2D1_GRADIENT_STOP {
                        position: i as f32 / (color.colors.len() - 1) as f32,
                        color: get_color(hex),
                    })
                    .collect();

                Color::Gradient(Gradient {
                    gradient_stops,
                    direction: Some(color.direction.to_vec()),
                    animation: color.animation,
                })
            }
        }
    }
}

impl From<Option<&ColorDefinition>> for Color {
    fn from(color_definition: Option<&ColorDefinition>) -> Self {
        match color_definition {
            Some(color) => match color {
                ColorDefinition::String(s) => Color::from(s.clone()),
                ColorDefinition::Map(gradient_def) => Color::from(gradient_def.clone()),
            },
            None => Color::default(), // Return a default color when None is provided
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::Solid(D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Gradient {
            direction: None,
            gradient_stops: vec![
                D2D1_GRADIENT_STOP {
                    position: 0.0,
                    color: D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                },
                D2D1_GRADIENT_STOP {
                    position: 1.0,
                    color: D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
            ],
            animation: Some(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Brush {
    pub render_target: ID2D1HwndRenderTarget,
    pub color: Color,
    pub rect: RECT,
    pub brush_properties: D2D1_BRUSH_PROPERTIES,
    pub use_animation: bool,
    pub gradient_angle: Option<f32>,
}

// Functions
fn get_accent_color() -> D2D1_COLOR_F {
    let mut pcr_colorization: u32 = 0;
    let mut pf_opaqueblend: BOOL = FALSE;
    let result = unsafe { DwmGetColorizationColor(&mut pcr_colorization, &mut pf_opaqueblend) };

    if result.is_err() {
        error!("Error getting windows accent color!");
        return D2D1_COLOR_F::default();
    }

    let red = ((pcr_colorization & 0x00FF0000) >> 16) as f32 / 255.0;
    let green = ((pcr_colorization & 0x0000FF00) >> 8) as f32 / 255.0;
    let blue = (pcr_colorization & 0x000000FF) as f32 / 255.0;

    D2D1_COLOR_F {
        r: red,
        g: green,
        b: blue,
        a: 1.0,
    }
}

fn get_color(color: &str) -> D2D1_COLOR_F {
    match color {
        "accent" => get_accent_color(),
        _ if color.starts_with("rgb(") || color.starts_with("rgba(") => parse_rgba_color(color),
        _ if color.starts_with("#") => parse_hex_color(color),
        _ => D2D1_COLOR_F::default(),
    }
}

fn is_direction(direction: &str) -> bool {
    matches!(
        direction,
        "to right"
            | "to left"
            | "to top"
            | "to bottom"
            | "to top right"
            | "to top left"
            | "to bottom right"
            | "to bottom left"
    ) || direction
        .strip_suffix("deg")
        .and_then(|angle| angle.parse::<f32>().ok())
        .is_some()
}

fn parse_hex_color(hex: &str) -> D2D1_COLOR_F {
    // Ensure the hex string starts with '#' and is of the correct length
    if hex.len() != 7 && hex.len() != 9 && hex.len() != 4 && hex.len() != 5 || !hex.starts_with('#')
    {
        error!("{}", format!("Invalid hex color format: {}", hex).as_str());
    }

    // Expand shorthand hex formats (#RGB or #RGBA to #RRGGBB or #RRGGBBAA)
    let expanded_hex = match hex.len() {
        4 => format!(
            "#{}{}{}{}{}{}",
            &hex[1..2],
            &hex[1..2],
            &hex[2..3],
            &hex[2..3],
            &hex[3..4],
            &hex[3..4]
        ),
        5 => format!(
            "#{}{}{}{}{}{}{}{}",
            &hex[1..2],
            &hex[1..2],
            &hex[2..3],
            &hex[2..3],
            &hex[3..4],
            &hex[3..4],
            &hex[4..5],
            &hex[4..5]
        ),
        _ => hex.to_string(),
    };

    // Parse RGB values
    let r = f32::from(u8::from_str_radix(&expanded_hex[1..3], 16).unwrap_or(0)) / 255.0;
    let g = f32::from(u8::from_str_radix(&expanded_hex[3..5], 16).unwrap_or(0)) / 255.0;
    let b = f32::from(u8::from_str_radix(&expanded_hex[5..7], 16).unwrap_or(0)) / 255.0;

    // Parse alpha value if present
    let a = if expanded_hex.len() == 9 {
        f32::from(u8::from_str_radix(&expanded_hex[7..9], 16).unwrap_or(0)) / 255.0
    } else {
        1.0
    };

    D2D1_COLOR_F { r, g, b, a }
}

fn parse_rgba_color(rgba: &str) -> D2D1_COLOR_F {
    let rgba = rgba
        .trim_start_matches("rgb(")
        .trim_start_matches("rgba(")
        .trim_end_matches(')');
    let components: Vec<&str> = rgba.split(',').map(|s| s.trim()).collect();
    if components.len() == 3 || components.len() == 4 {
        let r: f32 = components[0].parse::<u32>().unwrap_or(0) as f32 / 255.0;
        let g: f32 = components[1].parse::<u32>().unwrap_or(0) as f32 / 255.0;
        let b: f32 = components[2].parse::<u32>().unwrap_or(0) as f32 / 255.0;

        let a: f32 = if components.len() == 4 {
            (components[3].parse::<u32>().unwrap_or(0) as f32).clamp(0.0, 1.0)
        } else {
            1.0 // Default alpha value for rgb()
        };

        return D2D1_COLOR_F { r, g, b, a };
    }

    // Return a default color if parsing fails
    D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

pub fn generate_brush(props: Brush) -> Result<ID2D1Brush, std::io::Error> {
    let render_target = &props.render_target;
    let brush_properties = &props.brush_properties;
    match &props.color {
        Color::Solid(color) => {
            let solid_brush =
                unsafe { render_target.CreateSolidColorBrush(color, Some(brush_properties))? };

            Ok(solid_brush.into())
        }
        Color::Gradient(color) => {
            let gradient_stops = color.gradient_stops.clone();
            let gradient_stop_collection: ID2D1GradientStopCollection = unsafe {
                render_target.CreateGradientStopCollection(
                    &gradient_stops,
                    D2D1_GAMMA_2_2,
                    D2D1_EXTEND_MODE_CLAMP,
                )?
            };

            let width = get_rect_width(props.rect) as f32;
            let height = get_rect_height(props.rect) as f32;

            let (start_point, end_point) = if props.use_animation {
                let center_x = width / 2.0;
                let center_y = height / 2.0;
                let radius = (center_x.powi(2) + center_y.powi(2)).sqrt();

                let gradient_angle = props.gradient_angle.unwrap_or(0.0);
                let angle_rad = gradient_angle.to_radians();
                let (sin, cos) = angle_rad.sin_cos();
                (
                    D2D_POINT_2F {
                        x: center_x - radius * cos,
                        y: center_y - radius * sin,
                    },
                    D2D_POINT_2F {
                        x: center_x + radius * cos,
                        y: center_y + radius * sin,
                    },
                )
            } else {
                let (start_x, start_y, end_x, end_y) = match color.direction.clone() {
                    Some(coords) => (
                        coords[0] * width,
                        coords[1] * height,
                        coords[2] * width,
                        coords[3] * height,
                    ),
                    None => (0.0, 0.0, width, height),
                };

                (
                    D2D_POINT_2F {
                        x: start_x,
                        y: start_y,
                    },
                    D2D_POINT_2F { x: end_x, y: end_y },
                )
            };

            let gradient_properties = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                startPoint: start_point,
                endPoint: end_point,
            };

            let gradient_brush = unsafe {
                render_target.CreateLinearGradientBrush(
                    &gradient_properties,
                    Some(brush_properties),
                    Some(&gradient_stop_collection),
                )?
            };

            Ok(gradient_brush.into())
        }
    }
}
