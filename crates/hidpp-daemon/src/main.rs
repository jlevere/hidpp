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
use tracing::info;

use bridge::{DaemonCommand, DaemonEvent};

#[derive(Parser)]
#[command(
    name = "hidppd",
    about = "HID++ 2.0 — Logitech device daemon with menu bar UI"
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
    #[arg(long, global = true, value_parser = parse_device_index)]
    device_index: Option<DeviceIndex>,
}

#[derive(Subcommand)]
enum Command {
    /// Run with menu bar UI (default).
    Run,
    /// Listen to raw HID++ notifications (headless, no actions).
    Listen,
    /// Install as login item (launchd/systemd).
    Install,
    /// Uninstall login item.
    Uninstall,
    /// Print config file path.
    ConfigPath,
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

    match cli.command.unwrap_or(Command::Run) {
        Command::ConfigPath => {
            println!("{}", config::default_config_path().display());
            Ok(())
        }
        Command::SampleConfig => {
            print!("{}", daemon::SAMPLE_CONFIG);
            Ok(())
        }
        Command::Install => service::install(),
        Command::Uninstall => service::uninstall(),
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
    // Initialize enigo on main thread (needs NSApplication context on macOS).
    action::init()?;

    // Create tao event loop with our custom user event type.
    let event_loop = EventLoopBuilder::<DaemonEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // Command channel: tray UI → background daemon.
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<DaemonCommand>(8);

    // Build the menu bar.
    let menu = muda::Menu::new();
    let ts = tray::build(&menu)?;
    let mut connected = false;

    // Capture menu item IDs.
    let quit_id = ts.quit_item.id().clone();
    let reconnect_id = ts.reconnect_item.id().clone();
    let edit_config_id = ts.edit_config_item.id().clone();
    let login_id = ts.start_at_login_item.id().clone();
    let menu_channel = MenuEvent::receiver();

    // Spawn background daemon thread with tokio runtime.
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(daemon::run(&config_path, device_index, proxy, cmd_rx));
    });

    info!("menu bar app running");

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
                    connected = true;
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
                    connected = false;
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
                    ts.device_item.set_text(format!("Error: {msg}"));
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
                    let _ = cmd_tx.blocking_send(DaemonCommand::Shutdown);
                    *control_flow = ControlFlow::Exit;
                } else if ev.id == reconnect_id {
                    let _ = cmd_tx.blocking_send(DaemonCommand::Reconnect);
                } else if ev.id == edit_config_id {
                    let _ = std::process::Command::new("open")
                        .arg(config::default_config_path())
                        .spawn();
                } else if ev.id == login_id {
                    let currently_installed = service::is_installed();
                    if currently_installed {
                        let _ = service::uninstall();
                        ts.start_at_login_item.set_checked(false);
                    } else {
                        let _ = service::install();
                        ts.start_at_login_item.set_checked(true);
                    }
                }
            }
        }

        let _ = connected; // suppress unused warning from tao closure
    });
}
