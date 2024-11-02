use dirs::home_dir;
use regex::Regex;
use std::{
    fs::{self, DirBuilder},
    path::PathBuf,
};

use windows::{
    Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*, Win32::Graphics::Dwm::*,
    Win32::UI::WindowsAndMessaging::*,
};

use crate::*;

use crate::border_config::*;
use crate::logger::Logger;

pub const WM_APP_0: u32 = WM_APP;
pub const WM_APP_1: u32 = WM_APP + 1;
pub const WM_APP_2: u32 = WM_APP + 2;
pub const WM_APP_3: u32 = WM_APP + 3;
pub const WM_APP_4: u32 = WM_APP + 4;
pub const WM_APP_5: u32 = WM_APP + 5;

#[derive(Debug, Clone)]
pub enum Color {
    Solid(D2D1_COLOR_F),
    Gradient(Gradient),
}

// Implement Default for your own MyBrush enum
impl Default for Color {
    fn default() -> Self {
        Color::Solid(D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Gradient {
    pub direction: Option<Vec<f32>>,
    pub gradient_stops: Vec<D2D1_GRADIENT_STOP>, // Array of gradient stops
    pub animation: Option<bool>,
}

impl Default for Gradient {
    fn default() -> Self {
        Gradient {
            direction: None,
            gradient_stops: vec![
                D2D1_GRADIENT_STOP {
                    position: 0.0,
                    color: D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                },
                D2D1_GRADIENT_STOP {
                    position: 1.0,
                    color: D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
            ],
            animation: Some(false),
        }
    }
}

impl AsRef<GradientColor> for GradientColor {
    fn as_ref(&self) -> &GradientColor {
        self
    }
}

// Files
pub fn get_config() -> PathBuf {
    let home_dir = home_dir().expect("can't find home path");
    let config_dir = home_dir.join(".config").join("tacky-borders");
    let fallback_dir = home_dir.join(".tacky-borders");

    let dir_path = if fs::exists(&config_dir).expect("Couldn't check if config dir exists") {
        config_dir
    } else if fs::exists(&fallback_dir).expect("Couldn't check if config dir exists") {
        fallback_dir
    } else {
        DirBuilder::new()
            .recursive(true)
            .create(&config_dir)
            .expect("could not create config directory!");

        config_dir
    };

    dir_path
}

// Windows
pub fn get_rect_width(rect: RECT) -> i32 {
    rect.right - rect.left
}

pub fn get_rect_height(rect: RECT) -> i32 {
    rect.bottom - rect.top
}

pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

pub fn is_window_active(hwnd: HWND) -> bool {
    unsafe { GetForegroundWindow() == hwnd }
}

pub fn has_filtered_style(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) as u32 };
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };

    if style & WS_CHILD.0 != 0
        || ex_style & WS_EX_TOOLWINDOW.0 != 0
        || ex_style & WS_EX_NOACTIVATE.0 != 0
    {
        return true;
    }

    false
}

pub fn get_window_rule(hwnd: HWND) -> WindowRule {
    let mut title_arr: [u16; 256] = [0; 256];
    let mut class_arr: [u16; 256] = [0; 256];

    if unsafe { GetWindowTextW(hwnd, &mut title_arr) } == 0 {
        println!("error getting window title!");
    }
    if unsafe { GetClassNameW(hwnd, &mut class_arr) } == 0 {
        println!("error getting class name!");
    }

    let title = String::from_utf16_lossy(&title_arr)
        .trim_end_matches('\0')
        .to_string();
    let class = String::from_utf16_lossy(&class_arr)
        .trim_end_matches('\0')
        .to_string();

    // Lock the config mutex
    let config_mutex = &*CONFIG;
    let config = config_mutex.lock().unwrap();

    let mut global_rule: WindowRule = WindowRule {
        rule_match: MatchDetails {
            match_type: RuleMatch::Global,
            match_value: None,
            match_strategy: None,
            active_color: None,
            inactive_color: None,
            border_enabled: None,
        },
    };

    for rule in config.window_rules.iter() {
        let name = match rule.rule_match.match_type {
            RuleMatch::Title => &title,
            RuleMatch::Class => &class,
            RuleMatch::Global => {
                global_rule = rule.clone();
                continue;
            }
        };

        if let Some(contains_str) = &rule.rule_match.match_value {
            match rule.rule_match.match_strategy {
                Some(MatchType::Contains) => {
                    if name.to_lowercase().contains(&contains_str.to_lowercase()) {
                        return rule.clone();
                    }
                }
                Some(MatchType::Equals) => {
                    if name.to_lowercase() == contains_str.to_lowercase() {
                        return rule.clone();
                    }
                }
                Some(MatchType::Regex) => {
                    match Regex::new(contains_str) {
                        Ok(regex) => {
                            if regex.is_match(name) {
                                return rule.clone();
                            }
                        }
                        Err(e) => {
                            println!("Invalid regex pattern: {}", e); // Use your logger here
                        }
                    }
                }
                None => {}
            }
        }
    }

    // Return the global rule if no specific rule matches
    global_rule
}

pub fn is_cloaked(hwnd: HWND) -> bool {
    let mut is_cloaked = FALSE;
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            std::ptr::addr_of_mut!(is_cloaked) as *mut _,
            size_of::<BOOL>() as u32,
        )
    };
    if result.is_err() {
        println!("error getting is_cloaked");
        return true;
    }
    is_cloaked.as_bool()
}

// If the tracking window does not have a window edge or is maximized, then there should be no
// border.
pub fn has_native_border(hwnd: HWND) -> bool {
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        if ex_style & WS_EX_WINDOWEDGE.0 == 0 || style & WS_MAXIMIZE.0 != 0 {
            return false;
        }

        true
    }
}

pub fn get_show_cmd(hwnd: HWND) -> u32 {
    let mut wp: WINDOWPLACEMENT = WINDOWPLACEMENT::default();
    let result = unsafe { GetWindowPlacement(hwnd, std::ptr::addr_of_mut!(wp)) };
    if result.is_err() {
        println!("error getting window_placement!");
        return 0;
    }
    wp.showCmd
}

pub fn get_colors_for_window(_hwnd: HWND) -> (Color, Color) {
    let window_rule = get_window_rule(_hwnd);

    let get_color = |color_config: &Option<ColorConfig>, default: &str| match color_config {
        Some(ColorConfig::String(color)) => create_solid_color(color.to_string()),
        Some(ColorConfig::Struct(color)) => create_gradient_colors(color.clone()),
        None => create_solid_color(default.to_string()),
    };

    let mut colors = (
        Color::Solid(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
        Color::Solid(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
    );

    colors.0 = get_color(&window_rule.rule_match.active_color, "accent");
    colors.1 = get_color(&window_rule.rule_match.inactive_color, "accent");

    colors
}

pub fn create_border_for_window(tracking_window: HWND, delay: u64) -> Result<()> {
    let borders_mutex = &*BORDERS;
    let config_mutex = &*CONFIG;
    let window = SendHWND(tracking_window);

    let (active_color, inactive_color) = get_colors_for_window(tracking_window);

    let _ = std::thread::spawn(move || {
        let window_sent = window;

        // This delay can be used to wait for a window to finish its opening animation or for it to
        // become visible if it is not so at first
        std::thread::sleep(std::time::Duration::from_millis(delay));

        if !is_window_visible(window_sent.0) {
            return;
        }

        let window_rule = get_window_rule(window_sent.0);

        if window_rule.rule_match.border_enabled == Some(false) {
            println!("border is disabled for this window, exiting!");
            return;
        }

        let config = config_mutex.lock().unwrap();

        let use_active_animation = match active_color {
            Color::Gradient(ref color) => color.animation.unwrap_or(false),
            _ => false,
        };

        let use_inactive_animation = match inactive_color {
            Color::Gradient(ref color) => color.animation.unwrap_or(false),
            _ => false,
        };

        //println!("time it takes to get colors: {:?}", before.elapsed());

        let mut border = window_border::WindowBorder {
            tracking_window: window_sent.0,
            border_size: config.border_size,
            border_offset: config.border_offset,
            border_radius: config.get_border_radius(),
            active_color,
            inactive_color,
            use_active_animation,
            use_inactive_animation,
            ..Default::default()
        };
        drop(config);

        let mut borders_hashmap = borders_mutex.lock().unwrap();
        let window_isize = window_sent.0 .0 as isize;

        // Check to see if the key already exists in the hashmap. If not, then continue
        // adding the key and initializing the border. This is important because sometimes, the
        // event_hook function will call spawn_border_thread multiple times for the same window.
        if borders_hashmap.contains_key(&window_isize) {
            //println!("Duplicate window: {:?}", borders_hashmap);
            drop(borders_hashmap);
            return;
        }

        let hinstance: HINSTANCE = unsafe { std::mem::transmute(&__ImageBase) };
        let _ = border.create_border_window(hinstance);

        borders_hashmap.insert(window_isize, border.border_window.0 as isize);
        drop(borders_hashmap);

        let _ = border.init();
    });

    Ok(())
}

pub fn destroy_border_for_window(tracking_window: HWND) -> Result<()> {
    let mutex = &*BORDERS;
    let window = SendHWND(tracking_window);

    let _ = std::thread::spawn(move || {
        let window_sent = window;
        let mut borders_hashmap = mutex.lock().unwrap();
        let window_isize = window_sent.0 .0 as isize;
        let border_option = borders_hashmap.get(&window_isize);

        if let Some(option) = border_option {
            let border_window: HWND = HWND((*option) as *mut _);
            unsafe { SendMessageW(border_window, WM_DESTROY, WPARAM(0), LPARAM(0)) };
            borders_hashmap.remove(&window_isize);
        }

        drop(borders_hashmap);
    });

    Ok(())
}

pub fn get_border_from_window(hwnd: HWND) -> Option<HWND> {
    let mutex = &*BORDERS;
    let borders = mutex.lock().unwrap();
    let hwnd_isize = hwnd.0 as isize;
    let border_option = borders.get(&hwnd_isize);

    if let Some(option) = border_option {
        let border_window: HWND = HWND(*option as _);
        drop(borders);
        Some(border_window)
    } else {
        drop(borders);
        None
    }
}

// Return true if the border exists in the border hashmap. Otherwise, create a new border and
// return false.
// We can also specify a delay to prevent the border from appearing while a window is in its
// opening animation.
pub fn show_border_for_window(hwnd: HWND, delay: u64) -> bool {
    let border_window = get_border_from_window(hwnd);
    if let Some(window) = border_window {
        unsafe {
            let _ = PostMessageW(window, WM_APP_2, WPARAM(0), LPARAM(0));
        }
        true
    } else {
        if is_cloaked(hwnd) || has_filtered_style(hwnd) {
            return false;
        }
        let _ = create_border_for_window(hwnd, delay);
        false
    }
}

pub fn hide_border_for_window(hwnd: HWND) -> bool {
    let window = SendHWND(hwnd);

    let _ = std::thread::spawn(move || {
        let window_sent = window;
        let border_window = get_border_from_window(window_sent.0);
        if let Some(window) = border_window {
            unsafe {
                let _ = PostMessageW(window, WM_APP_3, WPARAM(0), LPARAM(0));
            }
        }
    });
    true
}

pub fn create_solid_color(color: String) -> Color {
    if color == "accent" {
        let mut pcr_colorization: u32 = 0;
        let mut pf_opaqueblend: BOOL = FALSE;
        let result = unsafe { DwmGetColorizationColor(&mut pcr_colorization, &mut pf_opaqueblend) };
        if result.is_err() {
            Logger::log("error", "Error getting windows accent color!");
            return Color::Solid(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            });
        }
        let red = ((pcr_colorization & 0x00FF0000) >> 16) as f32 / 255.0;
        let green = ((pcr_colorization & 0x0000FF00) >> 8) as f32 / 255.0;
        let blue = (pcr_colorization & 0x000000FF) as f32 / 255.0;
        Color::Solid(D2D1_COLOR_F {
            r: red,
            g: green,
            b: blue,
            a: 1.0,
        })
    } else if (color.starts_with("rgb(")) || (color.starts_with("rgba(")) {
        Color::Solid(get_color_from_rgba(&color))
    } else {
        Color::Solid(get_color_from_hex(&color))
    }
}

pub fn create_gradient_colors(color: GradientColor) -> Color {
    let num_colors = color.colors.len();
    if num_colors == 0 {
        return Color::Gradient(Gradient::default());
    }

    let gradient_stops: Vec<D2D1_GRADIENT_STOP> = color
        .colors
        .into_iter()
        .enumerate()
        .map(|(i, hex)| {
            let position = i as f32 / (num_colors - 1) as f32;
            D2D1_GRADIENT_STOP {
                position,
                color: get_color_from_hex(hex.as_str()),
            }
        })
        .collect();

    let direction = Some(color.direction);

    Color::Gradient(Gradient {
        direction: direction.map(|g| g.to_vec()),
        gradient_stops,
        animation: color.animation,
    })
}

pub fn get_color_from_hex(hex: &str) -> D2D1_COLOR_F {
    // Ensure the hex string starts with '#' and is of the correct length
    if hex.len() != 7 && hex.len() != 9 && hex.len() != 4 && hex.len() != 5 || !hex.starts_with('#')
    {
        Logger::log(
            "error",
            format!("Invalid hex color format: {}", hex).as_str(),
        );
    }

    // Expand shorthand hex formats (#RGB or #RGBA to #RRGGBB or #RRGGBBAA)
    let expanded_hex = match hex.len() {
        4 => format!(
            "#{}{}{}{}{}{}",
            &hex[1..2],
            &hex[1..2],
            &hex[2..3],
            &hex[2..3],
            &hex[3..4],
            &hex[3..4]
        ),
        5 => format!(
            "#{}{}{}{}{}{}{}{}",
            &hex[1..2],
            &hex[1..2],
            &hex[2..3],
            &hex[2..3],
            &hex[3..4],
            &hex[3..4],
            &hex[4..5],
            &hex[4..5]
        ),
        _ => hex.to_string(),
    };

    // Convert each color component to f32 between 0.0 and 1.0, handling errors
    let parse_component = |s: &str| -> f32 {
        match u8::from_str_radix(s, 16) {
            Ok(val) => f32::from(val) / 255.0,
            Err(_) => {
                println!("Error: Invalid component '{}' in hex: {}", s, expanded_hex);
                0.0
            }
        }
    };

    // Parse RGB values
    let r = parse_component(&expanded_hex[1..3]);
    let g = parse_component(&expanded_hex[3..5]);
    let b = parse_component(&expanded_hex[5..7]);

    // Parse alpha value if present
    let a = if expanded_hex.len() == 9 {
        parse_component(&expanded_hex[7..9])
    } else {
        1.0
    };

    D2D1_COLOR_F { r, g, b, a }
}

pub fn get_color_from_rgba(rgba: &str) -> D2D1_COLOR_F {
    let rgba = rgba
        .trim_start_matches("rgb(")
        .trim_start_matches("rgba(")
        .trim_end_matches(')');
    let components: Vec<&str> = rgba.split(',').map(|s| s.trim()).collect();

    // Check for correct number of components
    if components.len() == 3 || components.len() == 4 {
        // Parse red, green, and blue values
        let red: f32 = f32::from_bits(components[0].parse::<u32>().unwrap_or(0)) / 255.0;
        let green: f32 = f32::from_bits(components[1].parse::<u32>().unwrap_or(0)) / 255.0;
        let blue: f32 = f32::from_bits(components[2].parse::<u32>().unwrap_or(0)) / 255.0;

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
