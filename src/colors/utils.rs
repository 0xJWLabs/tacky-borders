use core::f32;

use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D1_GRADIENT_STOP};

#[derive(Debug, Clone)]
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

fn interpolate_color_hsl(color1: D2D1_COLOR_F, color2: D2D1_COLOR_F, t: f32) -> D2D1_COLOR_F {
    // Convert colors from D2D1_COLOR_F to HSL
    let hsla1 = d2d1_to_hsla(color1);
    let hsla2 = d2d1_to_hsla(color2);

    // Interpolate each component
    let h = interpolate_hue_normal(hsla1.h, hsla2.h, t);
    let s = hsla1.s + t * (hsla2.s - hsla1.s);
    let l = hsla1.l + t * (hsla2.l - hsla1.l);
    let a = hsla1.a + t * (hsla2.a - hsla1.a);

    // Convert back to D2D1_COLOR_F
    hsla_to_d2d1(Hsla { h, s, l, a })
}

fn interpolate_hue_normal(h1: f32, h2: f32, t: f32) -> f32 {
    // Ensure shortest path for hue interpolation
    let delta = ((h2 - h1 + 360.0) % 360.0).min((h1 - h2 + 360.0) % 360.0);
    if (h2 - h1 + 360.0) % 360.0 == delta {
        (h1 + t * delta) % 360.0
    } else {
        (h1 - t * delta + 360.0) % 360.0
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

    for position in (0..target_count).map(|i| i as f32 * step) {
        let (prev_stop, next_stop) = match source_stops.binary_search_by(|stop| {
            stop.position
                .partial_cmp(&position)
                .expect("comparison doesn't work")
        }) {
            Ok(idx) => {
                let stop = &source_stops[idx];
                (stop, stop)
            }
            Err(idx) => {
                if idx == 0 {
                    (&source_stops[0], &source_stops[0])
                } else if idx >= source_stops.len() {
                    let last = source_stops.last().unwrap();
                    (last, last)
                } else {
                    (&source_stops[idx - 1], &source_stops[idx])
                }
            }
        };

        let t = if (next_stop.position - prev_stop.position).abs() <= f32::EPSILON {
            0.0
        } else {
            (position - prev_stop.position) / (next_stop.position - prev_stop.position)
        };

        let color = interpolate_color_hsl(prev_stop.color, next_stop.color, t);
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
    println!("{}", interpolated.a);

    if (interpolated.a - end_color.a) * diff.signum() >= 0.0 {
        *finished = true;
        return *end_color;
    } else {
        *finished = false;
    }

    interpolated
}
