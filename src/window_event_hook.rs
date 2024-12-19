use crate::error::LogIfErr;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_FOREGROUND;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::BORDERS;
use anyhow::Context;
use anyhow::Result as AnyResult;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Accessibility::SetWinEventHook;
use windows::Win32::UI::Accessibility::UnhookWinEvent;
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
use windows::Win32::UI::WindowsAndMessaging::WINEVENT_OUTOFCONTEXT;
use windows::Win32::UI::WindowsAndMessaging::WINEVENT_SKIPOWNPROCESS;

pub static WIN_EVENT_HOOK: OnceLock<Arc<WindowEventHook>> = OnceLock::new();

#[derive(Debug)]
pub struct WindowEventHook {
    hook_handles: Mutex<Vec<HWINEVENTHOOK>>,
}

unsafe impl Send for WindowEventHook {}
unsafe impl Sync for WindowEventHook {}

impl WindowEventHook {
    pub fn new() -> anyhow::Result<Arc<Self>> {
        let win_event_hook = Arc::new(Self {
            hook_handles: Mutex::new(Vec::new()),
        });

        WIN_EVENT_HOOK
            .set(win_event_hook.clone())
            .map_err(|_| anyhow::anyhow!("Window event hook already running."))?;

        Ok(win_event_hook)
    }

    pub fn start(&self) -> anyhow::Result<()> {
        println!("hook started");
        let mut hook_handles = self.hook_handles.lock().unwrap();
        *hook_handles = Self::hook_win_events()?;

        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        for hook_handle in self.hook_handles.lock().unwrap().drain(..) {
            unsafe { UnhookWinEvent(hook_handle) }.ok()?;
        }

        Ok(())
    }

    fn hook_win_events() -> AnyResult<Vec<HWINEVENTHOOK>> {
        let event_ranges = [
            (EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
            (EVENT_OBJECT_REORDER, EVENT_OBJECT_REORDER),
        ];

        // Create separate hooks for each event range. This is more performant
        // than creating a single hook for all events and filtering them.
        event_ranges
            .iter()
            .try_fold(Vec::new(), |mut handles, event_range| {
                let hook_handle = Self::hook_win_event(event_range.0, event_range.1)?;
                handles.push(hook_handle);
                Ok(handles)
            })
    }

    /// Creates a window hook for the specified event range.
    fn hook_win_event(event_min: u32, event_max: u32) -> AnyResult<HWINEVENTHOOK> {
        let hook_handle = unsafe {
            SetWinEventHook(
                event_min,
                event_max,
                None,
                Some(window_event_hook_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };

        if hook_handle.is_invalid() {
            Err(anyhow::anyhow!("Failed to set window event hook."))
        } else {
            Ok(hook_handle)
        }
    }

    fn handle_event(&self, event_type: u32, handle: HWND) {
        match event_type {
            EVENT_OBJECT_LOCATIONCHANGE => {
                if WindowsApi::has_filtered_style(handle) {
                    return;
                }

                if let Some(border) = WindowsApi::get_border_from_window(handle) {
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
                if WindowsApi::has_filtered_style(handle) {
                    return;
                }

                let borders = BORDERS.lock().unwrap();

                for value in borders.values() {
                    let border_window: HWND = HWND(*value as _);
                    if WindowsApi::is_window_visible(border_window) {
                        WindowsApi::post_message_w(
                            border_window,
                            WM_APP_REORDER,
                            WPARAM(0),
                            LPARAM(0),
                        )
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
                    if WindowsApi::is_window_visible(border_window) || key == &(handle.0 as isize) {
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
                WindowsApi::show_border_for_window(handle);
            }
            EVENT_OBJECT_HIDE | EVENT_OBJECT_CLOAKED => {
                WindowsApi::hide_border_for_window(handle);
            }
            EVENT_SYSTEM_MINIMIZESTART => {
                if let Some(border) = WindowsApi::get_border_from_window(handle) {
                    WindowsApi::post_message_w(border, WM_APP_MINIMIZESTART, WPARAM(0), LPARAM(0))
                        .with_context(|| "EVENT_SYSTEM_MINIMIZESTART")
                        .log_if_err();
                }
            }
            EVENT_SYSTEM_MINIMIZEEND => {
                if let Some(border) = WindowsApi::get_border_from_window(handle) {
                    WindowsApi::post_message_w(border, WM_APP_MINIMIZEEND, WPARAM(0), LPARAM(0))
                        .with_context(|| "EVENT_SYSTEM_MINIMIZEEND")
                        .log_if_err();
                }
            }
            // TODO this is called an unnecessary number of times which may hurt performance?
            EVENT_OBJECT_DESTROY => {
                if !WindowsApi::has_filtered_style(handle) {
                    WindowsApi::destroy_border_for_window(handle);
                }
            }
            _ => {}
        }
    }
}

extern "system" fn window_event_hook_proc(
    _hook: HWINEVENTHOOK,
    event_type: u32,
    handle: HWND,
    id_object: i32,
    id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if id_object == OBJID_CURSOR.0 {
        return;
    }

    let is_window_event = (id_object == OBJID_WINDOW.0 || id_object == OBJID_CLIENT.0)
        && id_child == 0
        && handle != HWND(std::ptr::null_mut());

    // Check whether the event is associated with a window object instead
    // of a UI control.
    if !is_window_event {
        return;
    }

    if let Some(hook) = WIN_EVENT_HOOK.get() {
        hook.handle_event(event_type, handle);
    }
}
