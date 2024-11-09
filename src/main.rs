// #![allow(unused)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[macro_use]
extern crate log;
extern crate sp_log;

use sp_log::{
    ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::c_ulong;
use std::sync::{LazyLock, Mutex};
use std::thread;
use utils::*;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE};

use windows::{
    core::*, Win32::Foundation::*, Win32::System::SystemServices::IMAGE_DOS_HEADER,
    Win32::UI::Accessibility::*, Win32::UI::HiDpi::*, Win32::UI::Input::Ime::*,
    Win32::UI::WindowsAndMessaging::*,
};

mod border_config;
mod colors;
mod event_hook;
mod sys_tray_icon;
mod utils;
mod window_border;

extern "C" {
    pub static __ImageBase: IMAGE_DOS_HEADER;
}

thread_local! {
    pub static EVENT_HOOK: Cell<HWINEVENTHOOK> = Cell::new(HWINEVENTHOOK::default());
}

pub static BORDERS: LazyLock<Mutex<HashMap<isize, isize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub static INITIAL_WINDOWS: LazyLock<Mutex<Vec<isize>>> = LazyLock::new(|| Mutex::new(Vec::new()));

// This shit supposedly unsafe af but it works so idgaf.
#[derive(Debug, PartialEq, Clone)]
pub struct SendHWND(HWND);
unsafe impl Send for SendHWND {}
unsafe impl Sync for SendHWND {}

fn create_logger() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::Warn, Config::default(), get_log().unwrap()),
    ])
    .unwrap();
}

fn main() {
    create_logger();
    if unsafe { !ImmDisableIME(std::mem::transmute::<i32, u32>(-1)).as_bool() } {
        println!("Could not disable IME!");
    }

    if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_err() }
    {
        println!("Failed to make process DPI aware");
    }

    let tray_icon_option = sys_tray_icon::create_tray_icon();
    if tray_icon_option.is_err() {
        error!("Window class in registered!");
    }

    EVENT_HOOK.replace(set_event_hook());
    let _ = register_window_class();
    let _ = enum_windows();
    unsafe {
        debug!("Entering message loop!");
        let mut message = MSG::default();
        while GetMessageW(&mut message, HWND::default(), 0, 0).into() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
            thread::sleep(std::time::Duration::from_millis(16))
        }
        debug!("MESSSAGE LOOP IN MAIN.RS EXITED. THIS SHOULD NOT HAPPEN");
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
            println!("ERROR: RegisterClassExW(&wcex): {:?}", last_error);
        }
    }

    Ok(())
}

pub fn set_event_hook() -> HWINEVENTHOOK {
    unsafe {
        SetWinEventHook(
            EVENT_MIN,
            EVENT_MAX,
            None,
            Some(event_hook::handle_win_event),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
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
    debug!("Windows have been enumerated");

    Ok(())
}

pub fn restart_borders() {
    let mutex = &*BORDERS;
    let mut borders = mutex.lock().unwrap();
    for value in borders.values() {
        let border_window = HWND(*value as _);
        unsafe {
            let _ = PostMessageW(border_window, WM_DESTROY, WPARAM(0), LPARAM(0));
        }
    }
    borders.clear();
    drop(borders);
    let _ = enum_windows();
}

// Might use it to remove native border
fn _remove_border() {
    let mut visible_windows: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut visible_windows as *mut _ as isize),
        );
    }

    for hwnd in visible_windows {
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &DWMWA_COLOR_NONE as *const _ as _,
                std::mem::size_of::<c_ulong>() as u32,
            );
        }
    }
}

unsafe extern "system" fn enum_windows_callback(_hwnd: HWND, _lparam: LPARAM) -> BOOL {
    if !has_filtered_style(_hwnd) {
        if is_window_visible(_hwnd) && !is_cloaked(_hwnd) {
            let _ = create_border_for_window(_hwnd);
        }

        INITIAL_WINDOWS.lock().unwrap().push(_hwnd.0 as isize);
    }

    // let visible_windows: &mut Vec<HWND> = std::mem::transmute(_lparam.0);
    // visible_windows.push(_hwnd);

    TRUE
}
