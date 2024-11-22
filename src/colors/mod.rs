use gradient::GradientCoordinates;
use regex::Regex;
use std::sync::LazyLock;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

pub mod color;
pub mod gradient;

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

pub fn is_valid_direction(direction: &str) -> bool {
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

pub fn interpolate_d2d1_colors(
    current_color: &D2D1_COLOR_F,
    start_color: &D2D1_COLOR_F,
    end_color: &D2D1_COLOR_F,
    anim_elapsed: f32,
    anim_speed: f32,
    finished: &mut bool,
) -> D2D1_COLOR_F {
    // D2D1_COLOR_F has the copy trait so we can just do this to create an implicit copy
    let mut interpolated = *current_color;

    let anim_step = anim_elapsed * anim_speed;

    let diff_r = end_color.r - start_color.r;
    let diff_g = end_color.g - start_color.g;
    let diff_b = end_color.b - start_color.b;
    let diff_a = end_color.a - start_color.a;

    interpolated.r += diff_r * anim_step;
    interpolated.g += diff_g * anim_step;
    interpolated.b += diff_b * anim_step;
    interpolated.a += diff_a * anim_step;

    // Check if we have overshot the active_color
    // TODO if I also check the alpha here, then things start to break when opening windows, not
    // sure why. Might be some sort of conflict with interpoalte_d2d1_to_visible().
    if (interpolated.r - end_color.r) * diff_r.signum() >= 0.0
        && (interpolated.g - end_color.g) * diff_g.signum() >= 0.0
        && (interpolated.b - end_color.b) * diff_b.signum() >= 0.0
    {
        *finished = true;
        return *end_color;
    } else {
        *finished = false;
    }

    interpolated
}

pub fn interpolate_d2d1_to_visible(
    current_color: &D2D1_COLOR_F,
    end_color: &D2D1_COLOR_F,
    anim_elapsed: f32,
    anim_speed: f32,
    finished: &mut bool,
) -> D2D1_COLOR_F {
    let mut interpolated = *current_color;

    let anim_step = anim_elapsed * anim_speed;

    // Figure out which direction we should be interpolating
    let diff = end_color.a - interpolated.a;
    match diff.is_sign_positive() {
        true => interpolated.a += anim_step,
        false => interpolated.a -= anim_step,
    }

    if (interpolated.a - end_color.a) * diff.signum() >= 0.0 {
        *finished = true;
        return *end_color;
    } else {
        *finished = false;
    }

    interpolated
}

pub fn interpolate_direction(
    current_direction: &GradientCoordinates,
    start_direction: &GradientCoordinates,
    end_direction: &GradientCoordinates,
    anim_elapsed: f32,
    anim_speed: f32,
) -> GradientCoordinates {
    let mut interpolated = (*current_direction).clone();

    let x_start_step = end_direction.start[0] - start_direction.start[0];
    let y_start_step = end_direction.start[1] - start_direction.start[1];
    let x_end_step = end_direction.end[0] - start_direction.end[0];
    let y_end_step = end_direction.end[1] - start_direction.end[1];

    // Not gonna bother checking if we overshot the direction tbh
    let anim_step = anim_elapsed * anim_speed;
    interpolated.start[0] += x_start_step * anim_step;
    interpolated.start[1] += y_start_step * anim_step;
    interpolated.end[0] += x_end_step * anim_step;
    interpolated.end[1] += y_end_step * anim_step;

    interpolated
}
