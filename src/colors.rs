use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;
use std::sync::Mutex;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::Common::D2D1_GRADIENT_STOP;
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;

// Constants
const COLOR_PATTERN: &str = r"(?i)#[0-9A-F]{3,8}|rgba?\([0-9]{1,3},\s*[0-9]{1,3},\s*[0-9]{1,3}(?:,\s*[0-9]*(?:\.[0-9]+)?)?\)|accent|transparent";
static COLOR_REGEX: LazyLock<Mutex<Regex>> =
    LazyLock::new(|| Mutex::new(Regex::new(COLOR_PATTERN).unwrap()));

// Enums
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GradientDirection {
    String(String),
    Map(GradientDirectionCoordinates),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColorConfig {
    String(String),
    Map(GradientConfig),
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
pub struct GradientConfig {
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

// Traits
trait ToColor {
    fn to_color(self) -> D2D1_COLOR_F;
}

trait ToDirection {
    fn to_direction(self) -> Vec<f32>;
}

// Impl
impl ToDirection for GradientDirection {
    fn to_direction(self) -> Vec<f32> {
        match self {
            GradientDirection::String(direction) => {
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

                match direction.as_str() {
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
            GradientDirection::Map(gradient_struct) => {
                let start_slice: &[f32] = &gradient_struct.start;
                let end_slice: &[f32] = &gradient_struct.end;

                [start_slice, end_slice].concat()
            }
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

        let color_re = COLOR_REGEX.lock().unwrap();

        // Collect valid colors using regex
        let color_matches: Vec<&str> = color_re
            .captures_iter(&color)
            .filter_map(|cap| cap.get(0).map(|m| m.as_str()))
            .collect();

        drop(color_re);

        if color_matches.len() == 1 {
            return Color::Solid(color_matches[0].to_string().to_color());
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

        let (mut animation, mut direction) = (false, None);
        let colors: Vec<D2D1_COLOR_F> = color_matches
            .iter()
            .map(|&color| color.to_string().to_color())
            .collect();

        for input in remaining_input_arr {
            match input.to_lowercase().as_str() {
                "true" => animation = true,
                "false" => animation = false,
                _ if is_valid_direction(input) && direction.is_none() => {
                    direction = Some(input.to_string())
                }
                _ => {}
            }
        }

        if colors.is_empty() {
            return Color::Gradient(Gradient::default());
        }

        if direction.is_none() {
            direction = Some("to_right".to_string());
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

        let direction = GradientDirection::String(direction.unwrap()).to_direction();

        // Return the GradientColor
        Color::Gradient(Gradient {
            gradient_stops,
            direction: Some(direction),
            animation: Some(animation),
        })
    }
}

impl From<GradientConfig> for Color {
    fn from(color: GradientConfig) -> Self {
        match color.colors.len() {
            0 => Color::Gradient(Gradient::default()),
            1 => Color::Solid(color.colors[0].clone().to_color()),
            _ => {
                let gradient_stops: Vec<_> = color
                    .colors
                    .iter()
                    .enumerate()
                    .map(|(i, hex)| D2D1_GRADIENT_STOP {
                        position: i as f32 / (color.colors.len() - 1) as f32,
                        color: hex.to_string().to_color(),
                    })
                    .collect();

                Color::Gradient(Gradient {
                    gradient_stops,
                    direction: Some(color.direction.to_direction()),
                    animation: color.animation,
                })
            }
        }
    }
}

impl From<Option<&ColorConfig>> for Color {
    fn from(color_definition: Option<&ColorConfig>) -> Self {
        match color_definition {
            Some(color) => match color {
                ColorConfig::String(s) => Color::from(s.clone()),
                ColorConfig::Map(gradient_def) => Color::from(gradient_def.clone()),
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

impl ToColor for u32 {
    fn to_color(self) -> D2D1_COLOR_F {
        let r = ((self & 0x00FF0000) >> 16) as f32 / 255.0;
        let g = ((self & 0x0000FF00) >> 8) as f32 / 255.0;
        let b = (self & 0x000000FF) as f32 / 255.0;

        D2D1_COLOR_F { r, g, b, a: 1.0 }
    }
}

impl ToColor for String {
    fn to_color(self) -> D2D1_COLOR_F {
        if self == "accent" {
            let mut pcr_colorization: u32 = 0;
            let mut pf_opaqueblend: BOOL = FALSE;
            let result =
                unsafe { DwmGetColorizationColor(&mut pcr_colorization, &mut pf_opaqueblend) };

            if result.is_err() {
                error!("Error getting windows accent color!");
                return D2D1_COLOR_F::default();
            }

            return pcr_colorization.to_color();
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
            let rgba = self
                .trim_start_matches("rgb(")
                .trim_start_matches("rgba(")
                .trim_end_matches(')');
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
        }

        D2D1_COLOR_F::default()
    }
}

// Functions
fn is_valid_direction(direction: &str) -> bool {
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