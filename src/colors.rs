use regex::Regex;
use serde::Deserialize;
use serde::Deserializer;
use std::f32::consts::PI;
use windows::{
    Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*, Win32::Graphics::Dwm::*,
};

use crate::logger::Logger;

#[derive(Debug, Clone, Deserialize)]
pub struct GradientDirectionCoordinate {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub enum GradientDirectionPoint {
    Sequence([f32; 2]),
    Coordinate(GradientDirectionCoordinate)
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientDirectionStruct {
    pub start: GradientDirectionPoint,
    pub end: GradientDirectionPoint,
}

#[derive(Debug, Clone, Deserialize)]
pub enum GradientDirection {
    String(String),
    Struct(GradientDirectionStruct),
}

impl GradientDirection {
    pub fn to_vec(&self) -> Vec<f32> {
        match self {
            GradientDirection::String(direction) => Self::parse_direction(direction),
            GradientDirection::Struct(gradient_struct) => [
                Self::extract_point(&gradient_struct.start),
                Self::extract_point(&gradient_struct.end),
            ]
            .concat(),
        }
    }

    fn parse_direction(direction: &str) -> Vec<f32> {
        if let Some(degree) = direction.strip_suffix("deg").and_then(|d| d.trim().parse::<f32>().ok()) {
            let rad = degree * PI / 180.0;
            let (cos, sin) = (rad.cos(), rad.sin());
            return vec![
                0.5 + 0.5 * cos, 0.5 + 0.5 * sin, // Start point (x1, y1)
                0.5 - 0.5 * cos, 0.5 - 0.5 * sin, // End point (x2, y2)
            ];
        }

        match direction {
            "to right" => vec![0.0, 0.5, 1.0, 0.5],
            "to left" => vec![1.0, 0.5, 0.0, 0.5],
            "to top" => vec![0.5, 1.0, 0.5, 0.0],
            "to bottom" => vec![0.5, 0.0, 0.5, 1.0],
            "to top right" => vec![0.5, 1.0, 1.0, 0.0],
            "to top left" => vec![0.0, 1.0, 0.5, 0.0],
            "to bottom right" => vec![0.5, 0.0, 1.0, 1.0],
            "to bottom left" => vec![0.0, 0.0, 0.5, 1.0],
            _ => vec![], // Handle any other unspecified directions
        }
    }

    fn extract_point(point: &GradientDirectionPoint) -> Vec<f32> {
        match point {
            GradientDirectionPoint::Coordinate(coordinate) => vec![coordinate.x, coordinate.y],
            GradientDirectionPoint::Sequence(sequence) => {
                vec![*sequence.get(0).unwrap_or(&0.0), *sequence.get(1).unwrap_or(&0.0)]
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawGradient {
    pub colors: Vec<String>,
    pub direction: Option<GradientDirection>,
    pub animation: Option<bool>,
}

impl AsRef<RawGradient> for RawGradient {
    fn as_ref(&self) -> &RawGradient {
        self
    }
}

#[derive(Debug, Clone)]
pub enum RawColor {
    String(String),
    Struct(RawGradient),
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

#[derive(Debug, Clone)]
pub enum Color {
    Solid(D2D1_COLOR_F),
    Gradient(Gradient),
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

#[derive(Debug, Clone)]
pub struct Gradient {
    pub direction: Option<Vec<f32>>,
    pub gradient_stops: Vec<D2D1_GRADIENT_STOP>, // Array of gradient stops
    pub animation: Option<bool>,
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
        Logger::log("error", "Error getting windows accent color!");
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
    let valid_directions = vec![
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

fn parse_component(s: &str) -> f32 {
    match u8::from_str_radix(s, 16) {
        Ok(val) => f32::from(val) / 255.0,
        Err(_) => {
            Logger::log(
                "error",
                format!("Invalid component '{}' in hex.", s).as_str(),
            );
            0.0
        }
    }
}

pub fn parse_color_string(color: String) -> Color {
    if color.starts_with("gradient(") && color.ends_with(")") {
        let stripped_color = color.strip_prefix("gradient(")
            .and_then(|s| s.strip_suffix(")"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| color.to_string());

        return parse_color_string(stripped_color);
    }
    let color_re = Regex::new(
        r"(?i)(#(?:[0-9A-F]{3,8})|rgba?\(\d{1,3},\s*\d{1,3},\s*\d{1,3}(?:,\s*\d*(?:\.\d+)?)?\))",
    )
    .unwrap();

    // Collect valid colors using regex
    let colors_vec_1: Vec<&str> = color_re
        .captures_iter(&color)
        .filter_map(|cap| cap.get(0).map(|m| m.as_str()))
        .collect();

    // Extract the rest of the input after the last color
    let rest_of_input = if let Some(last_color) = colors_vec_1.last() {
        // Find the last occurrence index of the last color in the original input
        let last_color_end = color.rfind(last_color).unwrap() + last_color.len();
        // Trim leading whitespace and return the rest
        color[last_color_end..].trim_start()
    } else {
        // If no colors were found, return the full input trimmed
        color.trim_start()
    };

    // Split the remaining input into parts
    let rest_of_input_array: Vec<&str> = rest_of_input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty()) // Filter out any empty strings
        .collect();

    // Handle the case with a single color
    if colors_vec_1.len() == 1 {
        return Color::Solid(create_color(colors_vec_1[0]));
    }

    let mut direction = "to right".to_string(); // Default direction
    let mut animation = false; // Default animation to false
    let mut colors: Vec<D2D1_COLOR_F> = Vec::new();

    // Parse color inputs
    for color_part in &colors_vec_1 {
        match *color_part {
            _ if color_part.starts_with("rgb(") || color_part.starts_with("rgba(") => {
                colors.push(get_color_from_rgba(color_part));
            }
            _ if color_part.starts_with("#") => {
                colors.push(get_color_from_hex(color_part));
            }
            "accent" => {
                colors.push(get_accent_color());
            }
            _ => {}
        }
    }

    // Parse the rest of the input array for direction and animation
    for part in &rest_of_input_array {
        if part.eq_ignore_ascii_case("true") {
            animation = true;
        } else if part.eq_ignore_ascii_case("false") {
            animation = false;
        } else if is_direction(part) {
            direction = part.to_string();
        }
    }

    // Handle no colors case
    if colors.is_empty() {
        return Color::Gradient(Gradient::default());
    }

    let num_colors = colors.len();
    let gradient_stops: Vec<D2D1_GRADIENT_STOP> = (0..num_colors)
        .map(|i| D2D1_GRADIENT_STOP {
            position: i as f32 / (num_colors - 1) as f32,
            color: colors[i].clone(),
        })
        .collect();

    // Create a GradientColor if we have colors
    Color::Gradient(Gradient {
        gradient_stops,
        direction: Some(GradientDirection::String(direction).to_vec()),
        animation: Some(animation), // Wrap in Some to indicate optional
    })
}

fn get_color_from_hex(hex: &str) -> D2D1_COLOR_F {
    // Ensure the hex string starts with '#' and is of the correct length
    if hex.len() != 7 && hex.len() != 9 && hex.len() != 4 && hex.len() != 5 || !hex.starts_with('#')
    {
        Logger::log(
            "error",
            format!("Invalid hex color format: {}", hex).as_str(),
        );
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
    let r = parse_component(&expanded_hex[1..3]);
    let g = parse_component(&expanded_hex[3..5]);
    let b = parse_component(&expanded_hex[5..7]);

    // Parse alpha value if present
    let a = if expanded_hex.len() == 9 {
        parse_component(&expanded_hex[7..9])
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
    println!("{:?}", components);
    if components.len() == 3 || components.len() == 4 {
        // Parse red, green, and blue values
        let red: f32 = components[0].parse::<u32>().unwrap_or(0) as f32 / 255.0;
        let green: f32 = components[1].parse::<u32>().unwrap_or(0) as f32 / 255.0;
        let blue: f32 = components[2].parse::<u32>().unwrap_or(0) as f32 / 255.0;

        let alpha: f32 = if components.len() == 4 {
            components[3].parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0)
        } else {
            1.0 // Default alpha value for rgb()
        };

        return D2D1_COLOR_F {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        };
    }

    // Return a default color if parsing fails
    D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

fn parse_color_struct(color: RawGradient) -> Color {
    let num_colors = color.colors.len();
    if num_colors == 0 {
        return Color::Gradient(Gradient::default());
    }

    if num_colors == 1 {
        let color = color.colors.get(0);
        return Color::Solid(create_color(color.unwrap_or(&"accent".to_string())));
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

    let direction = color
        .direction
        .unwrap_or_else(|| GradientDirection::String("to right".to_string()));

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
