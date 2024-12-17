use crate::border_config::Config;
use crate::border_config::ConfigImpl;
use crate::border_config::ConfigType;
use crate::border_config::CONFIG_TYPE;
use crate::error::LogIfErr;
use crate::keybinding::CreateHotkeyHook;
use crate::keybinding::RegisterHotkeyHook;
use crate::keybinding::UnbindHotkeyHook;
use crate::reload_borders;
use crate::EVENT_HOOK;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result as AnyResult;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItem;
use tray_icon::menu::PredefinedMenuItem;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use win_hotkey::global::GlobalHotkey;
use win_hotkey::global::GlobalHotkeyManager;
use windows::Win32::System::Threading::ExitProcess;
use windows::Win32::UI::Accessibility::UnhookWinEvent;

thread_local! {
    static HOTKEY_HOOK: RefCell<GlobalHotkeyManager<()>> = RefCell::new(CreateHotkeyHook())
}

fn reload_config() {
    debug!("reloading border...");
    Config::reload();
    reload_borders();
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

fn exit_app() {
    HOTKEY_HOOK.with(|hook| {
        let hook = hook.borrow();
        UnbindHotkeyHook(&hook);
    });

    unsafe {
        if UnhookWinEvent(EVENT_HOOK.get()).as_bool() {
            debug!("exiting tacky-borders!");
            ExitProcess(0);
        } else {
            error!("could not unhook win event hook");
            ExitProcess(0);
        }
    }
}

pub fn create_tray_icon() -> AnyResult<TrayIcon> {
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

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(format!("tacky-borders v{}", env!("CARGO_PKG_VERSION")))
        .with_icon(icon)
        .build();

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
        "0" => open_config(),
        "1" => reload_config(),
        "2" => exit_app(),
        _ => {}
    }));

    bind_tray_hotkeys();

    tray_icon.map_err(Error::new)
}

fn bind_tray_hotkeys() {
    let mut bindings: FxHashMap<String, GlobalHotkey<()>> = FxHashMap::default();

    let mut create_binding = |name: &str, hotkey: &str, action: fn()| match hotkey.try_into()
        as Result<GlobalHotkey<()>, _>
    {
        Ok(mut binding) => {
            binding.set_action(action);
            bindings.insert(name.to_string(), binding);
        }
        Err(err) => error!("Failed to create binding for '{name}': {err:?}"),
    };

    create_binding("reload", "f8", reload_config);
    create_binding("open_config", "f9", open_config);

    HOTKEY_HOOK.with(|hook| {
        RegisterHotkeyHook(&hook.borrow(), Some(bindings));
    })
}
