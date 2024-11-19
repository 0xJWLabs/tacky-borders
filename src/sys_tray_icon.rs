use crate::border_config::Config;
use crate::reload_borders;
use crate::utils::get_config;
use crate::EVENT_HOOK;
use std::process::exit;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItem;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use windows::Win32::System::Threading::ExitProcess;
use windows::Win32::UI::Accessibility::UnhookWinEvent;

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
        "0" => {
            let config_dir = get_config();
            let config_path = config_dir.join("config.yaml");
            let _ = open::that(config_path);
        }
        "1" => {
            Config::reload();
            reload_borders();
        }
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
