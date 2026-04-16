use std::path::PathBuf;
use std::time::Duration;

use hidpp::feature_id;
use hidpp::report::LongReport;
use hidpp::types::{ControlId, DeviceIndex};
use hidpp_transport::native::HidapiEnumerator;
use tao::event_loop::EventLoopProxy;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::action::{self, ActionOutcome};
use crate::bridge::{DaemonCommand, DaemonEvent};
use crate::config::Config;
use crate::gesture::{self, GestureTracker};

const ACCESSIBILITY_ERROR: &str = "Grant Accessibility permission in System Settings";

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

const MIN_RETRY_DELAY: Duration = Duration::from_secs(2);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

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

    // Spawn watchers that fire on events requiring reconnection:
    // - wake watcher: system sleep/wake (power state change)
    // - HID watcher: Logitech device arrival/removal (BLE reconnect)
    // Both share the same channel — either event triggers re-divert.
    let (wake_tx, mut wake_rx) = tokio::sync::mpsc::channel(8);
    crate::platform::spawn_wake_watcher(wake_tx.clone());
    crate::platform::spawn_hid_watcher(wake_tx);

    let mut last_error: Option<String> = None;
    let mut retry_delay = MIN_RETRY_DELAY;

    loop {
        // Reload config on every iteration so ReloadConfig picks up changes.
        let cfg = match crate::config::load(&path) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("config: {e}");
                if last_error.as_deref() != Some(&msg) {
                    warn!("{msg}");
                    last_error = Some(msg);
                }
                let _ = proxy.send_event(DaemonEvent::Error("Config error".to_string()));
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let _ = proxy.send_event(DaemonEvent::Reconnecting);

        match connect_and_listen(&cfg, index_override, &proxy, &mut cmd_rx, &mut wake_rx).await {
            Ok(true) => {
                info!("shutdown requested");
                return;
            }
            Ok(false) => {
                info!("device disconnected, reconnecting...");
                last_error = None;
                retry_delay = MIN_RETRY_DELAY;
                let _ = proxy.send_event(DaemonEvent::Disconnected);
            }
            Err(e) => {
                let user_msg = classify_error(&e);
                if last_error.as_deref() != Some(user_msg) {
                    warn!("{user_msg}: {e}");
                    last_error = Some(user_msg.to_string());
                }
                let _ = proxy.send_event(DaemonEvent::Error(user_msg.to_string()));
            }
        }

        // Wait before reconnect with exponential backoff (2s → 30s).
        tokio::select! {
            _ = tokio::time::sleep(retry_delay) => {
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
            }
            cmd = cmd_rx.recv() => {
                if matches!(cmd, Some(DaemonCommand::Shutdown) | None) {
                    return;
                }
                retry_delay = MIN_RETRY_DELAY;
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

    info!("listen-only mode");

    let mut last_error: Option<String> = None;
    let mut retry_delay = MIN_RETRY_DELAY;

    loop {
        // Reload config so it picks up changes between retries.
        let _cfg = match crate::config::load(&path) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("config: {e}");
                if last_error.as_deref() != Some(&msg) {
                    warn!("{msg}");
                    last_error = Some(msg);
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        match connect_and_listen_headless(index_override).await {
            Ok(()) => {
                info!("device disconnected, reconnecting...");
                last_error = None;
                retry_delay = MIN_RETRY_DELAY;
            }
            Err(e) => {
                let msg = e.to_string();
                if last_error.as_deref() != Some(&msg) {
                    warn!("error: {e}");
                    last_error = Some(msg);
                }
                tokio::time::sleep(retry_delay).await;
                retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
            }
        }
    }
}

/// Map an error to a user-facing message.
fn classify_error(e: &anyhow::Error) -> &'static str {
    let s = e.to_string();
    if s.contains("no HID++") {
        "No device found"
    } else if s.contains("not permitted") || s.contains("IOHIDDevice") {
        "Grant Input Monitoring in System Settings"
    } else if s.contains("PingFailed") {
        "Device not responding"
    } else {
        "Connection failed"
    }
}

/// Enumerate, open, probe index, and open a Device.
async fn connect_device(
    index_override: Option<DeviceIndex>,
) -> anyhow::Result<hidpp_device::Device> {
    let enumerator = HidapiEnumerator::new()?;
    let devices = enumerator.enumerate();
    let dev_info = devices
        .first()
        .ok_or_else(|| anyhow::anyhow!("no HID++ devices found"))?;

    debug!(
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
    info!(
        "connected: {} ({} features)",
        device.name(),
        device.features().count()
    );
    Ok(device)
}

/// Connect, divert, listen — with tray event proxy.
async fn connect_and_listen(
    cfg: &Config,
    index_override: Option<DeviceIndex>,
    proxy: &EventLoopProxy<DaemonEvent>,
    cmd_rx: &mut tokio::sync::mpsc::Receiver<DaemonCommand>,
    wake_rx: &mut tokio::sync::mpsc::Receiver<()>,
) -> anyhow::Result<bool> {
    // Prevent system idle sleep during device setup (connect, discover, divert).
    let _power_guard =
        crate::platform::PowerAssertion::prevent_idle_sleep("HID++ device setup");

    let device = connect_device(index_override).await?;
    let name = device.name().to_string();

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
            match device
                .special_key_set_reporting(ControlId(cid), flags, ControlId(0), 0)
                .await
            {
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

    // Device setup complete — release the sleep assertion.
    drop(_power_guard);

    // Check Accessibility permission early so the user sees the error immediately.
    if !action::ensure_init() {
        let _ = proxy.send_event(DaemonEvent::Error(ACCESSIBILITY_ERROR.to_string()));
    }

    // Drain stale events — IOHIDManager fires matching callbacks for
    // already-connected devices on creation. Discard those here so we
    // don't immediately trigger a spurious reconnect.
    while wake_rx.try_recv().is_ok() {}

    // Keepalive: periodically verify diversion is still set. Catches
    // silent BLE reconnects where the HID handle stays valid but
    // the device firmware has reset its volatile diversion state.
    let keepalive_cid = cfg.all_diverted_cids().next();
    let mut keepalive = tokio::time::interval(Duration::from_secs(300));
    keepalive.tick().await; // Consume the immediate first tick.

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
                        action::retry_init();
                        return Ok(false);
                    }
                    Some(DaemonCommand::Shutdown) | None => {
                        return Ok(true);
                    }
                }
            }
            _ = wake_rx.recv() => {
                while wake_rx.try_recv().is_ok() {}
                info!("reconnection trigger (wake/HID), re-diverting buttons");
                return Ok(false);
            }
            _ = keepalive.tick(), if keepalive_cid.is_some() => {
                let cid = keepalive_cid.unwrap();
                match device.special_key_reporting(ControlId(cid)).await {
                    Ok(r) if r.is_diverted() => {
                        debug!("keepalive: CID {cid} still diverted");
                    }
                    Ok(_) => {
                        info!("keepalive: CID {cid} diversion lost, reconnecting");
                        return Ok(false);
                    }
                    Err(e) => {
                        warn!("keepalive: CID {cid} check failed ({e}), reconnecting");
                        return Ok(false);
                    }
                }
            }
        }
    }
}

/// Headless connect_and_listen — no diversion, no actions, just log.
async fn connect_and_listen_headless(
    index_override: Option<DeviceIndex>,
) -> anyhow::Result<()> {
    let device = connect_device(index_override).await?;

    let mut rx = device.subscribe();
    loop {
        match rx.recv().await {
            Ok(report) => {
                let feature_index = report.feature_index();
                let function_id = report.function_id();
                let params = report.params();
                let feature_id = device.feature_id_for_index(feature_index);
                let feature_name = feature_id
                    .and_then(hidpp::feature_id::feature_name)
                    .unwrap_or("Unknown");
                let fid = feature_id.map_or(0, |id| id.0);
                let hex = format_hex(params);
                info!("{feature_name} (0x{fid:04X}) fn={} [{hex}]", function_id.0);
            }
            Err(broadcast::error::RecvError::Lagged(n)) => warn!("dropped {n}"),
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

/// Execute an action and send the appropriate tray event.
fn execute_and_notify(
    action: &crate::config::Action,
    description: &str,
    proxy: &EventLoopProxy<DaemonEvent>,
) {
    match action::execute(action) {
        ActionOutcome::Executed => {
            info!("{description}");
            let _ = proxy.send_event(DaemonEvent::ActionExecuted {
                description: description.to_string(),
            });
        }
        ActionOutcome::PermissionDenied => {
            let _ = proxy.send_event(DaemonEvent::Error(ACCESSIBILITY_ERROR.to_string()));
        }
        ActionOutcome::Failed => {
            // Error already logged inside action::execute.
        }
    }
}

/// Format the first 8 bytes of params as a hex string.
fn format_hex(params: &[u8]) -> String {
    params
        .iter()
        .take(8)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
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
                                let dir_name = d.to_string();
                                let a = match d {
                                    gesture::GestureDirection::Up => gesture_cfg.up.as_ref(),
                                    gesture::GestureDirection::Down => gesture_cfg.down.as_ref(),
                                    gesture::GestureDirection::Left => gesture_cfg.left.as_ref(),
                                    gesture::GestureDirection::Right => {
                                        gesture_cfg.right.as_ref()
                                    }
                                };
                                (format!("swipe {dir_name}"), a)
                            }
                            gesture::GestureResult::Tap => {
                                ("tap".to_string(), gesture_cfg.tap.as_ref())
                            }
                        };
                        if let Some(action) = action {
                            let action_desc = action_description(action);
                            execute_and_notify(
                                action,
                                &format!("gesture CID {cid}: {desc} → {action_desc}"),
                                proxy,
                            );
                        } else {
                            info!("gesture CID {cid}: {desc} (no action mapped)");
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
                    let desc = action_description(action);
                    execute_and_notify(
                        action,
                        &format!("button CID {cid}: {desc}"),
                        proxy,
                    );
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

        _ => {
            let hex = format_hex(params);
            debug!(
                "unhandled notification: feature=0x{fid:04X} fn={} [{hex}]",
                function_id.0,
            );
        }
    }
}

/// Get a short description of an action for logging.
fn action_description(action: &crate::config::Action) -> &str {
    action
        .keystroke()
        .unwrap_or_else(|| action.command().unwrap_or("?"))
}
