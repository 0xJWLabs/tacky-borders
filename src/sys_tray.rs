use crate::border_config::Config;
use crate::border_config::ConfigImpl;
use crate::border_config::ConfigType;
use crate::border_config::CONFIG;
use crate::border_config::CONFIG_TYPE;
use crate::border_manager::reload_borders;
use crate::error::LogIfErr;
use crate::keyboard_hook::KeybindingConfig;
use crate::keyboard_hook::KeyboardHook;
use crate::keyboard_hook::KEYBOARD_HOOK;
use crate::window_event_hook::WindowEventHook;
use crate::window_event_hook::WIN_EVENT_HOOK;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result as AnyResult;
use tray_icon_win::menu::Menu;
use tray_icon_win::menu::MenuEvent;
use tray_icon_win::menu::MenuItem;
use tray_icon_win::menu::PredefinedMenuItem;
use tray_icon_win::Icon;
use tray_icon_win::TrayIcon;
use tray_icon_win::TrayIconBuilder;
use windows::Win32::System::Threading::ExitProcess;

#[allow(dead_code)]
pub struct SystemTray(TrayIcon);

impl SystemTray {
    pub fn new() -> AnyResult<Self> {
        let tray_icon = Self::create_tray()?;

        Ok(Self(tray_icon))
    }
    pub fn create_tray() -> AnyResult<TrayIcon> {
        let window_event_hook = WindowEventHook::new()?;
        let icon = match Icon::from_resource(1, Some((64, 64))) {
            Ok(icon) => icon,
            Err(e) => {
                error!("could not retrieve icon from tacky-borders.exe for tray menu: {e}");

                // If we could not retrieve an icon from the exe, then try to create an empty icon. If
                // even that fails, just return an Error using '?'.
                let rgba: Vec<u8> = vec![0, 0, 0, 0];
                Icon::from_rgba(rgba, 1, 1).context("could not create empty tray icon")?
            }
        };

        let tray_menu = Menu::new();
        tray_menu.append_items(&[
            &MenuItem::with_id("0", "Open config", true, None),
            &MenuItem::with_id("1", "Reload config", true, None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id("2", "Exit", true, None),
        ])?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(format!("tacky-borders v{}", env!("CARGO_PKG_VERSION")))
            .with_icon(icon)
            .build()
            .map_err(Error::new);

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
            "0" => open_config(),
            "1" => reload_app(),
            "2" => exit_app(),
            _ => {}
        }));

        let keyboard_hook = KeyboardHook::new(&create_bindings()?)?;
        keyboard_hook.start()?;

        window_event_hook.start()?;

        tray
    }
}

fn create_bindings() -> AnyResult<Vec<KeybindingConfig>> {
    let config_type_lock = CONFIG
        .read()
        .map_err(|e| anyhow!("failed to acquire read lock for CONFIG_TYPE: {}", e))?;

    let bindings = vec![
        KeybindingConfig::new(
            "open_config",
            config_type_lock.keybindings.open_config.clone().as_str(),
            Some(open_config),
        ),
        KeybindingConfig::new(
            "reload",
            config_type_lock.keybindings.reload.clone().as_str(),
            Some(reload_app),
        ),
        KeybindingConfig::new(
            "exit",
            config_type_lock.keybindings.exit.clone().as_str(),
            Some(exit_app),
        ),
    ];

    Ok(bindings)
}

fn exit_app() {
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.stop().log_if_err();
    }

    if let Some(hook) = WIN_EVENT_HOOK.get() {
        hook.stop().log_if_err();
    }

    debug!("exiting tacky-borders!");
    unsafe {
        ExitProcess(0);
    }
}

fn reload_app() {
    debug!("reloading border...");
    Config::reload();
    reload_borders();
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.update(&create_bindings().unwrap());
    }
}

fn open_config() {
    match Config::get_config_dir() {
        Ok(mut dir) => {
            let config_file = match *CONFIG_TYPE.read().unwrap() {
                ConfigType::Json => "config.json",
                ConfigType::Yaml => "config.yaml",
                ConfigType::Jsonc => "config.jsonc",
                _ => {
                    error!("Unsupported config file");
                    return;
                }
            };

            dir.push(config_file);

            open::that(dir).log_if_err();
        }
        Err(err) => error!("{err}"),
    }
}
