use border_config::CONFIG;

use crate::*;
use crate::utils::*;

pub fn animation_manager() {
    std::thread::spawn(|| loop {
        let mutex = &*BORDERS;
        let borders = mutex.lock().unwrap();

        for value in borders.values() {
            let border_window: HWND = HWND(*value as _);
            if is_window_visible(border_window) {
                unsafe { let _ = PostMessageW(border_window, WM_PAINT, WPARAM(0), LPARAM(0)); }
            }
        }
        drop(borders);
        std::thread::sleep(std::time::Duration::from_millis(100));
    });
}