use bitflags::bitflags;

/// Device index in HID++ reports.
///
/// - `0xFF` for BLE direct connections
/// - `1..=6` for devices paired through a Bolt/Unifying receiver
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceIndex(pub u8);

impl DeviceIndex {
    /// BLE direct connection (no receiver).
    pub const BLE_DIRECT: Self = Self(0xFF);

    /// Receiver device slots 1–6 (Bolt/Unifying/Lightspeed).
    pub const RECEIVER_1: Self = Self(0x01);
    pub const RECEIVER_2: Self = Self(0x02);
    pub const RECEIVER_3: Self = Self(0x03);
    pub const RECEIVER_4: Self = Self(0x04);
    pub const RECEIVER_5: Self = Self(0x05);
    pub const RECEIVER_6: Self = Self(0x06);

    /// All valid device indices in probe order: BLE first, then receiver slots.
    pub const PROBE_ORDER: &[Self] = &[
        Self::BLE_DIRECT,
        Self::RECEIVER_1,
        Self::RECEIVER_2,
        Self::RECEIVER_3,
        Self::RECEIVER_4,
        Self::RECEIVER_5,
        Self::RECEIVER_6,
    ];
}

/// A 16-bit HID++ feature identifier.
///
/// Feature IDs are fixed per the HID++ spec (e.g., `0x2110` = SmartShift).
/// They are resolved to runtime [`FeatureIndex`] values via feature `0x0000` (Root).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeatureId(pub u16);

impl core::fmt::Display for FeatureId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:04X}", self.0)
    }
}

/// Runtime feature index assigned by the device.
///
/// Obtained by querying feature `0x0000` (Root) with a [`FeatureId`].
/// Valid range is `0x00..=0xFF`. Index `0x00` is always Root itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeatureIndex(pub u8);

impl FeatureIndex {
    /// Root feature is always at index 0.
    pub const ROOT: Self = Self(0x00);

    /// Error indicator — responses with this index signal an error.
    pub const ERROR: Self = Self(0xFF);
}

/// HID++ function ID (0–15).
///
/// Upper nibble of byte 3 in a HID++ report. Each feature defines
/// up to 16 functions (e.g., function 0 = GetCapabilities, function 1 = GetState).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionId(pub u8);

impl FunctionId {
    pub const fn new(id: u8) -> Self {
        assert!(id <= 0x0F, "function ID must be 0-15");
        Self(id)
    }
}

/// Software ID (0–15).
///
/// Lower nibble of byte 3 in a HID++ report. Used to correlate
/// responses to requests. Must be unique among software talking
/// to the same device simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SoftwareId(pub u8);

impl SoftwareId {
    /// Default SW ID for our tool. Avoids 0 (used by firmware)
    /// and common values used by Logi Options+.
    pub const DEFAULT: Self = Self(0x01);

    pub const fn new(id: u8) -> Self {
        assert!(id <= 0x0F, "software ID must be 0-15");
        Self(id)
    }
}

bitflags! {
    /// Feature flags returned by FeatureSet::GetFeatureID.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct FeatureFlags: u8 {
        /// Feature is hidden from normal software enumeration.
        const ENGINEERING_HIDDEN = 0x80;
        /// Feature is hidden from user-facing software.
        const SW_HIDDEN = 0x40;
        /// Feature is obsolete and should not be used.
        const OBSOLETE = 0x20;
    }
}
