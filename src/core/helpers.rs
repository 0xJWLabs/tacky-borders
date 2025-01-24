// Helpers
pub fn type_name_of_val<T: ?Sized>(_val: &T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn serde_default_u32<const V: u32>() -> u32 {
    V
}

pub fn serde_default_i32<const V: i32>() -> i32 {
    V
}

pub fn serde_default_f32<const V: i32>() -> f32 {
    V as f32
}

pub fn serde_default_bool<const V: bool>() -> bool {
    V
}

pub fn parse_length_str(s: &str) -> Option<f64> {
    if let Some(s) = s.strip_suffix("px") {
        return s.parse().ok();
    }

    if let Some(s) = s.strip_suffix("in") {
        return s.parse::<f64>().ok().map(|t| t * 96.0);
    }
    if let Some(s) = s.strip_suffix("cm") {
        return s.parse::<f64>().ok().map(|t| t * 37.795);
    }

    if let Some(s) = s.strip_suffix("mm") {
        return s.parse::<f64>().ok().map(|t| t * 3.779); // Convert mm to pixels
    }

    if let Some(s) = s.strip_suffix("pt") {
        return s.parse::<f64>().ok().map(|t| t * 1.33); // Convert points to pixels
    }

    if let Some(s) = s.strip_suffix("pc") {
        return s.parse::<f64>().ok().map(|t| t * 16.0); // Convert picas to pixels
    }

    s.parse().ok()
}

pub fn parse_duration_str(s: &str) -> Option<f64> {
    if let Some(s) = s.strip_suffix("ms") {
        return s.parse().ok();
    }

    if let Some(s) = s.strip_suffix("s") {
        return s.parse::<f64>().ok().map(|t| t * 1000.0);
    }

    s.parse().ok()
}
