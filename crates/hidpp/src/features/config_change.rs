/// HID++ Feature 0x0020 — ConfigChange
///
/// Configuration change cookie system. Prevents race conditions when
/// multiple hosts configure the device simultaneously.
///
/// Function IDs (from decompilation):
/// - 0: GetConfigurationCookie → cookie (u16 BE)
/// - 1: SetConfigurationComplete(cookie) → success
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Function 0: GetConfigurationCookie
pub fn encode_get_cookie(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_cookie(report: &LongReport) -> Result<u16, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(u16::from_be_bytes([p[0], p[1]]))
}

/// Function 1: SetConfigurationComplete — signal that config is done.
pub fn encode_set_complete(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    cookie: u16,
    sw_id: SoftwareId,
) -> LongReport {
    let bytes = cookie.to_be_bytes();
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &bytes)
}
