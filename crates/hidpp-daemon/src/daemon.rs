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

/// How long after a gesture-button press we wait for the first rawXY
/// event. If none arrives in this window while the button is still held,
/// we assume the firmware silently dropped its rawXY divert and re-issue
/// it. The window only exists during a held press — never a free-running
/// timer.
const XY_VALIDATION_WINDOW: Duration = Duration::from_millis(150);

/// Side effect requested by `handle_notification` on the rawXY validation
/// deadline.
enum DeadlineUpdate {
    /// A gesture button was pressed — arm the validation window.
    Arm,
    /// rawXY arrived or the button was released — clear the window.
    Clear,
}

/// Drain any pending events from a channel without blocking.
fn drain<T>(rx: &mut tokio::sync::mpsc::Receiver<T>) {
    while rx.try_recv().is_ok() {}
}

/// Run the daemon with a tray UI event proxy.
///
/// Loops: connect → divert → listen → wait-for-event. Every wait in this
/// function is event-driven: either a user command, a system wake, or
/// a HID device arrival. No timers, no polling.
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

    // Event-driven watchers. Both fire on real OS signals, never on a clock.
    //   - wake watcher: macOS notify(3) on systempowerstate
    //   - HID watcher:  IOHIDManager device matched/removed callbacks
    let (wake_tx, mut wake_rx) = tokio::sync::mpsc::channel(8);
    crate::platform::spawn_wake_watcher(wake_tx.clone());
    crate::platform::spawn_hid_watcher(wake_tx);

    let mut last_error: Option<String> = None;

    loop {
        // Reload config on every iteration so Reconnect picks up changes.
        let cfg = match crate::config::load(&path) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("config: {e}");
                if last_error.as_deref() != Some(&msg) {
                    warn!("{msg}");
                    last_error = Some(msg);
                }
                let _ = proxy.send_event(DaemonEvent::Error("Config error".to_string()));
                // Wait for the user to fix it and click Reconnect from the
                // tray, or shut down. No polling.
                match cmd_rx.recv().await {
                    Some(DaemonCommand::Reconnect) => continue,
                    Some(DaemonCommand::Shutdown) | None => return,
                }
            }
        };

        let _ = proxy.send_event(DaemonEvent::Reconnecting);

        match connect_and_listen(&cfg, index_override, &proxy, &mut cmd_rx, &mut wake_rx).await {
            Ok(true) => {
                info!("shutdown requested");
                return;
            }
            Ok(false) => {
                info!("device disconnected, awaiting next event");
                last_error = None;
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

        // Event-driven wait between connection attempts. A device arrival
        // (HID watcher), system wake, or user command is the only thing
        // that wakes us. If none of those happen we stay parked — much
        // better than burning a retry timer that can't fix anything.
        tokio::select! {
            _ = wake_rx.recv() => {
                drain(&mut wake_rx);
            }
            cmd = cmd_rx.recv() => {
                if matches!(cmd, Some(DaemonCommand::Shutdown) | None) {
                    return;
                }
            }
        }
    }
}

/// Run in headless listen-only mode (no tray, no action execution).
///
/// Same event-driven discipline as `run`: waits on HID arrival for retries.
pub async fn run_listen_only(
    config_path: &Option<PathBuf>,
    index_override: Option<DeviceIndex>,
) -> anyhow::Result<()> {
    let path = config_path
        .clone()
        .unwrap_or_else(crate::config::default_config_path);

    info!("listen-only mode");

    let (wake_tx, mut wake_rx) = tokio::sync::mpsc::channel(8);
    crate::platform::spawn_hid_watcher(wake_tx);

    let mut last_error: Option<String> = None;

    loop {
        let _cfg = match crate::config::load(&path) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("config: {e}");
                if last_error.as_deref() != Some(&msg) {
                    warn!("{msg}");
                    last_error = Some(msg);
                }
                // No tray to surface this — wait for a device event before
                // retrying, so a broken file doesn't spam logs.
                let _ = wake_rx.recv().await;
                drain(&mut wake_rx);
                continue;
            }
        };

        match connect_and_listen_headless(index_override).await {
            Ok(()) => {
                info!("device disconnected, awaiting next arrival");
                last_error = None;
            }
            Err(e) => {
                let msg = e.to_string();
                if last_error.as_deref() != Some(&msg) {
                    warn!("error: {e}");
                    last_error = Some(msg);
                }
            }
        }

        let _ = wake_rx.recv().await;
        drain(&mut wake_rx);
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
///
/// Returns Ok(true) on shutdown, Ok(false) on disconnect / reload, Err on
/// connect/divert failure. The listen loop is a single select! with one
/// arm per event source — there are no timers other than the per-press
/// rawXY validation watchdog, which exists only while a gesture button is
/// physically held down.
async fn connect_and_listen(
    cfg: &Config,
    index_override: Option<DeviceIndex>,
    proxy: &EventLoopProxy<DaemonEvent>,
    cmd_rx: &mut tokio::sync::mpsc::Receiver<DaemonCommand>,
    wake_rx: &mut tokio::sync::mpsc::Receiver<()>,
) -> anyhow::Result<bool> {
    // Prevent system idle sleep during device setup (connect, discover, divert).
    let _power_guard = crate::platform::PowerAssertion::prevent_idle_sleep("HID++ device setup");

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
    // (On non-macOS PowerAssertion is a no-op unit struct, but the
    // explicit drop documents the intent.)
    #[allow(clippy::drop_non_drop)]
    drop(_power_guard);

    // Check Accessibility permission early so the user sees the error immediately.
    if !action::ensure_init() {
        let _ = proxy.send_event(DaemonEvent::Error(ACCESSIBILITY_ERROR.to_string()));
    }

    // Drain stale events — IOHIDManager fires matching callbacks for
    // already-connected devices on creation. Discard those here so we
    // don't immediately trigger a spurious reconnect.
    drain(wake_rx);

    let mut rx = device.subscribe();
    let mut gestures = GestureTracker::new();

    // rawXY validation deadline. Some(t) means a gesture button is held
    // and we expect rawXY by time t. Cleared by either the first rawXY
    // (proof divert is live) or the button release. If it fires, the
    // firmware almost certainly dropped its volatile rawXY divert and
    // we re-issue divert on all gesture CIDs.
    let mut xy_deadline: Option<tokio::time::Instant> = None;

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(report) => {
                        match handle_notification(&device, &report, cfg, &mut gestures, proxy) {
                            Some(DeadlineUpdate::Arm) => {
                                xy_deadline = Some(tokio::time::Instant::now() + XY_VALIDATION_WINDOW);
                            }
                            Some(DeadlineUpdate::Clear) => {
                                xy_deadline = None;
                            }
                            None => {}
                        }
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
                    Some(DaemonCommand::Reconnect) => {
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
                drain(wake_rx);
                info!("reconnection trigger (wake/HID), re-diverting buttons");
                return Ok(false);
            }
            _ = tokio::time::sleep_until(xy_deadline.unwrap_or_else(tokio::time::Instant::now)), if xy_deadline.is_some() => {
                xy_deadline = None;
                warn!("rawXY missing for held gesture button — re-diverting");
                let gesture_cids: Vec<u16> = cfg.gestures.keys().copied().collect();
                for cid in gesture_cids {
                    if let Err(e) = device
                        .special_key_set_reporting(
                            ControlId(cid),
                            DIVERT_RAW_XY_FLAGS,
                            ControlId(0),
                            0,
                        )
                        .await
                    {
                        warn!("re-divert CID {cid} failed: {e} — reconnecting");
                        return Ok(false);
                    }
                }
            }
        }
    }
}

/// Headless connect_and_listen — no diversion, no actions, just log.
async fn connect_and_listen_headless(index_override: Option<DeviceIndex>) -> anyhow::Result<()> {
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

/// Execute an action, log it with CID context, and surface the short
/// form to the tray's "Last:" line.
fn execute_and_notify(
    action: &crate::config::Action,
    log_desc: &str,
    tray_desc: &str,
    proxy: &EventLoopProxy<DaemonEvent>,
) {
    match action::execute(action) {
        ActionOutcome::Executed => {
            info!("{log_desc}");
            let _ = proxy.send_event(DaemonEvent::ActionExecuted {
                description: tray_desc.to_string(),
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

/// Handle a notification: execute any mapped action, update gesture state,
/// and report what the caller should do with the rawXY validation deadline.
fn handle_notification(
    device: &hidpp_device::Device,
    report: &LongReport,
    cfg: &Config,
    gestures: &mut GestureTracker,
    proxy: &EventLoopProxy<DaemonEvent>,
) -> Option<DeadlineUpdate> {
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
                                    gesture::GestureDirection::Right => gesture_cfg.right.as_ref(),
                                };
                                (format!("swipe {dir_name}"), a)
                            }
                            gesture::GestureResult::Tap => {
                                ("tap".to_string(), gesture_cfg.tap.as_ref())
                            }
                        };
                        if let Some(action) = action {
                            let action_desc = action_description(action);
                            let tray_desc = format!("{desc} → {action_desc}");
                            execute_and_notify(
                                action,
                                &format!("gesture CID {cid}: {tray_desc}"),
                                &tray_desc,
                                proxy,
                            );
                        } else {
                            info!("gesture CID {cid}: {desc} (no action mapped)");
                        }
                    }
                }
                return Some(DeadlineUpdate::Clear);
            }

            // Button(s) pressed.
            let mut arm = false;
            for &cid in &cids {
                if cfg.is_gesture_cid(cid) {
                    gestures.button_pressed(cid);
                    arm = true;
                } else if let Some(action) = cfg.buttons.get(&cid) {
                    let desc = action_description(action);
                    execute_and_notify(
                        action,
                        &format!("button CID {cid}: {desc}"),
                        desc,
                        proxy,
                    );
                }
            }
            if arm {
                Some(DeadlineUpdate::Arm)
            } else {
                None
            }
        }

        // SpecialKeys v4 — diverted rawXY event (fn=1).
        0x1B04 if function_id.0 == 1 && params.len() >= 4 => {
            let dx = i16::from_be_bytes([params[0], params[1]]);
            let dy = i16::from_be_bytes([params[2], params[3]]);
            gestures.feed_raw_xy(dx, dy);
            Some(DeadlineUpdate::Clear)
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
            None
        }

        _ => {
            let hex = format_hex(params);
            debug!(
                "unhandled notification: feature=0x{fid:04X} fn={} [{hex}]",
                function_id.0,
            );
            None
        }
    }
}

/// Get a short description of an action for logging.
fn action_description(action: &crate::config::Action) -> &str {
    action
        .keystroke()
        .unwrap_or_else(|| action.command().unwrap_or("?"))
}
