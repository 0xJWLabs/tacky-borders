use crate::border_config::Config;
use crate::reload_borders;
use crate::utils::get_config;
use crate::EVENT_HOOK;
use std::process::exit;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItem;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use win_binder::listen;
use win_binder::Event;
use win_binder::EventType;
use win_binder::Key;
use windows::Win32::System::Threading::ExitProcess;
use windows::Win32::UI::Accessibility::UnhookWinEvent;

// Testing Purpose
const RELOAD_KEYBIND: &str = "F8";
const OPEN_CONFIG: &str = "F9";

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
    let _ = tray_menu.append(&MenuItem::with_id("0", "Open Config", true, None));
    let _ = tray_menu.append(&MenuItem::with_id("1", "Reload", true, None));
    let _ = tray_menu.append(&MenuItem::with_id("2", "Close", true, None));

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
                ExitProcess(0);
            } else {
                error!("could not unhook win event hook");
            }
        },
        _ => {}
    }));

    tray_icon
}

pub fn bind_tray_hotkeys() {
    let (tx, rx) = channel();
    thread::spawn(move || listen_for_hotkeys(tx));

    thread::spawn(move || {
        for action in rx {
            match action.as_str() {
                "reload" => reload_config(),
                "open_config" => open_config(),
                _ => {}
            }
        }
    });
}

fn listen_for_hotkeys(tx: Sender<String>) {
    let reload_key = key_from_string(RELOAD_KEYBIND).unwrap();
    let open_key = key_from_string(OPEN_CONFIG).unwrap();
    if let Err(error) = listen(move |event: Event| {
        if let EventType::KeyPress(key) = event.event_type {
            if let Some(key_name) = key_from_string(&format!("{:?}", key)) {
                if key_name == reload_key {
                    tx.send("reload".to_string()).unwrap()
                } else if key_name == open_key {
                    tx.send("open_config".to_string()).unwrap()
                }
            }
        }
    }) {
        eprintln!("Error listening to global hotkeys: {:?}", error);
    }
}

fn key_from_string(key_str: &str) -> Option<Key> {
    match key_str.to_lowercase().as_str() {
        "alt" => Some(Key::Alt),
        "altgr" => Some(Key::AltGr),
        "backspace" => Some(Key::Backspace),
        "capslock" => Some(Key::CapsLock),
        "controlleft" => Some(Key::ControlLeft),
        "controlright" => Some(Key::ControlRight),
        "delete" => Some(Key::Delete),
        "downarrow" => Some(Key::DownArrow),
        "end" => Some(Key::End),
        "escape" => Some(Key::Escape),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "home" => Some(Key::Home),
        "leftarrow" => Some(Key::LeftArrow),
        "metaleft" => Some(Key::MetaLeft),
        "metaright" => Some(Key::MetaRight),
        "pagedown" => Some(Key::PageDown),
        "pageup" => Some(Key::PageUp),
        "return" => Some(Key::Return),
        "rightarrow" => Some(Key::RightArrow),
        "shiftleft" => Some(Key::ShiftLeft),
        "shiftright" => Some(Key::ShiftRight),
        "space" => Some(Key::Space),
        "tab" => Some(Key::Tab),
        "uparrow" => Some(Key::UpArrow),
        "printscreen" => Some(Key::PrintScreen),
        "scrolllock" => Some(Key::ScrollLock),
        "pause" => Some(Key::Pause),
        "numlock" => Some(Key::NumLock),
        "backquote" => Some(Key::BackQuote),
        "num1" => Some(Key::Num1),
        "num2" => Some(Key::Num2),
        "num3" => Some(Key::Num3),
        "num4" => Some(Key::Num4),
        "num5" => Some(Key::Num5),
        "num6" => Some(Key::Num6),
        "num7" => Some(Key::Num7),
        "num8" => Some(Key::Num8),
        "num9" => Some(Key::Num9),
        "num0" => Some(Key::Num0),
        "minus" => Some(Key::Minus),
        "equal" => Some(Key::Equal),
        "keyq" => Some(Key::KeyQ),
        "keyw" => Some(Key::KeyW),
        "keye" => Some(Key::KeyE),
        "keyr" => Some(Key::KeyR),
        "keyt" => Some(Key::KeyT),
        "keyy" => Some(Key::KeyY),
        "keyu" => Some(Key::KeyU),
        "keyi" => Some(Key::KeyI),
        "keyo" => Some(Key::KeyO),
        "keyp" => Some(Key::KeyP),
        "leftbracket" => Some(Key::LeftBracket),
        "rightbracket" => Some(Key::RightBracket),
        "keya" => Some(Key::KeyA),
        "keys" => Some(Key::KeyS),
        "keyd" => Some(Key::KeyD),
        "keyf" => Some(Key::KeyF),
        "keyg" => Some(Key::KeyG),
        "keyh" => Some(Key::KeyH),
        "keyj" => Some(Key::KeyJ),
        "keyk" => Some(Key::KeyK),
        "keyl" => Some(Key::KeyL),
        "semicolon" => Some(Key::SemiColon),
        "quote" => Some(Key::Quote),
        "backslash" => Some(Key::BackSlash),
        "intlbackslash" => Some(Key::IntlBackslash),
        "keyz" => Some(Key::KeyZ),
        "keyx" => Some(Key::KeyX),
        "keyc" => Some(Key::KeyC),
        "keyv" => Some(Key::KeyV),
        "keyb" => Some(Key::KeyB),
        "keyn" => Some(Key::KeyN),
        "keym" => Some(Key::KeyM),
        "comma" => Some(Key::Comma),
        "dot" => Some(Key::Dot),
        "slash" => Some(Key::Slash),
        "insert" => Some(Key::Insert),
        "kpreturn" => Some(Key::KpReturn),
        "kpminus" => Some(Key::KpMinus),
        "kpplus" => Some(Key::KpPlus),
        "kpmultiply" => Some(Key::KpMultiply),
        "kpdivide" => Some(Key::KpDivide),
        "kp0" => Some(Key::Kp0),
        "kp1" => Some(Key::Kp1),
        "kp2" => Some(Key::Kp2),
        "kp3" => Some(Key::Kp3),
        "kp4" => Some(Key::Kp4),
        "kp5" => Some(Key::Kp5),
        "kp6" => Some(Key::Kp6),
        "kp7" => Some(Key::Kp7),
        "kp8" => Some(Key::Kp8),
        "kp9" => Some(Key::Kp9),
        "kpdelete" => Some(Key::KpDelete),
        "function" => Some(Key::Function),
        _ => None, // Return None if no match is found
    }
}
