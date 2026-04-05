use crate::error::{DecodeError, HidppError};
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// A raw HID++ report of `N` bytes (including report ID).
///
/// HID++ defines three report sizes:
/// - Short:     7 bytes  (report ID `0x10`, 3 bytes params)
/// - Long:     20 bytes  (report ID `0x11`, 16 bytes params)
/// - Very Long: 64 bytes (report ID `0x12`, 59 bytes params)
#[derive(Clone, PartialEq, Eq)]
pub struct Report<const N: usize> {
    buf: [u8; N],
}

/// Short HID++ report (7 bytes). Used by HID++ 1.0 and simple commands.
pub type ShortReport = Report<7>;

/// Long HID++ report (20 bytes). The standard HID++ 2.0 message format.
/// This is what the MX Master 3S uses over BLE.
pub type LongReport = Report<20>;

/// Very long HID++ report (64 bytes). Used for bulk transfers (DFU, profiles).
pub type VeryLongReport = Report<64>;

/// Report ID constants.
pub const REPORT_ID_SHORT: u8 = 0x10;
pub const REPORT_ID_LONG: u8 = 0x11;
pub const REPORT_ID_VERY_LONG: u8 = 0x12;

impl<const N: usize> Report<N> {
    /// Create a zeroed report with the appropriate report ID.
    pub fn new() -> Self {
        let mut buf = [0u8; N];
        buf[0] = match N {
            7 => REPORT_ID_SHORT,
            20 => REPORT_ID_LONG,
            64 => REPORT_ID_VERY_LONG,
            _ => 0,
        };
        Self { buf }
    }

    /// Create from raw bytes. Returns `None` if length doesn't match.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != N {
            return None;
        }
        let mut buf = [0u8; N];
        buf.copy_from_slice(bytes);
        Some(Self { buf })
    }

    /// The raw byte buffer.
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.buf
    }

    /// Mutable access to the raw buffer.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; N] {
        &mut self.buf
    }

    /// Report ID (byte 0).
    pub fn report_id(&self) -> u8 {
        self.buf[0]
    }

    /// Device index (byte 1).
    pub fn device_index(&self) -> DeviceIndex {
        DeviceIndex(self.buf[1])
    }

    /// Set device index (byte 1).
    pub fn set_device_index(&mut self, idx: DeviceIndex) {
        self.buf[1] = idx.0;
    }

    /// Feature index (byte 2).
    pub fn feature_index(&self) -> FeatureIndex {
        FeatureIndex(self.buf[2])
    }

    /// Set feature index (byte 2).
    pub fn set_feature_index(&mut self, idx: FeatureIndex) {
        self.buf[2] = idx.0;
    }

    /// Function ID — upper nibble of byte 3.
    pub fn function_id(&self) -> FunctionId {
        FunctionId(self.buf[3] >> 4)
    }

    /// Software ID — lower nibble of byte 3.
    pub fn sw_id(&self) -> SoftwareId {
        SoftwareId(self.buf[3] & 0x0F)
    }

    /// Set function ID and software ID (byte 3).
    pub fn set_function_sw(&mut self, func: FunctionId, sw: SoftwareId) {
        self.buf[3] = (func.0 << 4) | (sw.0 & 0x0F);
    }

    /// Parameter bytes (bytes 4..N).
    pub fn params(&self) -> &[u8] {
        &self.buf[4..]
    }

    /// Mutable parameter bytes.
    pub fn params_mut(&mut self) -> &mut [u8] {
        &mut self.buf[4..]
    }

    /// Check if this report is an error response (feature index == 0xFF).
    pub fn is_error(&self) -> bool {
        self.buf[2] == 0xFF
    }

    /// If this is an error response, decode it.
    /// Returns `(original_feature_index, error_code)`.
    pub fn decode_error(&self) -> Option<(FeatureIndex, HidppError)> {
        if !self.is_error() {
            return None;
        }
        let orig_feature_index = FeatureIndex(self.buf[3]);
        let error_code = HidppError::from_code(self.buf[4]);
        Some((orig_feature_index, error_code))
    }

    /// Check for error response and return a `DecodeError` if present.
    pub fn check_error(&self) -> Result<(), DecodeError> {
        if let Some((feature_index, error)) = self.decode_error() {
            Err(DecodeError::DeviceError {
                feature_index,
                error,
            })
        } else {
            Ok(())
        }
    }
}

impl LongReport {
    /// Build a HID++ 2.0 long report request.
    pub fn request(
        device: DeviceIndex,
        feature_index: FeatureIndex,
        function: FunctionId,
        sw_id: SoftwareId,
        params: &[u8],
    ) -> Self {
        let mut report = Self::new();
        report.set_device_index(device);
        report.set_feature_index(feature_index);
        report.set_function_sw(function, sw_id);

        let param_len = params.len().min(16);
        report.buf[4..4 + param_len].copy_from_slice(&params[..param_len]);

        report
    }
}

impl<const N: usize> Default for Report<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> core::fmt::Debug for Report<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Report<{N}>({:02X}:{:02X}:{:02X}:{:02X} ",
            self.buf[0], self.buf[1], self.buf[2], self.buf[3],
        )?;
        for (i, b) in self.buf[4..].iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{b:02X}")?;
        }
        write!(f, ")")
    }
}

impl<const N: usize> AsRef<[u8]> for Report<N> {
    fn as_ref(&self) -> &[u8] {
        &self.buf
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn long_report_construction() {
        let report = LongReport::request(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x05),
            FunctionId(0x02),
            SoftwareId::DEFAULT,
            &[0xAA, 0xBB],
        );

        assert_eq!(report.report_id(), REPORT_ID_LONG);
        assert_eq!(report.device_index(), DeviceIndex::BLE_DIRECT);
        assert_eq!(report.feature_index(), FeatureIndex(0x05));
        assert_eq!(report.function_id(), FunctionId(0x02));
        assert_eq!(report.sw_id(), SoftwareId::DEFAULT);
        assert_eq!(report.params()[0], 0xAA);
        assert_eq!(report.params()[1], 0xBB);
        assert_eq!(report.params()[2], 0x00); // zero-padded
    }

    #[test]
    fn error_response_decoding() {
        let mut report = LongReport::new();
        report.buf[1] = 0xFF; // device index
        report.buf[2] = 0xFF; // error indicator
        report.buf[3] = 0x05; // original feature index
        report.buf[4] = 0x02; // InvalidArgument

        assert!(report.is_error());
        let (idx, err) = report.decode_error().unwrap();
        assert_eq!(idx, FeatureIndex(0x05));
        assert_eq!(err, HidppError::InvalidArgument);
    }

    #[test]
    fn function_sw_packing() {
        let mut report = LongReport::new();
        report.set_function_sw(FunctionId(0x0A), SoftwareId(0x03));
        assert_eq!(report.buf[3], 0xA3);
        assert_eq!(report.function_id(), FunctionId(0x0A));
        assert_eq!(report.sw_id(), SoftwareId(0x03));
    }

    #[test]
    fn from_bytes() {
        let bytes = [0x11u8; 20];
        let report = LongReport::from_bytes(&bytes).unwrap();
        assert_eq!(report.as_bytes(), &bytes);

        assert!(LongReport::from_bytes(&[0u8; 19]).is_none());
        assert!(LongReport::from_bytes(&[0u8; 21]).is_none());
    }
}
