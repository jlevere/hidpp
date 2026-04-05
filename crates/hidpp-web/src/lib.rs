mod webhid;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Log to browser console AND to our log server via fetch.
fn wlog(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
    // Also fire-and-forget POST to log server.
    let _ = js_sys::eval(&format!(
        "fetch('http://localhost:5555/log',{{method:'POST',body:{}}}).catch(()=>{{}})",
        serde_json::json!(msg),
    ));
}

use hidpp::features::{
    adjustable_dpi, change_host, device_name, feature_set, root, smart_shift, unified_battery,
};
use hidpp::report::{LongReport, REPORT_ID_LONG};
use hidpp::types::{DeviceIndex, FeatureId, FeatureIndex, FunctionId, SoftwareId};

const LOGITECH_VID: u16 = 0x046D;
const HIDPP_USAGE_PAGE: u16 = 0xFF43;
const HIDPP_USAGE: u16 = 0x0202;

/// Shared state for the response callback.
struct Pending {
    feature_index: FeatureIndex,
    function_id: FunctionId,
    sw_id: SoftwareId,
    resolve: js_sys::Function,
}

/// Internal device state.
struct Inner {
    device: webhid::HidDevice,
    pending: Vec<Pending>,
    sw_id: SoftwareId,
    device_index: DeviceIndex,
    features: BTreeMap<FeatureId, FeatureIndex>,
    name: String,
}

/// A connected HID++ device in the browser via WebHID.
#[wasm_bindgen]
pub struct WasmDevice {
    inner: Rc<RefCell<Inner>>,
    _input_callback: Closure<dyn FnMut(JsValue)>,
}

#[wasm_bindgen]
impl WasmDevice {
    /// Check if WebHID is available in this browser.
    #[wasm_bindgen(js_name = isSupported)]
    pub fn is_supported() -> bool {
        webhid::get_hid().is_some()
    }

    /// Connect to an already-granted device (no picker dialog).
    ///
    /// Uses `navigator.hid.getDevices()` which returns previously-authorized devices.
    /// Returns null if no device is available — caller should fall back to `connect()`.
    #[wasm_bindgen(js_name = connectGranted)]
    pub async fn connect_granted() -> Result<Option<WasmDevice>, JsValue> {
        let hid = webhid::get_hid()
            .ok_or_else(|| JsValue::from_str("WebHID not available"))?;

        let devices_js = JsFuture::from(hid.get_devices()).await?;
        let devices: js_sys::Array = devices_js.into();

        // Find first Logitech HID++ device.
        let mut target: Option<webhid::HidDevice> = None;
        for i in 0..devices.length() {
            let dev: webhid::HidDevice = devices.get(i).unchecked_into();
            if dev.vendor_id() == LOGITECH_VID {
                target = Some(dev);
                break;
            }
        }

        let Some(hid_device) = target else {
            return Ok(None);
        };

        wlog(&format!("WASM: found granted device: {}", hid_device.product_name()));

        if !hid_device.opened() {
            JsFuture::from(hid_device.open()).await?;
        }

        let device = Self::setup_device(hid_device).await?;
        Ok(Some(device))
    }

    /// Request and connect to a Logitech HID++ device.
    ///
    /// Shows the browser's device picker dialog. User must grant permission.
    pub async fn connect() -> Result<WasmDevice, JsValue> {
        let hid = webhid::get_hid()
            .ok_or_else(|| JsValue::from_str("WebHID not available in this browser"))?;

        // Build request filter for Logitech HID++ devices.
        let filter = js_sys::Object::new();
        js_sys::Reflect::set(&filter, &"vendorId".into(), &LOGITECH_VID.into())?;
        js_sys::Reflect::set(&filter, &"usagePage".into(), &HIDPP_USAGE_PAGE.into())?;
        js_sys::Reflect::set(&filter, &"usage".into(), &HIDPP_USAGE.into())?;

        let filters = js_sys::Array::new();
        filters.push(&filter);

        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"filters".into(), &filters)?;

        // Request device — browser shows picker.
        let devices_js = JsFuture::from(hid.request_device(&options.into())).await?;
        let devices: js_sys::Array = devices_js.into();

        if devices.length() == 0 {
            return Err(JsValue::from_str("No device selected"));
        }

        let hid_device: webhid::HidDevice = devices.get(0).unchecked_into();
        Self::setup_device(hid_device).await
    }

    /// Internal: set up a device (open, register callback, discover features).
    async fn setup_device(hid_device: webhid::HidDevice) -> Result<WasmDevice, JsValue> {
        wlog(&format!("WASM: device: {} (opened={})", hid_device.product_name(), hid_device.opened()));
        if !hid_device.opened() {
            wlog("WASM: opening device...");
            JsFuture::from(hid_device.open()).await?;
            wlog("WASM: device opened.");
        }

        let sw_id = SoftwareId::DEFAULT;
        let device_index = DeviceIndex::BLE_DIRECT;

        let inner = Rc::new(RefCell::new(Inner {
            device: hid_device,
            pending: Vec::new(),
            sw_id,
            device_index,
            features: BTreeMap::new(),
            name: String::new(),
        }));

        // Set up input report callback.
        wlog("WASM: registering inputreport callback...");
        let inner_cb = Rc::clone(&inner);
        let input_callback = Closure::new(move |event: JsValue| {
            // Log that callback fired at all.
            wlog("WASM: inputreport callback fired!");

            let event: webhid::HidInputReportEvent = event.unchecked_into();
            let rid = event.report_id();
            wlog(&format!("WASM: inputreport rid=0x{:02X}", rid));
            if rid != REPORT_ID_LONG {
                wlog(&format!("WASM: skipping non-long report (0x{:02X})", rid));
                return;
            }

            let data_view = event.data();
            let len = data_view.byte_length() as usize;
            let mut buf = vec![0u8; len];
            for (i, byte) in buf.iter_mut().enumerate() {
                *byte = data_view.get_uint8(i);
            }

            // Build full report with report ID prepended.
            let mut full = [0u8; 20];
            full[0] = REPORT_ID_LONG;
            let copy_len = len.min(19);
            full[1..1 + copy_len].copy_from_slice(&buf[..copy_len]);

            let Some(report) = LongReport::from_bytes(&full) else {
                wlog("WASM: failed to parse report bytes");
                return;
            };

            wlog(&format!(
                "WASM: parsed report: dev={:02X} fidx={:02X} fn={} sw={} err={}",
                report.device_index().0,
                report.feature_index().0,
                report.function_id().0,
                report.sw_id().0,
                report.is_error(),
            ));

            // Try to match pending request.
            let mut inner = inner_cb.borrow_mut();
            wlog(&format!("WASM: {} pending requests", inner.pending.len()));
            let matched = inner.pending.iter().position(|p| {
                if report.is_error() {
                    FeatureIndex(report.as_bytes()[3]) == p.feature_index
                } else {
                    report.feature_index() == p.feature_index
                        && report.function_id() == p.function_id
                        && report.sw_id() == p.sw_id
                }
            });

            if let Some(idx) = matched {
                let pending = inner.pending.swap_remove(idx);
                // Serialize report bytes as JSON array for JS.
                let bytes = js_sys::Uint8Array::from(report.as_ref());
                let _ = pending.resolve.call1(&JsValue::NULL, &bytes);
            }
        });

        {
            let inner_ref = inner.borrow();
            inner_ref
                .device
                .add_event_listener("inputreport", &input_callback);
        }

        let mut wasm_device = WasmDevice {
            inner,
            _input_callback: input_callback,
        };

        // Discover features.
        wasm_device.discover_features().await?;

        Ok(wasm_device)
    }

    /// Send a HID++ request and wait for the response.
    async fn request_report(&self, report: &LongReport) -> Result<LongReport, JsValue> {
        // Create a promise that resolves when we get the matching response.
        let (promise, resolve) = {
            let mut resolve_fn: Option<js_sys::Function> = None;
            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                resolve_fn = Some(resolve);
            });
            (promise, resolve_fn.ok_or_else(|| JsValue::from_str("failed to create promise"))?)
        };

        // Register pending request.
        {
            let mut inner = self.inner.borrow_mut();
            inner.pending.push(Pending {
                feature_index: report.feature_index(),
                function_id: report.function_id(),
                sw_id: report.sw_id(),
                resolve,
            });
        }

        // Send the report (skip byte 0 which is the report ID).
        let data = js_sys::Uint8Array::from(&report.as_ref()[1..]);
        wlog(&format!(
            "WASM: sendReport 0x{:02X} fidx={:02X} fn={} sw={}",
            REPORT_ID_LONG,
            report.feature_index().0,
            report.function_id().0,
            report.sw_id().0,
        ));

        // CRITICAL: Get the promise BEFORE awaiting — don't hold borrow across await.
        // The inputreport callback needs borrow_mut() and fires during our await.
        let send_promise = {
            let inner = self.inner.borrow();
            inner.device.send_report(REPORT_ID_LONG, &data)
        };
        JsFuture::from(send_promise).await?;
        wlog("WASM: sendReport done, waiting for response...");

        // Wait for response.
        let result = JsFuture::from(promise).await?;
        let bytes: js_sys::Uint8Array = result.unchecked_into();
        let mut buf = [0u8; 20];
        bytes.copy_to(&mut buf);

        LongReport::from_bytes(&buf)
            .ok_or_else(|| JsValue::from_str("invalid response length"))
    }

    /// Discover all features on the device.
    async fn discover_features(&mut self) -> Result<(), JsValue> {
        let (device_index, sw_id) = {
            let inner = self.inner.borrow();
            (inner.device_index, inner.sw_id)
        };

        // Ping.
        let ping = root::encode_ping(device_index, sw_id);
        self.request_report(&ping).await?;

        // Find FeatureSet.
        let fs_req = root::encode_get_feature(device_index, hidpp::feature_id::FEATURE_SET, sw_id);
        let fs_resp = self.request_report(&fs_req).await?;
        let (fs_index, _) = root::decode_get_feature(&fs_resp)
            .map_err(|e| JsValue::from_str(&format!("decode error: {e}")))?;

        if fs_index == FeatureIndex(0x00) {
            return Ok(());
        }

        // Get count.
        let count_req = feature_set::encode_get_count(device_index, fs_index, sw_id);
        let count_resp = self.request_report(&count_req).await?;
        let count = feature_set::decode_get_count(&count_resp)
            .map_err(|e| JsValue::from_str(&format!("decode error: {e}")))?;

        // Enumerate.
        let mut features = BTreeMap::new();
        features.insert(hidpp::feature_id::ROOT, FeatureIndex::ROOT);

        for i in 1..=count {
            let req = feature_set::encode_get_feature_id(device_index, fs_index, i, sw_id);
            let resp = self.request_report(&req).await?;
            let info = feature_set::decode_get_feature_id(&resp)
                .map_err(|e| JsValue::from_str(&format!("decode error: {e}")))?;
            features.insert(info.feature_id, FeatureIndex(i));
        }

        // Read device name.
        let name = self.read_name(&features, device_index, sw_id).await;

        {
            let mut inner = self.inner.borrow_mut();
            inner.features = features;
            inner.name = name;
        }

        Ok(())
    }

    async fn read_name(
        &self,
        features: &BTreeMap<FeatureId, FeatureIndex>,
        device_index: DeviceIndex,
        sw_id: SoftwareId,
    ) -> String {
        let Some(&idx) = features.get(&hidpp::feature_id::DEVICE_NAME_TYPE) else {
            return "Unknown".into();
        };

        let len_req = device_name::encode_get_name_length(device_index, idx, sw_id);
        let Ok(len_resp) = self.request_report(&len_req).await else {
            return "Unknown".into();
        };
        let Ok(name_len) = device_name::decode_get_name_length(&len_resp) else {
            return "Unknown".into();
        };

        let mut name_bytes = Vec::with_capacity(name_len as usize);
        let mut offset = 0u8;
        while name_bytes.len() < name_len as usize {
            let chunk_req = device_name::encode_get_name_chunk(device_index, idx, sw_id, offset);
            let Ok(chunk_resp) = self.request_report(&chunk_req).await else {
                break;
            };
            let chunk = device_name::decode_get_name_chunk(&chunk_resp);
            let remaining = name_len as usize - name_bytes.len();
            let take = remaining.min(chunk.len());
            name_bytes.extend_from_slice(&chunk[..take]);
            offset = name_bytes.len() as u8;
        }

        String::from_utf8(name_bytes).unwrap_or_else(|_| "Unknown".into())
    }

    fn feature_index(&self, feature_id: FeatureId) -> Result<FeatureIndex, JsValue> {
        let inner = self.inner.borrow();
        inner
            .features
            .get(&feature_id)
            .copied()
            .ok_or_else(|| JsValue::from_str(&format!("Feature {} not supported", feature_id)))
    }

    // --- Public JS API ---

    /// Device name as reported by the device.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.borrow().name.clone()
    }

    /// Number of discovered features.
    #[wasm_bindgen(getter, js_name = featureCount)]
    pub fn feature_count(&self) -> usize {
        self.inner.borrow().features.len()
    }

    /// Get all features as a JSON array of `{id, index, name}`.
    #[wasm_bindgen(js_name = getFeatures)]
    pub fn get_features(&self) -> JsValue {
        let inner = self.inner.borrow();
        let features: Vec<serde_json::Value> = inner
            .features
            .iter()
            .map(|(id, idx)| {
                let name = hidpp::feature_id::feature_name(*id).unwrap_or("Unknown");
                serde_json::json!({
                    "id": format!("0x{:04X}", id.0),
                    "index": idx.0,
                    "name": name,
                })
            })
            .collect();
        serde_wasm_bindgen::to_value(&features).unwrap_or(JsValue::NULL)
    }

    /// Read battery status. Returns `{percentage, level, charging}`.
    #[wasm_bindgen(js_name = getBattery)]
    pub async fn get_battery(&self) -> Result<JsValue, JsValue> {
        let (di, sw, idx) = self.ctx(hidpp::feature_id::UNIFIED_BATTERY)?;
        let req = unified_battery::encode_get_status(di, idx, sw);
        let resp = self.request_report(&req).await?;
        let status = unified_battery::decode_get_status(&resp)
            .map_err(|e| JsValue::from_str(&format!("{e}")))?;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"percentage".into(), &status.percentage.into())?;
        js_sys::Reflect::set(&obj, &"level".into(), &format!("{:?}", status.level).into())?;
        js_sys::Reflect::set(&obj, &"charging".into(), &format!("{:?}", status.charging).into())?;
        js_sys::Reflect::set(&obj, &"externalPower".into(), &status.external_power.into())?;
        Ok(obj.into())
    }

    /// Read current DPI.
    #[wasm_bindgen(js_name = getDpi)]
    pub async fn get_dpi(&self) -> Result<u16, JsValue> {
        let (di, sw, idx) = self.ctx(hidpp::feature_id::ADJUSTABLE_DPI)?;
        let req = adjustable_dpi::encode_get_dpi(di, idx, 0, sw);
        let resp = self.request_report(&req).await?;
        adjustable_dpi::decode_get_dpi(&resp).map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    /// Set DPI. Returns the applied value.
    #[wasm_bindgen(js_name = setDpi)]
    pub async fn set_dpi(&self, dpi: u16) -> Result<u16, JsValue> {
        let (di, sw, idx) = self.ctx(hidpp::feature_id::ADJUSTABLE_DPI)?;
        let req = adjustable_dpi::encode_set_dpi(di, idx, 0, dpi, sw);
        let resp = self.request_report(&req).await?;
        adjustable_dpi::decode_set_dpi(&resp).map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    /// Read SmartShift state. Returns `{mode, autoDisengage, torque}`.
    #[wasm_bindgen(js_name = getSmartShift)]
    pub async fn get_smart_shift(&self) -> Result<JsValue, JsValue> {
        let idx = self.smart_shift_index()?;
        let enhanced = self.has_enhanced_smart_shift();
        let inner = self.inner.borrow();
        let req = if enhanced {
            smart_shift::encode_get_mode_v1(inner.device_index, idx, inner.sw_id)
        } else {
            smart_shift::encode_get_mode_v0(inner.device_index, idx, inner.sw_id)
        };
        drop(inner);
        let resp = self.request_report(&req).await?;
        let state = smart_shift::decode_get_mode(&resp)
            .map_err(|e| JsValue::from_str(&format!("{e}")))?;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"mode".into(), &format!("{:?}", state.mode).into())?;
        js_sys::Reflect::set(&obj, &"autoDisengage".into(), &state.auto_disengage.into())?;
        js_sys::Reflect::set(&obj, &"torque".into(), &state.tunable_torque.into())?;
        Ok(obj.into())
    }

    /// Set SmartShift mode. `mode` is "ratchet" or "freespin".
    #[wasm_bindgen(js_name = setSmartShift)]
    pub async fn set_smart_shift(
        &self,
        mode: &str,
        auto_disengage: u8,
        torque: u8,
    ) -> Result<JsValue, JsValue> {
        let idx = self.smart_shift_index()?;
        let enhanced = self.has_enhanced_smart_shift();
        let wheel_mode = match mode {
            "freespin" | "free" => smart_shift::WheelMode::FreeScroll,
            _ => smart_shift::WheelMode::Ratchet,
        };
        let state = smart_shift::SmartShiftState {
            mode: wheel_mode,
            auto_disengage,
            tunable_torque: torque,
        };
        let inner = self.inner.borrow();
        let req = if enhanced {
            smart_shift::encode_set_mode_v1(inner.device_index, idx, inner.sw_id, &state)
        } else {
            smart_shift::encode_set_mode_v0(inner.device_index, idx, inner.sw_id, &state)
        };
        drop(inner);
        let resp = self.request_report(&req).await?;
        let applied = smart_shift::decode_set_mode(&resp)
            .map_err(|e| JsValue::from_str(&format!("{e}")))?;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"mode".into(), &format!("{:?}", applied.mode).into())?;
        js_sys::Reflect::set(&obj, &"autoDisengage".into(), &applied.auto_disengage.into())?;
        js_sys::Reflect::set(&obj, &"torque".into(), &applied.tunable_torque.into())?;
        Ok(obj.into())
    }

    /// Read host info. Returns `{currentHost, numHosts}`.
    #[wasm_bindgen(js_name = getHostInfo)]
    pub async fn get_host_info(&self) -> Result<JsValue, JsValue> {
        let (di, sw, idx) = self.ctx(hidpp::feature_id::CHANGE_HOST)?;
        let req = change_host::encode_get_host_info(di, idx, sw);
        let resp = self.request_report(&req).await?;
        let info = change_host::decode_get_host_info(&resp)
            .map_err(|e| JsValue::from_str(&format!("{e}")))?;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"currentHost".into(), &info.current_host.into())?;
        js_sys::Reflect::set(&obj, &"numHosts".into(), &info.num_hosts.into())?;
        Ok(obj.into())
    }

    /// Helper to get device context for a feature.
    fn ctx(&self, feature_id: FeatureId) -> Result<(DeviceIndex, SoftwareId, FeatureIndex), JsValue> {
        let idx = self.feature_index(feature_id)?;
        let inner = self.inner.borrow();
        Ok((inner.device_index, inner.sw_id, idx))
    }

    /// Check if device has 0x2111 (enhanced SmartShift) vs 0x2110 (legacy).
    fn has_enhanced_smart_shift(&self) -> bool {
        let inner = self.inner.borrow();
        inner.features.contains_key(&hidpp::feature_id::SMART_SHIFT_TUNABLE_TORQUE)
    }

    /// Get SmartShift feature index (tries 0x2111 then 0x2110).
    fn smart_shift_index(&self) -> Result<FeatureIndex, JsValue> {
        self.feature_index(hidpp::feature_id::SMART_SHIFT_TUNABLE_TORQUE)
            .or_else(|_| self.feature_index(hidpp::feature_id::SMART_SHIFT))
    }
}
