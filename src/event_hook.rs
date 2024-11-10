use windows::{
    Win32::Foundation::*, Win32::UI::Accessibility::*, Win32::UI::WindowsAndMessaging::*,
};

use crate::BORDERS;
use crate::winapi::*;

pub extern "system" fn handle_win_event(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _dw_event_thread: u32,
    _dwms_event_time: u32,
) {
    if _id_object == OBJID_CURSOR.0 {
        return;
    }

    match _event {
        EVENT_OBJECT_LOCATIONCHANGE => {
            if WinApi::has_filtered_style(_hwnd) {
                return;
            }

            let border_window = WinApi::get_border_from_window(_hwnd);
            if let Some(hwnd) = border_window {
                unsafe {
                    let _ = SendNotifyMessageW(hwnd, WM_APP_LOCATIONCHANGE, WPARAM(0), LPARAM(0));
                }
            }
        }
        EVENT_OBJECT_REORDER => {
            if WinApi::has_filtered_style(_hwnd) {
                return;
            }

            let borders = BORDERS.lock().unwrap();

            for value in borders.values() {
                let border_window: HWND = HWND(*value as _);
                if WinApi::is_window_visible(border_window) {
                    unsafe {
                        let _ = PostMessageW(border_window, WM_APP_REORDER, WPARAM(0), LPARAM(0));
                    }
                }
            }
            drop(borders);
        }
        EVENT_OBJECT_SHOW | EVENT_OBJECT_UNCLOAKED => {
            WinApi::show_border_for_window(_hwnd);
        }
        EVENT_OBJECT_HIDE => {
            // I have to check IsWindowVisible because for some reason, EVENT_OBJECT_HIDE can be
            // sent even while the window is still visible (it does this for Vesktop)
            if !WinApi::is_window_visible(_hwnd) {
                WinApi::hide_border_for_window(_hwnd);
            }
        }
        EVENT_OBJECT_CLOAKED => {
            WinApi::hide_border_for_window(_hwnd);
        }
        EVENT_SYSTEM_MINIMIZESTART => {
            let border_option = WinApi::get_border_from_window(_hwnd);
            if let Some(border_window) = border_option {
                unsafe {
                    let _ = PostMessageW(border_window, WM_APP_MINIMIZESTART, WPARAM(0), LPARAM(0));
                }
            }
        }
        EVENT_SYSTEM_MINIMIZEEND => {
            let border_option = WinApi::get_border_from_window(_hwnd);
            if let Some(border_window) = border_option {
                unsafe {
                    let _ = PostMessageW(border_window, WM_APP_MINIMIZEEND, WPARAM(0), LPARAM(0));
                }
            }
        }
        // TODO this is called an unnecessary number of times which may hurt performance?
        EVENT_OBJECT_DESTROY => {
            if !WinApi::has_filtered_style(_hwnd) {
                let _ = WinApi::destroy_border_for_window(_hwnd);

                // Use below to debug whether window borders are properly destroyed
                /*let mutex = &*BORDERS;
                let borders = mutex.lock().unwrap();
                println!("borders after destroying window: {:?}", borders);
                drop(borders);*/
            }
        }
        _ => {}
    }
}