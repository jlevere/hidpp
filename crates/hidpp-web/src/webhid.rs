/// WebHID API bindings via wasm-bindgen.
///
/// The WebHID API is not yet in web-sys, so we define the bindings ourselves.
/// Only the subset we need for HID++ communication is bound.
///
/// Reference: https://wicg.github.io/webhid/
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // --- HID (navigator.hid) ---

    /// The HID interface — accessed via `navigator.hid`.
    #[wasm_bindgen(js_name = HID)]
    pub type Hid;

    /// Request access to a HID device. Returns a `Promise<HIDDevice[]>`.
    #[wasm_bindgen(method, js_name = requestDevice)]
    pub fn request_device(this: &Hid, options: &JsValue) -> js_sys::Promise;

    /// Get already-granted devices. Returns `Promise<HIDDevice[]>`.
    #[wasm_bindgen(method, js_name = getDevices)]
    pub fn get_devices(this: &Hid) -> js_sys::Promise;

    // --- HIDDevice ---

    /// A WebHID device handle.
    #[wasm_bindgen(js_name = HIDDevice)]
    pub type HidDevice;

    /// Whether the device connection is open.
    #[wasm_bindgen(method, getter)]
    pub fn opened(this: &HidDevice) -> bool;

    /// Vendor ID.
    #[wasm_bindgen(method, getter, js_name = vendorId)]
    pub fn vendor_id(this: &HidDevice) -> u16;

    /// Product ID.
    #[wasm_bindgen(method, getter, js_name = productId)]
    pub fn product_id(this: &HidDevice) -> u16;

    /// Product name string.
    #[wasm_bindgen(method, getter, js_name = productName)]
    pub fn product_name(this: &HidDevice) -> String;

    /// Open the device. Returns `Promise<void>`.
    #[wasm_bindgen(method)]
    pub fn open(this: &HidDevice) -> js_sys::Promise;

    /// Close the device. Returns `Promise<void>`.
    #[wasm_bindgen(method)]
    pub fn close(this: &HidDevice) -> js_sys::Promise;

    /// Send an output report. Returns `Promise<void>`.
    ///
    /// `report_id` is the HID report ID (0x11 for HID++ long).
    /// `data` is a `Uint8Array` of the report payload (without the report ID byte).
    #[wasm_bindgen(method, js_name = sendReport)]
    pub fn send_report(this: &HidDevice, report_id: u8, data: &js_sys::Uint8Array) -> js_sys::Promise;

    /// Register an event listener for input reports.
    #[wasm_bindgen(method, js_name = addEventListener)]
    pub fn add_event_listener(this: &HidDevice, event: &str, callback: &Closure<dyn FnMut(JsValue)>);

    // --- HIDInputReportEvent ---

    /// Event fired when an input report is received.
    #[wasm_bindgen(js_name = HIDInputReportEvent)]
    pub type HidInputReportEvent;

    /// The report ID of the received report.
    #[wasm_bindgen(method, getter, js_name = reportId)]
    pub fn report_id(this: &HidInputReportEvent) -> u8;

    /// The report data as a DataView.
    #[wasm_bindgen(method, getter)]
    pub fn data(this: &HidInputReportEvent) -> js_sys::DataView;
}

/// Get `navigator.hid`. Returns `None` if WebHID is not available.
pub fn get_hid() -> Option<Hid> {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &"navigator".into()).ok()?;
    let hid = js_sys::Reflect::get(&navigator, &"hid".into()).ok()?;
    if hid.is_undefined() {
        return None;
    }
    Some(hid.unchecked_into())
}
