/// Platform-specific system wake detection.
///
/// On macOS, uses the `notify(3)` API to get a file descriptor that fires
/// on power state changes. A background thread blocks on `read()` and
/// signals through an mpsc channel — zero polling, pure fd-based events.

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod imp {
    use std::ffi::CString;
    use std::io::Read;
    use std::os::fd::FromRawFd;

    unsafe extern "C" {
        fn notify_register_file_descriptor(
            name: *const std::ffi::c_char,
            notify_fd: *mut i32,
            flags: i32,
            out_token: *mut i32,
        ) -> u32;
        fn notify_cancel(token: i32) -> u32;
    }

    /// Spawn a thread that blocks on a Darwin `notify(3)` fd for
    /// `com.apple.system.powermanagement.systempowerstate`. Sends `()`
    /// on the channel whenever power state changes (sleep ↔ wake).
    pub fn spawn_wake_watcher(tx: tokio::sync::mpsc::Sender<()>) {
        let name = CString::new("com.apple.system.powermanagement.systempowerstate")
            .expect("static CString");
        let mut fd: i32 = -1;
        let mut token: i32 = 0;

        let status =
            unsafe { notify_register_file_descriptor(name.as_ptr(), &mut fd, 0, &mut token) };

        if status != 0 || fd < 0 {
            tracing::warn!(
                "failed to register for power state notifications (status={status})"
            );
            return;
        }

        // File takes ownership of the fd and closes it on drop.
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };

        let res = std::thread::Builder::new()
            .name("wake-watcher".into())
            .spawn(move || {
                // Each notification writes a 4-byte token to the fd.
                let mut buf = [0u8; 4];
                loop {
                    match file.read_exact(&mut buf) {
                        Ok(()) => {
                            if tx.blocking_send(()).is_err() {
                                break; // Receiver dropped — daemon shutting down.
                            }
                        }
                        Err(_) => break,
                    }
                }
                unsafe { notify_cancel(token) };
            });

        if let Err(e) = res {
            tracing::warn!("failed to spawn wake-watcher thread: {e}");
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    /// No-op on non-macOS. Linux hidapi reports USB device removal via
    /// read errors, so the existing disconnect detection is sufficient.
    pub fn spawn_wake_watcher(_tx: tokio::sync::mpsc::Sender<()>) {}
}

pub use imp::spawn_wake_watcher;

// ── Power assertion (prevent sleep during active HID operations) ──

/// RAII guard that holds a macOS IOPMAssertion to prevent system idle sleep
/// while the device is being configured (connect, feature discovery, divert).
/// Automatically released on drop.
#[cfg(target_os = "macos")]
pub struct PowerAssertion {
    assertion_id: u32,
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
impl PowerAssertion {
    /// Create a power assertion that prevents idle sleep.
    /// Returns None if the assertion couldn't be created (non-fatal).
    pub fn prevent_idle_sleep(reason: &str) -> Option<Self> {
        use std::ffi::c_void;

        unsafe extern "C" {
            fn IOPMAssertionCreateWithName(
                assertion_type: *const c_void,
                level: u32,
                name: *const c_void,
                assertion_id: *mut u32,
            ) -> i32;
            fn CFStringCreateWithCString(
                alloc: *const c_void,
                c_str: *const u8,
                encoding: u32,
            ) -> *const c_void;
            fn CFRelease(cf: *const c_void);
        }

        const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
        const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;

        let type_str = b"PreventUserIdleSystemSleep\0";
        let reason_c = std::ffi::CString::new(reason).ok()?;

        let cf_type = unsafe {
            CFStringCreateWithCString(
                std::ptr::null(),
                type_str.as_ptr(),
                K_CF_STRING_ENCODING_UTF8,
            )
        };
        let cf_reason = unsafe {
            CFStringCreateWithCString(
                std::ptr::null(),
                reason_c.as_ptr().cast(),
                K_CF_STRING_ENCODING_UTF8,
            )
        };

        if cf_type.is_null() || cf_reason.is_null() {
            return None;
        }

        let mut assertion_id: u32 = 0;
        let result = unsafe {
            IOPMAssertionCreateWithName(
                cf_type,
                K_IOPM_ASSERTION_LEVEL_ON,
                cf_reason,
                &mut assertion_id,
            )
        };

        unsafe {
            CFRelease(cf_type);
            CFRelease(cf_reason);
        }

        if result == 0 {
            Some(Self { assertion_id })
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
impl Drop for PowerAssertion {
    fn drop(&mut self) {
        unsafe extern "C" {
            fn IOPMAssertionRelease(assertion_id: u32) -> i32;
        }
        unsafe {
            IOPMAssertionRelease(self.assertion_id);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct PowerAssertion;

#[cfg(not(target_os = "macos"))]
impl PowerAssertion {
    pub fn prevent_idle_sleep(_reason: &str) -> Option<Self> {
        Some(Self)
    }
}
