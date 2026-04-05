/// HID++ Feature 0x0007 — DeviceFriendlyName
///
/// Read and write the user-visible Bluetooth name.
/// Name is chunked (15 bytes per call) since HID++ payloads are 16 bytes.
///
/// Function IDs (from decompilation):
/// - 0: GetFriendlyNameLen → (nameLen, nameMaxLen, defaultNameLen)
/// - 1: GetFriendlyName(charIndex) → chunk
/// - 2: GetDefaultFriendlyName(charIndex) → chunk
/// - 3: SetFriendlyName(charIndex, chunk)
/// - 4: ResetFriendlyName
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Name length info.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NameLengths {
    pub name_len: u8,
    pub max_len: u8,
    pub default_len: u8,
}

/// Function 0: GetFriendlyNameLen
pub fn encode_get_name_len(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_name_len(report: &LongReport) -> Result<NameLengths, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(NameLengths {
        name_len: p[0],
        max_len: p[1],
        default_len: p[2],
    })
}

/// Function 1: GetFriendlyName(charIndex) → up to 15 bytes of name.
pub fn encode_get_name(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    char_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[char_index])
}

/// Returns the chunk bytes (up to 15). Caller concatenates chunks.
pub fn decode_get_name_chunk(report: &LongReport) -> &[u8] {
    // param[0] is the echoed char_index, params[1..] is the name data.
    &report.params()[1..]
}

/// Function 3: SetFriendlyName(charIndex, name bytes).
pub fn encode_set_name(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    char_index: u8,
    name_bytes: &[u8],
    sw_id: SoftwareId,
) -> LongReport {
    let mut params = [0u8; 16];
    params[0] = char_index;
    let len = name_bytes.len().min(15);
    params[1..1 + len].copy_from_slice(&name_bytes[..len]);
    LongReport::request(device, feature_index, FunctionId(3), sw_id, &params)
}

/// Function 4: ResetFriendlyName — restore factory default.
pub fn encode_reset_name(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(4), sw_id, &[])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_name_len() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 14; // current length
        report.as_bytes_mut()[5] = 26; // max
        report.as_bytes_mut()[6] = 14; // default

        let lengths = decode_get_name_len(&report).unwrap();
        assert_eq!(lengths.name_len, 14);
        assert_eq!(lengths.max_len, 26);
        assert_eq!(lengths.default_len, 14);
    }
}
