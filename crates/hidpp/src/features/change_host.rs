/// HID++ Feature 0x1814 — ChangeHost (Easy-Switch)
///
/// Manages multi-host connectivity. The MX Master 3S supports 3 host slots.
///
/// Function IDs:
/// - 0: GetHostInfo
/// - 1: SetCurrentHost
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Host connection info.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HostInfo {
    /// Total number of host slots.
    pub num_hosts: u8,
    /// Currently active host index (0-based).
    pub current_host: u8,
}

/// Function 0: GetHostInfo
pub fn encode_get_host_info(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_host_info(report: &LongReport) -> Result<HostInfo, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(HostInfo {
        num_hosts: p[0],
        current_host: p[1],
    })
}

/// Function 1: SetCurrentHost — switch to a different host slot.
///
/// The device will disconnect from the current host and connect to
/// the specified host slot. Valid range: 0 to (num_hosts - 1).
pub fn encode_set_current_host(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    host_index: u8,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[host_index])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_host_info() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 3; // 3 hosts
        report.as_bytes_mut()[5] = 1; // currently on host 1

        let info = decode_get_host_info(&report).unwrap();
        assert_eq!(info.num_hosts, 3);
        assert_eq!(info.current_host, 1);
    }

    #[test]
    fn encode_switch_host() {
        let report = encode_set_current_host(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x09),
            SoftwareId::DEFAULT,
            2,
        );
        assert_eq!(report.function_id(), FunctionId(1));
        assert_eq!(report.params()[0], 2);
    }
}
