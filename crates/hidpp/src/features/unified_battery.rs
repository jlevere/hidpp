/// HID++ Feature 0x1004 — UnifiedBattery
///
/// Provides battery level, charging status, and power source info.
/// This is the modern battery feature used by recent Logitech devices
/// including the MX Master 3S.
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Battery level categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BatteryLevel {
    Critical,
    Low,
    Good,
    Full,
}

/// Charging status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChargingStatus {
    Discharging,
    Charging,
    ChargingComplete,
    ChargingError,
}

/// Battery capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BatteryCapabilities {
    /// Supported battery levels (bitmask).
    pub supported_levels: u8,
    /// Whether the device reports exact percentage.
    pub supports_percentage: bool,
    /// Whether the device supports rechargeable battery.
    pub rechargeable: bool,
}

/// Current battery status.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BatteryStatus {
    /// State of charge percentage (0–100). May be 0 if not supported.
    pub percentage: u8,
    /// Discrete battery level.
    pub level: BatteryLevel,
    /// Current charging status.
    pub charging: ChargingStatus,
    /// Whether external power is connected.
    pub external_power: bool,
}

/// Function 0: GetCapabilities
pub fn encode_get_capabilities(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_capabilities(report: &LongReport) -> Result<BatteryCapabilities, DecodeError> {
    report.check_error()?;
    let params = report.params();
    Ok(BatteryCapabilities {
        supported_levels: params[0],
        supports_percentage: (params[1] & 0x01) != 0,
        rechargeable: (params[1] & 0x02) != 0,
    })
}

/// Function 1: GetStatus
pub fn encode_get_status(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[])
}

pub fn decode_get_status(report: &LongReport) -> Result<BatteryStatus, DecodeError> {
    report.check_error()?;
    let params = report.params();

    let percentage = params[0];

    let level = match params[1] {
        1 => BatteryLevel::Critical,
        2 => BatteryLevel::Low,
        4 => BatteryLevel::Good,
        8 => BatteryLevel::Full,
        _ => BatteryLevel::Good, // default if unknown
    };

    let charging = match params[2] {
        0 => ChargingStatus::Discharging,
        1 => ChargingStatus::Charging,
        2 => ChargingStatus::ChargingComplete,
        _ => ChargingStatus::ChargingError,
    };

    let external_power = (params[3] & 0x01) != 0;

    Ok(BatteryStatus {
        percentage,
        level,
        charging,
        external_power,
    })
}

/// Event 0: StatusChanged — notification sent when battery status changes.
///
/// Uses the same format as GetStatus response.
pub fn decode_status_event(report: &LongReport) -> Result<BatteryStatus, DecodeError> {
    decode_get_status(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_battery_status() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 75; // 75% battery
        report.as_bytes_mut()[5] = 4; // Good level
        report.as_bytes_mut()[6] = 0; // Discharging
        report.as_bytes_mut()[7] = 0; // No external power

        let status = decode_get_status(&report).unwrap();
        assert_eq!(status.percentage, 75);
        assert_eq!(status.level, BatteryLevel::Good);
        assert_eq!(status.charging, ChargingStatus::Discharging);
        assert!(!status.external_power);
    }

    #[test]
    fn decode_battery_charging() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 50;
        report.as_bytes_mut()[5] = 2; // Low
        report.as_bytes_mut()[6] = 1; // Charging
        report.as_bytes_mut()[7] = 1; // External power

        let status = decode_get_status(&report).unwrap();
        assert_eq!(status.percentage, 50);
        assert_eq!(status.level, BatteryLevel::Low);
        assert_eq!(status.charging, ChargingStatus::Charging);
        assert!(status.external_power);
    }
}
