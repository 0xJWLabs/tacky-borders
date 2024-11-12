use std::thread;
use regex::Regex;

use windows::core::Result;
use windows::core::PWSTR;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::WPARAM;

use windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_EXTEND_MODE_CLAMP;
use windows::Win32::Graphics::Direct2D::D2D1_GAMMA_2_2;
use windows::Win32::Graphics::Direct2D::D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::Direct2D::ID2D1HwndRenderTarget;
use windows::Win32::Graphics::Direct2D::Common::D2D_POINT_2F;

use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;
use windows::Win32::Graphics::Dwm::DWMWA_CLOAKED;
use windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE;
use windows::Win32::Graphics::Dwm::DWM_WINDOW_CORNER_PREFERENCE;
use windows::Win32::Graphics::Dwm::DWMWCP_DEFAULT;
use windows::Win32::Graphics::Dwm::DWMWCP_DONOTROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUNDSMALL;
use windows::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;

use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::System::Threading::QueryFullProcessImageNameW;

use windows::Win32::UI::HiDpi::GetDpiForWindow;

use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;

use windows::Win32::UI::WindowsAndMessaging::WS_CHILD;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_WINDOWEDGE;
use windows::Win32::UI::WindowsAndMessaging::WS_MAXIMIZE;

use crate::BORDERS;
use crate::__ImageBase;
use crate::INITIAL_WINDOWS;
use crate::border_config::BorderRadius;
use crate::border_config::BorderRadiusOption;
use crate::border_config::MatchKind;
use crate::border_config::MatchStrategy;
use crate::border_config::WindowRule;
use crate::border_config::CONFIG;
use crate::colors::Color;
use crate::colors::ColorConfig;
use crate::enum_windows_callback;
use crate::window_border::WindowBorder;

pub const WM_APP_LOCATIONCHANGE: u32 = WM_APP;
pub const WM_APP_REORDER: u32 = WM_APP + 1;
pub const WM_APP_SHOWUNCLOAKED: u32 = WM_APP + 2;
pub const WM_APP_HIDECLOAKED: u32 = WM_APP + 3;
pub const WM_APP_MINIMIZESTART: u32 = WM_APP + 4;
pub const WM_APP_MINIMIZEEND: u32 = WM_APP + 5;

#[derive(Debug, PartialEq, Clone)]
pub struct SendHWND(pub HWND);
unsafe impl Send for SendHWND {}
unsafe impl Sync for SendHWND {}

pub enum ErrorMsg<F>
where
    F: FnOnce(),
{
    Fn(F),
    String(String),
}

#[derive(Debug, Clone)]
pub struct Brush {
    pub render_target: ID2D1HwndRenderTarget,
    pub color: Color,
    pub rect: RECT,
    pub brush_properties: D2D1_BRUSH_PROPERTIES,
    pub use_animation: bool,
    pub gradient_angle: Option<f32>,
}

impl Brush {
    pub fn to_id2d1_brush(&self) -> Result<ID2D1Brush> {
        let render_target = &self.render_target;
        let brush_properties = &self.brush_properties;

        match &self.color {
            Color::Solid(color) => {
                // Create Solid Color Brush
                let solid_brush =
                    unsafe { render_target.CreateSolidColorBrush(color, Some(brush_properties))? };
                Ok(solid_brush.into()) // Convert to ID2D1Brush
            }
            Color::Gradient(gradient_color) => {
                // Create Gradient Brush
                let gradient_stops = gradient_color.gradient_stops.clone();
                let gradient_stop_collection = unsafe {
                    render_target.CreateGradientStopCollection(
                        &gradient_stops,
                        D2D1_GAMMA_2_2,
                        D2D1_EXTEND_MODE_CLAMP,
                    )?
                };

                // Calculate gradient points based on the animation flag and direction
                let (start_point, end_point) = if self.use_animation {
                    let width = WindowsApi::get_rect_width(self.rect) as f32;
                    let height = WindowsApi::get_rect_height(self.rect) as f32;

                    let center_x = width / 2.0;
                    let center_y = height / 2.0;
                    let radius = (center_x.powi(2) + center_y.powi(2)).sqrt();

                    let gradient_angle = self.gradient_angle.unwrap_or(0.0);
                    let angle_rad = gradient_angle.to_radians();
                    let (sin, cos) = angle_rad.sin_cos();

                    (
                        D2D_POINT_2F {
                            x: center_x - radius * cos,
                            y: center_y - radius * sin,
                        },
                        D2D_POINT_2F {
                            x: center_x + radius * cos,
                            y: center_y + radius * sin,
                        },
                    )
                } else {
                    let width = WindowsApi::get_rect_width(self.rect) as f32;
                    let height = WindowsApi::get_rect_height(self.rect) as f32;

                    let (start_x, start_y, end_x, end_y) = match gradient_color.direction.clone() {
                        Some(coords) => (
                            coords[0] * width,
                            coords[1] * height,
                            coords[2] * width,
                            coords[3] * height,
                        ),
                        None => (0.0, 0.0, width, height),
                    };

                    (
                        D2D_POINT_2F {
                            x: start_x,
                            y: start_y,
                        },
                        D2D_POINT_2F { x: end_x, y: end_y },
                    )
                };

                let gradient_properties = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                    startPoint: start_point,
                    endPoint: end_point,
                };

                let gradient_brush = unsafe {
                    render_target.CreateLinearGradientBrush(
                        &gradient_properties,
                        Some(brush_properties),
                        Some(&gradient_stop_collection),
                    )?
                };

                Ok(gradient_brush.into()) // Convert to ID2D1Brush
            }
        }
    }
}

pub struct WindowsApi;

impl WindowsApi {
    pub fn get_rect_width(rect: RECT) -> i32 {
        rect.right - rect.left
    }

    pub fn get_rect_height(rect: RECT) -> i32 {
        rect.bottom - rect.top
    }

    pub fn is_rect_visible(rect: &RECT) -> bool {
        rect.top >= 0 || rect.left >= 0 || rect.bottom >= 0 || rect.right >= 0
    }

    pub fn are_rects_same_size(rect1: &RECT, rect2: &RECT) -> bool {
        rect1.right - rect1.left == rect2.right - rect2.left
            && rect1.bottom - rect1.top == rect2.bottom - rect2.top
    }

    pub fn set_layered_window_attributes<E>(
        hwnd: HWND,
        crkey: COLORREF,
        alpha: u8,
        flags: LAYERED_WINDOW_ATTRIBUTES_FLAGS,
        err: Option<ErrorMsg<E>>,
    ) -> Result<()>
    where
        E: FnOnce(),
    {
        let result = unsafe { SetLayeredWindowAttributes(hwnd, crkey, alpha, flags) };
        if result.is_err() {
            match err {
                Some(ErrorMsg::Fn(f)) => f(), // Call the function if it's a `Fn` variant
                Some(ErrorMsg::String(msg)) => println!("Error: {}", msg), // Print the message if it's a `String` variant,
                None => println!("Error: Setting window layered attributes"),
            };
        }

        Ok(())
    }

    pub fn _dwm_set_window_attribute<T, E>(
        hwnd: HWND,
        attribute: DWMWINDOWATTRIBUTE,
        value: &T,
        err: Option<ErrorMsg<E>>,
    ) -> Result<()>
    where
        E: FnOnce(),
    {
        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                (value as *const T).cast(),
                u32::try_from(std::mem::size_of::<T>())?,
            )
        };

        if result.is_err() {
            match err {
                Some(ErrorMsg::Fn(f)) => f(), // Call the function if it's a `Fn` variant
                Some(ErrorMsg::String(msg)) => println!("Error: {}", msg), // Print the message if it's a `String` variant,
                None => println!("Error: Setting window attribute"),
            };
        }

        Ok(())
    }

    pub fn dwm_get_window_attribute<T, E>(
        hwnd: HWND,
        attribute: DWMWINDOWATTRIBUTE,
        value: &mut T,
        err: Option<ErrorMsg<E>>,
    ) -> Result<()>
    where
        E: FnOnce(),
    {
        let result = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                attribute,
                (value as *mut T).cast(),
                u32::try_from(std::mem::size_of::<T>())?,
            )
        };

        if result.is_err() {
            match err {
                Some(ErrorMsg::Fn(f)) => f(), // Call the function if it's a `Fn` variant
                Some(ErrorMsg::String(msg)) => println!("Error: {}", msg), // Print the message if it's a `String` variant,
                None => println!("Error: Getting window attribute"),
            };
        }

        Ok(())
    }

    pub fn enum_windows() -> Result<Vec<HWND>> {
        let mut windows: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(
                Some(enum_windows_callback),
                LPARAM(&mut windows as *mut _ as isize),
                // LPARAM::default(),
            );
        }
        debug!("Windows have been enumerated");

        Ok(windows)
    }

    pub fn is_window_cloaked(hwnd: HWND) -> bool {
        let mut is_cloaked = FALSE;
        let _ = Self::dwm_get_window_attribute::<BOOL, fn()>(
            hwnd,
            DWMWA_CLOAKED,
            &mut is_cloaked,
            Some(ErrorMsg::String("Getting is_window_cloaked".to_string())),
        );

        is_cloaked.as_bool()
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

    pub fn has_native_border(hwnd: HWND) -> bool {
        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) as u32 };
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };

        ex_style & WS_EX_WINDOWEDGE.0 != 0 && style & WS_MAXIMIZE.0 == 0
    }

    pub fn get_window_title(hwnd: HWND) -> String {
        let mut buffer: [u16; 256] = [0; 256];

        if unsafe { GetWindowTextW(hwnd, &mut buffer) } == 0 {
            error!("Error getting window title!");
        }

        unsafe { GetWindowTextW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string()
    }

    pub fn get_window_class(hwnd: HWND) -> String {
        let mut buffer: [u16; 256] = [0; 256];

        if unsafe { GetClassNameW(hwnd, &mut buffer) } == 0 {
            error!("Error getting window class name!");
        }

        String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string()
    }

    pub fn get_process_name(hwnd: HWND) -> String {
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

    pub fn get_window_rule(hwnd: HWND) -> WindowRule {
        let title = Self::get_window_title(hwnd);
        let class = Self::get_window_class(hwnd);
        let process = Self::get_process_name(hwnd);

        // Lock the config mutex
        let config_mutex = &*CONFIG;
        let config = config_mutex.lock().unwrap();

        for rule in config.window_rules.iter() {
            if let Some(name) = match rule.rule_match.match_type {
                Some(MatchKind::Title) => Some(&title),
                Some(MatchKind::Process) => Some(&process),
                Some(MatchKind::Class) => Some(&class),
                _ => None,
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

    pub fn get_border_from_window(hwnd: HWND) -> Option<HWND> {
        let borders = BORDERS.lock().unwrap();
        let hwnd_isize = hwnd.0 as isize;
        let Some(border_isize) = borders.get(&hwnd_isize) else {
            drop(borders);
            return None;
        };

        let border_window: HWND = HWND(*border_isize as _);
        drop(borders);
        Some(border_window)
    }

    // Return true if the border exists in the border hashmap. Otherwise, create a new border and
    // return false.
    pub fn show_border_for_window(hwnd: HWND) -> bool {
        let border_window = Self::get_border_from_window(hwnd);
        if let Some(hwnd) = border_window {
            unsafe {
                let _ = PostMessageW(hwnd, WM_APP_SHOWUNCLOAKED, WPARAM(0), LPARAM(0));
            }
            true
        } else {
            if Self::is_window_visible(hwnd)
                && !Self::is_window_cloaked(hwnd)
                && !Self::has_filtered_style(hwnd)
            {
                let _ = Self::create_border_for_window(hwnd);
            }
            false
        }
    }

    pub fn hide_border_for_window(hwnd: HWND) -> bool {
        let window = SendHWND(hwnd);

        let _ = thread::spawn(move || {
            let window_sent = window;
            let border_option = Self::get_border_from_window(window_sent.0);
            if let Some(border_window) = border_option {
                unsafe {
                    let _ = PostMessageW(border_window, WM_APP_HIDECLOAKED, WPARAM(0), LPARAM(0));
                }
            }
        });
        true
    }

    pub fn create_border_for_window(tracking_window: HWND) -> Result<()> {
        let window = SendHWND(tracking_window);

        let _ = std::thread::spawn(move || {
            let window_sent = window;

            let window_rule = Self::get_window_rule(window_sent.0);

            if window_rule.rule_match.border_enabled == Some(false) {
                error!("border is disabled for this window, exiting!");
                return;
            }

            let config = CONFIG.lock().unwrap();

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
                .unwrap_or(config.global_rule.border_radius.clone());

            let config_active = window_rule
                .rule_match
                .active_color
                .or(config.global_rule.active_color.clone());

            let config_inactive = window_rule
                .rule_match
                .inactive_color
                .or(config.global_rule.inactive_color.clone());

            let border_colors = convert_config_colors(&config_active, &config_inactive);
            let use_active_animation = match border_colors.0 {
                Color::Gradient(ref color) => color.animation.unwrap_or(false),
                _ => false,
            };

            let use_inactive_animation = match border_colors.1 {
                Color::Gradient(ref color) => color.animation.unwrap_or(false),
                _ => false,
            };

            let border_radius = convert_config_radius(config_size, config_radius, window_sent.0);
            let window_isize = window_sent.0 .0 as isize;

            let init_delay = if INITIAL_WINDOWS.lock().unwrap().contains(&window_isize) {
                0
            } else {
                window_rule
                    .rule_match
                    .init_delay
                    .unwrap_or(config.global_rule.init_delay.unwrap_or(250))
            };

            let unminimize_delay = window_rule
                .rule_match
                .unminimize_delay
                .unwrap_or(config.global_rule.unminimize_delay.unwrap_or(200));

            //println!("time it takes to get colors: {:?}", before.elapsed());

            let mut border = WindowBorder {
                tracking_window: window_sent.0,
                border_size: config_size,
                border_offset,
                border_radius,
                active_color: border_colors.0,
                inactive_color: border_colors.1,
                use_active_animation,
                use_inactive_animation,
                unminimize_delay,
                ..Default::default()
            };
            drop(config);

            let mut borders_hashmap = BORDERS.lock().unwrap();
            let window_isize = window_sent.0 .0 as isize;

            // Check to see if the key already exists in the hashmap. I don't think this should ever
            // return true, but it's just in case.
            if borders_hashmap.contains_key(&window_isize) {
                drop(borders_hashmap);
                return;
            }

            let hinstance: HINSTANCE = unsafe { std::mem::transmute(&__ImageBase) };
            let _ = border.create_border_window(hinstance);
            borders_hashmap.insert(window_isize, border.border_window.0 as isize);

            // Drop these values (to save some RAM?) before calling init and entering a message loop
            drop(borders_hashmap);
            let _ = window_sent;
            let _ = window_rule;
            let _ = config_size;
            let _ = border_offset;
            let _ = config_radius;
            let _ = config_active;
            let _ = config_inactive;
            let _ = border_colors;
            let _ = window_isize;
            let _ = hinstance;

            let _ = border.init(init_delay);

            drop(border);
        });

        Ok(())
    }

    pub fn destroy_border_for_window(tracking_window: HWND) -> Result<()> {
        let window = SendHWND(tracking_window);

        let _ = thread::spawn(move || {
            let window_sent = window;
            let mut borders_hashmap = BORDERS.lock().unwrap();
            let window_isize = window_sent.0 .0 as isize;
            let Some(border_isize) = borders_hashmap.get(&window_isize) else {
                drop(borders_hashmap);
                return;
            };

            let border_window: HWND = HWND(*border_isize as _);
            unsafe {
                let _ = PostMessageW(border_window, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            borders_hashmap.remove(&window_isize);

            drop(borders_hashmap);
        });

        Ok(())
    }
}

// Helpers
fn match_rule(name: &str, pattern: &str, strategy: &Option<MatchStrategy>) -> bool {
    match strategy {
        Some(MatchStrategy::Contains) => name.to_lowercase().contains(&pattern.to_lowercase()),
        Some(MatchStrategy::Equals) => name.to_lowercase().eq(&pattern.to_lowercase()),
        Some(MatchStrategy::Regex) => Regex::new(pattern)
            .map(|re| re.is_match(name))
            .unwrap_or(false),
        None => false,
    }
}

fn convert_config_radius(
    config_size: i32,
    config_radius: BorderRadius,
    tracking_window: HWND,
) -> f32 {
    let mut corner_preference = DWM_WINDOW_CORNER_PREFERENCE::default();
    let dpi = unsafe { GetDpiForWindow(tracking_window) } as f32;
    let base_radius = (config_size as f32) / 2.0;
    let scale_factor = dpi / 96.0;

    match config_radius {
        BorderRadius::Float(radius) => {
            if radius == -1.0 {
                // Get system corner preference if radius is -1
                let _ = WindowsApi::dwm_get_window_attribute::<DWM_WINDOW_CORNER_PREFERENCE, fn()>(
                    tracking_window,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &mut corner_preference,
                    Some(ErrorMsg::String(
                        "Getting window corner preference".to_string(),
                    )),
                );
                match corner_preference {
                    DWMWCP_DEFAULT | DWMWCP_ROUND => {
                        return 8.0 * scale_factor + base_radius;
                    }
                    DWMWCP_ROUNDSMALL => {
                        return 4.0 * scale_factor + base_radius;
                    }
                    DWMWCP_DONOTROUND => {
                        return 0.0;
                    }
                    _ => return base_radius, // fallback default
                }
            }
            // Return specified radius scaled by DPI
            radius * scale_factor
        }
        BorderRadius::String(radius) => match radius {
            BorderRadiusOption::Auto => {
                // Get system corner preference for Auto option
                let _ = WindowsApi::dwm_get_window_attribute::<DWM_WINDOW_CORNER_PREFERENCE, fn()>(
                    tracking_window,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &mut corner_preference,
                    Some(ErrorMsg::String(
                        "Getting window corner preference".to_string(),
                    )),
                );
                match corner_preference {
                    DWMWCP_DEFAULT | DWMWCP_ROUND => {
                        return 8.0 * scale_factor + base_radius;
                    }
                    DWMWCP_ROUNDSMALL => {
                        return 4.0 * scale_factor + base_radius;
                    }
                    DWMWCP_DONOTROUND => {
                        return 0.0;
                    }
                    _ => return base_radius, // fallback default
                }
            }
            BorderRadiusOption::Round => {
                // Round corner radius
                return 8.0 * scale_factor + base_radius;
            }
            BorderRadiusOption::SmallRound => {
                // Small round corner radius
                return 4.0 * scale_factor + base_radius;
            }
            BorderRadiusOption::Square => {
                // No rounding for square corners
                return 0.0;
            }
        },
    }
}

fn convert_config_colors(
    color_active: &Option<ColorConfig>,
    color_inactive: &Option<ColorConfig>,
) -> (Color, Color) {
    (
        Color::from(color_active.as_ref()),
        Color::from(color_inactive.as_ref()),
    )
}
