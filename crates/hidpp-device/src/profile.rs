/// Device profiles — optional enrichment data for known devices.
///
/// Profiles provide human-readable button names, DPI ranges, and feature flags
/// for known Logitech devices. The tool works without profiles (everything is
/// discovered at runtime via HID++), but profiles make the UI nicer.
///
/// The profile database is generated from Logitech's own device configs
/// and compiled into the binary via `include_str!`.
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

/// The full device profile database, compiled into the binary.
const PROFILES_JSON: &str = include_str!("../data/profiles.json");

static PROFILES: LazyLock<ProfileDb> = LazyLock::new(|| {
    let profiles: Vec<DeviceProfile> =
        serde_json::from_str(PROFILES_JSON).unwrap_or_default();

    let mut by_pid: HashMap<String, usize> = HashMap::new();
    let mut by_model: HashMap<String, usize> = HashMap::new();

    for (i, p) in profiles.iter().enumerate() {
        by_model.insert(p.model_id.clone(), i);
        for pid in &p.pids {
            by_pid.insert(pid.to_lowercase(), i);
        }
    }

    ProfileDb {
        profiles,
        by_pid,
        by_model,
    }
});

struct ProfileDb {
    profiles: Vec<DeviceProfile>,
    by_pid: HashMap<String, usize>,
    by_model: HashMap<String, usize>,
}

/// A device profile from the database.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceProfile {
    pub name: String,
    pub model_id: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub depot: String,
    #[serde(default)]
    pub pids: Vec<String>,
    #[serde(default)]
    pub buttons: Vec<u16>,
    #[serde(default)]
    pub hosts: u8,
    #[serde(default)]
    pub features: ProfileFeatures,
    #[serde(default)]
    pub dpi: Option<DpiProfile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProfileFeatures {
    #[serde(default)]
    pub battery: bool,
    #[serde(default)]
    pub dpi: bool,
    #[serde(default)]
    pub smartshift: bool,
    #[serde(default)]
    pub hires_scroll: bool,
    #[serde(default)]
    pub thumbwheel: bool,
    #[serde(default)]
    pub fn_inversion: bool,
    #[serde(default)]
    pub backlight: bool,
    #[serde(default)]
    pub disable_keys: bool,
    #[serde(default)]
    pub pointer_speed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DpiProfile {
    pub min: u16,
    pub max: u16,
    pub default: u16,
    pub step: u16,
}

impl DeviceProfile {
    /// Look up a profile by product ID (hex string, e.g., "b034").
    pub fn by_pid(pid: &str) -> Option<&'static DeviceProfile> {
        let idx = PROFILES.by_pid.get(&pid.to_lowercase())?;
        PROFILES.profiles.get(*idx)
    }

    /// Look up a profile by model ID (e.g., "2b034").
    pub fn by_model(model_id: &str) -> Option<&'static DeviceProfile> {
        let idx = PROFILES.by_model.get(model_id)?;
        PROFILES.profiles.get(*idx)
    }

    /// Get all profiles.
    pub fn all() -> &'static [DeviceProfile] {
        &PROFILES.profiles
    }

    /// Number of profiles in the database.
    pub fn count() -> usize {
        PROFILES.profiles.len()
    }

    /// Known button name for a control ID.
    pub fn button_name(&self, control_id: u16) -> Option<&'static str> {
        // Common control ID names from the HID++ spec.
        Some(match control_id {
            50 => "Left Click",
            51 => "Right Click",
            52 | 82 => "Middle Click",
            53 | 83 => "Back",
            56 | 86 => "Forward",
            195 => "Gesture Button",
            196 => "Mode Shift",
            199 => "Dictation",
            200 => "Emoji",
            110 => "Screen Capture",
            _ => return None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn load_profiles() {
        let count = DeviceProfile::count();
        assert!(count > 100, "Expected 100+ profiles, got {count}");
    }

    #[test]
    fn find_mx_master_3s() {
        let profile = DeviceProfile::by_pid("b034").unwrap();
        assert!(profile.name.contains("MX Master 3S"));
        assert_eq!(profile.buttons, vec![82, 83, 86, 195, 196]);
        assert!(profile.features.smartshift);
        assert!(profile.features.dpi);
    }

    #[test]
    fn find_mx_keys() {
        let profile = DeviceProfile::by_model("6b35b").unwrap();
        assert!(profile.name.contains("MX Keys"));
        assert!(profile.features.fn_inversion);
        assert!(profile.features.backlight);
        assert!(!profile.features.dpi);
    }
}
