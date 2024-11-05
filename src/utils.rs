use windows::{Win32::Foundation::*, Win32::Graphics::Dwm::*, Win32::UI::WindowsAndMessaging::*};

use dirs::home_dir;
use regex::Regex;
use std::fs::*;
use std::path::*;

use crate::border_config::*;
use crate::colors::*;
use crate::*;

pub const WM_APP_0: u32 = WM_APP;
pub const WM_APP_1: u32 = WM_APP + 1;
pub const WM_APP_2: u32 = WM_APP + 2;
pub const WM_APP_3: u32 = WM_APP + 3;
pub const WM_APP_4: u32 = WM_APP + 4;
pub const WM_APP_5: u32 = WM_APP + 5;

// Configuration
pub fn get_config() -> PathBuf {
    let home_dir = home_dir().expect("can't find home path");
    let config_dir = home_dir.join(".config").join("tacky-borders");
    let fallback_dir = home_dir.join(".tacky-borders");

    let dir_path = if exists(&config_dir).expect("Couldn't check if config dir exists") {
        config_dir
    } else if exists(&fallback_dir).expect("Couldn't check if config dir exists") {
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

// Windows Utility Functions
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
    let title = get_window_title(hwnd);
    let class = get_window_class(hwnd);
    let process = get_process_name(hwnd);

    // Lock the config mutex
    let config_mutex = &*CONFIG;
    let config = config_mutex.lock().unwrap();

    for rule in config.window_rules.iter() {
        if let Some(name) = match rule.rule_match.match_type {
            MatchKind::Title => Some(&title),
            MatchKind::Process => Some(&process),
            MatchKind::Class => Some(&class),
        } {
            if let Some(contains_str) = &rule.rule_match.match_value {
                if match_rule(name, contains_str, &rule.rule_match.match_strategy) {
                    return rule.clone();
                }
            }
        }
    }

    drop(config);

    WindowRule::default()
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
        log!("error", "Error getting is_cloacked");
        return true;
    }
    is_cloaked.as_bool()
}

// If the tracking window does not have a window edge or is maximized, then there should be no
// border.
pub fn has_native_border(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) as u32 };
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };
    ex_style & WS_EX_WINDOWEDGE.0 != 0 && style & WS_MAXIMIZE.0 == 0
}

pub fn _get_show_cmd(hwnd: HWND) -> u32 {
    let mut wp: WINDOWPLACEMENT = WINDOWPLACEMENT::default();
    let result = unsafe { GetWindowPlacement(hwnd, std::ptr::addr_of_mut!(wp)) };
    if result.is_err() {
        log!("error", "error getting window_placement!");
        return 0;
    }
    wp.showCmd
}

pub fn create_border_for_window(
    tracking_window: HWND,
    create_delay_override: Option<u64>,
) -> Result<()> {
    let borders_mutex = &*BORDERS;
    let config_mutex = &*CONFIG;
    let window = SendHWND(tracking_window);

    let _ = std::thread::spawn(move || {
        let window_sent = window;

        // This delay can be used to wait for a window to finish its opening animation or for it to
        // become visible if it is not so at first

        if !is_window_visible(window_sent.0) {
            return;
        }

        let window_rule = get_window_rule(window_sent.0);

        if window_rule.rule_match.border_enabled == Some(false) {
            log!("debug", "border is disabled for this window, exiting!");
            return;
        }

        let config = config_mutex.lock().unwrap();

        let config_size = window_rule
            .rule_match
            .border_size
            .unwrap_or(config.global_rule.border_size);
        let border_offset = window_rule
            .rule_match
            .border_offset
            .unwrap_or(config.global_rule.border_offset);
        let config_radius = window_rule
            .rule_match
            .border_radius
            .unwrap_or(config.global_rule.border_radius) as f32;

        let active_color = generate_color(
            &window_rule
                .rule_match
                .active_color
                .or(config.global_rule.active_color.clone()),
        );

        let inactive_color = generate_color(
            &window_rule
                .rule_match
                .inactive_color
                .or(config.global_rule.inactive_color.clone()),
        );

        let use_active_animation = match active_color {
            Color::Gradient(ref color) => color.animation.unwrap_or(false),
            _ => false,
        };

        let use_inactive_animation = match inactive_color {
            Color::Gradient(ref color) => color.animation.unwrap_or(false),
            _ => false,
        };

        let border_radius = convert_config_radius(config_size, config_radius, window_sent.0);

        let init_delay = match create_delay_override {
            Some(delay) => delay,
            None => window_rule
                .rule_match
                .init_delay
                .unwrap_or(config.global_rule.init_delay.unwrap_or(250)),
        };

        let unminimize_delay = window_rule
            .rule_match
            .unminimize_delay
            .unwrap_or(config.global_rule.unminimize_delay.unwrap_or(200));

        //println!("time it takes to get colors: {:?}", before.elapsed());

        let mut border = window_border::WindowBorder {
            tracking_window: window_sent.0,
            border_size: config_size,
            border_offset,
            border_radius,
            active_color,
            inactive_color,
            use_active_animation,
            use_inactive_animation,
            unminimize_delay,
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

        let _ = border.init(init_delay);
        drop(border);
    });

    Ok(())
}

pub fn convert_config_radius(config_size: i32, config_radius: f32, tracking_window: HWND) -> f32 {
    let mut corner_preference = DWM_WINDOW_CORNER_PREFERENCE::default();
    let dpi = unsafe { GetDpiForWindow(tracking_window) } as f32;

    // -1.0 means to use default Windows corner preference. I might want to use an enum to allow
    // for something like border_radius == "system" instead TODO
    if config_radius == -1.0 {
        let result = unsafe {
            DwmGetWindowAttribute(
                tracking_window,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                std::ptr::addr_of_mut!(corner_preference) as _,
                size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            )
        };
        if result.is_err() {
            log!("error", "Error getting window corner preference!");
        }
        match corner_preference {
            DWMWCP_DEFAULT => {
                return 8.0 * dpi / 96.0 + (config_size as f32) / 2.0;
            }
            DWMWCP_DONOTROUND => {
                return 0.0;
            }
            DWMWCP_ROUND => {
                return 8.0 * dpi / 96.0 + (config_size as f32) / 2.0;
            }
            DWMWCP_ROUNDSMALL => {
                return 4.0 * dpi / 96.0 + (config_size as f32) / 2.0;
            }
            _ => {}
        }
    }

    config_radius * dpi / 96.0
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
pub fn show_border_for_window(hwnd: HWND, create_delay_override: Option<u64>) -> bool {
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
        let _ = create_border_for_window(hwnd, create_delay_override);
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

// Helpers Functions
fn match_rule(name: &str, pattern: &str, strategy: &Option<MatchStrategy>) -> bool {
    match strategy {
        Some(MatchStrategy::Contains) => name.to_lowercase().contains(&pattern.to_lowercase()),
        Some(MatchStrategy::Equals) => name.to_lowercase() == pattern.to_lowercase(),
        Some(MatchStrategy::Regex) => Regex::new(pattern)
            .map(|re| re.is_match(name))
            .unwrap_or(false),
        None => false,
    }
}

fn get_window_title(hwnd: HWND) -> String {
    let mut buffer: [u16; 256] = [0; 256];

    if unsafe { GetWindowTextW(hwnd, &mut buffer) } == 0 {
        log!("error", "Error getting window title!");
    }

    unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer)
        .trim_end_matches('\0')
        .to_string()
}

fn get_window_class(hwnd: HWND) -> String {
    let mut buffer: [u16; 256] = [0; 256];

    if unsafe { GetClassNameW(hwnd, &mut buffer) } == 0 {
        log!("error", "Error getting window class name!");
    }

    String::from_utf16_lossy(&buffer)
        .trim_end_matches('\0')
        .to_string()
}

fn get_process_name(hwnd: HWND) -> String {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }

    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) };

    let process_handle = match process_handle {
        Ok(handle) => handle,
        Err(_) => return String::new(), // Return empty string on error
    };

    let mut buffer = [0u16; 256];
    let mut length = buffer.len() as u32;

    unsafe {
        // Query the process image name
        if QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32, // Use 0 to indicate no special flags
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
        .is_err()
        {
            CloseHandle(process_handle).ok();
            return String::new(); // Return empty string on error
        }

        CloseHandle(process_handle).ok(); // Ignore the result of CloseHandle
    }

    let exe_path = String::from_utf16_lossy(&buffer[..length as usize]);

    exe_path
        .split('\\')
        .last()
        .and_then(|file_name| file_name.split('.').next()) // Using `and_then`
        .unwrap_or("") // Return empty string if parsing fails
        .trim_end_matches('\0')
        .to_string()
}
