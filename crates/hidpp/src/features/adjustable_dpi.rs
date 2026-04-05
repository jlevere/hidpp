/// HID++ Feature 0x2201 — AdjustableDPI
///
/// Read and set the mouse sensor DPI (dots per inch).
/// The MX Master 3S supports 200–8000 DPI in steps of 50.
///
/// Function IDs (from RE of `devio::Feature2201AdjustableDPI`):
/// - 0: GetSensorCount
/// - 1: GetSensorDPIList
/// - 2: GetSensorDPI
/// - 3: SetSensorDPI
/// - 4: GetDefaultDPI
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// DPI range description.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DpiRange {
    /// Minimum DPI value.
    pub min: u16,
    /// Maximum DPI value.
    pub max: u16,
    /// Step size between valid DPI values.
    pub step: u16,
}

/// Function 0: GetSensorCount
pub fn encode_get_sensor_count(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_sensor_count(report: &LongReport) -> Result<u8, DecodeError> {
    report.check_error()?;
    Ok(report.params()[0])
}

/// Function 1: GetSensorDPIList
///
/// Returns the list of supported DPI values. This can be either:
/// - A range (min, step, max) if the step value is non-zero
/// - A discrete list of DPI values
///
/// From RE debug strings:
/// ```text
/// Feature2201AdjustableDPI::GetSensorDPIList (index=%d)
/// Feature2201AdjustableDPI::GetSensorDPIList: step=%d
/// Feature2201AdjustableDPI::GetSensorDPIList: value=%d
/// ```
pub fn encode_get_dpi_list(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sensor_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[sensor_index])
}

/// Decoded DPI list — either a continuous range or discrete values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DpiList {
    /// Continuous range with step.
    Range(DpiRange),
    /// Discrete list of supported values.
    Discrete(Vec<u16>),
}

pub fn decode_get_dpi_list(report: &LongReport) -> Result<DpiList, DecodeError> {
    report.check_error()?;
    let p = report.params();

    // The response contains pairs of big-endian u16 values.
    // If a value has bit 13 set (0x2000), it's a step indicator.
    // Format: [dpi_1_hi, dpi_1_lo, dpi_2_hi, dpi_2_lo, ...]
    // A step value (with 0xE000 mask) indicates range mode.

    let mut values: Vec<u16> = Vec::new();
    let mut step: Option<u16> = None;

    let mut i = 0;
    while i + 1 < p.len() {
        let val = u16::from_be_bytes([p[i], p[i + 1]]);
        if val == 0 {
            break;
        }

        if val & 0xE000 != 0 {
            // This is a step indicator — lower 13 bits are the step
            step = Some(val & 0x1FFF);
        } else {
            values.push(val);
        }
        i += 2;
    }

    if let Some(step) = step {
        if values.len() >= 2 {
            Ok(DpiList::Range(DpiRange {
                min: values[0],
                max: values[values.len() - 1],
                step,
            }))
        } else {
            Ok(DpiList::Discrete(values))
        }
    } else {
        Ok(DpiList::Discrete(values))
    }
}

/// Function 2: GetSensorDPI — read the current DPI value.
pub fn encode_get_dpi(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sensor_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(2), sw_id, &[sensor_index])
}

/// Decode GetSensorDPI response. Returns the current DPI value.
///
/// From RE: `GetSensorDPI(unsigned short &dpi, unsigned int sensorIndex)`
pub fn decode_get_dpi(report: &LongReport) -> Result<u16, DecodeError> {
    report.check_error()?;
    let p = report.params();
    // Params[0] = sensor index (echo)
    // Params[1..3] = DPI as big-endian u16
    Ok(u16::from_be_bytes([p[1], p[2]]))
}

/// Function 3: SetSensorDPI — set the DPI value.
///
/// From RE: `SetSensorDPI(unsigned short dpi, unsigned int sensorIndex, unsigned int dpiLevel)`
pub fn encode_set_dpi(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sensor_index: u8,
    dpi: u16,
    sw_id: SoftwareId,
) -> LongReport {
    let dpi_bytes = dpi.to_be_bytes();
    LongReport::request(
        device,
        feature_index,
        FunctionId(3),
        sw_id,
        &[sensor_index, dpi_bytes[0], dpi_bytes[1]],
    )
}

/// Decode SetSensorDPI response. Returns the actually applied DPI.
pub fn decode_set_dpi(report: &LongReport) -> Result<u16, DecodeError> {
    decode_get_dpi(report)
}

/// Function 4: GetDefaultDPI
pub fn encode_get_default_dpi(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sensor_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(4), sw_id, &[sensor_index])
}

pub fn decode_get_default_dpi(report: &LongReport) -> Result<u16, DecodeError> {
    decode_get_dpi(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn encode_set_dpi_1600() {
        let report = encode_set_dpi(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x08),
            0, // sensor 0
            1600,
            SoftwareId::DEFAULT,
        );

        assert_eq!(report.feature_index(), FeatureIndex(0x08));
        assert_eq!(report.function_id(), FunctionId(3));
        assert_eq!(report.params()[0], 0); // sensor index
        // 1600 = 0x0640 big-endian
        assert_eq!(report.params()[1], 0x06);
        assert_eq!(report.params()[2], 0x40);
    }

    #[test]
    fn decode_dpi_response() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 0; // sensor index
        report.as_bytes_mut()[5] = 0x06; // DPI high byte
        report.as_bytes_mut()[6] = 0x40; // DPI low byte (0x0640 = 1600)

        let dpi = decode_get_dpi(&report).unwrap();
        assert_eq!(dpi, 1600);
    }

    #[test]
    fn encode_set_dpi_max() {
        let report = encode_set_dpi(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x08),
            0,
            8000,
            SoftwareId::DEFAULT,
        );
        // 8000 = 0x1F40
        assert_eq!(report.params()[1], 0x1F);
        assert_eq!(report.params()[2], 0x40);
    }
}
