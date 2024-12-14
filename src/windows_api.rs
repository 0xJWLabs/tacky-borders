use anyhow::Context;
use regex::Regex;
use std::thread;
use win_color::Color;
use win_color::ColorImpl;
use win_color::GlobalColor;
use windows::core::Param;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT;
use windows::Win32::UI::Input::Ime::ImmDisableIME;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::SendNotifyMessageW;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;

use windows::core::Result as WinResult;
use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::WPARAM;

use windows::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows::Win32::Graphics::Dwm::DWMWA_CLOAKED;
use windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE;
use windows::Win32::Graphics::Dwm::DWMWCP_DEFAULT;
use windows::Win32::Graphics::Dwm::DWMWCP_DONOTROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUND;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUNDSMALL;
use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;
use windows::Win32::Graphics::Dwm::DWM_WINDOW_CORNER_PREFERENCE;

use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::QueryFullProcessImageNameW;
use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;

use windows::Win32::UI::HiDpi::GetDpiForWindow;

use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

use windows::Win32::UI::WindowsAndMessaging::WS_CHILD;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_WINDOWEDGE;
use windows::Win32::UI::WindowsAndMessaging::WS_MAXIMIZE;

use crate::__ImageBase;
use crate::border_config::BorderRadius;
use crate::border_config::MatchKind;
use crate::border_config::MatchStrategy;
use crate::border_config::WindowRule;
use crate::border_config::CONFIG;
use crate::enum_windows_callback;
use crate::utils::LogIfErr;
use crate::window_border::WindowBorder;
use crate::BORDERS;
use crate::INITIAL_WINDOWS;

pub const WM_APP_LOCATIONCHANGE: u32 = WM_APP;
pub const WM_APP_REORDER: u32 = WM_APP + 1;
pub const WM_APP_FOREGROUND: u32 = WM_APP + 2;
pub const WM_APP_SHOWUNCLOAKED: u32 = WM_APP + 3;
pub const WM_APP_HIDECLOAKED: u32 = WM_APP + 4;
pub const WM_APP_MINIMIZESTART: u32 = WM_APP + 5;
pub const WM_APP_MINIMIZEEND: u32 = WM_APP + 6;
pub const WM_APP_TIMER: u32 = WM_APP + 7;

#[derive(Debug, PartialEq, Clone)]
pub struct SendHWND(pub HWND);
unsafe impl Send for SendHWND {}
unsafe impl Sync for SendHWND {}

pub struct WindowsApi;

impl WindowsApi {
    pub fn post_message_w<P>(hwnd: P, msg: u32, wparam: WPARAM, lparam: LPARAM) -> WinResult<()>
    where
        P: Param<HWND>,
    {
        unsafe { PostMessageW(hwnd, msg, wparam, lparam) }
    }

    pub fn send_notify_message_w(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> WinResult<()> {
        unsafe { SendNotifyMessageW(hwnd, msg, wparam, lparam) }
    }

    pub fn imm_disable_ime(param0: u32) -> BOOL {
        unsafe { ImmDisableIME(param0) }
    }

    pub fn set_process_dpi_awareness_context(value: DPI_AWARENESS_CONTEXT) -> WinResult<()> {
        unsafe { SetProcessDpiAwarenessContext(value) }
    }

    pub fn get_message_w(
        lpmsg: *mut MSG,
        hwnd: HWND,
        wmsgfiltermin: u32,
        wmsgfiltermax: u32,
    ) -> BOOL {
        unsafe { GetMessageW(lpmsg, hwnd, wmsgfiltermin, wmsgfiltermax) }
    }

    pub fn translate_message(lpmsg: *const MSG) -> BOOL {
        unsafe { TranslateMessage(lpmsg) }
    }

    pub fn dispatch_message_w(lpmsg: *const MSG) -> LRESULT {
        unsafe { DispatchMessageW(lpmsg) }
    }

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

    pub fn set_layered_window_attributes(
        hwnd: HWND,
        crkey: COLORREF,
        alpha: u8,
        flags: LAYERED_WINDOW_ATTRIBUTES_FLAGS,
    ) -> WinResult<()> {
        unsafe { SetLayeredWindowAttributes(hwnd, crkey, alpha, flags) }
    }

    pub fn _dwm_set_window_attribute<T>(
        hwnd: HWND,
        attribute: DWMWINDOWATTRIBUTE,
        value: &T,
    ) -> WinResult<()> {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                (value as *const T).cast(),
                u32::try_from(std::mem::size_of::<T>())?,
            )
        }
    }

    pub fn dwm_get_window_attribute<T>(
        hwnd: HWND,
        attribute: DWMWINDOWATTRIBUTE,
        value: &mut T,
    ) -> WinResult<()> {
        unsafe {
            DwmGetWindowAttribute(
                hwnd,
                attribute,
                value as *mut _ as _, // Direct cast
                u32::try_from(std::mem::size_of::<T>())?,
            )
        }
    }

    pub fn enum_windows() -> WinResult<Vec<HWND>> {
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
        if let Err(e) = Self::dwm_get_window_attribute(hwnd, DWMWA_CLOAKED, &mut is_cloaked) {
            error!("could not check if window is cloaked: {e}");
            return true;
        }

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

        let config = CONFIG.read().unwrap();

        for rule in config.window_rules.iter() {
            let window_name = match rule.rule_match.match_kind {
                Some(MatchKind::Title) => &title,
                Some(MatchKind::Process) => &process,
                Some(MatchKind::Class) => &class,
                None => {
                    error!("expected 'kind' for window rule but none found!");
                    continue;
                }
            };

            let Some(match_value) = &rule.rule_match.match_value else {
                error!("expected `value` for window rule but non found!");
                continue;
            };

            let has_match = match rule.rule_match.match_strategy {
                Some(MatchStrategy::Equals) | None => {
                    window_name.to_lowercase().eq(&match_value.to_lowercase())
                }
                Some(MatchStrategy::Contains) => window_name
                    .to_lowercase()
                    .contains(&match_value.to_lowercase()),
                Some(MatchStrategy::Regex) => Regex::new(match_value)
                    .unwrap()
                    .captures(window_name)
                    .is_some(),
            };

            if has_match {
                return rule.clone();
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

    pub fn show_border_for_window(hwnd: HWND) {
        // If the border already exists, simply post a 'SHOW' message to its message queue. Otherwise,
        // create a new border.
        if let Some(border) = Self::get_border_from_window(hwnd) {
            Self::post_message_w(border, WM_APP_SHOWUNCLOAKED, WPARAM(0), LPARAM(0))
                .context("show_border_for_window")
                .log_if_err();
        } else if Self::is_window_visible(hwnd)
            && !Self::is_window_cloaked(hwnd)
            && !Self::has_filtered_style(hwnd)
        {
            Self::create_border_for_window(hwnd);
        }
    }

    pub fn hide_border_for_window(hwnd: HWND) -> bool {
        let window = SendHWND(hwnd);

        let _ = thread::spawn(move || {
            let window_sent = window;
            if let Some(border) = Self::get_border_from_window(window_sent.0) {
                Self::post_message_w(border, WM_APP_HIDECLOAKED, WPARAM(0), LPARAM(0))
                    .context("hide_border_for_window")
                    .log_if_err();
            }
        });
        true
    }

    pub fn create_border_for_window(tracking_window: HWND) {
        debug!("creating border for: {:?}", tracking_window);
        let window = SendHWND(tracking_window);

        let _ = std::thread::spawn(move || {
            let window_sent = window;
            let window_isize = window_sent.0 .0 as isize;

            let window_rule = Self::get_window_rule(window_sent.0);
            if window_rule.rule_match.border_enabled == Some(false) {
                info!("border is disabled for {:?}!", window_sent.0);
                return;
            }

            let mut border = create_border_struct(window_sent.0, &window_rule);

            let mut borders_hashmap = BORDERS.lock().unwrap();

            // Check to see if there is already a border for the given tracking window
            if borders_hashmap.contains_key(&window_isize) {
                return;
            }

            let hinstance: HINSTANCE = unsafe { std::mem::transmute(&__ImageBase) };
            if let Err(e) = border.create_border_window(hinstance) {
                error!("could not create border window: {e:?}");
                return;
            };

            borders_hashmap.insert(window_isize, border.border_window.0 as isize);

            drop(borders_hashmap);
            let _ = window_sent;
            let _ = window_isize;
            let _ = window_rule;
            let _ = hinstance;

            if let Err(e) = border.init() {
                error!("{e}");
            }
        });
    }

    pub fn destroy_border_for_window(tracking_window: HWND) {
        let window_isize = tracking_window.0 as isize;
        let Some(&border_isize) = BORDERS.lock().unwrap().get(&window_isize) else {
            return;
        };

        let border_window: HWND = HWND(border_isize as _);
        Self::post_message_w(border_window, WM_NCDESTROY, WPARAM(0), LPARAM(0))
            .context("destroy_border_for_window")
            .log_if_err();
    }
}

// Helpers
fn create_border_struct(tracking_window: HWND, window_rule: &WindowRule) -> WindowBorder {
    let config = CONFIG.read().unwrap();

    let config_width = window_rule
        .rule_match
        .border_width
        .unwrap_or(config.global_rule.border_width);
    let border_offset = window_rule
        .rule_match
        .border_offset
        .unwrap_or(config.global_rule.border_offset);
    let config_radius = window_rule
        .rule_match
        .border_radius
        .clone()
        .unwrap_or(config.global_rule.border_radius.clone());

    let config_active = window_rule
        .rule_match
        .active_color
        .clone()
        .unwrap_or(config.global_rule.active_color.clone());

    let config_inactive = window_rule
        .rule_match
        .inactive_color
        .clone()
        .unwrap_or(config.global_rule.inactive_color.clone());

    let (active_color, inactive_color) = convert_config_colors(&config_active, &config_inactive);

    let animations = window_rule
        .rule_match
        .animations
        .clone()
        .unwrap_or(config.global_rule.animations.clone().unwrap_or_default());

    let dpi = unsafe { GetDpiForWindow(tracking_window) } as f32;
    let border_width = (config_width * dpi / 96.0) as i32;
    let border_radius = convert_config_radius(border_width, config_radius, tracking_window, dpi);

    let initialize_delay = match INITIAL_WINDOWS
        .lock()
        .unwrap()
        .contains(&(tracking_window.0 as isize))
    {
        true => 0,
        false => window_rule
            .rule_match
            .initialize_delay
            .unwrap_or(config.global_rule.initialize_delay.unwrap_or(250)),
    };

    let unminimize_delay = window_rule
        .rule_match
        .unminimize_delay
        .unwrap_or(config.global_rule.unminimize_delay.unwrap_or(200));

    WindowBorder {
        tracking_window,
        border_width,
        border_offset,
        border_radius,
        active_color,
        inactive_color,
        animations,
        unminimize_delay,
        initialize_delay,
        ..Default::default()
    }
}

fn convert_config_radius(
    border_width: i32,
    config_radius: BorderRadius,
    tracking_window: HWND,
    dpi: f32,
) -> f32 {
    let base_radius = (border_width as f32) / 2.0;
    let scale_factor = dpi / 96.0;

    match config_radius {
        BorderRadius::Custom(-1.0) | BorderRadius::Auto => {
            let corner_preference = get_window_corner_preference(tracking_window);

            match corner_preference {
                DWMWCP_DEFAULT | DWMWCP_ROUND => 8.0 * scale_factor + base_radius,
                DWMWCP_ROUNDSMALL => 4.0 * scale_factor + base_radius,
                DWMWCP_DONOTROUND => 0.0,
                _ => base_radius, // fallback default
            }
        }
        BorderRadius::Round => 8.0 * scale_factor + base_radius,
        BorderRadius::SmallRound => 4.0 * scale_factor + base_radius,
        BorderRadius::Square => 0.0,
        BorderRadius::Custom(radius) => radius * scale_factor,
    }
}

fn get_window_corner_preference(tracking_window: HWND) -> DWM_WINDOW_CORNER_PREFERENCE {
    let mut corner_preference = DWM_WINDOW_CORNER_PREFERENCE::default();

    WindowsApi::dwm_get_window_attribute::<DWM_WINDOW_CORNER_PREFERENCE>(
        tracking_window,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &mut corner_preference,
    )
    .context("could not retrieve window corner preference")
    .log_if_err();

    corner_preference
}

fn convert_config_colors(
    color_active: &GlobalColor,
    color_inactive: &GlobalColor,
) -> (Color, Color) {
    (
        Color::fetch(color_active, Some(true)).unwrap(),
        Color::fetch(color_inactive, Some(false)).unwrap(),
    )
}
