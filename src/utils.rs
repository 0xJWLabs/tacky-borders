use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS};

use crate::{logger::Logger};

pub fn get_file_path(filename: &str) -> String {
    let user_profile_path = match std::env::var("USERPROFILE") {
        Ok(user_profile_path) => user_profile_path,
        Err(err) => {
            Logger::log("error", "Failed to find USERPROFILE environment variable");
            Logger::log("debug", &format!("{:?}", err));
            std::process::exit(1);
        }
    };
    let dirpath = format!("{}\\.tacky-borders", user_profile_path);
    let filepath = format!("{}\\{}", dirpath, filename);

    if !Path::new(&dirpath).exists() {
        if let Err(err) = fs::create_dir(&dirpath) {
            Logger::log("error", &format!("Failed to create directory: {}", &dirpath));
            Logger::log("debug", &format!("{:?}", err));
            std::process::exit(1);
        }
    }
    return filepath;
}

pub fn get_file(filename: &str, default_content: &str) -> std::fs::File {
    let filepath = get_file_path(filename);

    if !Path::new(&filepath).exists() {
        let mut file = match File::create(&filepath) {
            Ok(file) => file,
            Err(err) => {
                Logger::log("error", &format!("Failed to create file: {}", &filepath));
                Logger::log("debug", &format!("{:?}", err));
                std::process::exit(1);
            }
        };

        if let Err(err) = file.write_all(default_content.as_bytes()) {
            Logger::log("error", &format!("Failed to write to file: {}", &filepath));
            Logger::log("debug", &format!("{:?}", err));
            std::process::exit(1);
        }
    }

    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .open(&filepath)
    {
        Ok(file) => file,
        Err(err) => {
            Logger::log("error", &format!("Failed to open file: {}", &filepath));
            Logger::log("debug",&format!("{:?}", err));
            std::process::exit(1);
        }
    };

    file
}

pub fn get_color_from_hex(hex: &str) -> D2D1_COLOR_F {
    // Assuming hex is a string in the format "#FFFFFF" or "FFFFFF"
    let hex = hex.trim_start_matches('#'); // Remove leading '#'

    // Convert hex string to u32
    let value = u32::from_str_radix(hex, 16).expect("Invalid hex color");

    // Extract RGB components as f32 values
    let red = ((value & 0x00FF0000) >> 16) as f32 / 255.0; // Red component
    let green = ((value & 0x0000FF00) >> 8) as f32 / 255.0; // Green component
    let blue = (value & 0x000000FF) as f32 / 255.0; // Blue component

    // Return the D2D1_COLOR_F struct
    D2D1_COLOR_F {
        r: red,
        g: green,
        b: blue,
        a: 1.0,
    }
}

pub fn get_color_from_rgba(rgba: &str) -> D2D1_COLOR_F {
    let rgba = rgba.trim_start_matches("rgb(").trim_start_matches("rgba(").trim_end_matches(')');
    let components: Vec<&str> = rgba.split(',').map(|s| s.trim()).collect();

    // Check for correct number of components
    if components.len() == 3 || components.len() == 4 {
        // Parse red, green, and blue values
        let red: f32 = components[0].parse::<u32>().unwrap_or(0) as f32 / 255.0;
        let green: f32 = components[1].parse::<u32>().unwrap_or(0) as f32 / 255.0;
        let blue: f32 = components[2].parse::<u32>().unwrap_or(0) as f32 / 255.0;

        let alpha: f32 = if components.len() == 4 {
            components[3].parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0)
        } else {
            1.0
        };

        return D2D1_COLOR_F {
            r: red,
            g: green,
            b: blue,
            a: alpha, // Default alpha value for rgb()
        };
    }

    // Return a default color if parsing fails
    D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

pub fn get_color_from_oklch(oklch: &str) -> D2D1_COLOR_F {
    let oklch = oklch.trim_start_matches("oklch(").trim_end_matches(')');
    let components: Vec<&str> = oklch.split(',').map(|s| s.trim()).collect(); // Split by commas

    // Check for the correct number of components (3)
    if components.len() == 3 {
        // Parse lightness, chroma, and hue values
        let lightness_str = components[0];
        let lightness: f64 = if lightness_str.ends_with('%') {
            lightness_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0 // Convert percentage to a 0.0 - 1.0 range
        } else {
            lightness_str.parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0) // Handle non-percentage case
        };

        let chroma: f64 = components[1].parse::<f64>().unwrap_or(0.0).clamp(0.0, f64::MAX);
        let hue: f64 = components[2].parse::<f64>().unwrap_or(0.0).clamp(0.0, 360.0);

        // Convert OKLCH to RGB
        let (r, g, b) = oklch_to_rgb(lightness, chroma, hue);

        return D2D1_COLOR_F {
            r: r as f32, // Convert back to f32 for D2D1_COLOR_F
            g: g as f32,
            b: b as f32,
            a: 1.0, // Default alpha value
        };
    }

    // Return a default color if parsing fails
    D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

// Placeholder for the actual OKLCH to RGB conversion function
fn oklch_to_rgb(lightness: f64, chroma: f64, hue: f64) -> (f64, f64, f64) {
    // Implement the conversion from OKLCH to RGB here
    // For now, returning a placeholder RGB value
    (lightness, chroma, hue) // This is just a placeholder; replace with actual conversion logic
}

pub fn get_color_from_hsl(hsl: &str) -> D2D1_COLOR_F {
    let hsl = hsl.trim_start_matches("hsl(").trim_end_matches(')');
    let components: Vec<&str> = hsl.split(',').map(|s| s.trim()).collect(); // Split by commas

    // Check for the correct number of components (3)
    if components.len() == 3 {
        // Parse hue, saturation, and lightness values
        let hue: f64 = components[0].parse::<f64>().unwrap_or(0.0).clamp(0.0, 360.0);
        
        let saturation_str = components[1];
        let saturation: f64 = if saturation_str.ends_with('%') {
            saturation_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0 // Convert percentage to a 0.0 - 1.0 range
        } else {
            saturation_str.parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0) // Handle non-percentage case
        };

        let lightness_str = components[2];
        let lightness: f64 = if lightness_str.ends_with('%') {
            lightness_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0 // Convert percentage to a 0.0 - 1.0 range
        } else {
            lightness_str.parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0) // Handle non-percentage case
        };

        // Convert HSL to RGB
        let (r, g, b) = hsl_to_rgb(hue, saturation, lightness);

        return D2D1_COLOR_F {
            r: r as f32, // Convert back to f32 for D2D1_COLOR_F
            g: g as f32,
            b: b as f32,
            a: 1.0, // Default alpha value
        };
    }

    // Return a default color if parsing fails
    D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

// Placeholder for the actual HSL to RGB conversion function
fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (f64, f64, f64) {
    // Implement the conversion from HSL to RGB here
    // For now, returning a placeholder RGB value
    // This is just a placeholder; replace with actual conversion logic

    // HSL to RGB conversion logic
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation; // Chroma
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs()); // Second largest component
    let m = lightness - c / 2.0; // Match lightness
    
    let (r_prime, g_prime, b_prime) = match hue {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    // Convert to RGB and apply match lightness
    let r = (r_prime + m).clamp(0.0, 1.0);
    let g = (g_prime + m).clamp(0.0, 1.0);
    let b = (b_prime + m).clamp(0.0, 1.0);

    (r, g, b)
}
