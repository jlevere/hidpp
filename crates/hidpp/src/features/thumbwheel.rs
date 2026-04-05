/// HID++ Feature 0x2150 — Thumbwheel
///
/// Controls the horizontal scroll thumbwheel reporting mode and inversion.
///
/// Function IDs (confirmed from decompilation):
/// - 0: GetThumbwheelInfo
/// - 1: GetThumbwheelStatus
/// - 2: SetThumbwheelReporting
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Thumbwheel reporting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ReportingMode {
    /// Native HID scrolling (OS handles it).
    Native = 0,
    /// Diverted to software (agent handles it).
    Diverted = 1,
}

impl ReportingMode {
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::Diverted,
            _ => Self::Native,
        }
    }
}

/// Thumbwheel hardware information.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThumbwheelInfo {
    /// Counts per revolution in native mode.
    pub native_resolution: u16,
    /// Counts per revolution in diverted mode.
    pub diverted_resolution: u16,
    /// Wheel orientation direction.
    pub direction: u8,
    /// Has auto-disengage feature.
    pub has_auto_disengage: bool,
    /// Has proximity sensor.
    pub has_proxy: bool,
    /// Has touch proximity sensor.
    pub has_touch_proxy: bool,
    /// Supports timestamps on events.
    pub has_timestamp: bool,
    /// Timestamp unit in microseconds.
    pub timestamp_unit: u16,
}

/// Current thumbwheel status.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThumbwheelStatus {
    /// Current reporting mode.
    pub reporting_mode: ReportingMode,
    /// Whether events are diverted to software.
    pub diverted: bool,
    /// Whether scroll direction is inverted.
    pub inverted: bool,
    /// Whether proximity sensor is active.
    pub proxy: bool,
}

/// Function 0: GetThumbwheelInfo
pub fn encode_get_info(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

/// Decode GetThumbwheelInfo response.
///
/// Byte layout (confirmed from decompilation):
/// ```text
/// param[0:2]: nativeResolution (uint16 big-endian)
/// param[2:4]: divertedResolution (uint16 big-endian)
/// param[4]:   direction
/// param[5]:   capability flags (bit0=autoDisengage, bit1=proxy, bit2=touchProxy, bit3=timestamp)
/// param[6:8]: timestampUnit (uint16 big-endian)
/// ```
pub fn decode_get_info(report: &LongReport) -> Result<ThumbwheelInfo, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(ThumbwheelInfo {
        native_resolution: u16::from_be_bytes([p[0], p[1]]),
        diverted_resolution: u16::from_be_bytes([p[2], p[3]]),
        direction: p[4],
        has_auto_disengage: (p[5] & 0x01) != 0,
        has_proxy: (p[5] & 0x02) != 0,
        has_touch_proxy: (p[5] & 0x04) != 0,
        has_timestamp: (p[5] & 0x08) != 0,
        timestamp_unit: u16::from_be_bytes([p[6], p[7]]),
    })
}

/// Function 1: GetThumbwheelStatus
pub fn encode_get_status(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[])
}

/// Decode GetThumbwheelStatus response.
///
/// Byte layout:
/// ```text
/// param[0]: reportingMode (0=native, 1=diverted)
/// param[1]: flags (bit0=diverted, bit1=inverted, bit2=proxy)
/// ```
pub fn decode_get_status(report: &LongReport) -> Result<ThumbwheelStatus, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(ThumbwheelStatus {
        reporting_mode: ReportingMode::from_byte(p[0]),
        diverted: (p[1] & 0x01) != 0,
        inverted: (p[1] & 0x02) != 0,
        proxy: (p[1] & 0x04) != 0,
    })
}

/// Function 2: SetThumbwheelReporting
pub fn encode_set_reporting(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    reporting_mode: ReportingMode,
    invert: bool,
) -> LongReport {
    LongReport::request(
        device,
        feature_index,
        FunctionId(2),
        sw_id,
        &[reporting_mode as u8, invert as u8],
    )
}

/// Decode SetThumbwheelReporting response.
pub fn decode_set_reporting(report: &LongReport) -> Result<ThumbwheelStatus, DecodeError> {
    decode_get_status(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_thumbwheel_info() {
        let mut report = LongReport::new();
        // native res = 120 (0x0078)
        report.as_bytes_mut()[4] = 0x00;
        report.as_bytes_mut()[5] = 0x78;
        // diverted res = 360 (0x0168)
        report.as_bytes_mut()[6] = 0x01;
        report.as_bytes_mut()[7] = 0x68;
        report.as_bytes_mut()[8] = 0x01; // direction
        report.as_bytes_mut()[9] = 0x03; // has_auto_disengage + has_proxy
        // timestamp unit = 1000 (0x03E8)
        report.as_bytes_mut()[10] = 0x03;
        report.as_bytes_mut()[11] = 0xE8;

        let info = decode_get_info(&report).unwrap();
        assert_eq!(info.native_resolution, 120);
        assert_eq!(info.diverted_resolution, 360);
        assert_eq!(info.direction, 1);
        assert!(info.has_auto_disengage);
        assert!(info.has_proxy);
        assert!(!info.has_touch_proxy);
        assert_eq!(info.timestamp_unit, 1000);
    }

    #[test]
    fn encode_set_native_mode() {
        let report = encode_set_reporting(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x06),
            SoftwareId::DEFAULT,
            ReportingMode::Native,
            false,
        );
        assert_eq!(report.params()[0], 0); // native
        assert_eq!(report.params()[1], 0); // not inverted
    }
}
