const SUBDIVISION_PRECISION: f32 = 0.0000001; // Precision for binary subdivision
const SUBDIVISION_MAX_ITERATIONS: u32 = 10; // Maximum number of iterations for binary subdivision

pub enum BezierError {
    InvalidControlPoint,
}

impl std::fmt::Display for BezierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BezierError::InvalidControlPoint => {
                write!(f, "Control points must be in the range [0, 1]")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point(pub f32, pub f32);

impl Point {
    fn lerp(a: Point, b: Point, t: f32) -> Point {
        Point(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    }
}

/// Computes the cubic Bézier curve using De Casteljau's algorithm.
///
/// De Casteljau's algorithm is a recursive method for evaluating Bézier curves
/// at a specific parameter `t`. It works by linearly interpolating between
/// control points at each level until a single point is obtained.
///
/// Parameters:
/// - `t`: The parameter (0 ≤ t ≤ 1) at which to evaluate the curve.
/// - `p0`: The first control point (start of the curve).
/// - `p1`: The second control point (first "pull" control).
/// - `p2`: The third control point (second "pull" control).
/// - `p3`: The fourth control point (end of the curve).
///
/// Returns:
/// - The y-coordinate of the Bézier curve at parameter `t`.
fn de_casteljau(t: f32, p0: Point, p1: Point, p2: Point, p3: Point) -> Point {
    // First level: Linearly interpolate between the control points
    // Compute intermediate points `q0`, `q1`, and `q2` at the first level.
    let q0 = Point::lerp(p0, p1, t);
    let q1 = Point::lerp(p1, p2, t);
    let q2 = Point::lerp(p2, p3, t);

    // Second level: Interpolate between the intermediate points from the first level
    // Compute `r0` and `r1` as the second-level intermediate points.
    let r0 = Point::lerp(q0, q1, t);
    let r1 = Point::lerp(q1, q2, t);

    // Final level: Interpolate between the second-level points to get the final result
    // Compute the final point on the curve corresponding to `t`.
    Point::lerp(r0, r1, t) // Interpolates between r0 and r1 to get the curve's value
}

/// Uses binary subdivision to find the parameter `t` for a given x-coordinate on the Bézier curve.
///
/// # Parameters
/// - `x`: The target x-coordinate.
/// - `p0`, `p1`, `p2`, `p3`: The control points of the Bézier curve.
///
/// # Returns
/// The parameter `t` corresponding to the given x-coordinate.
fn get_t_for_x(x: f32, p0: Point, p1: Point, p2: Point, p3: Point) -> f32 {
    let mut t0 = 0.0;
    let mut t1 = 1.0;
    let mut t = x;

    for _ in 0..SUBDIVISION_MAX_ITERATIONS {
        // Evaluate the Bézier curve at `t` to find its x-coordinate.
        let x_val = de_casteljau(t, p0, p1, p2, p3);
        let error = x - x_val.0;

        // Adjust the range based on the error.
        if error.abs() < SUBDIVISION_PRECISION {
            break;
        }
        if error > 0.0 {
            t0 = t;
        } else {
            t1 = t;
        }
        t = (t0 + t1) / 2.0;
    }

    t
}

pub fn bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Result<impl Fn(f32) -> f32, BezierError> {
    // Ensure control points are within bounds (for x-coordinates of p1 and p2).
    if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) || !(0.0..=1.0).contains(&y1) || !(0.0..=1.0).contains(&y2) {
        return Err(BezierError::InvalidControlPoint);
    }

    Ok(move |x: f32| {
        // Shortcut for linear curves (control points are on the line y = x).
        if x1 == y1 && x2 == y2 {
            return x; // Return the same x for a linear curve (y = x).
        }

        // Handle boundary cases explicitly.
        if x == 0.0 || x == 1.0 {
            return x;
        }

        let p0 = Point(0.0, 0.0);
        let p1 = Point(x1, y1);
        let p2 = Point(x2, y2);
        let p3 = Point(1.0, 1.0);

        // Find the parameter `t` corresponding to the x-coordinate.
        // Once `t` is found, evaluate the Bézier curve for the y-coordinate.
        de_casteljau(get_t_for_x(x, p0, p1, p2, p3), p0, p1, p2, p3).1 // Return the y-coordinate
    })
}