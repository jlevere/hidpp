/// Device profiles — Logitech's full device database (170 devices).
///
/// Compiled into the binary from `data/devices_full.json`. Provides
/// display names, capabilities, DPI ranges, button lists, and PIDs
/// for known devices. The tool works without profiles (HID++ is
/// self-describing), but profiles provide enrichment.
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

const DB_JSON: &str = include_str!("../data/devices_full.json");

static DB: LazyLock<ProfileDb> = LazyLock::new(|| {
    let raw: RawDb = serde_json::from_str(DB_JSON).unwrap_or_default();

    let mut by_pid: HashMap<String, usize> = HashMap::new();
    let mut by_model: HashMap<String, usize> = HashMap::new();

    for (i, dev) in raw.devices.iter().enumerate() {
        by_model.insert(dev.model_id.clone(), i);

        // Extract PIDs from modes.interfaces.id ("046d_XXXX")
        for mode in &dev.modes {
            for iface in &mode.interfaces {
                if let Some(pid) = iface.id.split('_').nth(1) {
                    by_pid.insert(pid.to_lowercase(), i);
                }
            }
        }
    }

    ProfileDb {
        devices: raw.devices,
        by_pid,
        by_model,
    }
});

struct ProfileDb {
    devices: Vec<DeviceProfile>,
    by_pid: HashMap<String, usize>,
    by_model: HashMap<String, usize>,
}

// --- Raw JSON types matching Logitech's format ---

#[derive(Default, Deserialize)]
struct RawDb {
    #[serde(default)]
    devices: Vec<DeviceProfile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub extended_display_name: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub depot: String,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub modes: Vec<Mode>,
}

// Logitech's JSON uses MIXED naming (camelCase + snake_case). We alias both.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub flow: Option<Flow>,
    #[serde(default, alias = "has_battery_status")]
    pub has_battery_status: bool,
    #[serde(default, alias = "has_high_resolution_sensor")]
    pub has_high_resolution_sensor: bool,
    #[serde(default, alias = "highResolutionSensorInfo")]
    pub high_resolution_sensor_info: Option<DpiInfo>,
    #[serde(default, alias = "pointerSpeed")]
    pub pointer_speed: bool,
    #[serde(default)]
    pub unified_battery: bool,
    #[serde(default, alias = "hostInfos")]
    pub host_infos: bool,
    #[serde(default, alias = "fnInversion")]
    pub fn_inversion: bool,
    #[serde(default, alias = "disableKeys")]
    pub disable_keys: bool,
    #[serde(default, alias = "specialKeys")]
    pub special_keys: Option<SpecialKeysInfo>,
    #[serde(default)]
    pub scroll_wheel_capabilities: Option<ScrollCapabilities>,
    #[serde(default, alias = "mouseScrollWheelOverride")]
    pub mouse_scroll_wheel_override: Option<serde_json::Value>,
    #[serde(default, alias = "mouseThumbWheelOverride")]
    pub mouse_thumb_wheel_override: Option<serde_json::Value>,
    #[serde(default, alias = "backlightSettingsOverride")]
    pub backlight_settings_override: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    #[serde(default)]
    pub host_count: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiInfo {
    #[serde(default)]
    pub min_dpi_value_sensor_off: u16,
    #[serde(default)]
    pub max_dpi_value_sensor_off: u16,
    #[serde(default)]
    pub default_dpi_value_sensor_off: u16,
    #[serde(default)]
    pub steps_sensor_off: u16,
    #[serde(default)]
    pub min_dpi_value_sensor_on: u16,
    #[serde(default)]
    pub max_dpi_value_sensor_on: u16,
    #[serde(default)]
    pub default_dpi_value_sensor_on: u16,
    #[serde(default)]
    pub steps_sensor_on: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpecialKeysInfo {
    #[serde(default)]
    pub programmable: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollCapabilities {
    #[serde(default)]
    pub smartshift: bool,
    #[serde(default)]
    pub high_resolution: bool,
    #[serde(default)]
    pub adjustable_speed: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Mode {
    #[serde(default)]
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Interface {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub r#type: String,
}

// --- Public API ---

impl DeviceProfile {
    /// Look up a profile by product ID (hex string, e.g., "b034").
    pub fn by_pid(pid: &str) -> Option<&'static DeviceProfile> {
        let idx = DB.by_pid.get(&pid.to_lowercase())?;
        DB.devices.get(*idx)
    }

    /// Look up a profile by model ID (e.g., "2b034").
    pub fn by_model(model_id: &str) -> Option<&'static DeviceProfile> {
        let idx = DB.by_model.get(model_id)?;
        DB.devices.get(*idx)
    }

    /// All profiles.
    pub fn all() -> &'static [DeviceProfile] {
        &DB.devices
    }

    /// Number of profiles.
    pub fn count() -> usize {
        DB.devices.len()
    }

    /// DPI range for this device.
    pub fn dpi_range(&self) -> Option<(u16, u16, u16, u16)> {
        let info = self.capabilities.high_resolution_sensor_info.as_ref()?;
        Some((
            info.min_dpi_value_sensor_off,
            info.max_dpi_value_sensor_off,
            info.default_dpi_value_sensor_off,
            info.steps_sensor_off,
        ))
    }

    /// Number of Easy-Switch host slots.
    pub fn host_count(&self) -> u8 {
        self.capabilities
            .flow
            .as_ref()
            .map(|f| f.host_count)
            .unwrap_or(0)
    }

    /// Programmable button control IDs.
    pub fn buttons(&self) -> &[u16] {
        self.capabilities
            .special_keys
            .as_ref()
            .map(|s| s.programmable.as_slice())
            .unwrap_or(&[])
    }

    /// Known button name for a control ID.
    pub fn button_name(&self, control_id: u16) -> Option<&'static str> {
        Some(match control_id {
            50 | 80 => "Left Click",
            51 | 81 => "Right Click",
            52 | 82 => "Middle Click",
            53 | 83 => "Back",
            56 | 86 => "Forward",
            195 => "Gesture Button",
            196 => "Mode Shift",
            199 => "Dictation",
            200 => "Emoji",
            110 => "Screen Capture",
            215 => "Thumbwheel Click",
            _ => return None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn load_full_database() {
        let count = DeviceProfile::count();
        assert!(count >= 150, "Expected 150+ profiles, got {count}");
    }

    #[test]
    fn find_mx_master_3s_by_pid() {
        let profile = DeviceProfile::by_pid("b034").unwrap();
        assert!(profile.display_name.contains("MX Master 3S"));
        assert!(profile.capabilities.pointer_speed);
        assert!(
            profile
                .capabilities
                .scroll_wheel_capabilities
                .as_ref()
                .is_some_and(|s| s.smartshift)
        );
    }

    #[test]
    fn mx_master_3s_dpi_range() {
        let profile = DeviceProfile::by_pid("b034").unwrap();
        let (min, max, default, step) = profile.dpi_range().unwrap();
        assert_eq!(min, 200);
        assert_eq!(max, 4000);
        assert_eq!(default, 1000);
        assert_eq!(step, 50);
    }

    #[test]
    fn mx_master_3s_buttons() {
        let profile = DeviceProfile::by_pid("b034").unwrap();
        let buttons = profile.buttons();
        assert!(buttons.contains(&82)); // Middle click
        assert!(buttons.contains(&195)); // Gesture
    }

    #[test]
    fn find_mx_keys() {
        let profile = DeviceProfile::by_model("6b35b").unwrap();
        assert!(profile.display_name.contains("MX Keys"));
        assert!(profile.capabilities.fn_inversion);
    }

    #[test]
    fn host_count() {
        let profile = DeviceProfile::by_pid("b034").unwrap();
        assert_eq!(profile.host_count(), 3);
    }
}
