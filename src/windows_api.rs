extern crate windows;

use crate::border_config::BorderRadius;
use crate::border_config::MatchKind;
use crate::border_config::MatchStrategy;
use crate::border_config::WindowRule;
use crate::border_config::CONFIG;
use crate::error::LogIfErr;
use crate::windows_callback::enum_windows;
use anyhow::Context;
use anyhow::Result as AnyResult;
use regex::Regex;
use std::ffi::c_void;
use std::ffi::OsString;
use std::hash::Hash;
use std::hash::Hasher;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use win_color::Color;
use win_color::ColorImpl;
use win_color::GlobalColor;
use windows::core::Param;
use windows::core::Result as WinResult;
use windows::core::PCWSTR;
use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
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
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::QueryFullProcessImageNameW;
use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT;
use windows::Win32::UI::Input::Ime::ImmDisableIME;
use windows::Win32::UI::Shell::FOLDERID_Profile;
use windows::Win32::UI::Shell::SHGetKnownFolderPath;
use windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG;
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::SendNotifyMessageW;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows::Win32::UI::WindowsAndMessaging::HMENU;
use windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;
use windows::Win32::UI::WindowsAndMessaging::WNDENUMPROC;
use windows::Win32::UI::WindowsAndMessaging::WS_CHILD;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_WINDOWEDGE;
use windows::Win32::UI::WindowsAndMessaging::WS_MAXIMIZE;

pub const WM_APP_LOCATIONCHANGE: u32 = WM_APP;
pub const WM_APP_REORDER: u32 = WM_APP + 1;
pub const WM_APP_FOREGROUND: u32 = WM_APP + 2;
pub const WM_APP_SHOWUNCLOAKED: u32 = WM_APP + 3;
pub const WM_APP_HIDECLOAKED: u32 = WM_APP + 4;
pub const WM_APP_MINIMIZESTART: u32 = WM_APP + 5;
pub const WM_APP_MINIMIZEEND: u32 = WM_APP + 6;
pub const WM_APP_TIMER: u32 = WM_APP + 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendHWND(pub HWND);
unsafe impl Send for SendHWND {}
unsafe impl Sync for SendHWND {}

impl Hash for SendHWND {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Convert the HWND to a pointer and hash that pointer as a usize
        let hwnd_ptr = self.0 .0 as usize; // Convert HWND to pointer (usize)
        hwnd_ptr.hash(state); // Hash the pointer value
    }
}

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

    pub fn post_quit_message(nexitcode: i32) {
        unsafe { PostQuitMessage(nexitcode) }
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

    #[allow(clippy::too_many_arguments)]
    pub fn create_window_ex_w<P0, P1, P2, P3, P4>(
        dwexstyle: WINDOW_EX_STYLE,
        lpclassname: P0,
        lpwindowname: P1,
        dwstyle: WINDOW_STYLE,
        x: i32,
        y: i32,
        nwidth: i32,
        nheight: i32,
        hwndparent: P2,
        hmenu: P3,
        hinstance: P4,
        lpparam: Option<*const c_void>,
    ) -> WinResult<HWND>
    where
        P0: Param<PCWSTR>,
        P1: Param<PCWSTR>,
        P2: Param<HWND>,
        P3: Param<HMENU>,
        P4: Param<HINSTANCE>,
    {
        unsafe {
            CreateWindowExW(
                dwexstyle,
                lpclassname,
                lpwindowname,
                dwstyle,
                x,
                y,
                nwidth,
                nheight,
                hwndparent,
                hmenu,
                hinstance,
                lpparam,
            )
        }
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

    pub fn enum_windows(callback: WNDENUMPROC, callback_data_address: isize) -> WinResult<()> {
        unsafe { EnumWindows(callback, LPARAM(callback_data_address)) }
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

    pub fn available_window_handles(cb: Option<&dyn Fn(HWND)>) -> AnyResult<Vec<isize>> {
        let mut handles: Vec<isize> = Vec::new();

        // Enumerate windows and collect handles
        Self::enum_windows(Some(enum_windows), &mut handles as *mut Vec<isize> as isize)?;

        if let Some(cb) = cb {
            // Call the provided callback for each handle
            for hwnd_u in &handles {
                let hwnd = HWND(*hwnd_u as *mut _);
                cb(hwnd);
            }
        }

        Ok(handles)
    }

    pub fn get_window_corner_preference(hwnd: HWND) -> DWM_WINDOW_CORNER_PREFERENCE {
        let mut corner_preference = DWM_WINDOW_CORNER_PREFERENCE::default();

        Self::dwm_get_window_attribute::<DWM_WINDOW_CORNER_PREFERENCE>(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &mut corner_preference,
        )
        .context("could not retrieve window corner preference")
        .log_if_err();

        corner_preference
    }

    pub fn home_dir() -> AnyResult<PathBuf> {
        unsafe {
            // Call SHGetKnownFolderPath with NULL token (default user)
            let path_ptr =
                SHGetKnownFolderPath(&FOLDERID_Profile, KNOWN_FOLDER_FLAG(0), HANDLE::default())
                    .unwrap();

            if path_ptr.0.is_null() {
                anyhow::bail!("SHGetKnownFolderPath returned a null pointer");
            }

            // Convert PWSTR to OsString
            let len = (0..).take_while(|&i| *path_ptr.0.add(i) != 0).count();
            let wide_slice = std::slice::from_raw_parts(path_ptr.0, len);
            let os_string = OsString::from_wide(wide_slice);

            // Free the memory allocated by SHGetKnownFolderPath
            CoTaskMemFree(Some(path_ptr.0 as *const _));

            // Return the PathBuf wrapped in Ok
            Ok(PathBuf::from(os_string))
        }
    }
}

pub struct WindowsApiUtility;

impl WindowsApiUtility {
    pub fn convert_config_colors(
        color_active: &GlobalColor,
        color_inactive: &GlobalColor,
    ) -> (Color, Color) {
        (
            Color::fetch(color_active, Some(true)).unwrap(),
            Color::fetch(color_inactive, Some(false)).unwrap(),
        )
    }

    pub fn convert_config_radius(
        border_width: i32,
        config_radius: BorderRadius,
        tracking_window: HWND,
        dpi: f32,
    ) -> f32 {
        let base_radius = (border_width as f32) / 2.0;
        let scale_factor = dpi / 96.0;

        match config_radius {
            BorderRadius::Custom(-1.0) | BorderRadius::Auto => {
                match WindowsApi::get_window_corner_preference(tracking_window) {
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
}
