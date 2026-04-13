mod action;
mod bridge;
mod config;
mod daemon;
mod gesture;
mod icon;
mod platform;
mod service;
mod tray;

// Embed Info.plist into the Mach-O binary so macOS TCC (Input Monitoring,
// Accessibility) can identify the app by bundle ID across binary updates,
// avoiding duplicate permission entries in System Settings.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
#[unsafe(link_section = "__TEXT,__info_plist")]
#[used]
static INFO_PLIST: [u8; include_bytes!("../../../bundle/Info.plist").len()] =
    *include_bytes!("../../../bundle/Info.plist");

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use hidpp::types::DeviceIndex;
use muda::MenuEvent;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tracing::{info, warn};
use tracing_subscriber::prelude::*;

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

    init_logging(&cli.log_level);

    // Panic hook — ensure crashes are logged to the log file and stderr.
    std::panic::set_hook(Box::new(|info| {
        tracing::error!("hidppd crashed: {info}");
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
    let signal_proxy = event_loop.create_proxy();

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
    let device_id = ts.device_item.id().clone();
    let menu_channel = MenuEvent::receiver();

    // URL to open when the device item is clicked (set on permission errors).
    let mut device_item_url: Option<&str> = None;
    let mut permission_error = false;

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

    // Catch SIGTERM (sent by launchd on service stop / system shutdown)
    // and trigger a graceful shutdown through both the daemon and the event loop.
    {
        let sigterm_tx = cmd_tx.clone();
        std::thread::Builder::new()
            .name("signal-handler".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("signal handler runtime");
                rt.block_on(async {
                    #[cfg(unix)]
                    {
                        let mut sigterm =
                            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                                .expect("SIGTERM handler");
                        sigterm.recv().await;
                    }
                    #[cfg(not(unix))]
                    {
                        tokio::signal::ctrl_c().await.ok();
                    }
                    info!("received shutdown signal");
                    // Tell the daemon to clean up HID resources.
                    let _ = sigterm_tx.send(DaemonCommand::Shutdown).await;
                    // Tell the event loop to exit (works even if daemon already exited).
                    let _ = signal_proxy.send_event(DaemonEvent::Shutdown);
                });
            })
            .expect("signal handler thread");
    }

    // Spawn background daemon thread.
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(daemon::run(&config_path, device_index, proxy.clone(), cmd_rx));
        // Daemon exited (shutdown command or signal) — tell the event loop to quit.
        let _ = proxy.send_event(DaemonEvent::Shutdown);
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
                    ts.device_item.set_enabled(false);
                    device_item_url = None;
                    if permission_error {
                        permission_error = false;
                        ts.reconnect_item.set_text("Reconnect");
                    }
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
                    ts.device_item.set_enabled(false);
                    device_item_url = None;
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
                DaemonEvent::Shutdown => {
                    *control_flow = ControlFlow::Exit;
                }
                DaemonEvent::Error(msg) => {
                    let short = if msg.len() > 40 { &msg[..40] } else { msg };
                    // Permission errors become clickable — open the relevant settings pane.
                    // Also swap Reconnect → Relaunch since macOS caches TCC at process start.
                    if msg.contains("Input Monitoring") {
                        ts.device_item.set_text(format!("{short} →"));
                        ts.device_item.set_enabled(true);
                        device_item_url = Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent");
                        if !permission_error {
                            permission_error = true;
                            ts.reconnect_item.set_text("Relaunch");
                        }
                    } else if msg.contains("Accessibility") {
                        ts.device_item.set_text(format!("{short} →"));
                        ts.device_item.set_enabled(true);
                        device_item_url = Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility");
                        if !permission_error {
                            permission_error = true;
                            ts.reconnect_item.set_text("Relaunch");
                        }
                    } else {
                        ts.device_item.set_text(format!("Error: {short}"));
                        ts.device_item.set_enabled(false);
                        device_item_url = None;
                    }
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
                    if permission_error {
                        // macOS caches TCC permissions at process start.
                        // Graceful shutdown lets launchd/systemd restart us fresh.
                        let _ = cmd_tx.try_send(DaemonCommand::Shutdown);
                        *control_flow = ControlFlow::Exit;
                    } else {
                        let _ = cmd_tx.try_send(DaemonCommand::Reconnect);
                    }
                } else if ev.id == edit_config_id {
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open")
                        .arg(config::default_config_path())
                        .spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open")
                        .arg(config::default_config_path())
                        .spawn();
                } else if ev.id == device_id {
                    if let Some(url) = device_item_url {
                        let _ = std::process::Command::new("open").arg(url).spawn();
                    }
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

/// Set up tracing with dual output: stderr + platform log.
///
/// - macOS: stderr + os_log (visible in Console.app / `log show`)
/// - Linux: stderr + log file at `$XDG_STATE_HOME/hidpp/hidppd.log`
fn init_logging(log_level: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_level(true);

    #[cfg(target_os = "macos")]
    {
        let oslog_layer = tracing_oslog::OsLogger::new("com.jlevere.hidpp", "default");

        tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .with(oslog_layer)
            .init();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let state_dir = std::env::var("XDG_STATE_HOME")
            .unwrap_or_else(|_| format!("{home}/.local/state"));
        let log_path = std::path::Path::new(&state_dir).join("hidpp/hidppd.log");

        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            Ok(file) => {
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_level(true)
                    .with_ansi(false)
                    .with_writer(std::sync::Mutex::new(file));

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(stderr_layer)
                    .with(file_layer)
                    .init();
            }
            Err(e) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(stderr_layer)
                    .init();

                tracing::warn!("could not open log file {}: {e}", log_path.display());
            }
        }
    }
}
