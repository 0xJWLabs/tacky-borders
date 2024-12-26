mod border;

use crate::as_ptr;
use crate::error::LogIfErr;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_SHOWUNCLOAKED;
use anyhow::Context;
use anyhow::Result as AnyResult;
pub use border::Border;
use rustc_hash::FxHashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::MutexGuard;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassW;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;

static WINDOW_BORDERS: LazyLock<Mutex<FxHashMap<isize, Border>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static ACTIVE_WINDOW: LazyLock<Mutex<isize>> =
    LazyLock::new(|| Mutex::new(WindowsApi::get_foreground_window().0 as isize));

pub fn show_border_for_window(hwnd: isize) {
    // If the border already exists, simply post a 'SHOW' message to its message queue. Otherwise,
    // create a new border.
    if let Some(border) = window_border(hwnd) {
        WindowsApi::post_message_w(
            HWND(as_ptr!(border.border_window)),
            WM_APP_SHOWUNCLOAKED,
            WPARAM(0),
            LPARAM(0),
        )
        .context("show_border_for_window")
        .log_if_err();
    } else if WindowsApi::is_window_visible_on_screen(hwnd) && WindowsApi::is_window_top_level(hwnd)
    {
        let window_rule = WindowsApi::get_window_rule(hwnd);

        if window_rule.match_window.enabled == Some(false) {
            info!("border is disabled for {:?}", HWND(as_ptr!(hwnd)));
        } else if window_rule.match_window.enabled == Some(true)
            || !WindowsApi::has_filtered_style(hwnd)
        {
            Border::create(hwnd, window_rule);
        }
    }
}

pub fn window_borders() -> MutexGuard<'static, FxHashMap<isize, Border>> {
    WINDOW_BORDERS.lock().unwrap()
}

pub fn window_border(hwnd: isize) -> Option<Border> {
    window_borders().get(&hwnd).cloned()
}

pub fn get_active_window() -> MutexGuard<'static, isize> {
    ACTIVE_WINDOW.lock().unwrap()
}

pub fn set_active_window(handle: isize) {
    *ACTIVE_WINDOW.lock().unwrap() = handle;
}

pub fn register_border_class() -> AnyResult<()> {
    unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(Border::wnd_proc),
            hInstance: WindowsApi::module_handle_w()?.into(),
            lpszClassName: w!("border"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };

        let result = RegisterClassW(&wc);
        if result == 0 {
            let last_error = GetLastError();
            error!("could not register window border class: {last_error:?}");
        }
    }

    Ok(())
}

pub fn destroy_all_borders() -> AnyResult<()> {
    let mut borders = window_borders();
    info!("destroying all borders...");

    for (_, border) in borders.iter() {
        WindowsApi::destroy_window(border.border_window)
            .context("reload_borders")
            .log_if_err();
    }

    borders.clear();
    drop(borders);

    Ok(())
}

pub fn reload_borders() {
    if destroy_all_borders().is_ok() {
        info!("reloading borders...");

        WindowsApi::process_window_handles(&Border::create).log_if_err();
    }
}
