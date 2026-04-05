/// Device configuration — TOML-serializable snapshot of all device settings.
///
/// Used for config export/import. Human-readable and hand-editable.
use serde::{Deserialize, Serialize};

/// Complete device configuration snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub device: DeviceSection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpi: Option<DpiSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smartshift: Option<SmartShiftSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wheel: Option<WheelSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbwheel: Option<ThumbwheelSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<HostSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSection {
    pub name: String,
    pub pid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpiSection {
    pub value: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartShiftSection {
    /// "ratchet" or "freespin".
    pub mode: String,
    pub auto_disengage: u8,
    pub torque: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelSection {
    pub high_resolution: bool,
    pub inverted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbwheelSection {
    pub inverted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostSection {
    pub current: u8,
    pub count: u8,
}

impl DeviceConfig {
    /// Serialize to TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Deserialize from TOML string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_toml() {
        let config = DeviceConfig {
            device: DeviceSection {
                name: "MX Master 3S For Mac".into(),
                pid: "B034".into(),
                device_type: Some("Mouse".into()),
                protocol: "4.5".into(),
            },
            dpi: Some(DpiSection { value: 1600 }),
            smartshift: Some(SmartShiftSection {
                mode: "ratchet".into(),
                auto_disengage: 50,
                torque: 20,
            }),
            wheel: Some(WheelSection {
                high_resolution: true,
                inverted: false,
            }),
            thumbwheel: Some(ThumbwheelSection { inverted: false }),
            host: Some(HostSection {
                current: 0,
                count: 3,
            }),
        };

        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("MX Master 3S For Mac"));
        assert!(toml_str.contains("ratchet"));
        assert!(toml_str.contains("1600"));

        let parsed = DeviceConfig::from_toml(&toml_str).unwrap();
        assert_eq!(parsed.device.name, "MX Master 3S For Mac");
        assert_eq!(parsed.dpi.as_ref().map(|d| d.value), Some(1600));
    }

    #[test]
    fn minimal_config() {
        let toml_str = r#"
[device]
name = "MX Master 3S"
pid = "B034"
protocol = "4.5"

[dpi]
value = 2000
"#;
        let config = DeviceConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.dpi.as_ref().map(|d| d.value), Some(2000));
        assert!(config.smartshift.is_none());
        assert!(config.wheel.is_none());
    }
}
