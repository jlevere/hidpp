use std::sync::Arc;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};
use tokio::sync::{broadcast, mpsc, Mutex};

use hidpp::report::{LongReport, REPORT_ID_LONG};
use hidpp::types::{FeatureIndex, FunctionId, SoftwareId};

use crate::{DeviceInfo, TransportError};

/// Logitech vendor ID.
const LOGITECH_VID: u16 = 0x046D;

/// HID++ vendor-specific usage pages.
const HIDPP_USAGE_PAGE_FF43: u16 = 0xFF43;
const HIDPP_USAGE_PAGE_FF00: u16 = 0xFF00;

/// Default timeout for waiting on a response (ms).
const RESPONSE_TIMEOUT_MS: i32 = 2000;

/// Enumerate connected Logitech HID++ devices.
pub struct HidapiEnumerator {
    api: HidApi,
}

impl HidapiEnumerator {
    pub fn new() -> Result<Self, TransportError> {
        let api = HidApi::new().map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(Self { api })
    }

    /// List all connected Logitech HID++ devices.
    ///
    /// Filters for Logitech VID and vendor-specific usage pages
    /// (`0xFF43` or `0xFF00`) which carry HID++ protocol traffic.
    pub fn enumerate(&self) -> Vec<DeviceInfo> {
        self.api
            .device_list()
            .filter(|dev| {
                dev.vendor_id() == LOGITECH_VID
                    && (dev.usage_page() == HIDPP_USAGE_PAGE_FF43
                        || dev.usage_page() == HIDPP_USAGE_PAGE_FF00)
            })
            .map(|dev| DeviceInfo {
                vendor_id: dev.vendor_id(),
                product_id: dev.product_id(),
                name: dev.product_string().map(String::from),
                path: dev.path().to_string_lossy().into_owned(),
            })
            .collect()
    }

    /// Open a transport to a specific device by path.
    pub fn open(&self, info: &DeviceInfo) -> Result<HidapiTransport, TransportError> {
        let c_path =
            std::ffi::CString::new(info.path.as_bytes()).map_err(|e| TransportError::Io(e.to_string()))?;

        let device = self
            .api
            .open_path(&c_path)
            .map_err(|e| TransportError::Io(format!("failed to open {}: {e}", info.path)))?;

        // Non-blocking mode — we poll with timeouts in the reader task.
        device
            .set_blocking_mode(false)
            .map_err(|e| TransportError::Io(e.to_string()))?;

        HidapiTransport::new(device)
    }
}

/// Pending request waiting for a matching response.
struct PendingRequest {
    feature_index: FeatureIndex,
    function_id: FunctionId,
    sw_id: SoftwareId,
    response_tx: tokio::sync::oneshot::Sender<LongReport>,
}

impl PendingRequest {
    /// Check if an incoming report matches this pending request.
    fn matches(&self, report: &LongReport) -> bool {
        report.feature_index() == self.feature_index
            && report.function_id() == self.function_id
            && report.sw_id() == self.sw_id
    }
}

/// Native HID transport using `hidapi`.
///
/// Spawns a background reader task that demuxes incoming reports:
/// - Responses matching pending requests are routed to the requester.
/// - Unsolicited notifications are broadcast to all subscribers.
pub struct HidapiTransport {
    device: Arc<Mutex<HidDevice>>,
    pending_tx: mpsc::UnboundedSender<PendingRequest>,
    notification_tx: broadcast::Sender<LongReport>,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl HidapiTransport {
    fn new(device: HidDevice) -> Result<Self, TransportError> {
        let device = Arc::new(Mutex::new(device));
        let (pending_tx, pending_rx) = mpsc::unbounded_channel();
        let (notification_tx, _) = broadcast::channel(64);

        let reader_handle = {
            let device = Arc::clone(&device);
            let notification_tx = notification_tx.clone();
            tokio::spawn(Self::reader_loop(device, pending_rx, notification_tx))
        };

        Ok(Self {
            device,
            pending_tx,
            notification_tx,
            _reader_handle: reader_handle,
        })
    }

    /// Background reader task. Reads HID reports and dispatches them.
    async fn reader_loop(
        device: Arc<Mutex<HidDevice>>,
        mut pending_rx: mpsc::UnboundedReceiver<PendingRequest>,
        notification_tx: broadcast::Sender<LongReport>,
    ) {
        let mut buf = [0u8; 64];
        let mut pending: Vec<PendingRequest> = Vec::new();

        loop {
            // Drain any newly registered pending requests.
            while let Ok(req) = pending_rx.try_recv() {
                pending.push(req);
            }

            // Try to read a report (non-blocking, short poll).
            let read_result = {
                let dev = device.lock().await;
                dev.read_timeout(&mut buf, 10)
            };

            match read_result {
                Ok(0) => {
                    // No data available. Yield to avoid busy-spinning.
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Ok(n) if n >= 4 => {
                    // Prepend report ID if the OS stripped it.
                    // macOS hidapi does NOT include the report ID in read().
                    // We always work with 20-byte long reports.
                    let report_bytes = if n == 19 {
                        // macOS: report ID stripped, 19 bytes of data.
                        let mut full = [0u8; 20];
                        full[0] = REPORT_ID_LONG;
                        full[1..20].copy_from_slice(&buf[..19]);
                        full
                    } else if n >= 20 && buf[0] == REPORT_ID_LONG {
                        let mut full = [0u8; 20];
                        full.copy_from_slice(&buf[..20]);
                        full
                    } else {
                        // Unknown format. Skip.
                        continue;
                    };

                    let Some(report) = LongReport::from_bytes(&report_bytes) else {
                        continue;
                    };

                    // Check if this is an error response (feature_index == 0xFF).
                    // Error responses match by the original feature index in byte 3.
                    if report.is_error() {
                        let orig_feature_index = FeatureIndex(report.as_bytes()[3]);
                        if let Some(idx) = pending
                            .iter()
                            .position(|p| p.feature_index == orig_feature_index)
                        {
                            let req = pending.swap_remove(idx);
                            let _ = req.response_tx.send(report);
                            continue;
                        }
                    }

                    // Try to match against pending requests.
                    if let Some(idx) = pending.iter().position(|p| p.matches(&report)) {
                        let req = pending.swap_remove(idx);
                        let _ = req.response_tx.send(report);
                    } else {
                        // No match — this is a notification.
                        let _ = notification_tx.send(report);
                    }
                }
                Ok(_) => {
                    // Too short to be a valid report.
                }
                Err(_) => {
                    // Read error. Brief pause before retry.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }

    /// Send a request and wait for the matching response.
    pub async fn request(&self, report: &LongReport) -> Result<LongReport, TransportError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Register the pending request before sending.
        let pending = PendingRequest {
            feature_index: report.feature_index(),
            function_id: report.function_id(),
            sw_id: report.sw_id(),
            response_tx,
        };
        self.pending_tx
            .send(pending)
            .map_err(|_| TransportError::Disconnected)?;

        // Send the report.
        self.send(report).await?;

        // Wait for response with timeout.
        match tokio::time::timeout(
            Duration::from_millis(RESPONSE_TIMEOUT_MS as u64),
            response_rx,
        )
        .await
        {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(TransportError::Disconnected),
            Err(_) => Err(TransportError::Timeout),
        }
    }

    /// Send a report without waiting for a response.
    pub async fn send(&self, report: &LongReport) -> Result<(), TransportError> {
        let dev = self.device.lock().await;
        dev.write(report.as_ref())
            .map_err(|e| TransportError::Io(format!("write failed: {e}")))?;
        Ok(())
    }

    /// Subscribe to unsolicited notifications from the device.
    pub fn subscribe(&self) -> broadcast::Receiver<LongReport> {
        self.notification_tx.subscribe()
    }
}
