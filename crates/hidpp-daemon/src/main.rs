mod action;
mod bridge;
mod config;
mod daemon;
mod gesture;
mod icon;
mod service;
mod tray;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use hidpp::types::DeviceIndex;
use muda::MenuEvent;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tracing::{info, warn};

use bridge::{DaemonCommand, DaemonEvent};

#[derive(Parser)]
#[command(name = "hidppd", about = "HID++ — Logitech device configurator")]
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
    #[arg(long, global = true, value_parser = parse_device_index)]
    device_index: Option<DeviceIndex>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the app (default).
    Run,
    /// Headless mode — log notifications, no UI, no actions.
    Listen,
    /// Print sample config.
    SampleConfig,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level)),
        )
        .with_target(false)
        .with_level(true)
        .init();

    // Panic hook — ensure crashes are logged to stderr (captured by launchd).
    std::panic::set_hook(Box::new(|info| {
        eprintln!("hidppd crashed: {info}");
    }));

    match cli.command.unwrap_or(Command::Run) {
        Command::SampleConfig => {
            print!("{}", daemon::SAMPLE_CONFIG);
            Ok(())
        }
        Command::Listen => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(daemon::run_listen_only(&cli.config, cli.device_index))
        }
        Command::Run => run_tray_app(cli.config, cli.device_index),
    }
}

fn parse_device_index(s: &str) -> Result<DeviceIndex, String> {
    let val = u8::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16)
        .map_err(|e| format!("invalid hex device index: {e}"))?;
    Ok(DeviceIndex(val))
}

fn run_tray_app(
    config_path: Option<PathBuf>,
    device_index: Option<DeviceIndex>,
) -> anyhow::Result<()> {
    // Single instance enforcement.
    let _lock = match single_instance_lock() {
        Ok(lock) => lock,
        Err(msg) => {
            eprintln!("{msg}");
            // On macOS, show a dialog so the user knows why nothing happened.
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(r#"display dialog "HID++ is already running." buttons {"OK"} default button "OK" with icon caution"#)
                    .output();
            }
            std::process::exit(0);
        }
    };

    // Create config on first launch if needed.
    let cfg_path = config_path
        .clone()
        .unwrap_or_else(config::default_config_path);
    let first_launch = !cfg_path.exists();
    if first_launch {
        if let Some(parent) = cfg_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&cfg_path, daemon::SAMPLE_CONFIG) {
            Ok(()) => info!("created default config at {}", cfg_path.display()),
            Err(e) => warn!("failed to create config at {}: {e}", cfg_path.display()),
        }
    }

    // Create tao event loop.
    let mut event_loop = EventLoopBuilder::<DaemonEvent>::with_user_event().build();

    // Hide dock icon.
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::EventLoopExtMacOS;
        event_loop.set_activation_policy(tao::platform::macos::ActivationPolicy::Accessory);
    }

    let proxy = event_loop.create_proxy();

    // Command channel: tray UI → background daemon.
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<DaemonCommand>(8);

    // Build the menu bar.
    let menu = muda::Menu::new();
    let ts = tray::build(&menu)?;

    // Capture menu item IDs.
    let quit_id = ts.quit_item.id().clone();
    let reconnect_id = ts.reconnect_item.id().clone();
    let edit_config_id = ts.edit_config_item.id().clone();
    let login_id = ts.start_at_login_item.id().clone();
    let menu_channel = MenuEvent::receiver();

    // First-launch notification.
    if first_launch {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("osascript")
                .arg("-e")
                .arg(r#"display notification "Click the mouse icon in the menu bar to configure." with title "HID++""#)
                .spawn();
        }
    }

    // Spawn background daemon thread.
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(daemon::run(&config_path, device_index, proxy, cmd_rx));
    });

    info!("HID++ running");

    // Main event loop.
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Handle daemon events from background thread.
        if let Event::UserEvent(daemon_event) = &event {
            match daemon_event {
                DaemonEvent::Connected {
                    name,
                    battery_pct,
                    dpi,
                } => {
                    ts.device_item.set_text(name);
                    if let Some(pct) = battery_pct {
                        ts.battery_item.set_text(format!("Battery: {pct}%"));
                        ts.tray.set_title(Some(&format!("{pct}%")));
                    } else {
                        ts.battery_item.set_text("Battery: --");
                        ts.tray.set_title(Some(""));
                    }
                    if let Some(d) = dpi {
                        ts.dpi_item.set_text(format!("DPI: {d}"));
                    }
                    let _ = ts.tray.set_icon(Some(ts.icon_connected.clone()));
                }
                DaemonEvent::Disconnected | DaemonEvent::Reconnecting => {
                    ts.device_item.set_text("Searching...");
                    ts.battery_item.set_text("Battery: --");
                    ts.dpi_item.set_text("DPI: --");
                    ts.tray.set_title(Some("--"));
                    let _ = ts.tray.set_icon(Some(ts.icon_disconnected.clone()));
                }
                DaemonEvent::BatteryUpdate {
                    percentage,
                    charging,
                } => {
                    let status = if *charging { " ⚡" } else { "" };
                    ts.battery_item
                        .set_text(format!("Battery: {percentage}%{status}"));
                    ts.tray.set_title(Some(&format!("{percentage}%")));
                }
                DaemonEvent::ActionExecuted { description } => {
                    ts.last_action_item.set_text(format!("Last: {description}"));
                }
                DaemonEvent::Error(msg) => {
                    // Truncate long error messages so the menu doesn't stretch.
                    let short = if msg.len() > 40 { &msg[..40] } else { msg };
                    ts.device_item.set_text(format!("Error: {short}"));
                }
            }
        }

        // Handle hidpp:// URL scheme (config push from web UI).
        if let Event::Opened { urls } = &event {
            for url in urls {
                if url.scheme() == "hidpp" {
                    info!("received URL: {url}");
                    if let Some(toml_str) = url
                        .query_pairs()
                        .find(|(k, _)| k == "toml")
                        .map(|(_, v)| v.into_owned())
                    {
                        match handle_config_url(&toml_str) {
                            Ok(()) => {
                                ts.last_action_item.set_text("Config updated from web UI");
                                let _ = cmd_tx.try_send(DaemonCommand::ReloadConfig);
                                info!("config updated from web UI, reloading");
                            }
                            Err(e) => {
                                warn!("invalid config from URL: {e}");
                                ts.last_action_item.set_text(format!("Config error: {e}"));
                            }
                        }
                    }
                }
            }
        }

        // Handle menu clicks.
        if matches!(
            &event,
            Event::NewEvents(StartCause::Poll)
                | Event::NewEvents(StartCause::Init)
                | Event::NewEvents(StartCause::WaitCancelled { .. })
        ) {
            while let Ok(ev) = menu_channel.try_recv() {
                if ev.id == quit_id {
                    let _ = cmd_tx.try_send(DaemonCommand::Shutdown);
                    *control_flow = ControlFlow::Exit;
                } else if ev.id == reconnect_id {
                    let _ = cmd_tx.try_send(DaemonCommand::Reconnect);
                } else if ev.id == edit_config_id {
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open")
                        .arg(config::default_config_path())
                        .spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open")
                        .arg(config::default_config_path())
                        .spawn();
                } else if ev.id == login_id {
                    if service::is_installed() {
                        if service::uninstall().is_ok() {
                            ts.start_at_login_item.set_checked(false);
                        }
                    } else if service::register_login_item().is_ok() {
                        ts.start_at_login_item.set_checked(true);
                    }
                }
            }
        }
    });
}

/// Validate and write a TOML config received from a hidpp:// URL.
///
/// Security: rejects configs containing `command` actions to prevent
/// arbitrary code execution from malicious websites opening hidpp:// URLs.
fn handle_config_url(toml_str: &str) -> anyhow::Result<()> {
    if toml_str.contains("type") && toml_str.contains("command") {
        anyhow::bail!("command actions are not allowed via URL — edit config.toml directly");
    }

    config::validate(toml_str)?;

    let path = config::default_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml_str)?;
    info!("wrote config to {}", path.display());
    Ok(())
}

/// Acquire a single-instance lock. Returns a guard that releases on drop.
/// Returns Err if another instance is already running.
#[cfg(unix)]
fn single_instance_lock() -> Result<std::os::unix::net::UnixListener, String> {
    use std::os::unix::net::UnixListener;

    let sock_path = std::env::temp_dir().join("hidpp-daemon.sock");

    // Try to bind. If it succeeds, we're the only instance.
    match UnixListener::bind(&sock_path) {
        Ok(listener) => Ok(listener),
        Err(_) => {
            // Socket exists — try to clean up stale socket and retry.
            let _ = std::fs::remove_file(&sock_path);
            UnixListener::bind(&sock_path).map_err(|_| "HID++ is already running.".to_string())
        }
    }
}

#[cfg(not(unix))]
fn single_instance_lock() -> Result<(), String> {
    // No single-instance enforcement on non-Unix platforms.
    Ok(())
}
