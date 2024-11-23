use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_FOCUS;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::BORDERS;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;
use windows::Win32::UI::WindowsAndMessaging::GetAncestor;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::Win32::UI::WindowsAndMessaging::SendNotifyMessageW;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_CLOAKED;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_FOCUS;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_HIDE;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_LOCATIONCHANGE;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_REORDER;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_SHOW;
use windows::Win32::UI::WindowsAndMessaging::EVENT_OBJECT_UNCLOAKED;
use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_MINIMIZEEND;
use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_MINIMIZESTART;
use windows::Win32::UI::WindowsAndMessaging::GA_ROOT;
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

            let border_window = WindowsApi::get_border_from_window(_hwnd);
            if let Some(hwnd) = border_window {
                unsafe {
                    let _ = SendNotifyMessageW(hwnd, WM_APP_LOCATIONCHANGE, WPARAM(0), LPARAM(0));
                }
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
                    unsafe {
                        let _ = PostMessageW(border_window, WM_APP_REORDER, WPARAM(0), LPARAM(0));
                    }
                }
            }
            drop(borders);
        }
        EVENT_OBJECT_FOCUS => {
            // TODO not sure if I should use GA_ROOT or GA_ROOTOWNER
            //let before = std::time::Instant::now();
            let parent = unsafe { GetAncestor(_hwnd, GA_ROOT) };
            //println!("time elapsed: {:?}", before.elapsed());

            if WindowsApi::has_filtered_style(parent) {
                return;
            }

            for val in BORDERS.lock().unwrap().values() {
                let border_window: HWND = HWND(*val as _);
                if WindowsApi::is_window_visible(border_window) {
                    unsafe {
                        let _ = PostMessageW(border_window, WM_APP_FOCUS, WPARAM(0), LPARAM(0));
                    }
                }
            }
        }
        EVENT_OBJECT_SHOW | EVENT_OBJECT_UNCLOAKED => {
            WindowsApi::show_border_for_window(_hwnd);
        }
        EVENT_OBJECT_HIDE => {
            // I have to check IsWindowVisible because for some reason, EVENT_OBJECT_HIDE can be
            // sent even while the window is still visible (it does this for Vesktop)
            if !WindowsApi::is_window_visible(_hwnd) {
                WindowsApi::hide_border_for_window(_hwnd);
            }
        }
        EVENT_OBJECT_CLOAKED => {
            WindowsApi::hide_border_for_window(_hwnd);
        }
        EVENT_SYSTEM_MINIMIZESTART => {
            let border_option = WindowsApi::get_border_from_window(_hwnd);
            if let Some(border_window) = border_option {
                unsafe {
                    let _ = PostMessageW(border_window, WM_APP_MINIMIZESTART, WPARAM(0), LPARAM(0));
                }
            }
        }
        EVENT_SYSTEM_MINIMIZEEND => {
            let border_option = WindowsApi::get_border_from_window(_hwnd);
            if let Some(border_window) = border_option {
                unsafe {
                    let _ = PostMessageW(border_window, WM_APP_MINIMIZEEND, WPARAM(0), LPARAM(0));
                }
            }
        }
        // TODO this is called an unnecessary number of times which may hurt performance?
        EVENT_OBJECT_DESTROY => {
            if _id_object == OBJID_WINDOW.0 && !WindowsApi::has_filtered_style(_hwnd) {
                let _ = WindowsApi::destroy_border_for_window(_hwnd);
            }
        }
        _ => {}
    }
}
