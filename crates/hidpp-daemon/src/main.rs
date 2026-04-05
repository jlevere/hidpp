mod action;
mod config;
mod gesture;
mod service;

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use hidpp::feature_id;
use hidpp::report::LongReport;
use hidpp::types::DeviceIndex;
use hidpp_transport::native::HidapiEnumerator;
use tokio::sync::broadcast;
use tracing::{info, warn};

use config::Config;
use gesture::GestureTracker;

#[derive(Parser)]
#[command(
    name = "hidppd",
    about = "HID++ 2.0 daemon — catches diverted events and maps them to actions"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Config file path.
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Log verbosity (set RUST_LOG for fine control).
    #[arg(short, long, global = true, default_value = "info")]
    log_level: String,

    /// Override device index (hex: FF for BLE, 01-06 for receiver slots).
    /// Auto-detected if not specified.
    #[arg(long, global = true, value_parser = parse_device_index)]
    device_index: Option<DeviceIndex>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the daemon (default).
    Run,
    /// Listen to raw HID++ notifications and print them (no action execution).
    Listen,
    /// Install the daemon as a system service (launchd on macOS, systemd on Linux).
    Install,
    /// Uninstall the daemon service.
    Uninstall,
    /// Print the default config path.
    ConfigPath,
    /// Generate a sample config file.
    SampleConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level)),
        )
        .with_target(false)
        .with_level(true)
        .init();

    match cli.command.unwrap_or(Command::Run) {
        Command::ConfigPath => {
            println!("{}", config::default_config_path().display());
            Ok(())
        }
        Command::SampleConfig => {
            print!("{SAMPLE_CONFIG}");
            Ok(())
        }
        Command::Install => service::install(),
        Command::Uninstall => service::uninstall(),
        Command::Listen => run_daemon(&cli.config, cli.device_index, false).await,
        Command::Run => run_daemon(&cli.config, cli.device_index, true).await,
    }
}

fn parse_device_index(s: &str) -> Result<DeviceIndex, String> {
    let val = u8::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16)
        .map_err(|e| format!("invalid hex device index: {e}"))?;
    Ok(DeviceIndex(val))
}

const SAMPLE_CONFIG: &str = r#"# hidppd config — maps diverted buttons and gestures to actions.
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
const DIVERT_FLAGS: u8 = 0x03; // 0b0000_0011 = divert + dvalid
const DIVERT_RAW_XY_FLAGS: u8 = 0x33; // 0b0011_0011 = divert + dvalid + rawXY + rvalid

/// Main daemon loop: connect → subscribe → handle notifications → reconnect.
async fn run_daemon(
    config_path: &Option<PathBuf>,
    index_override: Option<DeviceIndex>,
    execute_actions: bool,
) -> anyhow::Result<()> {
    let path = config_path
        .clone()
        .unwrap_or_else(config::default_config_path);

    let cfg = config::load(&path)?;

    if execute_actions {
        action::init()?;
        info!("action execution enabled");
    } else {
        info!("listen-only mode (no action execution)");
    }

    info!("hidppd starting");

    loop {
        match connect_and_listen(&cfg, index_override, execute_actions).await {
            Ok(true) => {
                info!("shutting down");
                return Ok(());
            }
            Ok(false) => {
                info!("device disconnected, reconnecting...");
            }
            Err(e) => {
                warn!("error: {e}");
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            _ = shutdown_signal() => {
                info!("shutting down");
                return Ok(());
            }
        }
    }
}

/// Connect, subscribe to notifications, process until disconnect or shutdown.
/// Returns `Ok(true)` for shutdown, `Ok(false)` for disconnect.
async fn connect_and_listen(
    cfg: &Config,
    index_override: Option<DeviceIndex>,
    execute_actions: bool,
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

    info!(
        "connected: {} ({} features)",
        device.name(),
        device.features().count()
    );

    // Auto-divert buttons and gesture buttons.
    if execute_actions && device.supports(feature_id::SPECIAL_KEYS_V4) {
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

    let mut rx = device.subscribe();
    let mut gestures = GestureTracker::new();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(report) => {
                        handle_notification(&device, &report, cfg, execute_actions, &mut gestures);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("dropped {n} notifications");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Ok(false);
                    }
                }
            }
            _ = shutdown_signal() => {
                return Ok(true);
            }
        }
    }
}

/// Decode and handle an incoming HID++ notification.
fn handle_notification(
    device: &hidpp_device::Device,
    report: &LongReport,
    cfg: &Config,
    execute_actions: bool,
    gestures: &mut GestureTracker,
) {
    let feature_index = report.feature_index();
    let function_id = report.function_id();
    let params = report.params();
    let feature_id = device.feature_id_for_index(feature_index);

    let feature_name = feature_id
        .and_then(hidpp::feature_id::feature_name)
        .unwrap_or("Unknown");

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
                // All buttons released.
                if execute_actions {
                    for (&cid, gesture_cfg) in &cfg.gestures {
                        if let Some(result) = gestures.button_released(cid, gesture_cfg.threshold) {
                            let action = match &result {
                                gesture::GestureResult::Direction(d) => {
                                    info!("gesture CID {cid}: swipe {d:?}");
                                    match d {
                                        gesture::GestureDirection::Up => gesture_cfg.up.as_ref(),
                                        gesture::GestureDirection::Down => {
                                            gesture_cfg.down.as_ref()
                                        }
                                        gesture::GestureDirection::Left => {
                                            gesture_cfg.left.as_ref()
                                        }
                                        gesture::GestureDirection::Right => {
                                            gesture_cfg.right.as_ref()
                                        }
                                    }
                                }
                                gesture::GestureResult::Tap => {
                                    info!("gesture CID {cid}: tap");
                                    gesture_cfg.tap.as_ref()
                                }
                            };
                            if let Some(action) = action {
                                action::execute(action);
                            }
                        }
                    }
                }
                return;
            }

            // Button(s) pressed.
            let names: Vec<String> = cids
                .iter()
                .map(|&cid| format!("CID {cid} (0x{cid:04X})"))
                .collect();
            info!("button: {}", names.join(" + "));

            if execute_actions {
                for &cid in &cids {
                    if cfg.is_gesture_cid(cid) {
                        gestures.button_pressed(cid);
                    } else if let Some(action) = cfg.buttons.get(&cid) {
                        action::execute(action);
                    }
                }
            }
        }

        // SpecialKeys v4 — diverted rawXY event (fn=1).
        0x1B04 if function_id.0 == 1 && params.len() >= 4 => {
            let dx = i16::from_be_bytes([params[0], params[1]]);
            let dy = i16::from_be_bytes([params[2], params[3]]);
            gestures.feed_raw_xy(dx, dy);
        }

        // HiResWheel — scroll event.
        0x2121 if function_id.0 == 0 && params.len() >= 3 => {
            let delta = i16::from_be_bytes([params[1], params[2]]);
            let direction = if delta > 0 { "down" } else { "up" };
            info!("scroll {direction} delta={delta}");
        }

        // Thumbwheel — rotation event.
        0x2150 if function_id.0 == 0 && params.len() >= 2 => {
            let rotation = i16::from_be_bytes([params[0], params[1]]);
            let direction = if rotation > 0 { "right" } else { "left" };
            info!("thumbwheel {direction} rotation={rotation}");
        }

        // UnifiedBattery — battery status change.
        0x1004 if function_id.0 == 0 && !params.is_empty() => {
            info!("battery {}%", params[0]);
        }

        // ConfigChange.
        0x0020 if function_id.0 == 0 => {
            info!("config changed on device");
        }

        // Unknown.
        _ => {
            let hex: String = params
                .iter()
                .take(8)
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            info!(
                "notification: {feature_name} (0x{fid:04X}) fn={} [{hex}]",
                function_id.0,
            );
        }
    }
}

/// Wait for Ctrl-C.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
}
