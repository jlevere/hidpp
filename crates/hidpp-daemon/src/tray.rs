use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::icon;

/// Holds references to menu items that get updated dynamically.
pub struct TrayState {
    pub tray: TrayIcon,
    pub device_item: MenuItem,
    pub battery_item: MenuItem,
    pub dpi_item: MenuItem,
    pub last_action_item: MenuItem,
    pub start_at_login_item: CheckMenuItem,
    pub reconnect_item: MenuItem,
    pub edit_config_item: MenuItem,
    pub quit_item: MenuItem,
    pub icon_connected: tray_icon::Icon,
    pub icon_disconnected: tray_icon::Icon,
}

pub fn build(menu: &Menu) -> anyhow::Result<TrayState> {
    let icon_connected = icon::connected()?;
    let icon_disconnected = icon::disconnected()?;

    // Info section.
    let device_item = MenuItem::new("No device", false, None);
    let battery_item = MenuItem::new("Battery: --", false, None);
    let dpi_item = MenuItem::new("DPI: --", false, None);

    menu.append(&device_item)?;
    menu.append(&battery_item)?;
    menu.append(&dpi_item)?;
    menu.append(&PredefinedMenuItem::separator())?;

    // Last action.
    let last_action_item = MenuItem::new("Last: --", false, None);
    menu.append(&last_action_item)?;
    menu.append(&PredefinedMenuItem::separator())?;

    // Actions.
    let edit_config_item = MenuItem::new("Edit Config...", true, None);
    let reconnect_item = MenuItem::new("Reconnect", true, None);
    menu.append(&edit_config_item)?;
    menu.append(&reconnect_item)?;
    menu.append(&PredefinedMenuItem::separator())?;

    // Start at login + quit.
    let login_installed = crate::service::is_installed();
    let start_at_login_item = CheckMenuItem::new("Start at Login", true, login_installed, None);
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&start_at_login_item)?;
    menu.append(&quit_item)?;

    let tray = TrayIconBuilder::new()
        .with_icon(icon_disconnected.clone())
        .with_icon_as_template(true)
        .with_menu(Box::new(menu.clone()))
        .with_title("--")
        .with_tooltip("HID++")
        .build()?;

    Ok(TrayState {
        tray,
        device_item,
        battery_item,
        dpi_item,
        last_action_item,
        start_at_login_item,
        reconnect_item,
        edit_config_item,
        quit_item,
        icon_connected,
        icon_disconnected,
    })
}
