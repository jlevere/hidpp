use std::collections::BTreeMap;
use std::time::Duration;

use hidpp::error::DecodeError;
use hidpp::features::{
    adjustable_dpi, change_host, device_name, feature_set, firmware_info, friendly_name,
    hires_wheel, hosts_info, root, smart_shift, special_keys, thumbwheel, unified_battery,
    wireless_status,
};
use hidpp::report::LongReport;
use hidpp::types::{DeviceIndex, FeatureFlags, FeatureId, FeatureIndex, FunctionId, SoftwareId};
use hidpp_transport::native::HidapiTransport;

/// Error type for device operations.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("transport error: {0}")]
    Transport(#[from] hidpp_transport::TransportError),
    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),
    #[error("feature {0} not supported by this device")]
    FeatureNotSupported(FeatureId),
    #[error("device did not respond to ping")]
    PingFailed,
}

/// Discovered feature entry.
#[derive(Debug, Clone)]
pub struct FeatureEntry {
    pub id: FeatureId,
    pub index: FeatureIndex,
    pub flags: FeatureFlags,
    pub version: u8,
}

/// A connected HID++ 2.0 device session.
///
/// Wraps a transport and provides typed access to device features.
/// All features are discovered automatically on open via FeatureSet enumeration.
pub struct Device {
    transport: HidapiTransport,
    device_index: DeviceIndex,
    sw_id: SoftwareId,
    protocol_version: (u8, u8),
    name: String,
    device_type: Option<device_name::DeviceType>,
    features: BTreeMap<FeatureId, FeatureEntry>,
}

impl Device {
    /// Probe which device index responds to a ping on this transport.
    ///
    /// Tries BLE direct (0xFF) first, then receiver slots 1–6.
    /// Uses a short timeout per probe (300ms) to avoid blocking.
    /// Each probe uses a distinct sw_id so stale pending requests
    /// from timed-out probes don't match subsequent responses.
    pub async fn probe_device_index(
        transport: &HidapiTransport,
    ) -> Result<DeviceIndex, DeviceError> {
        for (i, &index) in DeviceIndex::PROBE_ORDER.iter().enumerate() {
            let sw_id = SoftwareId::new((i as u8 + 1).min(0x0F));
            let ping = root::encode_ping(index, sw_id);
            match tokio::time::timeout(Duration::from_millis(300), transport.request(&ping)).await
            {
                Ok(Ok(resp)) if root::decode_ping(&resp).is_ok() => {
                    tracing::info!("probed device index 0x{:02X} — responded", index.0);
                    return Ok(index);
                }
                _ => continue,
            }
        }
        Err(DeviceError::PingFailed)
    }

    /// Connect to a device, ping it, and discover all features.
    pub async fn open(
        transport: HidapiTransport,
        device_index: DeviceIndex,
    ) -> Result<Self, DeviceError> {
        let sw_id = SoftwareId::DEFAULT;

        // Step 1: Ping to verify the device is alive and get protocol version.
        let ping_req = root::encode_ping(device_index, sw_id);
        let ping_resp = transport.request(&ping_req).await?;
        let protocol_version =
            root::decode_ping(&ping_resp).map_err(|_| DeviceError::PingFailed)?;

        tracing::info!(
            "Device ping OK, protocol version {}.{}",
            protocol_version.0,
            protocol_version.1,
        );

        // Step 2: Find the FeatureSet feature index.
        let fs_req =
            root::encode_get_feature(device_index, hidpp::feature_id::FEATURE_SET, sw_id);
        let fs_resp = transport.request(&fs_req).await?;
        let (fs_index, _) = root::decode_get_feature(&fs_resp)?;

        if fs_index == FeatureIndex(0x00) {
            tracing::warn!("Device does not support FeatureSet (0x0001)");
            return Ok(Self {
                transport,
                device_index,
                sw_id,
                protocol_version,
                name: "Unknown Device".into(),
                device_type: None,
                features: BTreeMap::new(),
            });
        }

        // Step 3: Get feature count.
        let count_req = feature_set::encode_get_count(device_index, fs_index, sw_id);
        let count_resp = transport.request(&count_req).await?;
        let count = feature_set::decode_get_count(&count_resp)?;

        tracing::info!("Device has {count} features");

        // Step 4: Enumerate all features.
        let mut features = BTreeMap::new();

        // Root is always at index 0.
        features.insert(
            hidpp::feature_id::ROOT,
            FeatureEntry {
                id: hidpp::feature_id::ROOT,
                index: FeatureIndex::ROOT,
                flags: FeatureFlags::empty(),
                version: 0,
            },
        );

        for i in 1..=count {
            let req = feature_set::encode_get_feature_id(device_index, fs_index, i, sw_id);
            let resp = transport.request(&req).await?;
            let info = feature_set::decode_get_feature_id(&resp)?;

            let name = hidpp::feature_id::feature_name(info.feature_id)
                .unwrap_or("Unknown");

            tracing::debug!(
                "  [{:02X}] {} ({}), flags={:?}, v{}",
                i,
                info.feature_id,
                name,
                info.flags,
                info.version,
            );

            features.insert(
                info.feature_id,
                FeatureEntry {
                    id: info.feature_id,
                    index: FeatureIndex(i),
                    flags: info.flags,
                    version: info.version,
                },
            );
        }

        // Step 5: Read device name if available.
        let (name, device_type) =
            Self::read_device_name(&transport, device_index, sw_id, &features).await;

        tracing::info!("Connected to: {name}");

        Ok(Self {
            transport,
            device_index,
            sw_id,
            protocol_version,
            name,
            device_type,
            features,
        })
    }

    /// Read device name and type via feature 0x0005.
    async fn read_device_name(
        transport: &HidapiTransport,
        device_index: DeviceIndex,
        sw_id: SoftwareId,
        features: &BTreeMap<FeatureId, FeatureEntry>,
    ) -> (String, Option<device_name::DeviceType>) {
        let Some(entry) = features.get(&hidpp::feature_id::DEVICE_NAME_TYPE) else {
            return ("Unknown Device".into(), None);
        };
        let idx = entry.index;

        // Get name length.
        let len_req = device_name::encode_get_name_length(device_index, idx, sw_id);
        let Ok(len_resp) = transport.request(&len_req).await else {
            return ("Unknown Device".into(), None);
        };
        let Ok(name_len) = device_name::decode_get_name_length(&len_resp) else {
            return ("Unknown Device".into(), None);
        };

        // Read name chunks (16 bytes per call).
        let mut name_bytes = Vec::with_capacity(name_len as usize);
        let mut offset = 0u8;
        while (name_bytes.len()) < name_len as usize {
            let chunk_req =
                device_name::encode_get_name_chunk(device_index, idx, sw_id, offset);
            let Ok(chunk_resp) = transport.request(&chunk_req).await else {
                break;
            };
            let chunk = device_name::decode_get_name_chunk(&chunk_resp);
            let remaining = name_len as usize - name_bytes.len();
            let take = remaining.min(chunk.len());
            name_bytes.extend_from_slice(&chunk[..take]);
            offset = name_bytes.len() as u8;
        }

        let name = String::from_utf8(name_bytes).unwrap_or_else(|_| "Unknown Device".into());

        // Get device type.
        let dtype_req = device_name::encode_get_device_type(device_index, idx, sw_id);
        let device_type = transport
            .request(&dtype_req)
            .await
            .ok()
            .and_then(|r| device_name::decode_get_device_type(&r).ok());

        (name, device_type)
    }

    /// HID++ protocol version `(major, minor)`.
    pub fn protocol_version(&self) -> (u8, u8) {
        self.protocol_version
    }

    /// Device name as reported by the device.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Device type as reported by the device.
    pub fn device_type(&self) -> Option<&device_name::DeviceType> {
        self.device_type.as_ref()
    }

    /// All discovered features.
    pub fn features(&self) -> impl Iterator<Item = &FeatureEntry> {
        self.features.values()
    }

    /// Check if the device supports a given feature.
    pub fn supports(&self, feature_id: FeatureId) -> bool {
        self.features.contains_key(&feature_id)
    }

    /// Subscribe to unsolicited HID++ notifications from this device.
    ///
    /// Returns a broadcast receiver that yields raw reports for diverted
    /// button presses, scroll events, battery changes, etc.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LongReport> {
        self.transport.subscribe()
    }

    /// Reverse lookup: FeatureIndex → FeatureId.
    pub fn feature_id_for_index(&self, index: FeatureIndex) -> Option<FeatureId> {
        self.features
            .iter()
            .find(|(_, e)| e.index == index)
            .map(|(id, _)| *id)
    }

    /// Get the runtime feature index for a feature ID.
    fn feature_index(&self, feature_id: FeatureId) -> Result<FeatureIndex, DeviceError> {
        self.features
            .get(&feature_id)
            .map(|e| e.index)
            .ok_or(DeviceError::FeatureNotSupported(feature_id))
    }

    /// Send a raw HID++ request for any feature.
    ///
    /// This is the escape hatch for features we haven't typed yet.
    pub async fn raw_request(
        &self,
        feature_id: FeatureId,
        function: u8,
        params: &[u8],
    ) -> Result<LongReport, DeviceError> {
        let idx = self.feature_index(feature_id)?;
        let report = LongReport::request(
            self.device_index,
            idx,
            FunctionId(function),
            self.sw_id,
            params,
        );
        Ok(self.transport.request(&report).await?)
    }

    // --- Typed feature accessors ---

    /// Read battery status (feature 0x1004).
    pub async fn battery_status(
        &self,
    ) -> Result<unified_battery::BatteryStatus, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::UNIFIED_BATTERY)?;
        let req =
            unified_battery::encode_get_status(self.device_index, idx, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(unified_battery::decode_get_status(&resp)?)
    }

    /// Read SmartShift state.
    ///
    /// Automatically uses the correct function IDs based on whether the device
    /// has 0x2111 (enhanced, Fn1=GET) or 0x2110 (legacy, Fn0=GET).
    pub async fn smart_shift_get(
        &self,
    ) -> Result<smart_shift::SmartShiftState, DeviceError> {
        let (idx, enhanced) = self.smart_shift_info()?;
        let req = if enhanced {
            smart_shift::encode_get_mode_v1(self.device_index, idx, self.sw_id)
        } else {
            smart_shift::encode_get_mode_v0(self.device_index, idx, self.sw_id)
        };
        let resp = self.transport.request(&req).await?;
        Ok(smart_shift::decode_get_mode(&resp)?)
    }

    /// Set SmartShift state.
    ///
    /// Uses correct function IDs: 0x2111 Fn2=SET (3 bytes) or 0x2110 Fn1=SET (2 bytes).
    pub async fn smart_shift_set(
        &self,
        state: &smart_shift::SmartShiftState,
    ) -> Result<smart_shift::SmartShiftState, DeviceError> {
        let (idx, enhanced) = self.smart_shift_info()?;
        let req = if enhanced {
            smart_shift::encode_set_mode_v1(self.device_index, idx, self.sw_id, state)
        } else {
            smart_shift::encode_set_mode_v0(self.device_index, idx, self.sw_id, state)
        };
        let resp = self.transport.request(&req).await?;
        Ok(smart_shift::decode_set_mode(&resp)?)
    }

    /// Check if device has any SmartShift feature.
    pub fn has_smart_shift(&self) -> bool {
        self.supports(hidpp::feature_id::SMART_SHIFT_TUNABLE_TORQUE)
            || self.supports(hidpp::feature_id::SMART_SHIFT)
    }

    /// Get SmartShift feature index and whether it's the enhanced (0x2111) variant.
    fn smart_shift_info(&self) -> Result<(FeatureIndex, bool), DeviceError> {
        if let Ok(idx) = self.feature_index(hidpp::feature_id::SMART_SHIFT_TUNABLE_TORQUE) {
            Ok((idx, true))
        } else {
            let idx = self.feature_index(hidpp::feature_id::SMART_SHIFT)?;
            Ok((idx, false))
        }
    }

    // --- HiResWheel (0x2121) ---

    /// Read HiResWheel mode (feature 0x2121).
    pub async fn hires_wheel_get_mode(
        &self,
    ) -> Result<hires_wheel::WheelMode, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::HIRES_WHEEL)?;
        let req = hires_wheel::encode_get_mode(self.device_index, idx, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(hires_wheel::decode_get_mode(&resp)?)
    }

    /// Set HiResWheel mode (feature 0x2121).
    pub async fn hires_wheel_set_mode(
        &self,
        mode: &hires_wheel::WheelMode,
    ) -> Result<hires_wheel::WheelMode, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::HIRES_WHEEL)?;
        let req =
            hires_wheel::encode_set_mode(self.device_index, idx, self.sw_id, mode);
        let resp = self.transport.request(&req).await?;
        Ok(hires_wheel::decode_set_mode(&resp)?)
    }

    // --- Thumbwheel (0x2150) ---

    /// Read thumbwheel info (feature 0x2150).
    pub async fn thumbwheel_get_info(
        &self,
    ) -> Result<thumbwheel::ThumbwheelInfo, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::THUMBWHEEL)?;
        let req = thumbwheel::encode_get_info(self.device_index, idx, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(thumbwheel::decode_get_info(&resp)?)
    }

    /// Read thumbwheel status (feature 0x2150).
    pub async fn thumbwheel_get_status(
        &self,
    ) -> Result<thumbwheel::ThumbwheelStatus, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::THUMBWHEEL)?;
        let req = thumbwheel::encode_get_status(self.device_index, idx, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(thumbwheel::decode_get_status(&resp)?)
    }

    // --- ChangeHost / Easy-Switch (0x1814) ---

    /// Read host info (feature 0x1814).
    pub async fn host_info(&self) -> Result<change_host::HostInfo, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::CHANGE_HOST)?;
        let req =
            change_host::encode_get_host_info(self.device_index, idx, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(change_host::decode_get_host_info(&resp)?)
    }

    // --- FirmwareInfo (0x0003) ---

    /// Read firmware entity info (feature 0x0003).
    pub async fn firmware_info(
        &self,
    ) -> Result<Vec<firmware_info::EntityInfo>, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::FIRMWARE_INFO)?;
        let count_req =
            firmware_info::encode_get_entity_count(self.device_index, idx, self.sw_id);
        let count_resp = self.transport.request(&count_req).await?;
        let count = firmware_info::decode_get_entity_count(&count_resp)?;

        let mut entities = Vec::with_capacity(count as usize);
        for i in 0..count {
            let req =
                firmware_info::encode_get_fw_info(self.device_index, idx, i, self.sw_id);
            let resp = self.transport.request(&req).await?;
            entities.push(firmware_info::decode_get_fw_info(&resp)?);
        }
        Ok(entities)
    }

    // --- SpecialKeys (0x1B04) ---

    /// List all remappable controls (feature 0x1B04).
    pub async fn special_keys_list(
        &self,
    ) -> Result<Vec<special_keys::ControlInfo>, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::SPECIAL_KEYS_V4)?;
        let count_req =
            special_keys::encode_get_count(self.device_index, idx, self.sw_id);
        let count_resp = self.transport.request(&count_req).await?;
        let count = special_keys::decode_get_count(&count_resp)?;

        let mut controls = Vec::with_capacity(count as usize);
        for i in 0..count {
            let req = special_keys::encode_get_ctrl_id_info(
                self.device_index,
                idx,
                i,
                self.sw_id,
            );
            let resp = self.transport.request(&req).await?;
            controls.push(special_keys::decode_get_ctrl_id_info(&resp)?);
        }
        Ok(controls)
    }

    /// Get current reporting config for a button (feature 0x1B04).
    pub async fn special_key_reporting(
        &self,
        cid: u16,
    ) -> Result<special_keys::ControlReporting, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::SPECIAL_KEYS_V4)?;
        let req = special_keys::encode_get_ctrl_id_reporting(
            self.device_index,
            idx,
            cid,
            self.sw_id,
        );
        let resp = self.transport.request(&req).await?;
        Ok(special_keys::decode_get_ctrl_id_reporting(&resp)?)
    }

    /// Set reporting config for a button (feature 0x1B04).
    pub async fn special_key_set_reporting(
        &self,
        cid: u16,
        flags: u8,
        remapped_cid: u16,
        additional_flags: u8,
    ) -> Result<special_keys::ControlReporting, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::SPECIAL_KEYS_V4)?;
        let req = special_keys::encode_set_ctrl_id_reporting(
            self.device_index,
            idx,
            self.sw_id,
            cid,
            flags,
            remapped_cid,
            additional_flags,
        );
        let resp = self.transport.request(&req).await?;
        Ok(special_keys::decode_set_ctrl_id_reporting(&resp)?)
    }

    /// Switch to a different host slot (feature 0x1814).
    ///
    /// Warning: this will disconnect the device from the current host.
    pub async fn switch_host(&self, host_index: u8) -> Result<(), DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::CHANGE_HOST)?;
        let req = change_host::encode_set_current_host(
            self.device_index,
            idx,
            self.sw_id,
            host_index,
        );
        // SetCurrentHost causes a disconnect, so don't wait for response.
        self.transport.send(&req).await?;
        Ok(())
    }

    // --- FriendlyName (0x0007) ---

    /// Read the user-settable Bluetooth name (feature 0x0007).
    pub async fn friendly_name(&self) -> Result<String, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::DEVICE_FRIENDLY_NAME)?;

        let len_req =
            friendly_name::encode_get_name_len(self.device_index, idx, self.sw_id);
        let len_resp = self.transport.request(&len_req).await?;
        let lengths = friendly_name::decode_get_name_len(&len_resp)?;

        let mut name_bytes = Vec::with_capacity(lengths.name_len as usize);
        let mut offset = 0u8;
        while name_bytes.len() < lengths.name_len as usize {
            let req =
                friendly_name::encode_get_name(self.device_index, idx, offset, self.sw_id);
            let resp = self.transport.request(&req).await?;
            let chunk = friendly_name::decode_get_name_chunk(&resp);
            let remaining = lengths.name_len as usize - name_bytes.len();
            let take = remaining.min(chunk.len());
            name_bytes.extend_from_slice(&chunk[..take]);
            offset = name_bytes.len() as u8;
        }

        Ok(String::from_utf8(name_bytes).unwrap_or_else(|_| "?".into()))
    }

    // --- HostsInfos (0x1815) ---

    /// Get OS version for a specific host slot (feature 0x1815).
    pub async fn host_os_version(
        &self,
        host_index: u8,
    ) -> Result<hosts_info::HostOSVersion, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::HOSTS_INFOS)?;
        let req = hosts_info::encode_get_host_os_version(
            self.device_index,
            idx,
            host_index,
            self.sw_id,
        );
        let resp = self.transport.request(&req).await?;
        Ok(hosts_info::decode_get_host_os_version(&resp)?)
    }

    // --- WirelessStatus (0x1D4B) ---

    /// Read wireless connection status (feature 0x1D4B).
    pub async fn wireless_status(
        &self,
    ) -> Result<wireless_status::WirelessStatus, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::WIRELESS_STATUS)?;
        let req =
            wireless_status::encode_get_status(self.device_index, idx, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(wireless_status::decode_get_status(&resp)?)
    }

    /// Read current DPI (feature 0x2201).
    pub async fn dpi_get(&self) -> Result<u16, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::ADJUSTABLE_DPI)?;
        let req =
            adjustable_dpi::encode_get_dpi(self.device_index, idx, 0, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(adjustable_dpi::decode_get_dpi(&resp)?)
    }

    /// Set DPI (feature 0x2201).
    pub async fn dpi_set(&self, dpi: u16) -> Result<u16, DeviceError> {
        let idx = self.feature_index(hidpp::feature_id::ADJUSTABLE_DPI)?;
        let req =
            adjustable_dpi::encode_set_dpi(self.device_index, idx, 0, dpi, self.sw_id);
        let resp = self.transport.request(&req).await?;
        Ok(adjustable_dpi::decode_set_dpi(&resp)?)
    }

    // --- Config export/import ---

    /// Export current device state as a TOML-serializable config.
    pub async fn export_config(&self) -> Result<crate::DeviceConfig, DeviceError> {
        use crate::config::*;

        let (major, minor) = self.protocol_version;

        let dpi = if self.supports(hidpp::feature_id::ADJUSTABLE_DPI) {
            Some(DpiSection {
                value: self.dpi_get().await?,
            })
        } else {
            None
        };

        let smartshift = if self.has_smart_shift() {
            let state = self.smart_shift_get().await?;
            Some(SmartShiftSection {
                mode: match state.mode {
                    smart_shift::WheelMode::FreeScroll => "freespin".into(),
                    smart_shift::WheelMode::Ratchet => "ratchet".into(),
                },
                auto_disengage: state.auto_disengage,
                torque: state.tunable_torque,
            })
        } else {
            None
        };

        let wheel = if self.supports(hidpp::feature_id::HIRES_WHEEL) {
            let mode = self.hires_wheel_get_mode().await?;
            Some(WheelSection {
                high_resolution: mode.high_resolution,
                inverted: mode.inverted,
            })
        } else {
            None
        };

        let thumbwheel = if self.supports(hidpp::feature_id::THUMBWHEEL) {
            let status = self.thumbwheel_get_status().await?;
            Some(ThumbwheelSection {
                inverted: status.inverted,
            })
        } else {
            None
        };

        let host = if self.supports(hidpp::feature_id::CHANGE_HOST) {
            let info = self.host_info().await?;
            Some(HostSection {
                current: info.current_host,
                count: info.num_hosts,
            })
        } else {
            None
        };

        let device_type_str = self.device_type.as_ref().map(|dt| format!("{dt:?}"));

        Ok(DeviceConfig {
            device: DeviceSection {
                name: self.name.clone(),
                pid: format!("{:04X}", 0), // filled by caller if needed
                device_type: device_type_str,
                protocol: format!("{major}.{minor}"),
            },
            dpi,
            smartshift,
            wheel,
            thumbwheel,
            host,
        })
    }

    /// Apply settings from a config. Only applies sections that are present.
    pub async fn import_config(&self, config: &crate::DeviceConfig) -> Result<(), DeviceError> {
        if let Some(dpi) = &config.dpi {
            self.dpi_set(dpi.value).await?;
            tracing::info!("Applied DPI: {}", dpi.value);
        }

        if let Some(ss) = &config.smartshift {
            let mode = match ss.mode.as_str() {
                "freespin" | "free" => smart_shift::WheelMode::FreeScroll,
                _ => smart_shift::WheelMode::Ratchet,
            };
            let state = smart_shift::SmartShiftState {
                mode,
                auto_disengage: ss.auto_disengage,
                tunable_torque: ss.torque,
            };
            self.smart_shift_set(&state).await?;
            tracing::info!("Applied SmartShift: {mode:?}");
        }

        if let Some(wheel) = &config.wheel {
            let mut current = self.hires_wheel_get_mode().await?;
            current.high_resolution = wheel.high_resolution;
            current.inverted = wheel.inverted;
            self.hires_wheel_set_mode(&current).await?;
            tracing::info!("Applied wheel: hires={}, inverted={}", wheel.high_resolution, wheel.inverted);
        }

        Ok(())
    }
}
