use tray_icon::{menu::Menu, menu::MenuEvent, menu::MenuItem, Icon, TrayIcon, TrayIconBuilder};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;

use crate::border_config::Config;
use crate::log;
use crate::restart_borders;
use crate::utils::*;
use crate::Logger;

pub fn create_tray_icon(main_thread: u32) -> Result<TrayIcon, tray_icon::Error> {
    let icon = match Icon::from_resource(1, Some((64, 64))) {
        Ok(icon) => icon,
        Err(err) => {
            log!("error", "Failed to create icon");
            log!("debug", &format!("{:?}", err));
            std::process::exit(1);
        }
    };

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&MenuItem::with_id("0", "Open Config", true, None));
    let _ = tray_menu.append(&MenuItem::with_id("1", "Reload Config", true, None));
    let _ = tray_menu.append(&MenuItem::with_id("2", "Close", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("tacky-borders")
        .with_icon(icon)
        .build();

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
        "0" => {
            let config_dir = get_config();
            let config_path = config_dir.join("config.yaml");
            let _ = open::that(config_path);
        }
        "1" => {
            Config::reload();
            restart_borders();
        }
        "2" => {
            let result = unsafe { PostThreadMessageW(main_thread, WM_CLOSE, WPARAM(0), LPARAM(0)) };
            log!(
                "debug",
                format!("Sending WM_CLOSE to main thread: {:?}", result).as_str()
            );
        }
        _ => {}
    }));

    tray_icon
}
