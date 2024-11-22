use super::gradient::GradientCoordinates;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D1_GRADIENT_STOP};

pub struct Hsla {
    h: f32,
    s: f32,
    l: f32,
    a: f32,
}

pub fn d2d1_to_hsla(color: D2D1_COLOR_F) -> Hsla {
    let r = color.r;
    let g = color.g;
    let b = color.b;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut h = 0.0;
    let mut s = 0.0;
    let mut l = (max + min) / 2.0;

    if delta != 0.0 {
        if max == r {
            h = (g - b) / delta;
        } else if max == g {
            h = (b - r) / delta + 2.0;
        } else {
            h = (r - g) / delta + 4.0;
        }

        s = if l == 0.0 || l == 1.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }

    s *= 100.0;
    l *= 100.0;

    Hsla {
        h,
        s,
        l,
        a: color.a,
    }
}

pub fn hsla_to_d2d1(hsla: Hsla) -> D2D1_COLOR_F {
    let s = hsla.s / 100.0;
    let l = hsla.l / 100.0;
    let h = hsla.h;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    D2D1_COLOR_F {
        r: (r + m).clamp(0.0, 1.0),
        g: (g + m).clamp(0.0, 1.0),
        b: (b + m).clamp(0.0, 1.0),
        a: hsla.a,
    }
}

pub fn darken(color: D2D1_COLOR_F, percentage: f32) -> D2D1_COLOR_F {
    let mut hsla = d2d1_to_hsla(color);
    hsla.l -= hsla.l * percentage / 100.0;
    hsla_to_d2d1(hsla)
}

pub fn lighten(color: D2D1_COLOR_F, percentage: f32) -> D2D1_COLOR_F {
    let mut hsla = d2d1_to_hsla(color);
    hsla.l += hsla.l * percentage / 100.0;
    hsla_to_d2d1(hsla)
}

fn interpolate_color(color1: D2D1_COLOR_F, color2: D2D1_COLOR_F, t: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: color1.r + t * (color2.r - color1.r),
        g: color1.g + t * (color2.g - color1.g),
        b: color1.b + t * (color2.b - color1.b),
        a: color1.a + t * (color2.a - color1.a),
    }
}

pub fn adjust_gradient_stops(
    source_stops: Vec<D2D1_GRADIENT_STOP>,
    target_count: usize,
) -> Vec<D2D1_GRADIENT_STOP> {
    if source_stops.len() == target_count {
        return source_stops;
    }

    let mut adjusted_stops = Vec::with_capacity(target_count);
    let step = 1.0 / (target_count - 1).max(1) as f32;

    for i in 0..target_count {
        let position = i as f32 * step;
        let (prev_stop, next_stop) = match source_stops
            .windows(2)
            .find(|w| w[0].position <= position && position <= w[1].position)
        {
            Some(pair) => (&pair[0], &pair[1]),
            None => {
                if position <= source_stops[0].position {
                    (&source_stops[0], &source_stops[0])
                } else {
                    let last = source_stops.last().unwrap();
                    (last, last)
                }
            }
        };

        let t = if prev_stop.position == next_stop.position {
            0.0
        } else {
            (position - prev_stop.position) / (next_stop.position - prev_stop.position)
        };

        let color = interpolate_color(prev_stop.color, next_stop.color, t);
        adjusted_stops.push(D2D1_GRADIENT_STOP { color, position });
    }

    adjusted_stops
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
