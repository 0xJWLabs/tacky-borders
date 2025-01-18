use crate::exit_application;
use crate::user_config::UserConfig;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use std::str::FromStr;
use tray_icon_win::menu::Menu;
use tray_icon_win::menu::MenuEvent;
use tray_icon_win::menu::MenuItem;
use tray_icon_win::menu::PredefinedMenuItem;
use tray_icon_win::Icon;
use tray_icon_win::TrayIcon;
use tray_icon_win::TrayIconBuilder;
use crate::core::helpers::type_name_of_val;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTrayEvent {
    OpenConfig,
    ReloadConfig,
    Exit,
}

impl SystemTrayEvent {
    pub fn execute(&self) {
        match self {
            SystemTrayEvent::OpenConfig => UserConfig::open(),
            SystemTrayEvent::Exit => exit_application(),
            SystemTrayEvent::ReloadConfig => {
                let _ = UserConfig::reload();
            }
        }
    }

    pub fn as_function_name(&self) -> &'static str {
        match self {
            SystemTrayEvent::OpenConfig => type_name_of_val(&UserConfig::open),
            SystemTrayEvent::Exit => type_name_of_val(&exit_application),
            SystemTrayEvent::ReloadConfig => type_name_of_val(&UserConfig::reload),
        }
    }

    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }
}

impl core::fmt::Display for SystemTrayEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SystemTrayEvent {
    type Err = anyhow::Error;

    fn from_str(event: &str) -> Result<Self, Self::Err> {
        let event_name_split: Vec<&str> = event.split("_").collect();
        match event_name_split.as_slice() {
            ["open", "config"] => Ok(SystemTrayEvent::OpenConfig),
            ["reload", "config"] => Ok(SystemTrayEvent::ReloadConfig),
            ["exit"] => Ok(SystemTrayEvent::Exit),
            _ => bail!("Invalid menu event: {}", event),
        }
    }
}

impl From<SystemTrayEvent> for &'static str {
    fn from(value: SystemTrayEvent) -> Self {
        match value {
            SystemTrayEvent::OpenConfig => "open_config",
            SystemTrayEvent::ReloadConfig => "reload_config",
            SystemTrayEvent::Exit => "exit",
        }
    }
}

impl From<SystemTrayEvent> for String {
    fn from(event: SystemTrayEvent) -> Self {
        event.as_str().to_string()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SystemTray(TrayIcon);

impl SystemTray {
    pub fn new() -> anyhow::Result<Self> {
        let tray_icon = Self::create_tray_icon()?;

        Ok(Self(tray_icon))
    }

    pub fn create_tray_icon() -> anyhow::Result<TrayIcon> {
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
                    event.execute()
                }
            })
            .build()
            .map_err(Error::new);

        tray
    }
}