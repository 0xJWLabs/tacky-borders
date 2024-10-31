// TODO remove allow unused and fix all the warnings generated
#![allow(unused)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use border_config::Config;
use logger::Logger;
use std::collections::HashMap;
use std::ffi::*;
use std::sync::{Arc, LazyLock, Mutex};
use utils::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Dwm::*,
    Win32::System::SystemServices::IMAGE_DOS_HEADER, Win32::System::Threading::*,
    Win32::UI::Accessibility::*, Win32::UI::WindowsAndMessaging::*,
};

extern "C" {
    pub static __ImageBase: IMAGE_DOS_HEADER;
}

mod border_config;
mod event_hook;
mod logger;
mod sys_tray_icon;
mod utils;
mod window_border;

pub static BORDERS: LazyLock<Mutex<HashMap<isize, isize>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// This shit supposedly unsafe af but it works so idgaf.
pub struct SendHWND(HWND);
unsafe impl Send for SendHWND {}
unsafe impl Sync for SendHWND {}

const DWMWA_COLOR_NONE: u32 = 0xFFFFFFFE;

fn main() {
    let _ = register_window_class();
    apply_colors();
    Logger::log("debug", "Window class in registered!");
    let _ = enum_windows();

    let main_thread = unsafe { GetCurrentThreadId() };
    let tray_icon_option = sys_tray_icon::create_tray_icon(main_thread);
    if tray_icon_option.is_err() {
        Logger::log("error", "Error creating tray icon!");
    }

    let win_event_hook = set_event_hook();
    unsafe {
        Logger::log("debug", "Entering message loop!");
        let mut message = MSG::default();
        while GetMessageW(&mut message, HWND::default(), 0, 0).into() {
            if message.message == WM_CLOSE {
                let result = UnhookWinEvent(win_event_hook);
                if result.as_bool() {
                    ExitProcess(0);
                } else {
                    Logger::log("error", "Could not unhook win even hook");
                }
            }

            TranslateMessage(&message);
            DispatchMessageW(&message);
            std::thread::sleep(std::time::Duration::from_millis(16))
        }
        Logger::log(
            "debug",
            "MESSSAGE LOOP IN MAIN.RS EXITED. THIS SHOULD NOT HAPPEN",
        );
    }
}

pub fn register_window_class() -> Result<()> {
    unsafe {
        let window_class = w!("tacky-border");
        let hinstance: HINSTANCE = std::mem::transmute(&__ImageBase);

        let wcex = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_border::WindowBorder::s_wnd_proc),
            hInstance: hinstance,
            lpszClassName: window_class,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        let result = RegisterClassExW(&wcex);
            
        if result == 0 {
            let last_error = GetLastError();
            Logger::log("error", format!("RegisterClassExW(&wcex): {:?}", last_error).as_str());
        }
    }

    return Ok(());
}

pub fn set_event_hook() -> HWINEVENTHOOK {
    unsafe {
        return SetWinEventHook(
            EVENT_MIN,
            EVENT_MAX,
            None,
            Some(event_hook::handle_win_event_main),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
    }
}

pub fn enum_windows() -> Result<()> {
    let mut windows: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut _ as isize),
            // LPARAM::default(),
        );
    }
    Logger::log("debug", "Windows have been enumerated");
    return Ok(());
}

pub fn restart_borders() {
    let mutex = &*BORDERS;
    let mut borders = mutex.lock().unwrap();
    for value in borders.values() {
        let border_window = HWND(*value as *mut _);
        unsafe { SendMessageW(border_window, WM_DESTROY, WPARAM(0), LPARAM(0)) };
    }
    let _ = borders.drain();
    drop(borders);
    let _ = enum_windows();
}

fn apply_colors() {
    let mut visible_windows: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut visible_windows as *mut _ as isize),
        );
    }

    for hwnd in visible_windows {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &DWMWA_COLOR_NONE as *const _ as *const c_void,
                std::mem::size_of::<c_ulong>() as u32,
            );
        }
    }
}

fn create_borders() {
    let mut windows: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut _ as isize),
        );
    }

    for hwnd in windows {
        create_border_for_window(hwnd, 0);
    }
}

unsafe extern "system" fn enum_windows_callback(_hwnd: HWND, _lparam: LPARAM) -> BOOL {
    // Returning FALSE will exit the EnumWindows loop so we must return TRUE here
    if !is_window_visible(_hwnd) || is_cloaked(_hwnd) || has_filtered_style(_hwnd) || has_filtered_class_or_title(_hwnd) {
        return TRUE;
    }
    let _ = create_border_for_window(_hwnd, 0);

    let visible_windows: &mut Vec<HWND> = std::mem::transmute(_lparam.0);
    visible_windows.push(_hwnd);

    // First, safely cast the LPARAM's inner value (`isize`) to a raw pointer
    return TRUE;
}