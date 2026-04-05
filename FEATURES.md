# HID++ 2.0 Feature Reference — MX Master 3S For Mac

Device: MX Master 3S For Mac (PID 0xB034)
Protocol: HID++ 4.5
Total features: 37

## Core / System (0x0000–0x00FF)

| Index | ID | Name | Version | Status | Notes |
|-------|------|------|---------|--------|-------|
| 0x00 | 0x0000 | Root | v0 | Implemented | Ping, feature lookup |
| 0x01 | 0x0001 | FeatureSet | v2 | Implemented | Feature discovery |
| 0x02 | 0x0003 | FirmwareInfo | v4 | TODO | FW version, entity info |
| 0x03 | 0x0005 | DeviceNameType | v0 | Implemented | Device name + type |
| 0x07 | 0x0007 | DeviceFriendlyName | v0 | TODO | User-settable BT name |
| 0x05 | 0x0020 | ConfigChange | v0 | TODO | Config ownership cookie |
| 0x06 | 0x0021 | CryptoIdentifier | v1 | TODO | Cryptographic device ID |
| 0x12 | 0x00C3 | DFUControl v3 | v0 | SKIP | Firmware update — dangerous |

## Battery / Power (0x1000–0x10FF)

| Index | ID | Name | Version | Status | Notes |
|-------|------|------|---------|--------|-------|
| 0x08 | 0x1004 | UnifiedBattery | v3 | Implemented | Battery %, level, charging |

## Connectivity / Config (0x1600–0x1EFF)

| Index | ID | Name | Version | Status | Notes |
|-------|------|------|---------|--------|-------|
| 0x1D | 0x1602 | ??? | v0 | Unknown | |
| 0x13 | 0x1802 | DeviceReset | v0 | SKIP | Factory reset — dangerous |
| 0x14 | 0x1803 | ??? | v0 | Unknown | |
| 0x17 | 0x1805 | OOBState | v0 | TODO | Out-of-box state |
| 0x15 | 0x1806 | ConfigurableDeviceProperties | v8 | TODO | Device properties |
| 0x0A | 0x1814 | ChangeHost | v1 | Implemented | Easy-Switch |
| 0x0B | 0x1815 | HostsInfos | v2 | TODO | Host names per slot |
| 0x16 | 0x1816 | BleProPrepairing | v0 | TODO | BLE pairing management |
| 0x18 | 0x1830 | ??? | v0 | Unknown | |
| 0x1F | 0x1861 | ??? | v1 | Unknown | |
| 0x19 | 0x1891 | ??? | v7 | Unknown | |
| 0x1A | 0x18A1 | LEDState | v0 | TODO | LED indicator control |
| 0x09 | 0x1B04 | SpecialKeys v4 | v5 | TODO | Button remapping — HIGH PRIORITY |
| 0x04 | 0x1D4B | WirelessStatus | v0 | TODO | Connection quality |
| 0x1B | 0x1E00 | EnableHiddenFeatures | v0 | TODO | Unlock hidden features |
| 0x1C | 0x1E02 | ??? | v0 | Unknown | |
| 0x22 | 0x1E22 | ??? | v0 | Unknown | |
| 0x1E | 0x1EB0 | ??? | v0 | Unknown | |

## Mouse / Sensor (0x2000–0x2FFF)

| Index | ID | Name | Version | Status | Notes |
|-------|------|------|---------|--------|-------|
| 0x0E | 0x2110 | SmartShift | v0 | Implemented | Scroll ratchet/free mode |
| 0x0F | 0x2121 | HiResWheel | v1 | Implemented | Scroll resolution/inversion |
| 0x10 | 0x2150 | Thumbwheel | v0 | Implemented | Horizontal scroll |
| 0x0D | 0x2201 | AdjustableDPI | v2 | Implemented | DPI get/set |
| 0x0C | 0x2250 | ??? | v1 | Unknown | Probably AnalysisMode |
| 0x11 | 0x2251 | ??? | v0 | Unknown | |

## Manufacturing / Test (0x9000–0x9FFF)

| Index | ID | Name | Version | Status | Notes |
|-------|------|------|---------|--------|-------|
| 0x21 | 0x9001 | ??? | v0 | Unknown | Manufacturing range |
| 0x23 | 0x9203 | ??? | v0 | Unknown | Manufacturing range |
| 0x24 | 0x9205 | ??? | v0 | Unknown | Manufacturing range |
| 0x20 | 0x9300 | ??? | v0 | Unknown | Manufacturing range |

---

## SpecialKeys v4 (0x1B04) — Button Remapping Protocol

**Priority**: HIGH — this is the #1 reason people install Logi Options+.

### Function IDs (confirmed from decompilation)
| Fn | Name | Request | Response |
|----|------|---------|----------|
| 0 | GetCount | (none) | count: u8 |
| 1 | GetCtrlIdInfo | index: u8 | cid: u16(BE), tid: u16(BE), flags: u8, pos: u8, group: u8, groupMask: u8, additionalFlags: u8 |
| 2 | GetCtrlIdReporting | cid: u16(BE) | cid: u16(BE), flags: u8, remappedCid: u16(BE), additionalFlags: u8 |
| 3 | SetCtrlIdReporting | cid: u16(BE), flags: u8, remappedCid: u16(BE), additionalFlags: u8 | (echoes) |
| 4 | GetCapabilities | (none) | capabilities: u32 |
| 5 | ResetAllCidReportSettings | (none) | (none) |

### Data Model
- **CID** (Control ID): Physical button identifier (e.g., 82=middle click, 195=gesture)
- **TID** (Task ID): Default action for this control
- **Flags**: divertable, persistently divertable, virtual, etc.
- **Remapped CID**: When diverted, what CID the button reports as

### Event Reports
- **KeyReport**: Diverted button press/release events
- **RawXYReport**: Raw mouse XY when button diverted with rawXY flag
- **RawWheelReport**: Raw wheel data when wheel diverted

### MX Master 3S Buttons
| CID | Name | Position |
|-----|------|----------|
| 82 (0x52) | Middle Click | Wheel |
| 83 (0x53) | Back | Side lower |
| 86 (0x56) | Forward | Side upper |
| 195 (0xC3) | Gesture | Thumb |
| 196 (0xC4) | Mode Shift | Top |

---

## Unknown Features — Identification Status

| ID | Agent Result | Likely Identity |
|----|-------------|-----------------|
| 0x1602 | No class found in binary | Unknown — possibly device-only FW feature |
| 0x1803 | Near 0x1802 (DeviceReset) | Possibly GPIO/test access |
| 0x1830 | No class found | Possibly power management |
| 0x1861 | No class found | Unknown |
| 0x1891 | Near 0x1890 (RfTest) | Possibly extended RF test |
| 0x1E02 | No class found | Unknown — near EnableHiddenFeatures |
| 0x1E22 | No class found | Unknown |
| 0x1EB0 | No class found | Unknown |
| 0x2250 | **AnalysisMode** | Sensor calibration/diagnostics |
| 0x2251 | Not in binary | Device-only FW feature, no host handler |

---

## Implementation Summary

- **Implemented**: 9 features (Root, FeatureSet, DeviceName, Battery, SmartShift, HiResWheel, Thumbwheel, DPI, ChangeHost)
- **TODO**: 10 features (FirmwareInfo, FriendlyName, ConfigChange, CryptoId, HostsInfos, BlePrepairing, LEDState, **SpecialKeys**, WirelessStatus, EnableHidden)
- **Unknown/FW-only**: 8 features (0x1602, 0x1803, 0x1830, 0x1861, 0x1891, 0x1E02, 0x1E22, 0x1EB0)
- **Identified**: 2 features (0x2250=AnalysisMode, 0x2251=FW-only)
- **Skip**: 2 features (DFU, DeviceReset — dangerous)
- **Manufacturing**: 4 features (0x9001, 0x9203, 0x9205, 0x9300 — awaiting RE results)
