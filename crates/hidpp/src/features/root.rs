/// HID++ Feature 0x0000 — Root
///
/// Present on every HID++ 2.0 device at feature index 0x00.
/// Used to:
/// - Ping the device (verify it's alive and speaking HID++ 2.0)
/// - Resolve a feature ID to its runtime feature index
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureFlags, FeatureId, FeatureIndex, FunctionId, SoftwareId};

/// Function 0: GetFeature — resolve a feature ID to its runtime index.
pub fn encode_get_feature(
    device: DeviceIndex,
    feature_id: FeatureId,
    sw_id: SoftwareId,
) -> LongReport {
    let params = feature_id.0.to_be_bytes();
    LongReport::request(
        device,
        FeatureIndex::ROOT,
        FunctionId(0),
        sw_id,
        &params,
    )
}

/// Decode the response to GetFeature.
///
/// Returns `(feature_index, feature_type)`. A feature index of 0 means
/// the feature is not supported by this device.
pub fn decode_get_feature(report: &LongReport) -> Result<(FeatureIndex, FeatureFlags), DecodeError> {
    report.check_error()?;
    let params = report.params();
    let index = FeatureIndex(params[0]);
    let flags = FeatureFlags::from_bits_truncate(params[1]);
    Ok((index, flags))
}

/// Function 1: Ping — verify the device is alive.
///
/// The ping payload is arbitrary; the device echoes it back.
/// Also used to detect HID++ 2.0 protocol version.
pub fn encode_ping(device: DeviceIndex, sw_id: SoftwareId) -> LongReport {
    // Ping uses feature index 0x00 (Root), function 1
    // Byte 6 (params[2]) is the ping data byte, echoed back in the response
    let mut params = [0u8; 16];
    params[2] = 0x5A; // arbitrary ping marker
    LongReport::request(device, FeatureIndex::ROOT, FunctionId(1), sw_id, &params)
}

/// Decode ping response. Returns the protocol version `(major, minor)`.
pub fn decode_ping(report: &LongReport) -> Result<(u8, u8), DecodeError> {
    report.check_error()?;
    let params = report.params();
    // Params[0] = protocol major version
    // Params[1] = protocol minor version
    // Params[2] = ping data (echoed back)
    Ok((params[0], params[1]))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn encode_get_feature_smart_shift() {
        let report = encode_get_feature(
            DeviceIndex::BLE_DIRECT,
            FeatureId(0x2110),
            SoftwareId::DEFAULT,
        );

        assert_eq!(report.device_index(), DeviceIndex::BLE_DIRECT);
        assert_eq!(report.feature_index(), FeatureIndex::ROOT);
        assert_eq!(report.function_id(), FunctionId(0));
        // Feature ID 0x2110 in big-endian
        assert_eq!(report.params()[0], 0x21);
        assert_eq!(report.params()[1], 0x10);
    }

    #[test]
    fn decode_get_feature_response() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[1] = 0xFF; // device index
        report.as_bytes_mut()[2] = 0x00; // feature index (root)
        report.as_bytes_mut()[3] = 0x01; // function 0, sw_id 1
        report.as_bytes_mut()[4] = 0x07; // returned feature index
        report.as_bytes_mut()[5] = 0x00; // flags

        let (idx, flags) = decode_get_feature(&report).unwrap();
        assert_eq!(idx, FeatureIndex(0x07));
        assert!(flags.is_empty());
    }

    #[test]
    fn decode_feature_not_found() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[2] = 0x00;
        report.as_bytes_mut()[4] = 0x00; // index 0 = not found

        let (idx, _) = decode_get_feature(&report).unwrap();
        assert_eq!(idx, FeatureIndex(0x00)); // 0 means not supported
    }

    #[test]
    fn encode_ping_format() {
        let report = encode_ping(DeviceIndex::BLE_DIRECT, SoftwareId::DEFAULT);
        assert_eq!(report.function_id(), FunctionId(1));
        assert_eq!(report.params()[2], 0x5A);
    }
}
