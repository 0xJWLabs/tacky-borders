use serde::Deserialize;
use std::f32::consts::PI;
use windows::Win32::Graphics::Direct2D::Common::D2D1_GRADIENT_STOP;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GradientDirection {
    Direction(String),
    Coordinates(GradientCoordinates),
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientCoordinates {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradientConfig {
    pub colors: Vec<String>,
    pub direction: GradientDirection,
}

#[derive(Debug, Clone)]
pub struct Gradient {
    pub direction: GradientCoordinates,
    pub gradient_stops: Vec<D2D1_GRADIENT_STOP>,
}

#[derive(Debug)]
pub struct Line {
    m: f32,
    b: f32,
}

impl Line {
    pub fn plug_in_x(&self, x: f32) -> f32 {
        self.m * x + self.b
    }
}

impl From<&String> for GradientCoordinates {
    fn from(value: &String) -> Self {
        if let Some(degree) = value
            .strip_suffix("deg")
            .and_then(|d| d.trim().parse::<f32>().ok())
        {
            // let rad = (degree - 90.0) * PI / 180.0;

            let rad = -degree * PI / 180.0;

            let m = match degree.abs() % 360.0 {
                90.0 | 270.0 => degree.signum() * f32::MAX,
                _ => rad.sin() / rad.cos(),
            };

            let b = -m * 0.5 + 0.5;

            let line = Line { m, b };

            let (x_s, x_e) = match degree.abs() % 360.0 {
                0.0..90.0 => (0.0, 1.0),
                90.0..270.0 => (1.0, 0.0),
                270.0..360.0 => (0.0, 1.0),
                _ => {
                    debug!("Reached a gradient angle that is not covered by the match statement in colors.rs");
                    (0.0, 1.0)
                }
            };

            // I can't be bothered to explain this math lol. Basically we're just
            // checking the x and y-intercepts and seeing which one fits
            let start = match line.plug_in_x(x_s) {
                0.0..=1.0 => [x_s, line.plug_in_x(x_s)],
                1.0.. => [(1.0 - line.b) / line.m, 1.0],
                _ => [-line.b / line.m, 0.0],
            };

            let end = match line.plug_in_x(x_e) {
                0.0..=1.0 => [x_e, line.plug_in_x(x_e)],
                1.0.. => [(1.0 - line.b) / line.m, 1.0],
                _ => [-line.b / line.m, 0.0],
            };

            // Adjusting calculations based on the origin being (0.5, 0.5)
            return GradientCoordinates { start, end };
        }

        match value.as_str() {
            "to right" => GradientCoordinates {
                start: [0.0, 0.5],
                end: [1.0, 0.5],
            },
            "to left" => GradientCoordinates {
                start: [1.0, 0.5],
                end: [0.0, 0.5],
            },
            "to top" => GradientCoordinates {
                start: [0.5, 1.0],
                end: [0.5, 0.0],
            },
            "to bottom" => GradientCoordinates {
                start: [0.5, 0.0],
                end: [0.5, 1.0],
            },
            "to top right" => GradientCoordinates {
                start: [0.0, 1.0],
                end: [1.0, 0.0],
            },
            "to top left" => GradientCoordinates {
                start: [1.0, 1.0],
                end: [0.0, 0.0],
            },
            "to bottom right" => GradientCoordinates {
                start: [0.0, 0.0],
                end: [1.0, 1.0],
            },
            "to bottom left" => GradientCoordinates {
                start: [1.0, 0.0],
                end: [0.0, 1.0],
            },
            _ => GradientCoordinates {
                start: [0.5, 1.0],
                end: [0.5, 0.0],
            },
        }
    }
}
