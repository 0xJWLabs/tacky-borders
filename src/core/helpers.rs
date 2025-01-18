// Helpers
pub fn type_name_of_val<T: ?Sized>(_val: &T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn serde_default_u64<const V: u64>() -> u64 {
    V
}

pub fn serde_default_i32<const V: i32>() -> i32 {
    V
}

pub fn serde_default_f32<const V: i32>() -> f32 {
    V as f32
}

pub fn parse_length_str(s: &str) -> Option<f32> {
    if let Some(s) = s.strip_suffix("px") {
        return s.parse().ok();
    }

    if let Some(s) = s.strip_suffix("in") {
        return s.parse::<f32>().ok().map(|t| t * 96.0);
    }
    if let Some(s) = s.strip_suffix("cm") {
        return s.parse::<f32>().ok().map(|t| t * 37.795);
    }

    if let Some(s) = s.strip_suffix("mm") {
        return s.parse::<f32>().ok().map(|t| t * 3.779); // Convert mm to pixels
    }

    if let Some(s) = s.strip_suffix("pt") {
        return s.parse::<f32>().ok().map(|t| t * 1.33); // Convert points to pixels
    }

    if let Some(s) = s.strip_suffix("pc") {
        return s.parse::<f32>().ok().map(|t| t * 16.0); // Convert picas to pixels
    }

    s.parse().ok()
}

