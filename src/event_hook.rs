use crate::error::LogIfErr;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_FOREGROUND;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::BORDERS;
use anyhow::Context;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_CLOAKED;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_HIDE;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_LOCATIONCHANGE;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_REORDER;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_SHOW;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_UNCLOAKED;
use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_FOREGROUND;
use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_MINIMIZEEND;
use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_MINIMIZESTART;
use windows::Win32::UI::WindowsAndMessaging::OBJID_CLIENT;
use windows::Win32::UI::WindowsAndMessaging::OBJID_CURSOR;
use windows::Win32::UI::WindowsAndMessaging::OBJID_WINDOW;

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
            if WindowsApi::has_filtered_style(_hwnd) {
                return;
            }

            if let Some(border) = WindowsApi::get_border_from_window(_hwnd) {
                WindowsApi::send_notify_message_w(
                    border,
                    WM_APP_LOCATIONCHANGE,
                    WPARAM(0),
                    LPARAM(0),
                )
                .with_context(|| "EVENT_OBJECT_LOCATIONCHANGE")
                .log_if_err();
            }
        }
        EVENT_OBJECT_REORDER => {
            if WindowsApi::has_filtered_style(_hwnd) {
                return;
            }

            let borders = BORDERS.lock().unwrap();

            for value in borders.values() {
                let border_window: HWND = HWND(*value as _);
                if WindowsApi::is_window_visible(border_window) {
                    WindowsApi::post_message_w(border_window, WM_APP_REORDER, WPARAM(0), LPARAM(0))
                        .with_context(|| "EVENT_OBJECT_REORDER")
                        .log_if_err();
                }
            }
            drop(borders);
        }
        EVENT_SYSTEM_FOREGROUND => {
            for (key, val) in BORDERS.lock().unwrap().iter() {
                let border_window: HWND = HWND(*val as _);
                // Some apps like Flow Launcher can become focused even if they aren't visible yet,
                // so I also need to check if 'key' is equal to '_hwnd' (the foreground window)
                if WindowsApi::is_window_visible(border_window) || key == &(_hwnd.0 as isize) {
                    WindowsApi::post_message_w(
                        border_window,
                        WM_APP_FOREGROUND,
                        WPARAM(0),
                        LPARAM(0),
                    )
                    .with_context(|| "EVENT_OBJECT_FOCUS")
                    .log_if_err();
                }
            }
        }
        EVENT_OBJECT_SHOW | EVENT_OBJECT_UNCLOAKED => {
            if _id_object == OBJID_WINDOW.0 {
                WindowsApi::show_border_for_window(_hwnd);
            }
        }
        EVENT_OBJECT_HIDE | EVENT_OBJECT_CLOAKED => {
            if _id_object == OBJID_WINDOW.0 {
                WindowsApi::hide_border_for_window(_hwnd);
            }
        }
        EVENT_SYSTEM_MINIMIZESTART => {
            if let Some(border) = WindowsApi::get_border_from_window(_hwnd) {
                WindowsApi::post_message_w(border, WM_APP_MINIMIZESTART, WPARAM(0), LPARAM(0))
                    .with_context(|| "EVENT_SYSTEM_MINIMIZESTART")
                    .log_if_err();
            }
        }
        EVENT_SYSTEM_MINIMIZEEND => {
            if let Some(border) = WindowsApi::get_border_from_window(_hwnd) {
                WindowsApi::post_message_w(border, WM_APP_MINIMIZEEND, WPARAM(0), LPARAM(0))
                    .with_context(|| "EVENT_SYSTEM_MINIMIZEEND")
                    .log_if_err();
            }
        }
        // TODO this is called an unnecessary number of times which may hurt performance?
        EVENT_OBJECT_DESTROY => {
            if (_id_object == OBJID_WINDOW.0 || _id_object == OBJID_CLIENT.0)
                && !WindowsApi::has_filtered_style(_hwnd)
            {
                WindowsApi::destroy_border_for_window(_hwnd);
            }
        }
        _ => {}
    }
}
