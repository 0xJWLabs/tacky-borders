use crate::border_config::Config;
use crate::keybinding::CreateHotkeyHook;
use crate::keybinding::HotKeyParseError;
use crate::keybinding::HotkeyBinding;
use crate::keybinding::HotkeyHook;
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
use windows::Win32::System::Threading::ExitProcess;
use windows::Win32::UI::Accessibility::UnhookWinEvent;

static HOTKEY_HOOK: LazyLock<Arc<Mutex<HotkeyHook>>> =
    LazyLock::new(|| Arc::new(Mutex::new(CreateHotkeyHook(None))));

fn reload_config() {
    Config::reload();
    reload_borders();
}

fn open_config() {
    let config_dir = get_config();
    let config_path = config_dir.join("config.yaml");
    let _ = open::that(config_path);
}

fn lock_hotkey_hook() -> std::sync::MutexGuard<'static, HotkeyHook> {
    HOTKEY_HOOK.lock().unwrap_or_else(|_| {
        panic!("Failed to lock hotkey hook.");
    })
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
                let mut hotkey_hook = lock_hotkey_hook();
                UnbindHotkeyHook(&mut hotkey_hook);
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

fn create_binding(hotkey: &str) -> Result<HotkeyBinding, HotKeyParseError> {
    let reload_binding: HotkeyBinding = hotkey.try_into()?;
    Ok(reload_binding)
}

pub fn bind_tray_hotkeys() {
    let mut bindings = FxHashMap::default();

    let mut reload_binding = create_binding("f8").unwrap();
    reload_binding.set_action(Box::new(reload_config));
    bindings.insert("reload".to_string(), reload_binding);

    let mut open_config_binding = create_binding("f9").unwrap();
    open_config_binding.set_action(Box::new(open_config));
    bindings.insert("open_config".to_string(), open_config_binding);

    let mut hotkey_hook = lock_hotkey_hook();
    RegisterHotkeyHook(&mut hotkey_hook, Some(bindings));
}
