use clap::{Parser, Subcommand};
use hidpp::types::DeviceIndex;
use hidpp_transport::native::HidapiEnumerator;
use tracing::info;

#[derive(Parser)]
#[command(name = "hidpp", about = "HID++ 2.0 device configuration tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Log verbosity (set RUST_LOG for fine control).
    #[arg(short, long, global = true, default_value = "warn")]
    log_level: String,

    /// Override device index (hex: FF for BLE, 01-06 for receiver slots).
    /// Auto-detected if not specified.
    #[arg(long, global = true, value_parser = parse_device_index)]
    device_index: Option<DeviceIndex>,
}

#[derive(Subcommand)]
enum Command {
    /// List connected HID++ devices.
    List,
    /// Show device info, features, and current settings.
    Info,
    /// Get a device setting.
    Get {
        /// Setting: battery, dpi, smartshift, wheel, thumbwheel, host.
        setting: String,
    },
    /// Set a device setting.
    Set {
        /// Setting: dpi, smartshift, wheel.
        setting: String,
        /// Value to set.
        value: String,
    },
    /// Export device config to TOML (stdout).
    Export,
    /// Import device config from TOML file.
    Import {
        /// Path to TOML config file (or - for stdin).
        file: String,
    },
    /// Send a raw HID++ request (for experimentation).
    Raw {
        /// Feature ID in hex (e.g., 0x2110).
        feature: String,
        /// Function ID (0-15).
        function: u8,
        /// Parameter bytes in hex (e.g., "01 32 53").
        #[arg(default_value = "")]
        params: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(&cli.log_level)
                }),
        )
        .with_target(false)
        .with_level(true)
        .init();

    let idx = cli.device_index;
    match cli.command {
        Command::List => cmd_list()?,
        Command::Info => cmd_info(idx).await?,
        Command::Get { setting } => cmd_get(idx, &setting).await?,
        Command::Set { setting, value } => cmd_set(idx, &setting, &value).await?,
        Command::Export => cmd_export(idx).await?,
        Command::Import { file } => cmd_import(idx, &file).await?,
        Command::Raw {
            feature,
            function,
            params,
        } => cmd_raw(idx, &feature, function, &params).await?,
    }

    Ok(())
}

fn parse_device_index(s: &str) -> Result<DeviceIndex, String> {
    let val = u8::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16)
        .map_err(|e| format!("invalid hex device index: {e}"))?;
    Ok(DeviceIndex(val))
}

fn cmd_list() -> anyhow::Result<()> {
    let enumerator = HidapiEnumerator::new()?;
    let devices = enumerator.enumerate();

    if devices.is_empty() {
        println!("No HID++ devices found.");
        return Ok(());
    }

    for dev in &devices {
        let name = dev.name.as_deref().unwrap_or("Unknown");
        println!(
            "{:04X}:{:04X}  {}",
            dev.vendor_id, dev.product_id, name,
        );
    }

    Ok(())
}

/// Open the first available HID++ device, auto-detecting the device index.
async fn open_first_device(
    index_override: Option<DeviceIndex>,
) -> anyhow::Result<hidpp_device::Device> {
    let enumerator = HidapiEnumerator::new()?;
    let devices = enumerator.enumerate();

    let dev_info = devices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No HID++ devices found"))?;

    let transport = enumerator.open(dev_info)?;

    let device_index = match index_override {
        Some(idx) => {
            info!("using device index override: 0x{:02X}", idx.0);
            idx
        }
        None => {
            let idx = hidpp_device::Device::probe_device_index(&transport).await?;
            info!("auto-detected device index: 0x{:02X}", idx.0);
            idx
        }
    };

    let device = hidpp_device::Device::open(transport, device_index).await?;
    Ok(device)
}

async fn cmd_info(idx: Option<DeviceIndex>) -> anyhow::Result<()> {
    let device = open_first_device(idx).await?;

    println!("Device:   {}", device.name());
    if let Some(dtype) = device.device_type() {
        println!("Type:     {dtype:?}");
    }
    let (major, minor) = device.protocol_version();
    println!("Protocol: HID++ {major}.{minor}");
    println!("Features: {}", device.features().count());
    println!();

    // Print features table.
    for feature in device.features() {
        let name = hidpp::feature_id::feature_name(feature.id).unwrap_or("-");
        let flags = if feature.flags.contains(hidpp::types::FeatureFlags::ENGINEERING_HIDDEN) {
            " [hidden]"
        } else {
            ""
        };
        println!(
            "  [{:02X}] {:<8} {:<30} v{}{flags}",
            feature.index.0,
            format!("{}", feature.id),
            name,
            feature.version,
        );
    }
    println!();

    // Print current settings.
    if device.supports(hidpp::feature_id::UNIFIED_BATTERY) {
        match device.battery_status().await {
            Ok(s) => println!("Battery:     {}% ({:?}, {:?})", s.percentage, s.level, s.charging),
            Err(e) => println!("Battery:     error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::ADJUSTABLE_DPI) {
        match device.dpi_get().await {
            Ok(dpi) => println!("DPI:         {dpi}"),
            Err(e) => println!("DPI:         error ({e})"),
        }
    }

    if device.has_smart_shift() {
        match device.smart_shift_get().await {
            Ok(s) => println!(
                "SmartShift:  {:?} (auto_disengage={}, torque={})",
                s.mode, s.auto_disengage, s.tunable_torque,
            ),
            Err(e) => println!("SmartShift:  error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::HIRES_WHEEL) {
        match device.hires_wheel_get_mode().await {
            Ok(m) => println!(
                "Wheel:       hires={}, inverted={}",
                m.high_resolution, m.inverted,
            ),
            Err(e) => println!("Wheel:       error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::THUMBWHEEL) {
        match device.thumbwheel_get_status().await {
            Ok(s) => println!(
                "Thumbwheel:  {:?}, inverted={}",
                s.reporting_mode, s.inverted,
            ),
            Err(e) => println!("Thumbwheel:  error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::DEVICE_FRIENDLY_NAME) {
        match device.friendly_name().await {
            Ok(name) => println!("BT Name:     {name}"),
            Err(e) => println!("BT Name:     error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::CHANGE_HOST) {
        match device.host_info().await {
            Ok(h) => {
                println!("Easy-Switch: host {} of {}", h.current_host + 1, h.num_hosts);
                // Show OS for each host slot.
                if device.supports(hidpp::feature_id::HOSTS_INFOS) {
                    for i in 0..h.num_hosts {
                        if let Ok(os) = device.host_os_version(i).await {
                            let marker = if i == h.current_host { "→" } else { " " };
                            println!(
                                "  {marker} Slot {}: {:?} v{}.{}",
                                i + 1, os.os_type, os.version_major, os.version_minor,
                            );
                        }
                    }
                }
            }
            Err(e) => println!("Easy-Switch: error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::WIRELESS_STATUS) {
        match device.wireless_status().await {
            Ok(ws) => println!("Wireless:    {:?}", ws.status),
            Err(e) => println!("Wireless:    error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::FIRMWARE_INFO) {
        match device.firmware_info().await {
            Ok(entities) => {
                for e in &entities {
                    println!(
                        "Firmware:    {} {:?} v{:02X}.{:02X} build {}",
                        e.name, e.entity_type, e.version_major, e.version_minor, e.build,
                    );
                }
            }
            Err(e) => println!("Firmware:    error ({e})"),
        }
    }

    if device.supports(hidpp::feature_id::SPECIAL_KEYS_V4) {
        match device.special_keys_list().await {
            Ok(controls) => {
                println!("Buttons:     {} remappable", controls.len());
                for c in &controls {
                    let name = hidpp_device::DeviceProfile::by_pid("b034")
                        .and_then(|p| p.button_name(c.cid))
                        .unwrap_or("?");
                    let flags = if c.is_divertable() { "divertable" } else { "" };
                    println!(
                        "  CID {:>3} (0x{:04X}) → TID {:>3}  {:<15} {}",
                        c.cid, c.cid, c.tid, name, flags,
                    );
                }
            }
            Err(e) => println!("Buttons:     error ({e})"),
        }
    }

    Ok(())
}

async fn cmd_get(idx: Option<DeviceIndex>, setting: &str) -> anyhow::Result<()> {
    let device = open_first_device(idx).await?;

    match setting {
        "battery" => {
            let s = device.battery_status().await?;
            println!("{}% | {:?} | {:?} | external_power={}", s.percentage, s.level, s.charging, s.external_power);
        }
        "dpi" => {
            println!("{}", device.dpi_get().await?);
        }
        "smartshift" => {
            let s = device.smart_shift_get().await?;
            println!("mode={:?} auto_disengage={} torque={}", s.mode, s.auto_disengage, s.tunable_torque);
        }
        "wheel" => {
            let m = device.hires_wheel_get_mode().await?;
            println!("hires={} inverted={} diverted={}", m.high_resolution, m.inverted, m.diverted);
        }
        "thumbwheel" => {
            let s = device.thumbwheel_get_status().await?;
            println!("mode={:?} inverted={} diverted={}", s.reporting_mode, s.inverted, s.diverted);
        }
        "host" => {
            let h = device.host_info().await?;
            println!("host {} of {}", h.current_host + 1, h.num_hosts);
        }
        "firmware" | "fw" => {
            let entities = device.firmware_info().await?;
            for e in &entities {
                println!(
                    "{:?} {} v{:02X}.{:02X} build {}",
                    e.entity_type, e.name, e.version_major, e.version_minor, e.build,
                );
            }
        }
        "buttons" => {
            let controls = device.special_keys_list().await?;
            for c in &controls {
                let name = hidpp_device::DeviceProfile::by_pid("b034")
                    .and_then(|p| p.button_name(c.cid))
                    .unwrap_or("?");
                // Also read current reporting state.
                let reporting = device.special_key_reporting(c.cid).await;
                let status = match &reporting {
                    Ok(r) => {
                        let mut parts = vec![];
                        if r.is_diverted() { parts.push("diverted"); }
                        if r.raw_xy_enabled() { parts.push("rawXY"); }
                        if r.persist_enabled() { parts.push("persist"); }
                        if r.remapped_cid != c.cid && r.remapped_cid != 0 {
                            parts.push("remapped");
                        }
                        if parts.is_empty() { "default".to_string() } else { parts.join("+") }
                    }
                    Err(_) => "?".to_string(),
                };
                println!(
                    "CID {:>3} (0x{:04X}) → TID {:>3}  {:<15} [{}]  caps: divert={} persist={}",
                    c.cid, c.cid, c.tid, name, status,
                    c.is_divertable(), c.is_persistently_divertable(),
                );
            }
        }
        other => {
            anyhow::bail!("Unknown setting: {other}\nAvailable: battery, dpi, smartshift, wheel, thumbwheel, host, firmware, buttons");
        }
    }

    Ok(())
}

async fn cmd_set(idx: Option<DeviceIndex>, setting: &str, value: &str) -> anyhow::Result<()> {
    let device = open_first_device(idx).await?;

    match setting {
        "dpi" => {
            let dpi: u16 = value.parse()?;
            let applied = device.dpi_set(dpi).await?;
            println!("DPI set to {applied}");
        }
        "smartshift" => {
            let mut state = device.smart_shift_get().await?;
            match value {
                "free" | "freespin" => state.mode = hidpp::features::smart_shift::WheelMode::FreeScroll,
                "ratchet" => state.mode = hidpp::features::smart_shift::WheelMode::Ratchet,
                v => {
                    state.auto_disengage = v.parse()?;
                }
            }
            let applied = device.smart_shift_set(&state).await?;
            println!(
                "SmartShift: {:?}, auto_disengage={}, torque={}",
                applied.mode, applied.auto_disengage, applied.tunable_torque,
            );
        }
        "button" => {
            // Format: "CID:action" e.g. "82:divert" or "195:remap:82" or "82:default"
            let parts: Vec<&str> = value.split(':').collect();
            let cid: u16 = parts.first().ok_or_else(|| anyhow::anyhow!("usage: button CID:action"))?.parse()?;
            let action = parts.get(1).copied().unwrap_or("default");

            match action {
                "divert" => {
                    let result = device.special_key_set_reporting(cid, 0x01, 0, 0).await?;
                    println!("CID {cid}: diverted={}", result.is_diverted());
                }
                "undivide" | "default" => {
                    let result = device.special_key_set_reporting(cid, 0x00, 0, 0).await?;
                    println!("CID {cid}: diverted={}", result.is_diverted());
                }
                "remap" => {
                    let target: u16 = parts.get(2).ok_or_else(|| anyhow::anyhow!("usage: button CID:remap:TARGET_CID"))?.parse()?;
                    let result = device.special_key_set_reporting(cid, 0x00, target, 0).await?;
                    println!("CID {cid}: remapped to CID {}", result.remapped_cid);
                }
                _ => anyhow::bail!("button actions: divert, default, remap:CID"),
            }
        }
        "wheel" => {
            let mut mode = device.hires_wheel_get_mode().await?;
            match value {
                "hires" => mode.high_resolution = true,
                "lowres" => mode.high_resolution = false,
                "invert" => mode.inverted = !mode.inverted,
                _ => anyhow::bail!("wheel values: hires, lowres, invert"),
            }
            let applied = device.hires_wheel_set_mode(&mode).await?;
            println!("Wheel: hires={} inverted={}", applied.high_resolution, applied.inverted);
        }
        other => {
            anyhow::bail!("Unknown setting: {other}\nAvailable: dpi, smartshift, wheel");
        }
    }

    Ok(())
}

async fn cmd_export(idx: Option<DeviceIndex>) -> anyhow::Result<()> {
    let device = open_first_device(idx).await?;
    let config = device.export_config().await?;
    let toml_str = config
        .to_toml()
        .map_err(|e| anyhow::anyhow!("TOML serialize error: {e}"))?;
    print!("{toml_str}");
    Ok(())
}

async fn cmd_import(idx: Option<DeviceIndex>, file: &str) -> anyhow::Result<()> {
    let toml_str = if file == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        std::fs::read_to_string(file)?
    };

    let config = hidpp_device::DeviceConfig::from_toml(&toml_str)
        .map_err(|e| anyhow::anyhow!("TOML parse error: {e}"))?;

    let device = open_first_device(idx).await?;
    device.import_config(&config).await?;
    println!("Config applied.");
    Ok(())
}

async fn cmd_raw(idx: Option<DeviceIndex>, feature_hex: &str, function: u8, params_hex: &str) -> anyhow::Result<()> {
    let device = open_first_device(idx).await?;

    let feature_id = u16::from_str_radix(feature_hex.trim_start_matches("0x"), 16)?;
    let feature = hidpp::types::FeatureId(feature_id);

    let params: Vec<u8> = if params_hex.is_empty() {
        vec![]
    } else {
        params_hex
            .split_whitespace()
            .map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16))
            .collect::<Result<Vec<_>, _>>()?
    };

    let resp = device.raw_request(feature, function, &params).await?;

    let name = hidpp::feature_id::feature_name(feature).unwrap_or("?");
    println!("Feature: {} ({name}), Function: {function}", feature);
    print!("Response:");
    for b in resp.params() {
        print!(" {b:02X}");
    }
    println!();

    Ok(())
}
