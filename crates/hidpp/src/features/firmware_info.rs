/// HID++ Feature 0x0003 — FirmwareInfo
///
/// Read firmware version, entity info, hardware revision.
///
/// Function IDs (from decompilation):
/// - 0: GetEntityCount
/// - 1: GetFwInfo(entityIndex) → type, name, version, build, transport
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Entity type in the device firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityType {
    Firmware,
    Bootloader,
    Hardware,
    Unknown(u8),
}

impl EntityType {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Firmware,
            1 => Self::Bootloader,
            2 => Self::Hardware,
            other => Self::Unknown(other),
        }
    }
}

/// Firmware entity info.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityInfo {
    pub entity_type: EntityType,
    /// 3-character name (e.g., "MPM", "BL1", "HW1").
    pub name: String,
    /// Version major.minor (BCD encoded).
    pub version_major: u8,
    pub version_minor: u8,
    /// Build number.
    pub build: u16,
    /// Transport layer ID.
    pub transport: u8,
}

/// Function 0: GetEntityCount
pub fn encode_get_entity_count(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_entity_count(report: &LongReport) -> Result<u8, DecodeError> {
    report.check_error()?;
    Ok(report.params()[0])
}

/// Function 1: GetFwInfo
pub fn encode_get_fw_info(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    entity_index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(
        device,
        feature_index,
        FunctionId(1),
        sw_id,
        &[entity_index],
    )
}

pub fn decode_get_fw_info(report: &LongReport) -> Result<EntityInfo, DecodeError> {
    report.check_error()?;
    let p = report.params();

    let entity_type = EntityType::from_byte(p[0]);
    let name = String::from_utf8_lossy(&[p[1], p[2], p[3]])
        .trim_end_matches('\0')
        .to_string();
    let version_major = p[4];
    let version_minor = p[5];
    let build = u16::from_be_bytes([p[6], p[7]]);
    let transport = p[8];

    Ok(EntityInfo {
        entity_type,
        name,
        version_major,
        version_minor,
        build,
        transport,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_fw_info() {
        let mut report = LongReport::new();
        report.as_bytes_mut()[4] = 0;              // Firmware type
        report.as_bytes_mut()[5] = b'M';           // name
        report.as_bytes_mut()[6] = b'P';
        report.as_bytes_mut()[7] = b'M';
        report.as_bytes_mut()[8] = 0x12;           // major
        report.as_bytes_mut()[9] = 0x34;           // minor
        report.as_bytes_mut()[10] = 0x00;          // build hi
        report.as_bytes_mut()[11] = 0x42;          // build lo
        report.as_bytes_mut()[12] = 0x04;          // transport

        let info = decode_get_fw_info(&report).unwrap();
        assert_eq!(info.entity_type, EntityType::Firmware);
        assert_eq!(info.name, "MPM");
        assert_eq!(info.version_major, 0x12);
        assert_eq!(info.version_minor, 0x34);
        assert_eq!(info.build, 0x0042);
    }
}
