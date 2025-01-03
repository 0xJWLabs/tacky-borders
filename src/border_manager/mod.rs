mod border;

use crate::core::app_state::APP_STATE;
use crate::error::LogIfErr;
use crate::windows_api::WindowsApi;
use anyhow::Context;
use anyhow::Result as AnyResult;
pub use border::Border;
use rustc_hash::FxHashMap;
use std::sync::MutexGuard;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassW;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;

pub fn window_borders() -> MutexGuard<'static, FxHashMap<isize, Border>> {
    APP_STATE.borders.lock().unwrap()
}

pub fn window_border(hwnd: isize) -> Option<Border> {
    window_borders().get(&hwnd).cloned()
}

pub fn get_active_window() -> MutexGuard<'static, isize> {
    APP_STATE.active_window.lock().unwrap()
}

pub fn set_active_window(handle: isize) {
    *APP_STATE.active_window.lock().unwrap() = handle;
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
