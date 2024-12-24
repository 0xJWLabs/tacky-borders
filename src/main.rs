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
use border_manager::create_border_for_window;
use border_manager::register_border_class;
use border_manager::reload_borders;
use error::LogIfErr;
use keyboard_hook::KeybindingConfig;
use keyboard_hook::KeyboardHook;
use keyboard_hook::KEYBOARD_HOOK;
use sp_log::ColorChoice;
use sp_log::CombinedLogger;
use sp_log::Config;
use sp_log::FileLogger;
use sp_log::LevelFilter;
use sp_log::TermLogger;
use sp_log::TerminalMode;
use sys_tray::SystemTray;
use sys_tray::SystemTrayEvent;
use user_config::UserConfig;
use user_config::CONFIG;
use window_event_hook::WindowEventHook;
use window_event_hook::WIN_EVENT_HOOK;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows_api::WindowsApi;

mod animations;
mod border_manager;
mod error;
mod keyboard_hook;
mod sys_tray;
mod user_config;
mod window_event_hook;
mod windows_api;
mod windows_callback;

fn main() -> AnyResult<()> {
    let res = start_app();

    if let Err(err) = &res {
        error!("{err:?}");
        WindowsApi::show_error_dialog("Fatal error", &err.to_string());
    } else {
        debug!("exiting tacky-borders...");
    }

    res
}

fn start_app() -> AnyResult<()> {
    if let Err(e) = &create_logger() {
        error!("logger initialization failed: {e}");
    };

    if !WindowsApi::imm_disable_ime(0xFFFFFFFF).as_bool() {
        error!("could not disable ime!");
    }

    if let Err(e) =
        WindowsApi::set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
    {
        error!("could not make process dpi aware: {e}");
    }

    let bindings = create_bindings().map_err(|err| {
        error!("{err:?}");
        err
    })?;

    let window_event_hook = WindowEventHook::new().map_err(|err| {
        error!("{err:?}");
        err
    })?;

    let keyboard_hook = KeyboardHook::new(&bindings).map_err(|err| {
        error!("{err:?}");
        err
    })?;

    keyboard_hook.start().log_if_err();
    window_event_hook.start().log_if_err();

    let sys_tray = SystemTray::new();

    if let Err(e) = sys_tray {
        error!("could not create sys tray: {e}");
    }

    register_border_class().log_if_err();
    WindowsApi::process_window_handles(&create_border_for_window).log_if_err();
    run_message_loop()
}

fn restart_app() {
    debug!("reloading border...");
    UserConfig::reload();
    reload_borders();
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.update(&create_bindings().unwrap());
    }
}

fn exit_app() {
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.stop().log_if_err();
    }

    if let Some(hook) = WIN_EVENT_HOOK.get() {
        hook.stop().log_if_err();
    }

    kill_message_loop();
}

fn create_logger() -> AnyResult<()> {
    let log_path = UserConfig::get_config_dir()?.join("tacky-borders.log");
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

fn create_bindings() -> AnyResult<Vec<KeybindingConfig>> {
    let config_type_lock = CONFIG
        .read()
        .map_err(|e| anyhow!("failed to acquire read lock for CONFIG_TYPE: {}", e))?;

    let bindings = vec![
        KeybindingConfig::new(
            SystemTrayEvent::OpenConfig.into(),
            config_type_lock.keybindings.open_config.clone().as_str(),
            Some(UserConfig::open),
        ),
        KeybindingConfig::new(
            SystemTrayEvent::ReloadConfig.into(),
            config_type_lock.keybindings.reload.clone().as_str(),
            Some(restart_app),
        ),
        KeybindingConfig::new(
            SystemTrayEvent::Exit.into(),
            config_type_lock.keybindings.exit.clone().as_str(),
            Some(exit_app),
        ),
    ];

    Ok(bindings)
}

fn run_message_loop() -> AnyResult<()> {
    debug!("entering message loop...");

    let mut message = MSG::default();

    loop {
        // Get the next message from the message queue
        if WindowsApi::get_message_w(&mut message, None, 0, 0).as_bool() {
            // Translate and dispatch the message
            let _ = WindowsApi::translate_message(&message);
            WindowsApi::dispatch_message_w(&message);
        } else if message.message == WM_QUIT {
            // Exit the loop when WM_QUIT message is received
            debug!("received WM_QUIT message, exiting message loop...");
            break;
        } else {
            error!(
                "no valid messages received in the message queue; exiting the loop unexpectedly..."
            );
            return Err(anyhow!("unexpected exit from message loop.".to_string()));
        }
    }

    Ok(())
}

fn kill_message_loop() {
    WindowsApi::post_message_w(None, WM_QUIT, WPARAM(0), LPARAM(0)).log_if_err();
}
