#![allow(dead_code)]
extern crate windows;
use crate::app_manager::AppManager;
use crate::border_manager::Border;
use crate::core::rect::Rect;
use crate::error::LogIfErr;
use crate::user_config::MatchKind;
use crate::user_config::WindowRuleConfig;
use crate::windows_callback::enum_windows;
use anyhow::Context;
use anyhow::anyhow;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::ffi::c_void;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::ptr;
use std::thread::JoinHandle;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
use windows::Win32::Foundation::ERROR_INVALID_WINDOW_HANDLE;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::SetLastError;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Graphics::Dwm::DWM_CLOAKED_APP;
use windows::Win32::Graphics::Dwm::DWM_CLOAKED_INHERITED;
use windows::Win32::Graphics::Dwm::DWM_CLOAKED_SHELL;
use windows::Win32::Graphics::Dwm::DWM_WINDOW_CORNER_PREFERENCE;
use windows::Win32::Graphics::Dwm::DWMWA_CLOAKED;
use windows::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS;
use windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE;
use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;
use windows::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows::Win32::Graphics::Gdi::GetMonitorInfoW;
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST;
use windows::Win32::Graphics::Gdi::MONITORINFO;
use windows::Win32::Graphics::Gdi::MonitorFromWindow;
use windows::Win32::Graphics::Gdi::ValidateRect;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetThreadId;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::System::Threading::QueryFullProcessImageNameW;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows::Win32::UI::Input::Ime::ImmDisableIME;
use windows::Win32::UI::Shell::FOLDERID_Profile;
use windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG;
use windows::Win32::UI::Shell::SHGetKnownFolderPath;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GW_HWNDPREV;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::HMENU;
use windows::Win32::UI::WindowsAndMessaging::HWND_TOP;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use windows::Win32::UI::WindowsAndMessaging::MB_OK;
use windows::Win32::UI::WindowsAndMessaging::MB_SYSTEMMODAL;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
use windows::Win32::UI::WindowsAndMessaging::RealGetWindowClassW;
use windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOREDRAW;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOSENDCHANGING;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
use windows::Win32::UI::WindowsAndMessaging::SendNotifyMessageW;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;
use windows::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows::Win32::UI::WindowsAndMessaging::WNDENUMPROC;
use windows::Win32::UI::WindowsAndMessaging::WS_CHILD;
use windows::Win32::UI::WindowsAndMessaging::WS_DISABLED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_WINDOWEDGE;
use windows::Win32::UI::WindowsAndMessaging::WS_MAXIMIZE;
use windows::Win32::UI::WindowsAndMessaging::WS_MINIMIZE;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;
use windows::Win32::UI::WindowsAndMessaging::WS_SYSMENU;
use windows::core::PCWSTR;
use windows::core::PWSTR;
use windows::core::Param;
use windows::core::w;

pub const WM_APP_LOCATIONCHANGE: u32 = WM_APP;
pub const WM_APP_REORDER: u32 = WM_APP + 1;
pub const WM_APP_FOREGROUND: u32 = WM_APP + 2;
pub const WM_APP_SHOWUNCLOAKED: u32 = WM_APP + 3;
pub const WM_APP_HIDECLOAKED: u32 = WM_APP + 4;
pub const WM_APP_MINIMIZESTART: u32 = WM_APP + 5;
pub const WM_APP_MINIMIZEEND: u32 = WM_APP + 6;
pub const WM_APP_TIMER: u32 = WM_APP + 7;

pub trait PointerConversion {
    fn as_int(&self) -> isize;
    fn as_ptr(&self) -> *mut c_void;
    fn as_uint(&self) -> usize;
    fn as_hwnd(&self) -> HWND;
}

pub trait HWNDConversion {
    fn as_int(&self) -> isize;
}

impl HWNDConversion for HWND {
    fn as_int(&self) -> isize {
        self.0.as_int()
    }
}

macro_rules! impl_pointer_conversion {
    ($($t:ty),*) => {
        $(
            impl PointerConversion for $t {
                fn as_int(&self) -> isize {
                    *self as isize
                }
                fn as_ptr(&self) -> *mut c_void {
                    *self as *mut c_void
                }
                fn as_uint(&self) -> usize {
                    *self as usize
                }
                fn as_hwnd(&self) -> HWND {
                    HWND(self.as_ptr())
                }
            }
        )*
    };
}

// Use the macro for multiple types
impl_pointer_conversion!(isize, usize, *mut c_void);

pub struct WindowsApi;

impl WindowsApi {
    #[cfg(target_pointer_width = "64")]
    pub fn set_window_long_ptr_w(hwnd: isize, index: WINDOW_LONG_PTR_INDEX, new_value: isize) {
        let _ = unsafe { SetWindowLongPtrW(hwnd.as_hwnd(), index, new_value) };
    }

    #[cfg(target_pointer_width = "64")]
    pub fn window_long_ptr_w(hwnd: isize, index: WINDOW_LONG_PTR_INDEX) -> isize {
        unsafe { GetWindowLongPtrW(hwnd.as_hwnd(), index) }
    }

    pub fn def_window_proc_w(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> LRESULT {
        unsafe { DefWindowProcW(hwnd.as_hwnd(), msg, WPARAM(wparam), LPARAM(lparam)) }
    }

    pub fn validate_rect(hwnd: Option<isize>, rect: Option<Rect>) -> BOOL {
        let hwnd = hwnd.map(|hwnd| hwnd.as_hwnd());
        let lprect = rect.map(RECT::from);
        let lprect_ptr = lprect.as_ref().map(|r| r as *const RECT);

        unsafe { ValidateRect(hwnd, lprect_ptr) }
    }

    pub fn module_handle_w() -> windows::core::Result<HMODULE> {
        unsafe { GetModuleHandleW(None) }
    }

    pub fn imm_disable_ime() -> BOOL {
        unsafe { ImmDisableIME(0xFFFFFFFF) }
    }

    pub fn set_process_dpi_awareness_context() -> windows::core::Result<()> {
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    }

    pub fn get_dpi_for_window(hwnd: isize) -> anyhow::Result<u32> {
        match unsafe { GetDpiForWindow(hwnd.as_hwnd()) } {
            0 => Err(anyhow!("received invalid dpi of 0 for {hwnd:?}")),
            valid_dpi => Ok(valid_dpi),
        }
    }

    pub fn monitor_from_window(hwnd: isize) -> HMONITOR {
        unsafe { MonitorFromWindow(hwnd.as_hwnd(), MONITOR_DEFAULTTONEAREST) }
    }

    pub fn get_monitor_info(hmonitor: HMONITOR) -> anyhow::Result<MONITORINFO> {
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !unsafe { GetMonitorInfoW(hmonitor, &mut mi) }.as_bool() {
            return Err(anyhow!(
                "could not get monitor info for {:?}: {:?}",
                hmonitor,
                unsafe { GetLastError() }
            ));
        };
        Ok(mi)
    }

    pub fn post_message_w(
        hwnd: Option<HWND>,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> windows::core::Result<()> {
        unsafe { PostMessageW(hwnd, msg, wparam, lparam) }
    }

    pub fn send_notify_message_w(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> windows::core::Result<()> {
        unsafe { SendNotifyMessageW(hwnd, msg, wparam, lparam) }
    }

    pub fn get_message_w(
        lpmsg: *mut MSG,
        hwnd: Option<HWND>,
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

    #[allow(clippy::too_many_arguments)]
    pub fn create_window_ex_w<P1, P2>(
        dwexstyle: WINDOW_EX_STYLE,
        lpclassname: P1,
        lpwindowname: P2,
        dwstyle: WINDOW_STYLE,
        x: i32,
        y: i32,
        nwidth: i32,
        nheight: i32,
        hwndparent: Option<HWND>,
        hmenu: Option<HMENU>,
        hinstance: Option<HINSTANCE>,
        lpparam: Option<*const c_void>,
    ) -> windows::core::Result<HWND>
    where
        P1: Param<PCWSTR>,
        P2: Param<PCWSTR>,
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
        hwnd: isize,
        crkey: COLORREF,
        alpha: u8,
        flags: LAYERED_WINDOW_ATTRIBUTES_FLAGS,
    ) -> windows::core::Result<()> {
        unsafe { SetLayeredWindowAttributes(hwnd.as_hwnd(), crkey, alpha, flags) }
    }

    pub fn dwm_get_window_attribute<T>(
        hwnd: isize,
        attribute: DWMWINDOWATTRIBUTE,
        value: &mut T,
    ) -> windows::core::Result<()> {
        unsafe {
            DwmGetWindowAttribute(
                hwnd.as_hwnd(),
                attribute,
                (value as *mut T).cast(),
                u32::try_from(std::mem::size_of::<T>())?,
            )
        }
    }

    pub fn destroy_window(hwnd: isize) -> anyhow::Result<()> {
        match Self::post_message_w(Some(hwnd.as_hwnd()), WM_NCDESTROY, WPARAM(0), LPARAM(0)) {
            Ok(()) => Ok(()),
            Err(_) => Err(anyhow!("could not destroy window")),
        }
    }

    pub fn enum_windows(
        callback: WNDENUMPROC,
        callback_data_address: isize,
    ) -> windows::core::Result<()> {
        unsafe { EnumWindows(callback, LPARAM(callback_data_address)) }
    }

    pub fn get_window_style(hwnd: isize) -> WINDOW_STYLE {
        unsafe { WINDOW_STYLE(GetWindowLongW(hwnd.as_hwnd(), GWL_STYLE) as u32) }
    }

    pub fn get_window_ex_style(hwnd: isize) -> WINDOW_EX_STYLE {
        unsafe { WINDOW_EX_STYLE(GetWindowLongW(hwnd.as_hwnd(), GWL_EXSTYLE) as u32) }
    }

    pub fn get_foreground_window() -> isize {
        unsafe { GetForegroundWindow().0.as_int() }
    }

    pub fn is_window_cloaked(hwnd: isize) -> bool {
        let mut is_cloaked = 0;
        if let Err(e) = Self::dwm_get_window_attribute(hwnd, DWMWA_CLOAKED, &mut is_cloaked) {
            error!("could not check if window is cloaked: {e}");
            return true;
        }

        matches!(
            is_cloaked,
            DWM_CLOAKED_APP | DWM_CLOAKED_SHELL | DWM_CLOAKED_INHERITED
        )
    }

    pub fn window_rect(hwnd: isize) -> anyhow::Result<Rect> {
        let mut rect = unsafe { std::mem::zeroed() };

        if Self::dwm_get_window_attribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut rect).is_ok() {
            let window_scale = Self::get_dpi_for_window(hwnd)?;
            let system_scale = unsafe { GetDpiForSystem() };
            Ok(Rect::from(rect).scale(system_scale.try_into()?, window_scale.try_into()?))
        } else {
            unsafe { GetWindowRect(hwnd.as_hwnd(), &mut rect) }?;
            Ok(Rect::from(rect))
        }
    }

    pub fn is_window_visible(hwnd: isize) -> bool {
        unsafe { IsWindowVisible(hwnd.as_hwnd()) }.into()
    }

    pub fn is_window_active(hwnd: isize) -> bool {
        Self::get_foreground_window() == hwnd
    }

    #[allow(dead_code)]
    pub fn is_window_minimized(hwnd: isize) -> bool {
        let style = Self::get_window_style(hwnd);

        style.contains(WS_MINIMIZE)
    }

    pub fn is_window_visible_on_screen(hwnd: isize) -> bool {
        Self::is_window_visible(hwnd) && !Self::is_window_cloaked(hwnd)
    }

    pub fn is_window_top_level(hwnd: isize) -> bool {
        let style = Self::get_window_style(hwnd);

        !style.contains(WS_CHILD)
    }

    pub fn has_filtered_style(hwnd: isize) -> bool {
        let ex_style = Self::get_window_ex_style(hwnd);

        ex_style.contains(WS_EX_TOOLWINDOW) || ex_style.contains(WS_EX_NOACTIVATE)
    }

    pub fn has_native_border(hwnd: isize) -> bool {
        let style = Self::get_window_style(hwnd);
        let ex_style = Self::get_window_ex_style(hwnd);

        ex_style.contains(WS_EX_WINDOWEDGE) && !style.contains(WS_MAXIMIZE)
    }

    pub fn get_window_text_w(hwnd: isize, lpstring: &mut [u16]) -> i32 {
        unsafe { GetWindowTextW(hwnd.as_hwnd(), lpstring) }
    }

    pub fn get_window_class_w(hwnd: isize, lpstring: &mut [u16]) -> u32 {
        unsafe { RealGetWindowClassW(hwnd.as_hwnd(), lpstring) }
    }

    pub fn get_window_title(hwnd: isize) -> anyhow::Result<String> {
        let mut buffer: [u16; 256] = [0; 256];

        if Self::get_window_text_w(hwnd, &mut buffer) == 0 {
            let last_error = unsafe { GetLastError() };

            // ERROR_ENVVAR_NOT_FOUND just means the title is empty which isn't necessarily an issue
            // TODO figure out whats with the invalid window handles
            if !matches!(
                last_error,
                ERROR_ENVVAR_NOT_FOUND | ERROR_SUCCESS | ERROR_INVALID_WINDOW_HANDLE
            ) {
                // We manually reset LastError here because it doesn't seem to reset by itself
                unsafe { SetLastError(ERROR_SUCCESS) };
                return Err(anyhow!("failed to retrieve window title: {last_error:?}"));
            }
        }

        Ok(buffer.to_string_lossy().trim_end_matches('\0').to_string())
    }

    pub fn get_window_class(hwnd: isize) -> anyhow::Result<String> {
        let mut buffer: [u16; 256] = [0; 256];

        if Self::get_window_class_w(hwnd, &mut buffer) == 0 {
            let last_error = unsafe { GetLastError() };

            // Handle specific error cases, similar to the GetClassNameW approach
            if !matches!(
                last_error,
                ERROR_ENVVAR_NOT_FOUND | ERROR_SUCCESS | ERROR_INVALID_WINDOW_HANDLE
            ) {
                // Reset LastError as it doesn't seem to reset automatically
                unsafe { SetLastError(ERROR_SUCCESS) };
                return Err(anyhow!("failed to retrieve window class: {last_error:?}"));
            }
        }

        Ok(buffer.to_string_lossy().trim_end_matches('\0').to_string())
    }

    pub fn get_process_name(hwnd: isize) -> anyhow::Result<String> {
        let mut process_id = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd.as_hwnd(), Some(&mut process_id));
        }

        let process_handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) };

        let process_handle = match process_handle {
            Ok(handle) => handle,
            Err(_) => {
                let last_error = unsafe { GetLastError() };
                return Err(anyhow!("{last_error:?}"));
            }
        };

        let mut buffer = [0u16; 256];
        let mut length = buffer.len() as u32;

        let result = unsafe {
            QueryFullProcessImageNameW(
                process_handle,
                PROCESS_NAME_WIN32, // Use 0 to indicate no special flags
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        };

        if result.is_err() {
            let last_error = unsafe { GetLastError() };
            unsafe { CloseHandle(process_handle).ok() };
            return Err(anyhow!("{last_error:?}"));
        }

        unsafe {
            CloseHandle(process_handle).ok(); // Ensure the handle is closed, ignoring the result
        }

        let exe_path = String::from_utf16_lossy(&buffer[..length as usize]);

        let process_name = exe_path
            .split('\\')
            .next_back()
            .and_then(|file_name| file_name.split('.').next()) // Extract the file name without extension
            .unwrap_or("") // Fallback to empty string if parsing fails
            .trim_end_matches('\0')
            .to_string();

        Ok(process_name)
    }

    pub fn get_window_rule(hwnd: isize) -> WindowRuleConfig {
        let title = match Self::get_window_title(hwnd) {
            Ok(val) => val,
            Err(err) => {
                error!("could not retrieve window title for {hwnd:?}: {err}");
                "".to_string()
            }
        };

        let class = match Self::get_window_class(hwnd) {
            Ok(val) => val,
            Err(err) => {
                error!("could not retrieve window class for {hwnd:?}: {err}");
                "".to_string()
            }
        };

        let process = match Self::get_process_name(hwnd) {
            Ok(val) => val,
            Err(err) => {
                error!("could not retrieve process name for {hwnd:?}: {err}");
                "".to_string()
            }
        };

        let config = AppManager::get().config().clone();

        for rule in config.window_rules.iter() {
            let window_name = match rule.match_window.match_kind {
                Some(MatchKind::Title) => &title,
                Some(MatchKind::Process) => &process,
                Some(MatchKind::Class) => &class,
                None => {
                    error!("expected 'kind' for window rule but none found!");
                    continue;
                }
            };

            let Some(match_value) = &rule.match_window.match_value else {
                error!("expected `value` for window rule but non found!");
                continue;
            };

            let has_match = if let Some(strategy) = &rule.match_window.match_strategy {
                strategy.is_match(window_name, match_value)
            } else {
                window_name.to_lowercase().eq(&match_value.to_lowercase())
            };

            if has_match {
                return rule.clone();
            }
        }

        WindowRuleConfig::default()
    }

    pub fn collect_window_handles() -> anyhow::Result<Vec<isize>> {
        let mut handles: Vec<isize> = Vec::new();
        Self::enum_windows(Some(enum_windows), &mut handles as *mut Vec<isize> as isize)?;
        Ok(handles)
    }

    pub fn process_window_handles(
        callback: &dyn Fn(isize, WindowRuleConfig),
    ) -> anyhow::Result<()> {
        let handles = Self::collect_window_handles()?;

        handles.iter().for_each(|&hwnd| {
            if Self::is_window_visible_on_screen(hwnd) {
                let window_rule = Self::get_window_rule(hwnd);

                if window_rule.match_window.enabled == Some(false) {
                    info!("border is disabled for {:?}", hwnd.as_hwnd());
                } else if window_rule.match_window.enabled == Some(true)
                    || !Self::has_filtered_style(hwnd)
                {
                    callback(hwnd, window_rule);
                }
            }
        });

        Ok(())
    }

    pub fn get_window_corner_preference(hwnd: isize) -> DWM_WINDOW_CORNER_PREFERENCE {
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

    pub fn set_border_pos(
        hwnd: isize,
        layout: &Rect,
        position: isize,
        other_flags: Option<SET_WINDOW_POS_FLAGS>,
    ) -> windows::core::Result<()> {
        let hwnd_above_tracking = unsafe { GetWindow(position.as_hwnd(), GW_HWNDPREV) };

        let mut flags =
            SWP_NOSENDCHANGING | SWP_NOACTIVATE | SWP_NOREDRAW | other_flags.unwrap_or_default();

        if hwnd_above_tracking == Ok(hwnd.as_hwnd()) {
            flags |= SWP_NOZORDER;
        }

        Self::set_window_pos(
            hwnd.as_hwnd(),
            layout,
            Some(hwnd_above_tracking.unwrap_or(HWND_TOP)),
            flags,
        )
    }

    pub fn set_window_pos(
        hwnd: HWND,
        layout: &Rect,
        position: Option<HWND>,
        flags: SET_WINDOW_POS_FLAGS,
    ) -> windows::core::Result<()> {
        unsafe {
            SetWindowPos(
                hwnd,
                position,
                layout.left,
                layout.top,
                layout.width(),
                layout.height(),
                flags,
            )
        }
    }

    pub fn create_border_window(name: PCWSTR, border: &mut Border) -> windows::core::Result<isize> {
        match Self::create_window_ex_w(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            w!("border"),
            name,
            WS_POPUP | WS_DISABLED | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(Self::module_handle_w()?.into()),
            Some(ptr::addr_of!(*border) as _),
        ) {
            Ok(window) => Ok(window.0 as isize),
            Err(e) => Err(e),
        }
    }

    pub fn home_dir() -> anyhow::Result<PathBuf> {
        unsafe {
            // Call SHGetKnownFolderPath with NULL token (default user)
            let path_ptr = SHGetKnownFolderPath(
                &FOLDERID_Profile,
                KNOWN_FOLDER_FLAG(0),
                Some(HANDLE::default()),
            )?;

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

    pub fn show_error_dialog(title: &str, message: &str) {
        let title_wide = title.to_wide_string();
        let message_wide = message.to_wide_string();

        unsafe {
            MessageBoxW(
                None,
                PCWSTR(message_wide.as_ptr()),
                PCWSTR(title_wide.as_ptr()),
                MB_ICONERROR | MB_OK | MB_SYSTEMMODAL,
            );
        }
    }

    #[allow(dead_code)]
    pub fn kill_thread_message_loop<T>(thread: &JoinHandle<T>) -> anyhow::Result<()> {
        let handle = thread.as_raw_handle();
        let handle = HANDLE(handle);
        let thread_id = unsafe { GetThreadId(handle) };

        unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM::default(), LPARAM::default()) }?;

        Ok(())
    }
}

pub trait ToWideString: AsRef<OsStr> + Sized {
    fn to_wide_string(&self) -> Vec<u16> {
        to_wide_chars_iter(self).collect()
    }

    fn as_raw_pcwstr(&self) -> PCWSTR {
        let str = self.to_wide_string();
        PCWSTR::from_raw(str.as_ptr())
    }
}

#[allow(clippy::needless_lifetimes)]
fn to_wide_chars_iter<'a>(str: &'a (impl AsRef<OsStr> + ?Sized)) -> impl Iterator<Item = u16> + 'a {
    str.as_ref().encode_wide().chain(once(0))
}

impl<T: AsRef<OsStr> + Sized> ToWideString for T {}

pub trait FromWideString: AsRef<[u16]> + Sized {
    fn to_string_lossy(&self) -> String {
        self.to_os_string().to_string_lossy().into_owned()
    }

    fn to_os_string(&self) -> OsString {
        OsString::from_wide(self.as_ref())
    }
}
impl<T: AsRef<[u16]> + Sized> FromWideString for T {}
