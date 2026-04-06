/// HID++ Feature 0x1B04 — SpecialKeys v4
///
/// Button remapping and diversion. The most important feature for
/// mouse customization — maps physical buttons to actions.
///
/// Function IDs (confirmed from decompilation):
/// - 0: GetCount — number of remappable controls
/// - 1: GetCtrlIdInfo — info about a control at a given index
/// - 2: GetCtrlIdReporting — current reporting config for a CID
/// - 3: SetCtrlIdReporting — set reporting config for a CID
/// - 4: GetCapabilities
/// - 5: ResetAllCidReportSettings
use crate::error::DecodeError;
use crate::report::LongReport;
use crate::types::{ControlId, DeviceIndex, FeatureIndex, FunctionId, SoftwareId};

/// Information about a remappable control.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ControlInfo {
    /// Control ID — identifies the physical button.
    pub cid: ControlId,
    /// Task ID — the default action for this control.
    pub tid: ControlId,
    /// Capability flags.
    pub flags: u8,
    /// Position in the device layout.
    pub position: u8,
    /// Group this control belongs to.
    pub group: u8,
    /// Group mask.
    pub group_mask: u8,
    /// Additional capability flags.
    pub additional_flags: u8,
}

impl ControlInfo {
    /// Whether this control can be diverted to software.
    pub fn is_divertable(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Whether diversion persists across power cycles.
    pub fn is_persistently_divertable(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    /// Whether this is a virtual control (not a physical button).
    pub fn is_virtual(&self) -> bool {
        (self.flags & 0x04) != 0
    }
}

/// Current reporting configuration for a control.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ControlReporting {
    /// The control ID.
    pub cid: ControlId,
    /// Reporting flags (divert, rawXY, persist, etc.).
    pub flags: u8,
    /// Remapped control ID (0 = not remapped).
    pub remapped_cid: ControlId,
    /// Additional flags.
    pub additional_flags: u8,
}

impl ControlReporting {
    /// Whether this control is currently diverted to software.
    pub fn is_diverted(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Whether raw XY data is reported when diverted.
    pub fn raw_xy_enabled(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    /// Whether diversion persists across power cycles.
    pub fn persist_enabled(&self) -> bool {
        (self.flags & 0x04) != 0
    }
}

/// Function 0: GetCount — number of remappable controls.
pub fn encode_get_count(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(0), sw_id, &[])
}

pub fn decode_get_count(report: &LongReport) -> Result<u8, DecodeError> {
    report.check_error()?;
    Ok(report.params()[0])
}

/// Function 1: GetCtrlIdInfo — info about a control at index.
pub fn encode_get_ctrl_id_info(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    index: u8,
    sw_id: SoftwareId,
) -> LongReport {
    LongReport::request(device, feature_index, FunctionId(1), sw_id, &[index])
}

pub fn decode_get_ctrl_id_info(report: &LongReport) -> Result<ControlInfo, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(ControlInfo {
        cid: ControlId(u16::from_be_bytes([p[0], p[1]])),
        tid: ControlId(u16::from_be_bytes([p[2], p[3]])),
        flags: p[4],
        position: p[5],
        group: p[6],
        group_mask: p[7],
        additional_flags: p[8],
    })
}

/// Function 2: GetCtrlIdReporting — current reporting for a CID.
pub fn encode_get_ctrl_id_reporting(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    cid: ControlId,
    sw_id: SoftwareId,
) -> LongReport {
    let cid_bytes = cid.0.to_be_bytes();
    LongReport::request(device, feature_index, FunctionId(2), sw_id, &cid_bytes)
}

pub fn decode_get_ctrl_id_reporting(report: &LongReport) -> Result<ControlReporting, DecodeError> {
    report.check_error()?;
    let p = report.params();
    Ok(ControlReporting {
        cid: ControlId(u16::from_be_bytes([p[0], p[1]])),
        flags: p[2],
        remapped_cid: ControlId(u16::from_be_bytes([p[3], p[4]])),
        additional_flags: p[5],
    })
}

/// Function 3: SetCtrlIdReporting — set reporting for a CID.
pub fn encode_set_ctrl_id_reporting(
    device: DeviceIndex,
    feature_index: FeatureIndex,
    sw_id: SoftwareId,
    cid: ControlId,
    flags: u8,
    remapped_cid: ControlId,
    additional_flags: u8,
) -> LongReport {
    let cid_bytes = cid.0.to_be_bytes();
    let remap_bytes = remapped_cid.0.to_be_bytes();
    LongReport::request(
        device,
        feature_index,
        FunctionId(3),
        sw_id,
        &[
            cid_bytes[0],
            cid_bytes[1],
            flags,
            remap_bytes[0],
            remap_bytes[1],
            additional_flags,
        ],
    )
}

pub fn decode_set_ctrl_id_reporting(report: &LongReport) -> Result<ControlReporting, DecodeError> {
    decode_get_ctrl_id_reporting(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn decode_ctrl_id_info() {
        let mut report = LongReport::new();
        // CID 82 (0x0052) = middle click
        report.as_bytes_mut()[4] = 0x00;
        report.as_bytes_mut()[5] = 0x52;
        // TID 58 (0x003A)
        report.as_bytes_mut()[6] = 0x00;
        report.as_bytes_mut()[7] = 0x3A;
        // flags: divertable + persist
        report.as_bytes_mut()[8] = 0x03;
        report.as_bytes_mut()[9] = 0x01; // position
        report.as_bytes_mut()[10] = 0x02; // group
        report.as_bytes_mut()[11] = 0x04; // group mask
        report.as_bytes_mut()[12] = 0x00; // additional

        let info = decode_get_ctrl_id_info(&report).unwrap();
        assert_eq!(info.cid, ControlId(82));
        assert_eq!(info.tid, ControlId(58));
        assert!(info.is_divertable());
        assert!(info.is_persistently_divertable());
        assert!(!info.is_virtual());
    }

    #[test]
    fn encode_set_reporting() {
        let report = encode_set_ctrl_id_reporting(
            DeviceIndex::BLE_DIRECT,
            FeatureIndex(0x09),
            SoftwareId::DEFAULT,
            ControlId(82), // CID
            0x01,          // flags: divert
            ControlId(0),  // no remap
            0,
        );

        assert_eq!(report.function_id(), FunctionId(3));
        // CID 82 = 0x0052 big-endian
        assert_eq!(report.params()[0], 0x00);
        assert_eq!(report.params()[1], 0x52);
        assert_eq!(report.params()[2], 0x01); // flags
    }
}
