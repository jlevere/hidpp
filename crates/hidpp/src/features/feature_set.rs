/// HID++ Feature 0x0001 — FeatureSet
///
/// Enumerates all features supported by the device.
/// Used during device discovery to build the feature map.
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureFlags, FeatureId, FeatureIndex, FunctionId, SoftwareId};

/// Function 0: GetCount — how many features does this device support?
pub fn encode_get_count(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

/// Decode GetCount response. Returns the total number of features.
pub fn decode_get_count(report: &LongReport) -> Result<u8, DecodeError> {
    report.check_error()?;
    Ok(report.params()[0])
}

/// Function 1: GetFeatureID — get the feature ID at a given table index.
///
/// The table index is 0-based and goes up to (count - 1).
/// Returns the feature ID, flags, and version.
pub fn encode_get_feature_id(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    table_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[table_index])
}

/// Decoded feature info from FeatureSet::GetFeatureID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureInfo {
    pub feature_id: FeatureId,
    pub flags: FeatureFlags,
    pub version: u8,
}

/// Decode GetFeatureID response.
///
/// Response format (from RE):
/// ```text
/// Feature0001FeatureSet::GetFeatureID(
///   featureIndex = %d,
///   *featureID = 0x%08x,
///   *engineeringHidden = %d,
///   *swHidden = %d,
///   *obsolete = %d,
///   *profile = %d,
///   *version = %d
/// )
/// ```
pub fn decode_get_feature_id(report: &LongReport) -> Result<FeatureInfo, DecodeError> {
    report.check_error()?;
    let params = report.params();
    let feature_id = FeatureId(u16::from_be_bytes([params[0], params[1]]));
    let flags = FeatureFlags::from_bits_truncate(params[2]);
    let version = params[3];
    Ok(FeatureInfo {
        feature_id,
        flags,
        version,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn encode_get_count_format() {
        let report = encode_get_count(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x01),
            SoftwareId::DEFAULT,
        );
        assert_eq!(report.feature_index(), FeatureIndex(0x01));
        assert_eq!(report.function_id(), FunctionId(0));
    }

    #[test]
    fn decode_feature_id_response() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[2] = 0x01; // feature index for FeatureSet
        report.as_bytes_mut()[3] = 0x11; // function 1, sw_id 1
        // Feature ID 0x2110 (SmartShift) in big-endian
        report.as_bytes_mut()[4] = 0x21;
        report.as_bytes_mut()[5] = 0x10;
        report.as_bytes_mut()[6] = 0x00; // no flags
        report.as_bytes_mut()[7] = 0x01; // version 1

        let info = decode_get_feature_id(&report).unwrap();
        assert_eq!(info.feature_id, FeatureId(0x2110));
        assert!(info.flags.is_empty());
        assert_eq!(info.version, 1);
    }

    #[test]
    fn decode_hidden_feature() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 0x18;
        report.as_bytes_mut()[5] = 0x00;
        report.as_bytes_mut()[6] = 0x80; // engineering hidden

        let info = decode_get_feature_id(&report).unwrap();
        assert_eq!(info.feature_id, FeatureId(0x1800));
        assert!(info.flags.contains(FeatureFlags::ENGINEERING_HIDDEN));
    }
}
