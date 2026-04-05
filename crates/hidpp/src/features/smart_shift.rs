/// HID++ Feature 0x2111 — SmartShift Enhanced (with Tunable Torque)
///
/// Controls the scroll wheel ratchet/free-spin behavior.
/// The MX Master 3S uses the electromagnetic SmartShift mechanism
/// that automatically switches between ratchet and free-spin based
/// on scroll speed.
///
/// The MX Master 3S uses **0x2111** (enhanced with tunable torque),
/// not the legacy 0x2110. Confirmed by decompilation.
///
/// Function IDs (from decompiled `devio::Feature2111SmartShiftWithTunableTorque`):
/// - 0: GetCapabilities
/// - 1: GetRatchetControlMode
/// - 2: SetRatchetControlMode
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Scroll wheel mode.
///
/// Values confirmed by decompilation of `Feature2111SmartShiftWithTunableTorque`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WheelMode {
    /// Free-spin (smooth/fast) scrolling.
    FreeScroll = 1,
    /// Ratchet (click-click) scrolling.
    Ratchet = 2,
}

impl WheelMode {
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::FreeScroll,
            _ => Self::Ratchet,
        }
    }
}

/// SmartShift capabilities reported by the device.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SmartShiftCapabilities {
    /// Whether the device supports tunable torque (feature 0x2111).
    pub has_tunable_torque: bool,
    /// Default auto-disengage threshold.
    pub auto_disengage_default: u8,
    /// Default tunable torque value.
    pub default_tunable_torque: u8,
    /// Maximum force value.
    pub max_force: u8,
}

/// Current SmartShift state.
///
/// For feature 0x2111 (Enhanced), includes tunable torque.
/// For feature 0x2110 (Legacy), `tunable_torque` is always 0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SmartShiftState {
    /// Current wheel mode.
    pub mode: WheelMode,
    /// Auto-disengage threshold (0 = disabled, 1-255 = speed threshold).
    /// Higher values = requires faster scroll to switch to free-spin.
    pub auto_disengage: u8,
    /// Tunable torque (0x2111 only): resistance level for ratchet clicks.
    /// Range: 0 to `max_force` (from GetCapabilities).
    pub tunable_torque: u8,
}

/// Function 0: GetCapabilities
pub fn encode_get_capabilities(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

/// Decode GetCapabilities response.
///
/// From RE debug string:
/// ```text
/// read_capabilities: has_tunable_torque: %d, auto_disangage_default: %d,
///                    default_tunable_torque: %d, max_force: %d
/// ```
pub fn decode_get_capabilities(
    report: &LongReport,
) -> Result<SmartShiftCapabilities, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(SmartShiftCapabilities {
        has_tunable_torque: (p[0] & 0x01) != 0,
        auto_disengage_default: p[1],
        default_tunable_torque: p[2],
        max_force: p[3],
    })
}

// ---- Feature 0x2110 (legacy): Fn0=GET, Fn1=SET, 2 bytes ----

/// 0x2110 Function 0: GetRatchetControlMode (legacy — no tunable torque).
pub fn encode_get_mode_v0(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

/// 0x2110 Function 1: SetRatchetControlMode (legacy — 2 bytes).
pub fn encode_set_mode_v0(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    state: &SmartShiftState,
) -> LongReport {
    LongReport::request(
        device,
        feature_index,
        FunctionId(1),
        sw_id,
        &[state.mode as u8, state.auto_disengage],
    )
}

// ---- Feature 0x2111 (enhanced): Fn0=CAPS, Fn1=GET, Fn2=SET, 3 bytes ----

/// 0x2111 Function 1: GetRatchetControlMode (enhanced — with tunable torque).
pub fn encode_get_mode_v1(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[])
}

/// 0x2111 Function 2: SetRatchetControlMode (enhanced — 3 bytes).
pub fn encode_set_mode_v1(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    state: &SmartShiftState,
) -> LongReport {
    LongReport::request(
        device,
        feature_index,
        FunctionId(2),
        sw_id,
        &[state.mode as u8, state.auto_disengage, state.tunable_torque],
    )
}

// ---- Shared decode (same response format for both versions) ----

/// Decode GetRatchetControlMode response (works for both 0x2110 and 0x2111).
///
/// - param[0]: wheelMode (1=freespin, 2=ratchet)
/// - param[1]: autoDisengage (0=off, 1-255=threshold)
/// - param[2]: currentTunableTorque (0x2111 only; 0 for 0x2110)
pub fn decode_get_mode(report: &LongReport) -> Result<SmartShiftState, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(SmartShiftState {
        mode: WheelMode::from_byte(p[0]),
        auto_disengage: p[1],
        tunable_torque: p[2],
    })
}

/// Decode SetRatchetControlMode response (echoes back the applied state).
pub fn decode_set_mode(report: &LongReport) -> Result<SmartShiftState, DecodeError> {
    decode_get_mode(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn encode_set_mode_v0_ratchet() {
        let state = SmartShiftState {
            mode: WheelMode::Ratchet,
            auto_disengage: 50,
            tunable_torque: 0,
        };
        let report = encode_set_mode_v0(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x07),
            SoftwareId::DEFAULT,
            &state,
        );

        assert_eq!(report.function_id(), FunctionId(1)); // 0x2110: SET = Fn1
        assert_eq!(report.params()[0], 2); // Ratchet = 2
        assert_eq!(report.params()[1], 50); // auto_disengage
    }

    #[test]
    fn encode_set_mode_v1_with_torque() {
        let state = SmartShiftState {
            mode: WheelMode::FreeScroll,
            auto_disengage: 30,
            tunable_torque: 20,
        };
        let report = encode_set_mode_v1(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x07),
            SoftwareId::DEFAULT,
            &state,
        );

        assert_eq!(report.function_id(), FunctionId(2)); // 0x2111: SET = Fn2
        assert_eq!(report.params()[0], 1); // FreeScroll = 1
        assert_eq!(report.params()[1], 30);
        assert_eq!(report.params()[2], 20); // torque
    }

    #[test]
    fn encode_get_mode_v0_uses_fn0() {
        let report = encode_get_mode_v0(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x0E),
            SoftwareId::DEFAULT,
        );
        assert_eq!(report.function_id(), FunctionId(0)); // 0x2110: GET = Fn0
    }

    #[test]
    fn encode_get_mode_v1_uses_fn1() {
        let report = encode_get_mode_v1(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x0E),
            SoftwareId::DEFAULT,
        );
        assert_eq!(report.function_id(), FunctionId(1)); // 0x2111: GET = Fn1
    }

    #[test]
    fn decode_mode_response() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 1;  // FreeScroll = 1
        report.as_bytes_mut()[5] = 10; // auto_disengage
        report.as_bytes_mut()[6] = 15; // tunable_torque (only meaningful for 0x2111)

        let state = decode_get_mode(&report).unwrap();
        assert_eq!(state.mode, WheelMode::FreeScroll);
        assert_eq!(state.auto_disengage, 10);
        assert_eq!(state.tunable_torque, 15);
    }
}
