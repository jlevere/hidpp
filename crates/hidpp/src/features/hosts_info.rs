/// HID++ Feature 0x1815 — HostsInfos
///
/// Query information about paired hosts on multi-host devices.
/// Shows host name, OS type, and connection status per Easy-Switch slot.
///
/// Function IDs (from decompilation):
/// - 0: GetFeatureInfos → (numHosts, currentHost, capabilities)
/// - 1: GetHostInfos(hostIndex) → (status, busType, pageCount)
/// - 2: GetHostFriendlyName(hostIndex, page) → name chunk
/// - 3: GetHostOSVersion(hostIndex) → (osType, version)
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Host OS type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HostOS {
    Unknown,
    Windows,
    WinEmb,
    Linux,
    Chrome,
    Android,
    MacOS,
    IOS,
    Other(u8),
}

impl HostOS {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Unknown,
            1 => Self::Windows,
            2 => Self::WinEmb,
            3 => Self::Linux,
            4 => Self::Chrome,
            5 => Self::Android,
            6 => Self::MacOS,
            7 => Self::IOS,
            other => Self::Other(other),
        }
    }
}

/// Per-host information.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HostDetails {
    pub status: u8,
    pub bus_type: u8,
    pub name_page_count: u8,
}

/// Host OS version info.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HostOSVersion {
    pub os_type: HostOS,
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
}

/// Function 0: GetFeatureInfos
pub fn encode_get_feature_infos(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_feature_infos(report: &LongReport) -> Result<(u8, u8, u8), DecodeError> {
    report.check_error()?;
    let p = report.params();
    // (numHosts, currentHost, capabilities)
    Ok((p[0], p[1], p[2]))
}

/// Function 1: GetHostInfos
pub fn encode_get_host_infos(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    host_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[host_index])
}

pub fn decode_get_host_infos(report: &LongReport) -> Result<HostDetails, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(HostDetails {
        status: p[0],
        bus_type: p[1],
        name_page_count: p[2],
    })
}

/// Function 2: GetHostFriendlyName(hostIndex, page) → name chunk.
pub fn encode_get_host_name(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    host_index: u8,
    page: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(
        device,
        feature_index,
        FunctionId(2),
        sw_id,
        &[host_index, page],
    )
}

/// Decode host name chunk. Returns (nameLen, chunk bytes).
pub fn decode_get_host_name(report: &LongReport) -> Result<(u8, &[u8]), DecodeError> {
    report.check_error()?;
    let p = report.params();
    let name_len = p[0];
    Ok((name_len, &p[1..]))
}

/// Function 3: GetHostOSVersion
pub fn encode_get_host_os_version(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    host_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(3), sw_id, &[host_index])
}

pub fn decode_get_host_os_version(report: &LongReport) -> Result<HostOSVersion, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(HostOSVersion {
        os_type: HostOS::from_byte(p[0]),
        version_major: p[1],
        version_minor: p[2],
        version_patch: p[3],
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_host_os_macos() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 6;  // macOS
        report.as_bytes_mut()[5] = 14; // Sonoma major
        report.as_bytes_mut()[6] = 5;  // minor
        report.as_bytes_mut()[7] = 0;  // patch

        let os = decode_get_host_os_version(&report).unwrap();
        assert_eq!(os.os_type, HostOS::MacOS);
        assert_eq!(os.version_major, 14);
    }
}
