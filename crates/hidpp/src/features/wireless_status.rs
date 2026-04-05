/// HID++ Feature 0x1D4B — WirelessDeviceStatus
///
/// Monitor wireless connection quality and status changes.
/// Primarily event-driven — the device sends notifications when
/// connection status changes.
///
/// Function IDs:
/// - 0: GetStatus
/// Event: StatusBroadcast
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Wireless connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConnectionStatus {
    /// Device is connected and communicating.
    Connected,
    /// Device is disconnected or unreachable.
    Disconnected,
    /// Unknown status value.
    Unknown(u8),
}

impl ConnectionStatus {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Disconnected,
            1 => Self::Connected,
            other => Self::Unknown(other),
        }
    }
}

/// Wireless status report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WirelessStatus {
    pub status: ConnectionStatus,
    /// Additional status byte (device-specific meaning).
    pub extra: u8,
}

/// Function 0: GetStatus
pub fn encode_get_status(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_status(report: &LongReport) -> Result<WirelessStatus, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(WirelessStatus {
        status: ConnectionStatus::from_byte(p[0]),
        extra: p[1],
    })
}
