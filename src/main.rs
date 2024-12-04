// #![allow(unused)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#[macro_use]
extern crate log;
extern crate sp_log;

use anyhow::Result as AnyResult;

use sp_log::ColorChoice;
use sp_log::CombinedLogger;
use sp_log::Config;
use sp_log::LevelFilter;
use sp_log::TermLogger;
use sp_log::TerminalMode;
use sp_log::WriteLogger;
use std::cell::Cell;
use std::collections::HashMap;
use std::mem::transmute;
use std::sync::LazyLock;
use std::sync::Mutex;
use utils::get_log;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
use windows_api::WindowsApi;

use windows::core::w;
use windows::core::Result;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::TRUE;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;
use windows::Win32::UI::Accessibility::SetWinEventHook;
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassExW;
use windows::Win32::UI::WindowsAndMessaging::EVENT_MAX;
use windows::Win32::UI::WindowsAndMessaging::EVENT_MIN;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WINEVENT_OUTOFCONTEXT;
use windows::Win32::UI::WindowsAndMessaging::WINEVENT_SKIPOWNPROCESS;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXW;

mod animations;
mod border_config;
mod colors;
mod deserializer;
mod event_hook;
mod keybinding;
mod sys_tray_icon;
mod timer;
mod utils;
mod window_border;
mod windows_api;

extern "C" {
    pub static __ImageBase: IMAGE_DOS_HEADER;
}

thread_local! {
    pub static EVENT_HOOK: Cell<HWINEVENTHOOK> = Cell::new(HWINEVENTHOOK::default());
}

pub static BORDERS: LazyLock<Mutex<HashMap<isize, isize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub static INITIAL_WINDOWS: LazyLock<Mutex<Vec<isize>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn main() {
    match create_logger() {
        Ok(_) => {}
        Err(err) => println!("Error: {}", err),
    };

    if !WindowsApi::imm_disable_ime(0xFFFFFFFF).as_bool() {
        error!("could not disable IME!");
    }

    if WindowsApi::set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
        .is_err()
    {
        error!("Failed to make process DPI aware");
    }

    let tray_icon_option = sys_tray_icon::create_tray_icon();
    if tray_icon_option.is_err() {
        error!("failed to build tray icon");
    }

    EVENT_HOOK.replace(set_event_hook());
    let _ = register_window_class();
    let _ = WindowsApi::enum_windows();

    debug!("entering message loop!");
    let mut message = MSG::default();

    while WindowsApi::get_message_w(&mut message, HWND::default(), 0, 0).into() {
        let _ = WindowsApi::translate_message(&message);
        WindowsApi::dispatch_message_w(&message);
    }

    error!("MESSAGE LOOP IN MAIN.RS EXITED. THIS SHOULD NOT HAPPEN");
}

fn create_logger() -> AnyResult<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::Info, Config::default(), get_log()?),
    ])?;

    Ok(())
}

pub fn register_window_class() -> Result<()> {
    unsafe {
        let hinstance: HINSTANCE = transmute(&__ImageBase);

        let wcex = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_border::WindowBorder::s_wnd_proc),
            hInstance: hinstance,
            lpszClassName: w!("border"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };

        let result = RegisterClassExW(&wcex);
        if result == 0 {
            let last_error = GetLastError();
            error!("ERROR: RegisterClassExW(&wcex): {:?}", last_error);
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

pub fn reload_borders() {
    let mut borders = BORDERS.lock().unwrap();
    for value in borders.values() {
        let border_window = HWND(*value as _);
        let _ = WindowsApi::post_message_w(border_window, WM_CLOSE, WPARAM(0), LPARAM(0));
    }

    // Clear the borders hashmap
    borders.clear();
    drop(borders);

    INITIAL_WINDOWS.lock().unwrap().clear();

    let _ = WindowsApi::enum_windows();
}

unsafe extern "system" fn enum_windows_callback(_hwnd: HWND, _lparam: LPARAM) -> BOOL {
    if !WindowsApi::has_filtered_style(_hwnd) {
        if WindowsApi::is_window_visible(_hwnd) && !WindowsApi::is_window_cloaked(_hwnd) {
            let _ = WindowsApi::create_border_for_window(_hwnd);
        }

        INITIAL_WINDOWS.lock().unwrap().push(_hwnd.0 as isize);
    }

    TRUE
}
