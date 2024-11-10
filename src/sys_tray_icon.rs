use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItem;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use windows::Win32::System::Threading::ExitProcess;
use windows::Win32::UI::Accessibility::UnhookWinEvent;

use crate::border_config::Config;
use crate::restart_borders;
use crate::utils::*;
use crate::EVENT_HOOK;

pub fn create_tray_icon() -> Result<TrayIcon, tray_icon::Error> {
    let icon = match Icon::from_resource(1, Some((64, 64))) {
        Ok(icon) => icon,
        Err(err) => {
            error!("Failed to create icon");
            debug!("{}", &format!("{:?}", err));
            std::process::exit(1);
        }
    };

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&MenuItem::with_id("0", "Open Config", true, None));
    let _ = tray_menu.append(&MenuItem::with_id("1", "Reload Config", true, None));
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
            restart_borders();
        }
        "2" => {
            let event_hook = EVENT_HOOK.get();
            unsafe {
                let result = UnhookWinEvent(event_hook);
                if result.as_bool() {
                    debug!("Exiting tacky-borders!");
                    ExitProcess(0);
                } else {
                    error!("Could not unhook win event hook");
                }
            }
        }
        _ => {}
    }));

    tray_icon
}
