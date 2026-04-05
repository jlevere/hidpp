use std::path::PathBuf;
use std::time::Duration;

use hidpp::feature_id;
use hidpp::report::LongReport;
use hidpp::types::DeviceIndex;
use hidpp_transport::native::HidapiEnumerator;
use tao::event_loop::EventLoopProxy;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::action;
use crate::bridge::{DaemonCommand, DaemonEvent};
use crate::config::Config;
use crate::gesture::{self, GestureTracker};

pub const SAMPLE_CONFIG: &str = r#"# hidppd config — maps diverted buttons and gestures to actions.
#
# Button CIDs for MX Master 3S:
#   82  = Middle Click
#   83  = Back
#   86  = Forward
#   195 = Gesture Button (thumb)
#   196 = Mode Shift (scroll wheel click)
#
# Keystroke format: "modifier+modifier+key"
#   Modifiers: ctrl, alt, shift, cmd (or meta/super/win)
#   Keys: a-z, 0-9, f1-f20, left/right/up/down, tab, return,
#         space, escape, home, end, pageup, pagedown, delete,
#         playpause, next, prev, volumeup, volumedown, mute
#
# Simple button → keystroke:
#   [buttons]
#   83 = "alt+left"          # Back button → browser back
#   86 = "alt+right"         # Forward → browser forward
#
# Gesture button — hold + swipe for directional actions:
#   [gestures.195]
#   up = "ctrl+up"           # Swipe up → Mission Control
#   down = "ctrl+down"       # Swipe down → App Exposé
#   left = "ctrl+left"       # Swipe left → prev desktop
#   right = "ctrl+right"     # Swipe right → next desktop
#   tap = "playpause"        # Quick tap → play/pause
#   threshold = 50           # Min displacement (default: 50)
#
# Command actions:
#   83 = { type = "command", run = "open -a Safari" }

[buttons]
83 = "alt+left"
86 = "alt+right"

[gestures.195]
up = "ctrl+up"
down = "ctrl+down"
left = "ctrl+left"
right = "ctrl+right"
"#;

/// Divert flag constants for SetCtrlIdReporting (0x1B04 fn3).
///
/// SET request flags byte layout:
///   bit 0: divert (1 = divert to software)
///   bit 1: dvalid (1 = this write changes the divert bit)
///   bit 4: rawXY  (1 = enable raw XY delta reporting)
///   bit 5: rvalid (1 = this write changes the rawXY bit)
///
/// GET/SET response flags byte layout (different — state bits only):
///   bit 0: diverted
///   bit 1: rawXY enabled
///   bit 2: persist enabled
const DIVERT_FLAGS: u8 = 0x03;
const DIVERT_RAW_XY_FLAGS: u8 = 0x33;

/// Run the daemon with a tray UI event proxy.
/// Loops: connect → divert → listen → reconnect.
pub async fn run(
    config_path: &Option<PathBuf>,
    index_override: Option<DeviceIndex>,
    proxy: EventLoopProxy<DaemonEvent>,
    mut cmd_rx: tokio::sync::mpsc::Receiver<DaemonCommand>,
) {
    let path = config_path
        .clone()
        .unwrap_or_else(crate::config::default_config_path);

    info!("hidppd starting");

    loop {
        // Reload config on every iteration so ReloadConfig picks up changes.
        let cfg = match crate::config::load(&path) {
            Ok(c) => c,
            Err(e) => {
                let _ = proxy.send_event(DaemonEvent::Error(format!("config: {e}")));
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let _ = proxy.send_event(DaemonEvent::Reconnecting);

        match connect_and_listen(&cfg, index_override, &proxy, &mut cmd_rx).await {
            Ok(true) => {
                info!("shutdown requested");
                return;
            }
            Ok(false) => {
                info!("device disconnected, reconnecting...");
                let _ = proxy.send_event(DaemonEvent::Disconnected);
            }
            Err(e) => {
                warn!("error: {e}");
                let _ = proxy.send_event(DaemonEvent::Error(format!("{e}")));
            }
        }

        // Wait before reconnect. Check for shutdown command.
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            cmd = cmd_rx.recv() => {
                if matches!(cmd, Some(DaemonCommand::Shutdown) | None) {
                    return;
                }
            }
        }
    }
}

/// Run in headless listen-only mode (no tray, no action execution).
pub async fn run_listen_only(
    config_path: &Option<PathBuf>,
    index_override: Option<DeviceIndex>,
) -> anyhow::Result<()> {
    let path = config_path
        .clone()
        .unwrap_or_else(crate::config::default_config_path);
    let cfg = crate::config::load(&path)?;

    info!("listen-only mode");

    loop {
        match connect_and_listen_headless(&cfg, index_override).await {
            Ok(()) => info!("device disconnected, reconnecting..."),
            Err(e) => warn!("error: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Connect, divert, listen — with tray event proxy.
async fn connect_and_listen(
    cfg: &Config,
    index_override: Option<DeviceIndex>,
    proxy: &EventLoopProxy<DaemonEvent>,
    cmd_rx: &mut tokio::sync::mpsc::Receiver<DaemonCommand>,
) -> anyhow::Result<bool> {
    let enumerator = HidapiEnumerator::new()?;
    let devices = enumerator.enumerate();
    let dev_info = devices
        .first()
        .ok_or_else(|| anyhow::anyhow!("no HID++ devices found"))?;

    info!(
        "connecting to {} ({:04X}:{:04X})",
        dev_info.name.as_deref().unwrap_or("Unknown"),
        dev_info.vendor_id,
        dev_info.product_id,
    );

    let transport = enumerator.open(dev_info)?;
    let device_index = match index_override {
        Some(idx) => idx,
        None => {
            let idx = hidpp_device::Device::probe_device_index(&transport).await?;
            info!("auto-detected device index: 0x{:02X}", idx.0);
            idx
        }
    };

    let device = hidpp_device::Device::open(transport, device_index).await?;
    let name = device.name().to_string();
    info!("connected: {name} ({} features)", device.features().count());

    // Read initial state for the tray.
    let battery_pct = if device.supports(feature_id::UNIFIED_BATTERY) {
        device.battery_status().await.ok().map(|b| b.percentage)
    } else {
        None
    };
    let dpi = if device.supports(feature_id::ADJUSTABLE_DPI) {
        device.dpi_get().await.ok()
    } else {
        None
    };

    let _ = proxy.send_event(DaemonEvent::Connected {
        name,
        battery_pct,
        dpi,
    });

    // Auto-divert configured buttons.
    if device.supports(feature_id::SPECIAL_KEYS_V4) {
        for cid in cfg.all_diverted_cids() {
            let flags = if cfg.is_gesture_cid(cid) {
                DIVERT_RAW_XY_FLAGS
            } else {
                DIVERT_FLAGS
            };
            match device.special_key_set_reporting(cid, flags, 0, 0).await {
                Ok(r) => {
                    let mode = if cfg.is_gesture_cid(cid) {
                        if r.is_diverted() && r.raw_xy_enabled() {
                            "diverted+rawXY"
                        } else {
                            "divert+rawXY failed"
                        }
                    } else if r.is_diverted() {
                        "diverted"
                    } else {
                        "divert failed"
                    };
                    info!("CID {cid} (0x{cid:04X}): {mode}");
                }
                Err(e) => warn!("failed to divert CID {cid}: {e}"),
            }
        }
    }

    // Battery updates come via push notifications (0x1004 events) in handle_notification.
    let mut rx = device.subscribe();
    let mut gestures = GestureTracker::new();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(report) => {
                        handle_notification(&device, &report, cfg, &mut gestures, proxy);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("dropped {n} notifications");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Ok(false);
                    }
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(DaemonCommand::Reconnect | DaemonCommand::ReloadConfig) => {
                        info!("reconnect/reload requested");
                        return Ok(false);
                    }
                    Some(DaemonCommand::Shutdown) | None => {
                        return Ok(true);
                    }
                }
            }
        }
    }
}

/// Headless connect_and_listen (no proxy, no actions — just log).
async fn connect_and_listen_headless(
    _cfg: &Config,
    index_override: Option<DeviceIndex>,
) -> anyhow::Result<()> {
    let enumerator = HidapiEnumerator::new()?;
    let devices = enumerator.enumerate();
    let dev_info = devices
        .first()
        .ok_or_else(|| anyhow::anyhow!("no HID++ devices found"))?;

    let transport = enumerator.open(dev_info)?;
    let device_index = match index_override {
        Some(idx) => idx,
        None => hidpp_device::Device::probe_device_index(&transport).await?,
    };
    let device = hidpp_device::Device::open(transport, device_index).await?;
    info!(
        "connected: {} ({} features)",
        device.name(),
        device.features().count()
    );

    let mut rx = device.subscribe();
    loop {
        match rx.recv().await {
            Ok(report) => log_notification(&device, &report),
            Err(broadcast::error::RecvError::Lagged(n)) => warn!("dropped {n}"),
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

/// Handle notification with gesture tracking and action execution, send events to tray.
fn handle_notification(
    device: &hidpp_device::Device,
    report: &LongReport,
    cfg: &Config,
    gestures: &mut GestureTracker,
    proxy: &EventLoopProxy<DaemonEvent>,
) {
    let feature_index = report.feature_index();
    let function_id = report.function_id();
    let params = report.params();
    let feature_id = device.feature_id_for_index(feature_index);
    let fid = feature_id.map_or(0, |id| id.0);

    match fid {
        // SpecialKeys v4 — diverted button press/release (fn=0).
        0x1B04 if function_id.0 == 0 => {
            let mut cids = Vec::new();
            let mut i = 0;
            while i + 1 < params.len() {
                let cid = u16::from_be_bytes([params[i], params[i + 1]]);
                if cid == 0 {
                    break;
                }
                cids.push(cid);
                i += 2;
            }

            if cids.is_empty() {
                // All buttons released — resolve gestures.
                for (&cid, gesture_cfg) in &cfg.gestures {
                    if let Some(result) = gestures.button_released(cid, gesture_cfg.threshold) {
                        let (desc, action) = match &result {
                            gesture::GestureResult::Direction(d) => {
                                let dir_name = format!("{d:?}").to_lowercase();
                                let a = match d {
                                    gesture::GestureDirection::Up => gesture_cfg.up.as_ref(),
                                    gesture::GestureDirection::Down => gesture_cfg.down.as_ref(),
                                    gesture::GestureDirection::Left => gesture_cfg.left.as_ref(),
                                    gesture::GestureDirection::Right => gesture_cfg.right.as_ref(),
                                };
                                (format!("swipe {dir_name}"), a)
                            }
                            gesture::GestureResult::Tap => {
                                ("tap".to_string(), gesture_cfg.tap.as_ref())
                            }
                        };
                        info!("gesture CID {cid}: {desc}");
                        if let Some(action) = action {
                            let action_desc = action
                                .keystroke()
                                .unwrap_or_else(|| action.command().unwrap_or("?"));
                            let full_desc = format!("{desc} → {action_desc}");
                            action::execute(action);
                            let _ = proxy.send_event(DaemonEvent::ActionExecuted {
                                description: full_desc,
                            });
                        }
                    }
                }
                return;
            }

            // Button(s) pressed.
            for &cid in &cids {
                if cfg.is_gesture_cid(cid) {
                    gestures.button_pressed(cid);
                } else if let Some(action) = cfg.buttons.get(&cid) {
                    let action_desc = action
                        .keystroke()
                        .unwrap_or_else(|| action.command().unwrap_or("?"));
                    info!("button CID {cid}: {action_desc}");
                    action::execute(action);
                    let _ = proxy.send_event(DaemonEvent::ActionExecuted {
                        description: action_desc.to_string(),
                    });
                }
            }
        }

        // SpecialKeys v4 — diverted rawXY event (fn=1).
        0x1B04 if function_id.0 == 1 && params.len() >= 4 => {
            let dx = i16::from_be_bytes([params[0], params[1]]);
            let dy = i16::from_be_bytes([params[2], params[3]]);
            gestures.feed_raw_xy(dx, dy);
        }

        // UnifiedBattery — battery status change (push notification).
        0x1004 if function_id.0 == 0 && params.len() >= 3 => {
            let percentage = params[0];
            let charging = params[2] != 0;
            info!("battery {percentage}%");
            let _ = proxy.send_event(DaemonEvent::BatteryUpdate {
                percentage,
                charging,
            });
        }

        _ => {}
    }
}

/// Simple log-only notification handler for headless listen mode.
fn log_notification(device: &hidpp_device::Device, report: &LongReport) {
    let feature_index = report.feature_index();
    let function_id = report.function_id();
    let params = report.params();
    let feature_id = device.feature_id_for_index(feature_index);
    let feature_name = feature_id
        .and_then(hidpp::feature_id::feature_name)
        .unwrap_or("Unknown");
    let fid = feature_id.map_or(0, |id| id.0);

    let hex: String = params
        .iter()
        .take(8)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    info!("{feature_name} (0x{fid:04X}) fn={} [{hex}]", function_id.0,);
}
