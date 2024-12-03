use crate::border_config::Config;
use crate::keybinding::CreateHotkeyHook;
use crate::keybinding::RegisterHotkeyHook;
use crate::keybinding::UnbindHotkeyHook;
use crate::reload_borders;
use crate::utils::get_config;
use crate::EVENT_HOOK;
use rustc_hash::FxHashMap;
use std::process::exit;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItem;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use win_hotkey::global::GlobalHotkey;
use win_hotkey::global::GlobalHotkeyManager;
use windows::Win32::System::Threading::ExitProcess;
use windows::Win32::UI::Accessibility::UnhookWinEvent;

static HOTKEY_HOOK: LazyLock<Arc<Mutex<GlobalHotkeyManager<()>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(CreateHotkeyHook())));

fn reload_config() {
    Config::reload();
    reload_borders();
}

fn open_config() {
    let config_dir = get_config();
    let config_path = config_dir.join("config.yaml");
    let _ = open::that(config_path);
}

pub fn create_tray_icon() -> Result<TrayIcon, tray_icon::Error> {
    let icon = match Icon::from_resource(1, Some((64, 64))) {
        Ok(icon) => icon,
        Err(_) => {
            error!("could not retrieve tray icon!");
            exit(1);
        }
    };

    let tray_menu = Menu::new();
    let _ = tray_menu.append_items(&[
        &MenuItem::with_id("0", "Open Config", true, None),
        &MenuItem::with_id("1", "Reload", true, None),
        &MenuItem::with_id("2", "Close", true, None),
    ]);

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(format!("tacky-borders v{}", env!("CARGO_PKG_VERSION")))
        .with_icon(icon)
        .build();

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
        "0" => open_config(),
        "1" => reload_config(),
        "2" => unsafe {
            if UnhookWinEvent(EVENT_HOOK.get()).as_bool() {
                debug!("exiting tacky-borders!");
                UnbindHotkeyHook(&HOTKEY_HOOK.lock().unwrap());
                ExitProcess(0);
            } else {
                error!("could not unhook win event hook");
            }
        },
        _ => {}
    }));

    bind_tray_hotkeys();

    tray_icon
}

pub fn bind_tray_hotkeys() {
    let mut bindings: FxHashMap<String, GlobalHotkey<()>> = FxHashMap::default();

    let mut create_binding = |name: &str, hotkey: &str, action: fn()| match hotkey.try_into()
        as Result<GlobalHotkey<()>, _>
    {
        Ok(mut binding) => {
            binding.set_action(action);
            bindings.insert(name.to_string(), binding);
        }
        Err(err) => eprintln!("Failed to create binding for '{}': {:?}", name, err),
    };

    create_binding("reload", "f8", reload_config);
    create_binding("open_config", "f9", open_config);

    RegisterHotkeyHook(&HOTKEY_HOOK.lock().unwrap(), Some(bindings));
}
