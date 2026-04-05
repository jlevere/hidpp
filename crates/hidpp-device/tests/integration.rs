#![allow(clippy::unwrap_used, clippy::expect_used)]
/// Integration tests that talk to real hardware.
///
/// These require a Logitech HID++ device connected (BLE or via receiver).
/// Run with: `cargo test -p hidpp-device --test integration`
///
/// Skips gracefully if no device is found.
use hidpp::feature_id;
use hidpp::features::smart_shift;
use hidpp_device::Device;
use hidpp_transport::native::HidapiEnumerator;

async fn open_device() -> Option<Device> {
    let enumerator = HidapiEnumerator::new().ok()?;
    let devices = enumerator.enumerate();
    let info = devices.first()?;
    let transport = enumerator.open(info).ok()?;
    let index = Device::probe_device_index(&transport).await.ok()?;
    Device::open(transport, index).await.ok()
}

macro_rules! require_device {
    () => {
        match open_device().await {
            Some(d) => d,
            None => {
                eprintln!("SKIP: no HID++ device connected");
                return;
            }
        }
    };
}

#[tokio::test]
async fn device_discovery() {
    let device = require_device!();

    assert!(!device.name().is_empty());
    assert!(device.features().count() > 10);

    let (major, minor) = device.protocol_version();
    assert!(major >= 2, "Expected HID++ 2.0+, got {major}.{minor}");

    println!("Device: {}", device.name());
    println!("Protocol: {major}.{minor}");
    println!("Features: {}", device.features().count());

    // Root and FeatureSet must always be present.
    assert!(device.supports(feature_id::ROOT));
    assert!(device.supports(feature_id::FEATURE_SET));
}

#[tokio::test]
async fn feature_enumeration() {
    let device = require_device!();

    // Print all features for debugging.
    for feature in device.features() {
        let name = hidpp::feature_id::feature_name(feature.id).unwrap_or("?");
        println!(
            "  [{:02X}] {} ({}) v{}",
            feature.index.0, feature.id, name, feature.version,
        );
    }
}

#[tokio::test]
async fn battery_read() {
    let device = require_device!();
    if !device.supports(feature_id::UNIFIED_BATTERY) {
        eprintln!("SKIP: device doesn't support UnifiedBattery");
        return;
    }

    let status = device.battery_status().await.expect("battery read failed");

    assert!(status.percentage <= 100);
    println!(
        "Battery: {}% {:?} {:?}",
        status.percentage, status.level, status.charging,
    );

    // Sanity: battery should be a reasonable value.
    // A fully dead battery would still report 0, not 255.
    assert!(status.percentage <= 100);
}

#[tokio::test]
async fn dpi_read() {
    let device = require_device!();
    if !device.supports(feature_id::ADJUSTABLE_DPI) {
        eprintln!("SKIP: device doesn't support AdjustableDPI");
        return;
    }

    let dpi = device.dpi_get().await.expect("DPI read failed");

    assert!(dpi >= 200 && dpi <= 8000, "DPI out of range: {dpi}");
    println!("DPI: {dpi}");
}

#[tokio::test]
async fn dpi_write_and_restore() {
    let device = require_device!();
    if !device.supports(feature_id::ADJUSTABLE_DPI) {
        eprintln!("SKIP: device doesn't support AdjustableDPI");
        return;
    }

    let original = device.dpi_get().await.expect("DPI read failed");
    println!("Original DPI: {original}");

    // Set to 1600.
    let applied = device.dpi_set(1600).await.expect("DPI set failed");
    assert_eq!(applied, 1600, "DPI set returned wrong value");

    // Read back.
    let readback = device.dpi_get().await.expect("DPI readback failed");
    assert_eq!(readback, 1600, "DPI readback mismatch");

    // Restore.
    let restored = device.dpi_set(original).await.expect("DPI restore failed");
    assert_eq!(restored, original, "DPI restore mismatch");
    println!("DPI: {original} → 1600 → {restored}");
}

#[tokio::test]
async fn smartshift_read() {
    let device = require_device!();
    if !device.has_smart_shift() {
        eprintln!("SKIP: device doesn't support SmartShift");
        return;
    }

    let state = device.smart_shift_get().await.expect("SmartShift read failed");

    println!(
        "SmartShift: {:?} auto_disengage={} torque={}",
        state.mode, state.auto_disengage, state.tunable_torque,
    );

    // Mode should be a valid value.
    match state.mode {
        smart_shift::WheelMode::Ratchet | smart_shift::WheelMode::FreeScroll => {}
    }
}

#[tokio::test]
async fn hires_wheel_read() {
    let device = require_device!();
    if !device.supports(feature_id::HIRES_WHEEL) {
        eprintln!("SKIP: device doesn't support HiResWheel");
        return;
    }

    let mode = device
        .hires_wheel_get_mode()
        .await
        .expect("HiResWheel read failed");

    println!(
        "HiResWheel: hires={} inverted={} diverted={}",
        mode.high_resolution, mode.inverted, mode.diverted,
    );
}

#[tokio::test]
async fn thumbwheel_read() {
    let device = require_device!();
    if !device.supports(feature_id::THUMBWHEEL) {
        eprintln!("SKIP: device doesn't support Thumbwheel");
        return;
    }

    let status = device
        .thumbwheel_get_status()
        .await
        .expect("Thumbwheel read failed");

    println!(
        "Thumbwheel: {:?} inverted={} diverted={}",
        status.reporting_mode, status.inverted, status.diverted,
    );
}

#[tokio::test]
async fn host_info_read() {
    let device = require_device!();
    if !device.supports(feature_id::CHANGE_HOST) {
        eprintln!("SKIP: device doesn't support ChangeHost");
        return;
    }

    let info = device.host_info().await.expect("HostInfo read failed");

    assert!(info.num_hosts > 0, "num_hosts should be > 0");
    assert!(
        info.current_host < info.num_hosts,
        "current_host {} >= num_hosts {}",
        info.current_host,
        info.num_hosts,
    );
    println!("Easy-Switch: host {} of {}", info.current_host + 1, info.num_hosts);
}

#[tokio::test]
async fn config_export() {
    let device = require_device!();

    let config = device.export_config().await.expect("config export failed");
    let toml = config.to_toml().expect("TOML serialize failed");

    println!("--- Exported Config ---");
    println!("{toml}");

    // Should contain device name.
    assert!(toml.contains("MX Master") || toml.contains(&device.name()));

    // Round-trip: parse the TOML back.
    let parsed =
        hidpp_device::DeviceConfig::from_toml(&toml).expect("TOML parse failed");
    assert_eq!(parsed.device.name, config.device.name);
}

#[tokio::test]
async fn raw_request_feature_set() {
    let device = require_device!();

    // Raw request: FeatureSet GetCount (feature 0x0001, function 0).
    let resp = device
        .raw_request(feature_id::FEATURE_SET, 0, &[])
        .await
        .expect("raw request failed");

    let count = resp.params()[0];
    println!("Raw FeatureSet GetCount: {count} features");
    assert!(count > 10);
}
