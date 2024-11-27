const NEWTON_ITERATIONS: u32 = 4; // Number of iterations to run Newton-Raphson method
const NEWTON_MIN_SLOPE: f32 = 0.001; // Minimum slope for switching from Newton's method to binary subdivision
const SUBDIVISION_PRECISION: f32 = 0.0000001; // Precision for binary subdivision
const SUBDIVISION_MAX_ITERATIONS: u32 = 10; // Maximum number of iterations for binary subdivision
const K_SPLINE_TABLE_SIZE: usize = 11; // Number of sample points in the spline table
const K_SAMPLE_STEP_SIZE: f32 = 1.0 / (K_SPLINE_TABLE_SIZE as f32 - 1.0); // Step size for the spline sample

// Calculates the coefficient for the cubic Bezier function's term involving a1 and a2
#[inline]
fn a(a1: f32, a2: f32) -> f32 {
    1.0 - 3.0 * a2 + 3.0 * a1 // The coefficient for the cubic term
}

// Calculates the coefficient for the Bezier function's term involving a1 and a2
#[inline]
fn b(a1: f32, a2: f32) -> f32 {
    3.0 * a2 - 6.0 * a1 // The coefficient for the quadratic term
}

// Calculates the coefficient for the linear term of the Bezier function involving a1
#[inline]
fn c(a1: f32) -> f32 {
    3.0 * a1 // The coefficient for the linear term
}

// Returns y(t) for given t, x1, and x2 (or y1 and y2), which computes the cubic Bezier curve value
fn calc_bezier(t: f32, a1: f32, a2: f32) -> f32 {
    // Evaluate the cubic Bezier function at time t using a1 and a2 as control points
    ((a(a1, a2) * t + b(a1, a2)) * t + c(a1)) * t
}

// Returns the slope (dx/dt) for the Bezier curve at time t, using a1 and a2 as control points
fn get_slope(t: f32, a1: f32, a2: f32) -> f32 {
    // Compute the derivative of the cubic Bezier function at time t
    3.0 * a(a1, a2) * t * t + 2.0 * b(a1, a2) * t + c(a1)
}

// Performs binary subdivision to approximate t for the given x value
#[inline]
fn binary_subdivide(a_x: f32, a_a: f32, a_b: f32, m_x1: f32, m_x2: f32) -> f32 {
    let mut a = a_a; // Start of the search range
    let mut b = a_b; // End of the search range
    let mut i = 0; // Iteration counter

    // Perform binary search to find t for which calc_bezier(t) is close to a_x
    while (b - a).abs() > SUBDIVISION_PRECISION && i < SUBDIVISION_MAX_ITERATIONS {
        let current_t = a + (b - a) / 2.0; // Midpoint of the current range
        let current_x = calc_bezier(current_t, m_x1, m_x2) - a_x; // Evaluate Bezier and compare to a_x

        // Narrow the search range based on the comparison
        if current_x > 0.0 {
            b = current_t; // Move the upper bound closer
        } else {
            a = current_t; // Move the lower bound closer
        }

        i += 1; // Increment iteration count
    }

    a + (b - a) / 2.0 // Return the midpoint of the final range
}

// Applies Newton-Raphson method to approximate t for the given x value
#[inline]
fn newton_raphson_iterate(a_x: f32, a_guess_t: f32, m_x1: f32, m_x2: f32) -> f32 {
    let mut guess_t = a_guess_t; // Initial guess for t
    for _ in 0..NEWTON_ITERATIONS {
        let current_slope = get_slope(guess_t, m_x1, m_x2); // Calculate the slope (dx/dt)
        if current_slope == 0.0 {
            return guess_t; // If slope is zero, return the current guess for t
        }

        let current_x = calc_bezier(guess_t, m_x1, m_x2) - a_x; // Evaluate Bezier and find the difference from a_x
        guess_t -= current_x / current_slope; // Adjust guess using Newton-Raphson update
    }

    guess_t // Return the final t after all iterations
}

fn get_t_for_x(x: f32, m_x1: f32, m_x2: f32) -> f32 {
    // Precompute the sample values for the Bezier curve at different t values
    let sample_values: Vec<f32> = (0..K_SPLINE_TABLE_SIZE)
        .map(|i| calc_bezier(i as f32 * K_SAMPLE_STEP_SIZE, m_x1, m_x2))
        .collect();
    let mut low = 0usize; // Lower bound for binary search
    let mut high = K_SPLINE_TABLE_SIZE - 1; // Upper bound for binary search
    while high - low > 1 {
        let mid = (low + high) / 2;
        if sample_values[mid] < x {
            low = mid; // Narrow down the search range
        } else {
            high = mid;
        }
    }

    // Interpolate the guess for t using the binary search result
    let dist = (x - sample_values[low]) / (sample_values[low + 1] - sample_values[low]);
    let guess_for_t = low as f32 * K_SAMPLE_STEP_SIZE + dist * K_SAMPLE_STEP_SIZE;

    // Compute the initial slope and decide whether to use Newton's method or binary subdivision
    let initial_slope = get_slope(guess_for_t, m_x1, m_x2);
    let t = if initial_slope >= NEWTON_MIN_SLOPE {
        // Use Newton-Raphson iteration if the slope is large enough
        newton_raphson_iterate(x, guess_for_t, m_x1, m_x2)
    } else {
        // Otherwise, fall back to binary subdivision
        binary_subdivide(x, 0.0, 1.0, m_x1, m_x2)
    };
    t
}

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

// Public function to generate a Bezier function from two control points
pub fn bezier(
    m_x1: f32,
    m_y1: f32,
    m_x2: f32,
    m_y2: f32,
) -> Result<impl Fn(f32) -> f32, BezierError> {
    if !(0.0..=1.0).contains(&m_x1) || !(0.0..=1.0).contains(&m_x2) {
        return Err(BezierError::InvalidControlPoint);
    }

    Ok(move |x: f32| {
        if m_x1 == m_y1 && m_x2 == m_y2 {
            return x;
        }
        if x == 0.0 || x == 1.0 {
            return x; // If x is at the boundary, return x itself
        }

        // Return the final value of the Bezier curve for the calculated t
        calc_bezier(get_t_for_x(x, m_x1, m_x2), m_y1, m_y2)
    })
}
