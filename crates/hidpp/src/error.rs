use crate::types::FeatureIndex;

/// HID++ 2.0 error codes returned by the device.
///
/// When a device returns an error, the response has `feature_index = 0xFF`,
/// byte 3 contains the feature index that caused the error, and byte 4
/// contains the error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HidppError {
    #[error("no error")]
    NoError,
    #[error("unknown error")]
    Unknown,
    #[error("invalid argument")]
    InvalidArgument,
    #[error("out of range")]
    OutOfRange,
    #[error("hardware error")]
    HwError,
    #[error("logitech internal error")]
    LogitechInternal,
    #[error("invalid feature index")]
    InvalidFeatureIndex,
    #[error("invalid function ID")]
    InvalidFunctionId,
    #[error("device busy")]
    Busy,
    #[error("unsupported")]
    Unsupported,
    #[error("unknown error code: {0:#04x}")]
    Other(u8),
}

impl HidppError {
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => Self::NoError,
            0x01 => Self::Unknown,
            0x02 => Self::InvalidArgument,
            0x03 => Self::OutOfRange,
            0x04 => Self::HwError,
            0x05 => Self::LogitechInternal,
            0x06 => Self::InvalidFeatureIndex,
            0x07 => Self::InvalidFunctionId,
            0x08 => Self::Busy,
            0x09 => Self::Unsupported,
            other => Self::Other(other),
        }
    }
}

/// Error returned when decoding a HID++ report fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("device returned HID++ error for feature index {feature_index:?}: {error}")]
    DeviceError {
        feature_index: FeatureIndex,
        error: HidppError,
    },
    #[error("unexpected response: expected feature index {expected:?}, got {actual:?}")]
    WrongFeatureIndex {
        expected: FeatureIndex,
        actual: FeatureIndex,
    },
    #[error("unexpected response: expected function {expected}, got {actual}")]
    WrongFunction { expected: u8, actual: u8 },
    #[error("response too short: need {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
}
