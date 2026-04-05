use hidpp::report::LongReport;
use std::pin::Pin;

/// Errors from the transport layer.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("device not found")]
    DeviceNotFound,
    #[error("i/o error: {0}")]
    Io(String),
    #[error("timeout waiting for response")]
    Timeout,
    #[error("device disconnected")]
    Disconnected,
    #[error("HID++ error: {0}")]
    Hidpp(#[from] hidpp::error::HidppError),
}

/// Information about a discovered HID device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: Option<String>,
    pub path: String,
}

/// Async transport for sending/receiving HID++ reports.
#[async_trait::async_trait(?Send)]
pub trait Transport {
    /// Send a request and wait for the matching response.
    async fn request(&self, report: &LongReport) -> Result<LongReport, TransportError>;

    /// Send a report without waiting for a response.
    async fn send(&self, report: &LongReport) -> Result<(), TransportError>;

    /// Subscribe to unsolicited notifications from the device.
    fn notifications(&self) -> Pin<Box<dyn futures_core::Stream<Item = LongReport> + '_>>;
}

/// Enumerate and open HID++ devices.
#[async_trait::async_trait(?Send)]
pub trait Enumerator {
    /// List connected HID++ devices.
    async fn enumerate(&self) -> Result<Vec<DeviceInfo>, TransportError>;

    /// Open a transport to a specific device.
    async fn open(&self, info: &DeviceInfo) -> Result<Box<dyn Transport>, TransportError>;
}

#[cfg(feature = "native")]
pub mod native;
