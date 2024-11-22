use regex::Regex;
use std::sync::LazyLock;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

pub mod color;
pub mod gradient;
pub mod utils;

const HEX_PATTERN: &str = r"#[0-9A-F]{3,8}";
const RGBA_PATTERN: &str =
    r"rgba?\([0-9]{1,3},\s*[0-9]{1,3},\s*[0-9]{1,3}(?:,\s*[0-9]*(?:\.[0-9]+)?)?\)";
const ACCENT_TRANSPARENT_PATTERN: &str = r"(accent|transparent)";
const DARKEN_LIGHTEN_PATTERN: &str = r"(?:darken|lighten)\(\s*(?:#[0-9A-F]{3,8}|rgba?\([0-9]{1,3},\s*[0-9]{1,3},\s*[0-9]{1,3}(?:,\s*[0-9]*(?:\.[0-9]+)?)?\)|(?:black|red|green|yellow|blue|magenta|cyan|white|bright black|bright red|bright green|bright yellow|bright blue|bright magenta|bright cyan|bright white))\s*,\s*\d+(?:\.\d+)?%\s*\)";
const ANSI_COLOR_PATTERN: &str = r"\b(?:black|red|green|yellow|blue|magenta|cyan|white|bright black|bright red|bright green|bright yellow|bright blue|bright magenta|bright cyan|bright white)\b";
const DARKEN_LIGHTEN_FETCH_PATTERN: &str = r"(?i)(darken|lighten)\(\s*(#[0-9A-Fa-f]{3,8}|rgba?\(\s*\d{1,3},\s*\d{1,3},\s*\d{1,3}(?:,\s*(?:1|0(?:\.\d+)?))?\s*\)|(?:black|red|green|yellow|blue|magenta|cyan|white|bright black|bright red|bright green|bright yellow|bright blue|bright magenta|bright cyan|bright white))\s*,\s*(\d+(?:\.\d+)?)%\s*\)";

const ANSI_COLORS: [(&str, D2D1_COLOR_F); 16] = [
    (
        "black",
        D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    ),
    (
        "red",
        D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    ),
    (
        "green",
        D2D1_COLOR_F {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        },
    ),
    (
        "yellow",
        D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        },
    ),
    (
        "blue",
        D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        },
    ),
    (
        "magenta",
        D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        },
    ),
    (
        "cyan",
        D2D1_COLOR_F {
            r: 0.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
    ),
    (
        "white",
        D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
    ),
    // Bright colors
    (
        "bright_black",
        D2D1_COLOR_F {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
    ),
    (
        "bright_red",
        D2D1_COLOR_F {
            r: 1.0,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
    ),
    (
        "bright_green",
        D2D1_COLOR_F {
            r: 0.5,
            g: 1.0,
            b: 0.5,
            a: 1.0,
        },
    ),
    (
        "bright_yellow",
        D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 0.5,
            a: 1.0,
        },
    ),
    (
        "bright_blue",
        D2D1_COLOR_F {
            r: 0.5,
            g: 0.5,
            b: 1.0,
            a: 1.0,
        },
    ),
    (
        "bright_magenta",
        D2D1_COLOR_F {
            r: 1.0,
            g: 0.5,
            b: 1.0,
            a: 1.0,
        },
    ),
    (
        "bright_cyan",
        D2D1_COLOR_F {
            r: 0.5,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
    ),
    (
        "bright_white",
        D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
    ),
];

pub static COLOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        format!(
            r"(?i){}|{}|{}|{}|{}",
            HEX_PATTERN,
            RGBA_PATTERN,
            ACCENT_TRANSPARENT_PATTERN,
            DARKEN_LIGHTEN_PATTERN,
            ANSI_COLOR_PATTERN
        )
        .as_str(),
    )
    .unwrap()
});
pub static DARKEN_LIGHTEN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(DARKEN_LIGHTEN_FETCH_PATTERN).unwrap());

pub trait ToColor {
    fn to_d2d1_color(self, is_active: Option<bool>) -> D2D1_COLOR_F;
}
