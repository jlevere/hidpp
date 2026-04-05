/// HID++ Feature 0x0005 — DeviceNameType
///
/// Read the device name and type string from the device.
///
/// Function IDs:
/// - 0: GetDeviceNameCount — get the total length of the name string
/// - 1: GetDeviceName — read a chunk of the name (up to 16 chars per call)
/// - 2: GetDeviceType — get the device type enum
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Device type as reported by the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceType {
    Keyboard,
    RemoteControl,
    NumPad,
    Mouse,
    Touchpad,
    Trackball,
    Presenter,
    Receiver,
    Headset,
    Webcam,
    SteeringWheel,
    Joystick,
    Gamepad,
    Dock,
    Speaker,
    Microphone,
    Unknown(u8),
}

impl DeviceType {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Keyboard,
            1 => Self::RemoteControl,
            2 => Self::NumPad,
            3 => Self::Mouse,
            4 => Self::Touchpad,
            5 => Self::Trackball,
            6 => Self::Presenter,
            7 => Self::Receiver,
            8 => Self::Headset,
            9 => Self::Webcam,
            10 => Self::SteeringWheel,
            11 => Self::Joystick,
            12 => Self::Gamepad,
            13 => Self::Dock,
            14 => Self::Speaker,
            15 => Self::Microphone,
            other => Self::Unknown(other),
        }
    }
}

/// Function 0: GetDeviceNameCount — total byte length of device name.
pub fn encode_get_name_length(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_name_length(report: &LongReport) -> Result<u8, DecodeError> {
    report.check_error()?;
    Ok(report.params()[0])
}

/// Function 1: GetDeviceName — read a chunk of the name starting at `offset`.
///
/// Returns up to 16 bytes per call. Call repeatedly with increasing
/// offsets until the full name is read.
pub fn encode_get_name_chunk(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    offset: u8,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[offset])
}

/// Decode a name chunk. Returns the UTF-8 bytes at the requested offset.
/// The caller must concatenate chunks to build the full name.
pub fn decode_get_name_chunk(report: &LongReport) -> &[u8] {
    report.params()
}

/// Function 2: GetDeviceType
pub fn encode_get_device_type(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(2), sw_id, &[])
}

pub fn decode_get_device_type(report: &LongReport) -> Result<DeviceType, DecodeError> {
    report.check_error()?;
    Ok(DeviceType::from_byte(report.params()[0]))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_name_length() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 14; // "MX Master 3S M" = 14 chars

        let len = decode_get_name_length(&report).unwrap();
        assert_eq!(len, 14);
    }

    #[test]
    fn decode_device_type_mouse() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 3; // Mouse

        let dtype = decode_get_device_type(&report).unwrap();
        assert_eq!(dtype, DeviceType::Mouse);
    }
}
