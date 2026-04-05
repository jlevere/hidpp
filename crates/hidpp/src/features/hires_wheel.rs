/// HID++ Feature 0x2121 — HiResWheel
///
/// Controls high-resolution scroll wheel behavior including resolution,
/// ratchet/free-spin mode diversion, inversion, and analytics.
///
/// Function IDs (confirmed from decompilation):
/// - 0: GetWheelCapability
/// - 1: GetWheelMode
/// - 2: SetWheelMode
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Wheel capabilities reported by the device.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WheelCapabilities {
    /// Scroll event multiplier for hi-res mode.
    pub multiplier: u8,
    /// Device has priority control.
    pub has_priority: bool,
    /// Device has report rate control.
    pub has_rate: bool,
    /// Device has ratchet switch (hardware mode toggle).
    pub has_ratchet_switch: bool,
    /// Device supports scroll direction inversion.
    pub has_inversion: bool,
    /// Device supports analytics data reporting.
    pub has_analytics_data: bool,
    /// Number of ratchet detents per full wheel rotation.
    pub ratchets_per_rotation: u8,
    /// Wheel diameter (implementation-specific units).
    pub wheel_diameter: u8,
}

/// Current wheel mode configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WheelMode {
    /// Scroll events diverted to software (true) or native HID (false).
    pub diverted: bool,
    /// High-resolution scrolling enabled.
    pub high_resolution: bool,
    /// Scroll direction inverted.
    pub inverted: bool,
    /// Analytics data reporting enabled.
    pub analytics: bool,
    /// Scroll priority level.
    pub priority: u8,
    /// Target report rate.
    pub rate: u8,
}

/// Function 0: GetWheelCapability
pub fn encode_get_capabilities(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

/// Decode GetWheelCapability response.
///
/// Byte layout (confirmed from decompilation):
/// ```text
/// param[0]: multiplier
/// param[1]: capability flags (bit0=priority, bit1=rate, bit2=ratchet_switch,
///           bit3=inversion, bit4=analytics)
/// param[2]: ratchets_per_rotation
/// param[3]: wheel_diameter
/// ```
pub fn decode_get_capabilities(report: &LongReport) -> Result<WheelCapabilities, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(WheelCapabilities {
        multiplier: p[0],
        has_priority: (p[1] & 0x01) != 0,
        has_rate: (p[1] & 0x02) != 0,
        has_ratchet_switch: (p[1] & 0x04) != 0,
        has_inversion: (p[1] & 0x08) != 0,
        has_analytics_data: (p[1] & 0x10) != 0,
        ratchets_per_rotation: p[2],
        wheel_diameter: p[3],
    })
}

/// Function 1: GetWheelMode
pub fn encode_get_mode(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[])
}

/// Decode GetWheelMode response.
///
/// Byte layout (confirmed from decompilation):
/// ```text
/// param[0]: flags (bit0=diverted, bit1=high_resolution, bit2=inverted, bit3=analytics)
/// param[1]: priority
/// param[2]: rate
/// ```
pub fn decode_get_mode(report: &LongReport) -> Result<WheelMode, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(WheelMode {
        diverted: (p[0] & 0x01) != 0,
        high_resolution: (p[0] & 0x02) != 0,
        inverted: (p[0] & 0x04) != 0,
        analytics: (p[0] & 0x08) != 0,
        priority: p[1],
        rate: p[2],
    })
}

/// Function 2: SetWheelMode
pub fn encode_set_mode(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    mode: &WheelMode,
) -> LongReport {
    let flags = (mode.diverted as u8)
        | ((mode.high_resolution as u8) << 1)
        | ((mode.inverted as u8) << 2)
        | ((mode.analytics as u8) << 3);

    LongReport::request(
        device,
        feature_index,
        FunctionId(2),
        sw_id,
        &[flags, mode.priority, mode.rate],
    )
}

/// Decode SetWheelMode response (echoes back applied values).
pub fn decode_set_mode(report: &LongReport) -> Result<WheelMode, DecodeError> {
    decode_get_mode(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_capabilities() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 8;     // multiplier
        report.as_bytes_mut()[5] = 0x0F;  // all flags except analytics
        report.as_bytes_mut()[6] = 24;    // ratchets per rotation
        report.as_bytes_mut()[7] = 30;    // diameter

        let caps = decode_get_capabilities(&report).unwrap();
        assert_eq!(caps.multiplier, 8);
        assert!(caps.has_priority);
        assert!(caps.has_rate);
        assert!(caps.has_ratchet_switch);
        assert!(caps.has_inversion);
        assert!(!caps.has_analytics_data);
        assert_eq!(caps.ratchets_per_rotation, 24);
    }

    #[test]
    fn encode_set_mode_hires() {
        let mode = WheelMode {
            diverted: false,
            high_resolution: true,
            inverted: false,
            analytics: false,
            priority: 0,
            rate: 0,
        };
        let report = encode_set_mode(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x04),
            SoftwareId::DEFAULT,
            &mode,
        );
        assert_eq!(report.params()[0], 0x02); // bit1 = high_resolution
    }

    #[test]
    fn roundtrip_mode() {
        let mode = WheelMode {
            diverted: true,
            high_resolution: true,
            inverted: true,
            analytics: false,
            priority: 5,
            rate: 8,
        };
        let report = encode_set_mode(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x04),
            SoftwareId::DEFAULT,
            &mode,
        );
        let decoded = decode_get_mode(&report).unwrap();
        assert_eq!(decoded, mode);
    }
}
