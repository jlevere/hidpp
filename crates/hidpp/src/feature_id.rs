/// Complete HID++ 2.0 feature ID catalog.
///
/// Extracted from Logitech Options+ agent binary (`logioptionsplus_agent`).
/// Feature IDs are fixed 16-bit identifiers. Each device supports a subset
/// of these, discoverable at runtime via feature 0x0001 (FeatureSet).
use crate::types::FeatureId;

// Core / System
pub const ROOT: FeatureId = FeatureId(0x0000);
pub const FEATURE_SET: FeatureId = FeatureId(0x0001);
pub const ENG_TEST: FeatureId = FeatureId(0x0002);
pub const FIRMWARE_INFO: FeatureId = FeatureId(0x0003);
pub const DEVICE_NAME_TYPE: FeatureId = FeatureId(0x0005);
pub const DEVICE_FRIENDLY_NAME: FeatureId = FeatureId(0x0007);
pub const SWITCH_AND_KEEP_ALIVE: FeatureId = FeatureId(0x0008);
pub const SUBDEVICES: FeatureId = FeatureId(0x0009);
pub const PROPERTY_ACCESS: FeatureId = FeatureId(0x0011);
pub const CONFIG_CHANGE: FeatureId = FeatureId(0x0020);
pub const CRYPTO_IDENTIFIER: FeatureId = FeatureId(0x0021);

// DFU (Device Firmware Update)
pub const DFU_CONTROL_V0: FeatureId = FeatureId(0x00C0);
pub const DFU_CONTROL_V1: FeatureId = FeatureId(0x00C1);
pub const DFU_CONTROL_V2: FeatureId = FeatureId(0x00C2);
pub const DFU_CONTROL_V3: FeatureId = FeatureId(0x00C3);
pub const DFU: FeatureId = FeatureId(0x00D0);
pub const RESUMABLE_DFU: FeatureId = FeatureId(0x00D1);

// BLE / Wireless
pub const DEVICE_INFO: FeatureId = FeatureId(0x0100);
pub const DEVICE_NAME_BLE: FeatureId = FeatureId(0x0101);
pub const ROOT_BLE: FeatureId = FeatureId(0x0102);
pub const FEATURE_SET_BLE: FeatureId = FeatureId(0x0103);
pub const BATTERY_SOC: FeatureId = FeatureId(0x0104);
pub const BT_HOST_INFO: FeatureId = FeatureId(0x0305);
pub const LIGHTSPEED_PAIRING: FeatureId = FeatureId(0x0309);
pub const BT_GAMING_MODE: FeatureId = FeatureId(0x030A);

// Battery
pub const BATTERY_UNIFIED_LEVEL_STATUS: FeatureId = FeatureId(0x1000);
pub const BATTERY_VOLTAGE: FeatureId = FeatureId(0x1001);
pub const UNIFIED_BATTERY: FeatureId = FeatureId(0x1004);
pub const CHARGING_CONTROL: FeatureId = FeatureId(0x1010);

// Lighting / Backlight
pub const LED_CONTROL: FeatureId = FeatureId(0x1300);
pub const FORCE_PAIRING: FeatureId = FeatureId(0x1500);
pub const USER_ACTIVITY_MONITORING: FeatureId = FeatureId(0x1701);
pub const GENERIC_TEST: FeatureId = FeatureId(0x1800);
pub const MANUFACTURING_MODE: FeatureId = FeatureId(0x1801);
pub const DEVICE_RESET: FeatureId = FeatureId(0x1802);
pub const OOB_STATE: FeatureId = FeatureId(0x1805);
pub const CONFIGURABLE_DEVICE_PROPERTIES: FeatureId = FeatureId(0x1806);
pub const CONFIGURABLE_PROPERTIES: FeatureId = FeatureId(0x1807);

// Host / Connection
pub const CHANGE_HOST: FeatureId = FeatureId(0x1814);
pub const HOSTS_INFOS: FeatureId = FeatureId(0x1815);
pub const BLE_PRO_PREPAIRING: FeatureId = FeatureId(0x1816);
pub const LED_STATE: FeatureId = FeatureId(0x18A1);

// Keyboard Backlight
pub const KEYBOARD_BACKLIGHT_V1: FeatureId = FeatureId(0x1981);
pub const BACKLIGHT: FeatureId = FeatureId(0x1982);
pub const KEYBOARD_BACKLIGHT_V2: FeatureId = FeatureId(0x1983);
pub const ILLUMINATION_LIGHT: FeatureId = FeatureId(0x1990);

// Input
pub const HAPTIC_FEEDBACK: FeatureId = FeatureId(0x19B0);
pub const FORCE_SENSING_BUTTON: FeatureId = FeatureId(0x19C0);
pub const PRESENTER_CONTROL: FeatureId = FeatureId(0x1A00);

// Special Keys / Buttons
pub const SPECIAL_KEYS_V0: FeatureId = FeatureId(0x1B00);
pub const SPECIAL_KEYS_V3: FeatureId = FeatureId(0x1B03);
pub const SPECIAL_KEYS_V4: FeatureId = FeatureId(0x1B04);
pub const SPECIAL_KEYS_AND_BUTTONS: FeatureId = FeatureId(0x1B06);
pub const CONTROL_LIST: FeatureId = FeatureId(0x1B10);
pub const REPORT_HID_USAGES: FeatureId = FeatureId(0x1BC0);
pub const PERSISTENT_REMAPPABLE_ACTION: FeatureId = FeatureId(0x1C00);

// Wireless
pub const WIRELESS_STATUS: FeatureId = FeatureId(0x1D4B);
pub const ENABLE_HIDDEN_FEATURES: FeatureId = FeatureId(0x1E00);
pub const FIRMWARE_PROPERTIES: FeatureId = FeatureId(0x1F1F);
pub const ADC_MEASUREMENT: FeatureId = FeatureId(0x1F20);

// Mouse
pub const BUTTON_SWAP_CANCEL: FeatureId = FeatureId(0x2005);
pub const POINTER_AXES_ORIENTATION: FeatureId = FeatureId(0x2006);
pub const SMART_SHIFT: FeatureId = FeatureId(0x2110);
pub const SMART_SHIFT_TUNABLE_TORQUE: FeatureId = FeatureId(0x2111);
pub const HIRES_WHEEL: FeatureId = FeatureId(0x2121);
pub const RATCHET_WHEEL: FeatureId = FeatureId(0x2130);
pub const THUMBWHEEL: FeatureId = FeatureId(0x2150);
pub const ADJUSTABLE_DPI: FeatureId = FeatureId(0x2201);
pub const EXTENDED_ADJUSTABLE_DPI: FeatureId = FeatureId(0x2202);
pub const POINTER_MOTION_SCALING: FeatureId = FeatureId(0x2205);
pub const ANGLE_SNAPPING: FeatureId = FeatureId(0x2230);
pub const SURFACE_TUNING: FeatureId = FeatureId(0x2240);
pub const ANALYSIS_MODE: FeatureId = FeatureId(0x2250);
pub const HYBRID_TRACKING: FeatureId = FeatureId(0x2400);

// Keyboard
pub const FN_INVERSION_V0: FeatureId = FeatureId(0x40A0);
pub const FN_INVERSION_V2: FeatureId = FeatureId(0x40A2);
pub const FN_INVERSION_V3: FeatureId = FeatureId(0x40A3);
pub const LOCK_KEY_STATE: FeatureId = FeatureId(0x4220);
pub const DISABLE_KEYS: FeatureId = FeatureId(0x4521);
pub const DUAL_PLATFORM: FeatureId = FeatureId(0x4530);
pub const MULTI_PLATFORM: FeatureId = FeatureId(0x4531);
pub const KB_LAYOUT: FeatureId = FeatureId(0x4540);
pub const CROWN: FeatureId = FeatureId(0x4600);
pub const MULTI_ROLLER: FeatureId = FeatureId(0x4610);

// Gestures / Touchpad
pub const GESTURES_V1: FeatureId = FeatureId(0x6010);
pub const GESTURES_V2: FeatureId = FeatureId(0x6012);
pub const TOUCHPAD_RAW_XY: FeatureId = FeatureId(0x6100);
pub const GESTURES_V3: FeatureId = FeatureId(0x6500);
pub const GESTURES_V4: FeatureId = FeatureId(0x6501);

// Gaming
pub const GKEY: FeatureId = FeatureId(0x8010);
pub const MKEYS: FeatureId = FeatureId(0x8020);
pub const MR: FeatureId = FeatureId(0x8030);
pub const BRIGHTNESS_CONTROL: FeatureId = FeatureId(0x8040);
pub const REPORT_RATE: FeatureId = FeatureId(0x8060);
pub const EXTENDED_ADJUSTABLE_REPORT_RATE: FeatureId = FeatureId(0x8061);
pub const COLOR_LED_EFFECTS: FeatureId = FeatureId(0x8070);
pub const RGB_EFFECTS: FeatureId = FeatureId(0x8071);
pub const PER_KEY_LIGHTING_V1: FeatureId = FeatureId(0x8080);
pub const PER_KEY_LIGHTING_V2: FeatureId = FeatureId(0x8081);
pub const MODE_STATUS: FeatureId = FeatureId(0x8090);
pub const ONBOARD_PROFILES: FeatureId = FeatureId(0x8100);
pub const MOUSE_BUTTON_SPY: FeatureId = FeatureId(0x8110);
pub const LATENCY_MONITORING: FeatureId = FeatureId(0x8111);

// Audio (headsets)
pub const SIDETONE: FeatureId = FeatureId(0x8300);
pub const EQUALIZER: FeatureId = FeatureId(0x8310);
pub const MIC_POLAR_PATTERN: FeatureId = FeatureId(0x8330);

/// Look up a human-readable name for a known feature ID.
pub fn feature_name(id: FeatureId) -> Option<&'static str> {
    Some(match id {
        ROOT => "Root",
        FEATURE_SET => "FeatureSet",
        FeatureId(0x0002) => "EngTest",
        FIRMWARE_INFO => "FirmwareInfo",
        DEVICE_NAME_TYPE => "DeviceNameType",
        DEVICE_FRIENDLY_NAME => "FriendlyName",
        FeatureId(0x0008) => "KeepAlive",
        FeatureId(0x0009) => "Subdevices",
        FeatureId(0x0011) => "PropertyAccess",
        CONFIG_CHANGE => "ConfigChange",
        FeatureId(0x0021) => "CryptoId",
        FeatureId(0x00C0) | FeatureId(0x00C1) | FeatureId(0x00C2) | FeatureId(0x00C3) => "DFUControl",
        FeatureId(0x00D0) => "DFU",
        FeatureId(0x00D1) => "ResumableDFU",
        UNIFIED_BATTERY => "UnifiedBattery",
        BATTERY_UNIFIED_LEVEL_STATUS => "BatteryLevel",
        BATTERY_VOLTAGE => "BatteryVoltage",
        FeatureId(0x1010) => "ChargingControl",
        FeatureId(0x1300) => "LEDControl",
        FeatureId(0x1500) => "ForcePairing",
        FeatureId(0x1602) => "Unknown1602",
        FeatureId(0x1701) => "ActivityMonitor",
        FeatureId(0x1800) => "GenericTest",
        FeatureId(0x1801) => "MfgMode",
        FeatureId(0x1802) => "DeviceReset",
        FeatureId(0x1803) => "GPIOAccess",
        FeatureId(0x1805) => "OOBState",
        FeatureId(0x1806) => "DeviceProperties",
        FeatureId(0x1807) => "ConfigProperties",
        CHANGE_HOST => "ChangeHost",
        HOSTS_INFOS => "HostsInfos",
        FeatureId(0x1816) => "BLEPrepairing",
        FeatureId(0x1830) => "PowerManagement",
        FeatureId(0x1861) => "Unknown1861",
        FeatureId(0x1890) => "RfTest",
        FeatureId(0x1891) => "RfTestExt",
        FeatureId(0x18A1) => "LEDState",
        SPECIAL_KEYS_V4 => "SpecialKeys",
        FeatureId(0x1B00) | FeatureId(0x1B03) => "SpecialKeys",
        FeatureId(0x1B06) => "SpecialKeysButtons",
        FeatureId(0x1B10) => "ControlList",
        FeatureId(0x1BC0) => "HIDUsages",
        FeatureId(0x1C00) => "PersistentRemap",
        WIRELESS_STATUS => "WirelessStatus",
        FeatureId(0x1E00) => "HiddenFeatures",
        FeatureId(0x1E02) => "Unknown1E02",
        FeatureId(0x1E22) => "Unknown1E22",
        FeatureId(0x1EB0) => "Unknown1EB0",
        FeatureId(0x1F1F) => "FWProperties",
        FeatureId(0x1F20) => "ADCMeasure",
        SMART_SHIFT => "SmartShift",
        SMART_SHIFT_TUNABLE_TORQUE => "SmartShift+",
        HIRES_WHEEL => "HiResWheel",
        RATCHET_WHEEL => "RatchetWheel",
        THUMBWHEEL => "Thumbwheel",
        ADJUSTABLE_DPI => "AdjustableDPI",
        EXTENDED_ADJUSTABLE_DPI => "ExtAdjustDPI",
        FeatureId(0x2005) => "ButtonSwap",
        FeatureId(0x2006) => "PointerAxes",
        FeatureId(0x2205) => "PointerScaling",
        ANGLE_SNAPPING => "AngleSnapping",
        FeatureId(0x2240) => "SurfaceTuning",
        FeatureId(0x2250) => "AnalysisMode",
        HYBRID_TRACKING => "HybridTracking",
        FN_INVERSION_V3 => "FnInversion",
        FeatureId(0x40A0) | FeatureId(0x40A2) => "FnInversion",
        FeatureId(0x4220) => "LockKeyState",
        DISABLE_KEYS => "DisableKeys",
        FeatureId(0x4530) => "DualPlatform",
        MULTI_PLATFORM => "MultiPlatform",
        FeatureId(0x4540) => "KBLayout",
        CROWN => "Crown",
        FeatureId(0x4610) => "MultiRoller",
        FeatureId(0x6010) | FeatureId(0x6012) => "Gestures",
        FeatureId(0x6100) => "TouchpadRawXY",
        FeatureId(0x6500) => "Gestures",
        GESTURES_V4 => "Gestures",
        FeatureId(0x8010) => "GKey",
        FeatureId(0x8020) => "MKeys",
        FeatureId(0x8040) => "Brightness",
        REPORT_RATE => "ReportRate",
        FeatureId(0x8061) => "ExtReportRate",
        FeatureId(0x8070) => "ColorLED",
        RGB_EFFECTS => "RGBEffects",
        FeatureId(0x8080) | FeatureId(0x8081) => "PerKeyLight",
        FeatureId(0x8090) => "ModeStatus",
        ONBOARD_PROFILES => "OnboardProfiles",
        FeatureId(0x8110) => "ButtonSpy",
        FeatureId(0x8111) => "LatencyMonitor",
        KEYBOARD_BACKLIGHT_V2 => "Backlight",
        FeatureId(0x1981) => "Backlight",
        FeatureId(0x1990) => "IllumLight",
        HAPTIC_FEEDBACK => "HapticFeedback",
        FeatureId(0x19C0) => "ForceButton",
        FeatureId(0x1A00) => "Presenter",
        FeatureId(0x8300) => "Sidetone",
        FeatureId(0x8310) => "Equalizer",
        FeatureId(0x8330) => "MicPolar",
        FeatureId(0x9001) => "TestPMW",
        FeatureId(0x9203) | FeatureId(0x9205) | FeatureId(0x9300) => "MfgTest",
        _ => return None,
    })
}
