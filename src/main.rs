// #![allow(unused)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#[macro_use]
extern crate log;
extern crate sp_log;

use anyhow::anyhow;
use anyhow::Result as AnyResult;
use border_config::ConfigImpl;
use border_manager::create_border_for_window;
use border_manager::register_border_class;
use error::LogIfErr;
use sp_log::ColorChoice;
use sp_log::CombinedLogger;
use sp_log::Config;
use sp_log::FileLogger;
use sp_log::LevelFilter;
use sp_log::TermLogger;
use sp_log::TerminalMode;
use sys_tray::SystemTray;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows_api::WindowsApi;

mod animations;
mod border_config;
mod border_manager;
mod error;
mod keyboard_hook;
mod sys_tray;
mod window_event_hook;
mod windows_api;
mod windows_callback;

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

    let sys_tray = SystemTray::new();

    if let Err(e) = sys_tray {
        error!("could not create sys tray: {e}");
    }

    register_border_class().log_if_err();
    WindowsApi::available_window_handles(Some(&create_border_for_window)).unwrap();

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
