use crate::exit_app;
use crate::restart_app;
use crate::user_config::UserConfig;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result as AnyResult;
use std::str::FromStr;
use tray_icon_win::menu::Menu;
use tray_icon_win::menu::MenuEvent;
use tray_icon_win::menu::MenuItem;
use tray_icon_win::menu::PredefinedMenuItem;
use tray_icon_win::Icon;
use tray_icon_win::TrayIcon;
use tray_icon_win::TrayIconBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTrayEvent {
    OpenConfig,
    ReloadConfig,
    Exit,
}

impl SystemTrayEvent {
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }
}

impl std::fmt::Display for SystemTrayEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SystemTrayEvent {
    type Err = anyhow::Error;

    fn from_str(event: &str) -> Result<Self, Self::Err> {
        match event {
            "open_config" => Ok(SystemTrayEvent::OpenConfig),
            "reload_config" => Ok(SystemTrayEvent::ReloadConfig),
            "exit" => Ok(SystemTrayEvent::Exit),
            _ => bail!("Invalid menu event: {}", event),
        }
    }
}

impl From<SystemTrayEvent> for &'static str {
    fn from(event: SystemTrayEvent) -> Self {
        match event {
            SystemTrayEvent::OpenConfig => "open_config",
            SystemTrayEvent::ReloadConfig => "reload_config",
            SystemTrayEvent::Exit => "exit",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SystemTray(TrayIcon);

impl SystemTray {
    pub fn new() -> AnyResult<Self> {
        let tray_icon = Self::create_tray_icon()?;

        Ok(Self(tray_icon))
    }

    pub fn create_tray_icon() -> AnyResult<TrayIcon> {
        let icon = match Icon::from_resource(32152, Some((64, 64))) {
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
            &MenuItem::with_id(SystemTrayEvent::OpenConfig, "Open config", true, None),
            &MenuItem::with_id(SystemTrayEvent::ReloadConfig, "Reload config", true, None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(SystemTrayEvent::Exit, "Exit", true, None),
        ])?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu.clone()))
            .with_tooltip(format!("tacky-borders v{}", env!("CARGO_PKG_VERSION")))
            .with_icon(icon)
            .on_menu_event(move |event: MenuEvent| {
                if let Ok(event) = SystemTrayEvent::from_str(event.id.as_ref()) {
                    match event {
                        SystemTrayEvent::OpenConfig => UserConfig::open(),
                        SystemTrayEvent::ReloadConfig => restart_app(),
                        SystemTrayEvent::Exit => exit_app(),
                    }
                }
            })
            .build()
            .map_err(Error::new);

        tray
    }
}
