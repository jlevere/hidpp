use clap::{Parser, Subcommand};
use hidpp::types::DeviceIndex;
use hidpp_transport::native::HidapiEnumerator;

#[derive(Parser)]
#[command(name = "hidpp", about = "HID++ 2.0 device configuration tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Log verbosity (set RUST_LOG for fine control).
    #[arg(short, long, global = true, default_value = "warn")]
    log_level: String,
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

    match cli.command {
        Command::List => cmd_list()?,
        Command::Info => cmd_info().await?,
        Command::Get { setting } => cmd_get(&setting).await?,
        Command::Set { setting, value } => cmd_set(&setting, &value).await?,
        Command::Export => cmd_export().await?,
        Command::Import { file } => cmd_import(&file).await?,
        Command::Raw {
            feature,
            function,
            params,
        } => cmd_raw(&feature, function, &params).await?,
    }

    Ok(())
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

/// Open the first available HID++ device.
async fn open_first_device() -> anyhow::Result<hidpp_device::Device> {
    let enumerator = HidapiEnumerator::new()?;
    let devices = enumerator.enumerate();

    let info = devices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No HID++ devices found"))?;

    let transport = enumerator.open(info)?;
    let device = hidpp_device::Device::open(transport, DeviceIndex::BLE_DIRECT).await?;
    Ok(device)
}

async fn cmd_info() -> anyhow::Result<()> {
    let device = open_first_device().await?;

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

    if device.supports(hidpp::feature_id::CHANGE_HOST) {
        match device.host_info().await {
            Ok(h) => println!(
                "Easy-Switch: host {} of {}",
                h.current_host + 1,
                h.num_hosts,
            ),
            Err(e) => println!("Easy-Switch: error ({e})"),
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

async fn cmd_get(setting: &str) -> anyhow::Result<()> {
    let device = open_first_device().await?;

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
                println!(
                    "CID {:>3} (0x{:04X}) → TID {:>3}  {:<15} divert={} persist={} virtual={}",
                    c.cid, c.cid, c.tid, name,
                    c.is_divertable(), c.is_persistently_divertable(), c.is_virtual(),
                );
            }
        }
        other => {
            anyhow::bail!("Unknown setting: {other}\nAvailable: battery, dpi, smartshift, wheel, thumbwheel, host, firmware, buttons");
        }
    }

    Ok(())
}

async fn cmd_set(setting: &str, value: &str) -> anyhow::Result<()> {
    let device = open_first_device().await?;

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

async fn cmd_export() -> anyhow::Result<()> {
    let device = open_first_device().await?;
    let config = device.export_config().await?;
    let toml_str = config
        .to_toml()
        .map_err(|e| anyhow::anyhow!("TOML serialize error: {e}"))?;
    print!("{toml_str}");
    Ok(())
}

async fn cmd_import(file: &str) -> anyhow::Result<()> {
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

    let device = open_first_device().await?;
    device.import_config(&config).await?;
    println!("Config applied.");
    Ok(())
}

async fn cmd_raw(feature_hex: &str, function: u8, params_hex: &str) -> anyhow::Result<()> {
    let device = open_first_device().await?;

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
