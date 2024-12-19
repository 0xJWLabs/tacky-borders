// #![allow(unused)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#[macro_use]
extern crate log;
extern crate sp_log;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result as AnyResult;
use border_config::ConfigImpl;
use error::LogIfErr;
use rustc_hash::FxHashMap;
use sp_log::ColorChoice;
use sp_log::CombinedLogger;
use sp_log::Config;
use sp_log::FileLogger;
use sp_log::LevelFilter;
use sp_log::TermLogger;
use sp_log::TerminalMode;
use std::mem::transmute;
use std::sync::LazyLock;
use std::sync::Mutex;
use sys_tray::SystemTray;
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
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassExW;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXW;
use windows_api::WindowsApi;

mod animations;
mod border_config;
mod error;
mod keyboard_hook;
mod sys_tray;
mod window_border;
mod window_event_hook;
mod windows_api;

extern "C" {
    static __ImageBase: IMAGE_DOS_HEADER;
}

static BORDERS: LazyLock<Mutex<FxHashMap<isize, isize>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static INITIAL_WINDOWS: LazyLock<Mutex<Vec<isize>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn main() {
    if let Err(e) = create_logger() {
        println!("[ERROR] {}", e);
    };

    if !WindowsApi::imm_disable_ime(0xFFFFFFFF).as_bool() {
        error!("could not disable ime!");
    }

    if let Err(e) =
        WindowsApi::set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
    {
        error!("could not make process dpi aware: {e}");
    }

    let tray = SystemTray::new();

    if let Err(e) = tray {
        error!("could not create sys tray: {e}");
    }

    register_window_class().log_if_err();
    WindowsApi::enum_windows().log_if_err();

    debug!("entering message loop!");
    let mut message = MSG::default();

    while WindowsApi::get_message_w(&mut message, HWND::default(), 0, 0).into() {
        let _ = WindowsApi::translate_message(&message);
        WindowsApi::dispatch_message_w(&message);
    }

    error!("exited messsage loop in main.rs; this should not happen");
}

fn create_logger() -> AnyResult<()> {
    let log_path = border_config::Config::get_config_dir()?.join("tacky-borders.log");
    let Some(log_path) = log_path.to_str() else {
        return Err(anyhow!("could not convert log_path to str"));
    };

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
        FileLogger::new(
            LevelFilter::Info,
            Config::default(),
            log_path,
            Some(1024 * 1024),
        ),
    ])?;

    Ok(())
}

fn register_window_class() -> Result<()> {
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
            error!("could not register window class: {last_error:?}");
        }
    }

    Ok(())
}

fn reload_borders() {
    let mut borders = BORDERS.lock().unwrap();
    for value in borders.values() {
        let border_window = HWND(*value as _);
        WindowsApi::post_message_w(border_window, WM_NCDESTROY, WPARAM(0), LPARAM(0))
            .context("reload_borders")
            .log_if_err();
    }

    // Clear the borders hashmap
    borders.clear();
    drop(borders);

    INITIAL_WINDOWS.lock().unwrap().clear();

    WindowsApi::enum_windows().log_if_err();
}

unsafe extern "system" fn enum_windows_callback(_hwnd: HWND, _lparam: LPARAM) -> BOOL {
    if !WindowsApi::has_filtered_style(_hwnd) {
        if WindowsApi::is_window_visible(_hwnd) && !WindowsApi::is_window_cloaked(_hwnd) {
            WindowsApi::create_border_for_window(_hwnd);
        }

        INITIAL_WINDOWS.lock().unwrap().push(_hwnd.0 as isize);
    }

    TRUE
}
