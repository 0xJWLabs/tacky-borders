use regex::Regex;
use serde::Deserialize;
use serde::Deserializer;
use std::fmt;
use windows::{
    Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*, Win32::Graphics::Dwm::*,
};
use log::*;

// Enums
#[derive(Debug, Clone)]
pub enum GradientDirection {
    String(String),
    Struct(GradientDirectionStruct),
}

#[derive(Debug, Clone)]
pub enum RawColor {
    String(String),
    Struct(RawGradient),
}

#[derive(Debug, Clone)]
pub enum Color {
    Solid(D2D1_COLOR_F),
    Gradient(Gradient),
}
// Structs
#[derive(Debug, Clone, Deserialize)]
pub struct GradientDirectionStruct {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawGradient {
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
            GradientDirection::Struct(gradient_struct) => {
                let start_slice: &[f32] = &gradient_struct.start;
                let end_slice: &[f32] = &gradient_struct.end;

                // Combine the slices into a single Vec<f32>
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

impl AsRef<RawGradient> for RawGradient {
    fn as_ref(&self) -> &RawGradient {
        self
    }
}

impl<'de> Deserialize<'de> for GradientDirection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct GradientDirectionVisitor;

        impl<'de> Visitor<'de> for GradientDirectionVisitor {
            type Value = GradientDirection;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a struct with start and end arrays")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(GradientDirection::String(value.to_string()))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut start = None;
                let mut end = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "start" => {
                            if start.is_some() {
                                return Err(de::Error::duplicate_field("start"));
                            }
                            start = Some(map.next_value()?);
                        }
                        "end" => {
                            if end.is_some() {
                                return Err(de::Error::duplicate_field("end"));
                            }
                            end = Some(map.next_value()?);
                        }
                        _ => return Err(de::Error::unknown_field(&key, &["start", "end"])),
                    }
                }

                let start = start.ok_or_else(|| de::Error::missing_field("start"))?;
                let end = end.ok_or_else(|| de::Error::missing_field("end"))?;

                Ok(GradientDirection::Struct(GradientDirectionStruct {
                    start,
                    end,
                }))
            }
        }

        // Attempt to deserialize as either a string or a map
        deserializer.deserialize_any(GradientDirectionVisitor)
    }
}

impl<'de> Deserialize<'de> for RawColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        const FIELDS: &[&str] = &["colors", "direction", "animation"];

        struct ColorConfigVisitor;

        impl<'de> Visitor<'de> for ColorConfigVisitor {
            type Value = RawColor;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a map representing a gradient color")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(RawColor::String(value.to_string()))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut colors = None;
                let mut direction = None;
                let mut animation = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "colors" => {
                            if colors.is_some() {
                                return Err(de::Error::duplicate_field("colors"));
                            }
                            colors = Some(map.next_value()?);
                        }
                        "direction" => {
                            if direction.is_some() {
                                return Err(de::Error::duplicate_field("direction"));
                            }
                            direction = Some(map.next_value()?);
                        }
                        "animation" => {
                            if animation.is_some() {
                                return Err(de::Error::duplicate_field("animation"));
                            }
                            animation = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(&key, FIELDS));
                        }
                    }
                }

                let colors = colors.ok_or_else(|| de::Error::missing_field("colors"))?;
                let direction = direction.ok_or_else(|| de::Error::missing_field("direction"))?;

                Ok(RawColor::Struct(RawGradient {
                    colors,
                    direction,
                    animation,
                }))
            }
        }

        deserializer.deserialize_any(ColorConfigVisitor)
    }
}

// Implement Default for your own MyBrush enum
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

fn create_color(color: &str) -> D2D1_COLOR_F {
    if color == "accent" {
        return get_accent_color();
    }

    if color.starts_with("rgb(") || color.starts_with("rgba(") {
        return get_color_from_rgba(color);
    }

    if color.starts_with("#") {
        return get_color_from_hex(color);
    }

    D2D1_COLOR_F::default()
}

fn is_direction(direction: &str) -> bool {
    let valid_directions = [
        "to right",
        "to left",
        "to top",
        "to bottom",
        "to top right",
        "to top left",
        "to bottom right",
        "to bottom left",
    ];

    valid_directions.contains(&direction)
        || direction
            .strip_suffix("deg")
            .map_or(false, |angle| angle.parse::<i32>().is_ok())
}

fn get_color_from_hex(hex: &str) -> D2D1_COLOR_F {
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

fn get_color_from_rgba(rgba: &str) -> D2D1_COLOR_F {
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

pub fn parse_color_string(color: String) -> Color {
    if color.starts_with("gradient(") && color.ends_with(")") {
        return parse_color_string(
            color
                .strip_prefix("gradient(")
                .unwrap_or(&color)
                .strip_suffix(")")
                .unwrap_or(&color)
                .to_string(),
        );
    }

    let color_re = Regex::new(
        r"(?i)(#(?:[0-9A-F]{3,8})|rgba?\(\d{1,3},\s*\d{1,3},\s*\d{1,3}(?:,\s*\d*(?:\.\d+)?)?\)|accent|transparent)",
    ).unwrap();

    // Collect valid colors using regex
    let colors_vec: Vec<&str> = color_re
        .captures_iter(&color)
        .filter_map(|cap| cap.get(0).map(|m| m.as_str()))
        .collect();

    if colors_vec.len() == 1 {
        return Color::Solid(create_color(colors_vec[0]));
    }

    let last_color = colors_vec.last().unwrap();
    let last_color_end = color.rfind(last_color).unwrap() + last_color.len();
    let rest_of_input = color[last_color_end..].trim_start();

    let rest_of_input_array: Vec<&str> = rest_of_input
        .split(',')
        .filter_map(|s| {
            if !s.trim().is_empty() {
                Some(s.trim())
            } else {
                None
            }
        })
        .collect();

    let (mut animation, mut direction) = (false, None);
    let colors: Vec<D2D1_COLOR_F> = colors_vec.iter().map(|&part| create_color(part)).collect();

    for part in rest_of_input_array {
        match part.to_lowercase().as_str() {
            "true" => animation = true,
            "false" => animation = false,
            _ if is_direction(part) && direction.is_none() => direction = Some(part.to_string()),
            _ => {}
        }
    }

    if direction.is_none() {
        direction = Some("to right".to_string());
    }

    // Handle no colors case
    if colors.is_empty() {
        return Color::Gradient(Gradient::default());
    }

    // Create gradient stops
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

fn parse_color_struct(color: RawGradient) -> Color {
    let num_colors = color.colors.len();
    if num_colors == 0 {
        return Color::Gradient(Gradient::default());
    }

    if num_colors == 1 {
        let color = &color.colors[0];
        return Color::Solid(create_color(color));
    }

    let gradient_stops: Vec<D2D1_GRADIENT_STOP> = color
        .colors
        .into_iter()
        .enumerate()
        .map(|(i, hex)| {
            let position = i as f32 / (num_colors - 1) as f32;
            let color = create_color(hex.as_str());
            D2D1_GRADIENT_STOP { position, color }
        })
        .collect();

    let direction = color.direction;

    Color::Gradient(Gradient {
        direction: Some(direction.to_vec()),
        gradient_stops,
        animation: color.animation,
    })
}

pub fn generate_color(color_config: &Option<RawColor>) -> Color {
    match color_config {
        Some(RawColor::String(color)) => parse_color_string(color.to_string()),
        Some(RawColor::Struct(color)) => parse_color_struct(color.clone()),
        None => parse_color_string("accent".to_string()),
    }
}
