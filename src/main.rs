// #![allow(unused)]
#![feature(duration_millis_float)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#[macro_use]
extern crate log;
extern crate sp_log;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result as AnyResult;
use border_manager::register_border_class;
use border_manager::reload_borders;
use border_manager::Border;
use error::LogIfErr;
use keyboard_hook::KeybindingConfig;
use keyboard_hook::KeyboardHook;
use keyboard_hook::KEYBOARD_HOOK;
use notify_win_debouncer_full::new_debouncer;
use notify_win_debouncer_full::notify_win::Error as NotifyError;
use notify_win_debouncer_full::notify_win::RecursiveMode;
use notify_win_debouncer_full::DebouncedEvent;
use sp_log::ColorChoice;
use sp_log::CombinedLogger;
use sp_log::ConfigBuilder;
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
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows_api::WindowsApi;

mod animations;
mod border_manager;
mod core;
mod error;
mod keyboard_hook;
mod sys_tray;
mod user_config;
mod window_event_hook;
mod windows_api;
mod windows_callback;

static STOP_FLAG: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

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
    if let Err(e) = &initialize_logger() {
        error!("logger initialization failed: {e}");
    };

    if !WindowsApi::imm_disable_ime().as_bool() {
        error!("could not disable ime!");
    }

    WindowsApi::set_process_dpi_awareness_context()
        .log_if_err_message("could not make process dpi aware", false);

    let bindings = create_keybindings().map_err_with_log()?;
    let window_event_hook = WindowEventHook::new().map_err_with_log()?;
    let keyboard_hook = KeyboardHook::new(&bindings).map_err_with_log()?;

    keyboard_hook.start().log_if_err();
    window_event_hook.start().log_if_err();

    let sys_tray = SystemTray::new();
    sys_tray.log_if_err_message_pretty("could not create tray icon", true);

    register_border_class().log_if_err();

    WindowsApi::process_window_handles(&Border::create).log_if_err();
    let watcher_handle = watcher_config().map_err_with_log()?;

    debug!("tacky-borders event started");

    let mut message = MSG::default();
    loop {
        // Get the next message from the message queue
        if WindowsApi::get_message_w(&mut message, None, 0, 0).as_bool() {
            // Translate and dispatch the message
            let _ = WindowsApi::translate_message(&message);
            WindowsApi::dispatch_message_w(&message);
        } else if message.message == WM_QUIT {
            STOP_FLAG.store(true, Ordering::SeqCst);
            debug!("tacky-borders event is shutting down gracefully.");
            break;
        } else {
            STOP_FLAG.store(true, Ordering::SeqCst);
            let last_error = unsafe { GetLastError() };
            error!("unexpected termination of the message loop. Last error: {last_error:?}");
            return Err(anyhow!("unexpected exit from message loop.".to_string()));
        }
    }

    watcher_handle
        .join()
        .map_err(|_| anyhow!("failed to close watcher thread"))??;

    Ok(())
}

fn restart_application() {
    debug!("reloading application configuration and restarting borders.");
    UserConfig::reload();
    reload_borders();

    if let Some(hook) = KEYBOARD_HOOK.get() {
        if let Ok(bindings) = create_keybindings() {
            hook.update(&bindings);
        }
    }
}

fn exit_application() {
    debug!("stopping hooks and posting quit message to shut down the application.");
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.stop().log_if_err();
    }

    if let Some(hook) = WIN_EVENT_HOOK.get() {
        hook.stop().log_if_err();
    }

    WindowsApi::post_quit_message(0);
}

fn initialize_logger() -> AnyResult<()> {
    let log_path = UserConfig::get_config_dir()?.join("tacky-borders.log");
    let Some(log_path) = log_path.to_str() else {
        return Err(anyhow!("could not convert log_path to str"));
    };

    let mut config_builder = ConfigBuilder::new();

    if let Err(e) = config_builder.set_time_offset_to_local() {
        error!("time offset error: {e:?}");
    }

    let config = config_builder.build();

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
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

fn create_keybindings() -> AnyResult<Vec<KeybindingConfig>> {
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
            Some(restart_application),
        ),
        KeybindingConfig::new(
            SystemTrayEvent::Exit.into(),
            config_type_lock.keybindings.exit.clone().as_str(),
            Some(exit_application),
        ),
    ];

    debug!("keybindings created: {bindings:?}");

    Ok(bindings)
}

fn watcher_config() -> AnyResult<std::thread::JoinHandle<AnyResult<()>>> {
    debug!("configuration watcher has started.");

    // Spawn the watcher thread
    let handle = std::thread::spawn({
        let stop_flag = STOP_FLAG.clone(); // Use the static flag
        move || -> AnyResult<()> {
            let mut debouncer = new_debouncer(
                Duration::from_millis(500),
                None,
                move |result: Result<Vec<DebouncedEvent>, Vec<NotifyError>>| {
                    if let Ok(events) = result {
                        for event in events {
                            // Ensure `event` type is `DebouncedEvent`
                            if event.kind.is_modify() {
                                debug!("configuration file modified. Restarting...");
                                restart_application();
                                break;
                            }
                        }
                    }
                },
            )?;

            let config_dir = UserConfig::get_config_dir()?;
            let config_file = UserConfig::detect_config_file(&config_dir)?;

            debug!("watching configuration file: {config_file:?}");
            debouncer.watch(config_file.as_path(), RecursiveMode::Recursive)?;

            // Loop until the stop flag is set to true
            while !stop_flag.load(Ordering::SeqCst) {
                // Sleep to prevent tight looping
                std::thread::sleep(Duration::from_millis(100));
            }

            debug!("configuration watcher detected stop flag. Preparing to exit.");
            debouncer.unwatch(config_file.as_path())?;
            Ok(())
        }
    });

    Ok(handle)
}
