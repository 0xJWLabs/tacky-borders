mod border;

use crate::app_manager::AppManager;
use crate::error::LogIfErr;
use crate::windows_api::WindowsApi;
use anyhow::Context;
pub use border::Border;
#[cfg(feature = "fast-hash")]
use fx_hash::FxHashMap as HashMap;
#[cfg(not(feature = "fast-hash"))]
use std::collections::HashMap;
use std::sync::MutexGuard;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassW;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;

pub fn window_borders() -> MutexGuard<'static, HashMap<isize, Border>> {
    AppManager::get().borders()
}

pub fn window_border(hwnd: isize) -> Option<Border> {
    window_borders().get(&hwnd).cloned()
}

pub fn get_active_window() -> MutexGuard<'static, isize> {
    AppManager::get().active_window()
}

pub fn set_active_window(handle: isize) {
    AppManager::get().set_active_window(handle);
}

pub fn register_border_class() -> anyhow::Result<()> {
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

pub fn destroy_all_borders() -> anyhow::Result<()> {
    let mut borders = window_borders();
    info!("[destroy_all_borders] Borders: Destroying");

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
        info!("[reload_borders] Borders: Destroyed successfully");

        WindowsApi::process_window_handles(&Border::create).log_if_err();
    }
}
