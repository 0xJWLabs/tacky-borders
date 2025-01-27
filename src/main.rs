// #![allow(unused)]
#![feature(duration_millis_float)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
extern crate sp_log2;
#[macro_use]
extern crate tacky_borders_logger;

use anyhow::anyhow;
use app_manager::AppManager;
use border_manager::Border;
use border_manager::register_border_class;
use core::keybindings::KeybindingConfig;
use error::LogIfErr;
use keyboard_hook::KEYBOARD_HOOK;
use keyboard_hook::KeyboardHook;
use sp_log2::ColorChoice;
use sp_log2::CombinedLogger;
use sp_log2::ConfigBuilder;
use sp_log2::FileLogger;
use sp_log2::Format;
use sp_log2::LevelFilter;
use sp_log2::TermLogger;
use sp_log2::TerminalMode;
use sys_tray::SystemTray;
use user_config::UserConfig;
use window_event_hook::WIN_EVENT_HOOK;
use window_event_hook::WindowEventHook;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows_api::WindowsApi;

mod animation;
mod app_manager;
mod border_manager;
mod colors;
mod config_watcher;
mod core;
mod effect;
mod env;
mod error;
mod keyboard_hook;
mod parsed_config;
mod render_resources;
mod sys_tray;
mod theme_manager;
mod user_config;
mod window_event_hook;
mod windows_api;
mod windows_callback;

fn main() -> anyhow::Result<()> {
    if let Err(e) = &initialize_logger() {
        error!("logger initialization failed: {e}");
    };

    debug!("Application: Starting");
    let res = start_application();

    if let Err(err) = &res {
        error!("{err:?}");
        WindowsApi::show_error_dialog("Fatal error", &err.to_string());
    } else {
        debug!("Application: Exit");
    }

    res
}

fn start_application() -> anyhow::Result<()> {
    if !WindowsApi::imm_disable_ime().as_bool() {
        error!("could not disable ime!");
    }

    WindowsApi::set_process_dpi_awareness_context()
        .log_if_err_message("could not make process dpi aware", false);

    let config = AppManager::get().config().clone();
    let bindings = Vec::<KeybindingConfig>::from(&config.keybindings);
    let window_event_hook = WindowEventHook::new().map_err_with_log()?;
    let keyboard_hook = KeyboardHook::new(&bindings).map_err_with_log()?;

    keyboard_hook.start().log_if_err();
    window_event_hook.start().log_if_err();

    let sys_tray = SystemTray::new();
    sys_tray.log_if_err_message_pretty("could not create tray icon", true);

    register_border_class().log_if_err();

    WindowsApi::process_window_handles(&Border::create).log_if_err();

    debug!("Application: Started");

    let mut message = MSG::default();
    loop {
        // Get the next message from the message queue
        if WindowsApi::get_message_w(&mut message, None, 0, 0).as_bool() {
            // Translate and dispatch the message
            let _ = WindowsApi::translate_message(&message);
            WindowsApi::dispatch_message_w(&message);
        } else if message.message == WM_QUIT {
            debug!("Application: Shutting Down");
            break;
        } else {
            let last_error = unsafe { GetLastError() };
            error!("Unexpected termination of the message loop. Last error: {last_error:?}");
            return Err(anyhow!("unexpected exit from message loop".to_string()));
        }
    }

    debug!("Application: Shut Down");

    Ok(())
}

fn exit_application() {
    debug!("Stopping hooks and posting quit message to shut down the application");
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.stop().log_if_err();
    }

    if let Some(hook) = WIN_EVENT_HOOK.get() {
        hook.stop().log_if_err();
    }

    AppManager::get().stop_config_watcher();

    WindowsApi::post_quit_message(0);
}

fn initialize_logger() -> anyhow::Result<()> {
    let log_path = UserConfig::get_config_dir()?.join("tacky-borders.log");
    let Some(log_path) = log_path.to_str() else {
        return Err(anyhow!("could not convert log_path to str"));
    };

    std::fs::write(log_path, "").log_if_err();

    let mut config_builder = ConfigBuilder::new();

    config_builder.set_format(
        Format::LevelFlag | Format::Time | Format::Thread | Format::Target | Format::FileLocation,
    );

    config_builder.set_formatter(Some(
        "[time:#89dceb] [level:bold] ([thread]) [target:rgb(137 180 250):bold]: [message:bold] [[file:#6c7086]]\n",
    ));

    config_builder.set_time_format_custom("%d/%m/%Y %H:%M:%S,%3f");

    let config = config_builder.build();

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Debug,
            config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        FileLogger::new(
            LevelFilter::Info,
            config.clone(),
            log_path,
            Some(1024 * 1024),
        ),
    ])?;

    Ok(())
}
