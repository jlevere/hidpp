/// Events sent from the background daemon thread to the tray UI.
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    Connected {
        name: String,
        battery_pct: Option<u8>,
        dpi: Option<u16>,
    },
    Disconnected,
    Reconnecting,
    BatteryUpdate {
        percentage: u8,
        charging: bool,
    },
    ActionExecuted {
        description: String,
    },
    Error(String),
}

/// Commands sent from the tray UI to the background daemon thread.
#[derive(Debug)]
pub enum DaemonCommand {
    Reconnect,
    ReloadConfig,
    Shutdown,
}
