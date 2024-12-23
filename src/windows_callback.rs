use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;

use crate::windows_api::WindowsApi;

pub extern "system" fn enum_windows(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<isize>) };
    if WindowsApi::is_window_top_level(hwnd) {
        windows.push(hwnd.0 as isize);
    }

    true.into()
}
