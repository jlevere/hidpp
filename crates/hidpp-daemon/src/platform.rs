//! Platform-specific system event detection.
//!
//! On macOS, uses the `notify(3)` API for power state changes and
//! `IOHIDManager` for HID device arrival/removal. Both signal through
//! mpsc channels — zero polling, pure event-driven.

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod imp {
    use std::ffi::CString;
    use std::ffi::c_void;
    use std::io::Read;
    use std::os::fd::FromRawFd;

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    unsafe extern "C" {
        fn notify_register_file_descriptor(
            name: *const std::ffi::c_char,
            notify_fd: *mut i32,
            flags: i32,
            out_token: *mut i32,
        ) -> u32;
        fn notify_cancel(token: i32) -> u32;

        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const u8,
            encoding: u32,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);
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
            tracing::warn!("failed to register for power state notifications (status={status})");
            return;
        }

        // File takes ownership of the fd and closes it on drop.
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };

        let res = std::thread::Builder::new()
            .name("wake-watcher".into())
            .spawn(move || {
                // Each notification writes a 4-byte token to the fd.
                let mut buf = [0u8; 4];
                while file.read_exact(&mut buf).is_ok() {
                    if tx.blocking_send(()).is_err() {
                        break; // Receiver dropped — daemon shutting down.
                    }
                }
                unsafe { notify_cancel(token) };
            });

        if let Err(e) = res {
            tracing::warn!("failed to spawn wake-watcher thread: {e}");
        }
    }

    /// Watch for Logitech HID device arrival/removal via `IOHIDManager`.
    ///
    /// When a matching device appears or disappears, sends `()` on the
    /// channel so the daemon can reconnect and re-divert buttons.
    pub fn spawn_hid_watcher(tx: tokio::sync::mpsc::Sender<()>) {
        const LOGITECH_VID: i32 = 0x046D;
        const K_CF_NUMBER_INT_TYPE: u64 = 9; // kCFNumberIntType

        unsafe extern "C" {
            fn IOHIDManagerCreate(allocator: *const c_void, options: u32) -> *mut c_void;
            fn IOHIDManagerSetDeviceMatching(manager: *mut c_void, matching: *const c_void);
            fn IOHIDManagerRegisterDeviceMatchingCallback(
                manager: *mut c_void,
                callback: unsafe extern "C" fn(*mut c_void, i32, *mut c_void, *mut c_void),
                context: *mut c_void,
            );
            fn IOHIDManagerRegisterDeviceRemovalCallback(
                manager: *mut c_void,
                callback: unsafe extern "C" fn(*mut c_void, i32, *mut c_void, *mut c_void),
                context: *mut c_void,
            );
            fn IOHIDManagerScheduleWithRunLoop(
                manager: *mut c_void,
                run_loop: *mut c_void,
                mode: *const c_void,
            );
            fn IOHIDManagerOpen(manager: *mut c_void, options: u32) -> i32;
            fn CFNumberCreate(
                allocator: *const c_void,
                the_type: u64,
                value_ptr: *const c_void,
            ) -> *const c_void;
            fn CFDictionaryCreate(
                allocator: *const c_void,
                keys: *const *const c_void,
                values: *const *const c_void,
                num_values: isize,
                key_callbacks: *const c_void,
                value_callbacks: *const c_void,
            ) -> *const c_void;
            fn CFRunLoopGetCurrent() -> *mut c_void;
            fn CFRunLoopRun();

            static kCFTypeDictionaryKeyCallBacks: c_void;
            static kCFTypeDictionaryValueCallBacks: c_void;
            static kCFRunLoopDefaultMode: *const c_void;
        }

        unsafe extern "C" fn device_matched(
            context: *mut c_void,
            _result: i32,
            _sender: *mut c_void,
            _device: *mut c_void,
        ) {
            let tx = unsafe { &*(context as *const tokio::sync::mpsc::Sender<()>) };
            tracing::info!("HID device matched (arrival)");
            let _ = tx.blocking_send(());
        }

        unsafe extern "C" fn device_removed(
            context: *mut c_void,
            _result: i32,
            _sender: *mut c_void,
            _device: *mut c_void,
        ) {
            let tx = unsafe { &*(context as *const tokio::sync::mpsc::Sender<()>) };
            tracing::info!("HID device removed");
            let _ = tx.blocking_send(());
        }

        let res = std::thread::Builder::new()
            .name("hid-watcher".into())
            .spawn(move || {
                let manager = unsafe { IOHIDManagerCreate(std::ptr::null(), 0) };
                if manager.is_null() {
                    tracing::warn!("failed to create IOHIDManager");
                    return;
                }

                // Build matching dictionary: { "VendorID": 0x046D }
                let vid = LOGITECH_VID;
                let cf_vid = unsafe {
                    CFNumberCreate(
                        std::ptr::null(),
                        K_CF_NUMBER_INT_TYPE,
                        &vid as *const i32 as *const c_void,
                    )
                };
                let cf_key = unsafe {
                    CFStringCreateWithCString(
                        std::ptr::null(),
                        c"VendorID".as_ptr().cast(),
                        K_CF_STRING_ENCODING_UTF8,
                    )
                };

                if cf_vid.is_null() || cf_key.is_null() {
                    tracing::warn!("failed to create matching dictionary values");
                    unsafe { CFRelease(manager) };
                    return;
                }

                let keys = [cf_key];
                let values = [cf_vid];
                let dict = unsafe {
                    CFDictionaryCreate(
                        std::ptr::null(),
                        keys.as_ptr(),
                        values.as_ptr(),
                        1,
                        &kCFTypeDictionaryKeyCallBacks as *const c_void,
                        &kCFTypeDictionaryValueCallBacks as *const c_void,
                    )
                };
                unsafe {
                    CFRelease(cf_key);
                    CFRelease(cf_vid);
                }

                if dict.is_null() {
                    tracing::warn!("failed to create matching dictionary");
                    unsafe { CFRelease(manager) };
                    return;
                }

                unsafe { IOHIDManagerSetDeviceMatching(manager, dict) };
                unsafe { CFRelease(dict) };

                // Leak the sender so the raw pointer stays valid for the process lifetime.
                let tx_ptr = Box::into_raw(Box::new(tx)) as *mut c_void;

                unsafe {
                    IOHIDManagerRegisterDeviceMatchingCallback(manager, device_matched, tx_ptr);
                    IOHIDManagerRegisterDeviceRemovalCallback(manager, device_removed, tx_ptr);
                }

                // Schedule on this thread's CFRunLoop (available since macOS 10.5,
                // no activation lifecycle — works reliably under launchd).
                let run_loop = unsafe { CFRunLoopGetCurrent() };
                unsafe {
                    IOHIDManagerScheduleWithRunLoop(manager, run_loop, kCFRunLoopDefaultMode);
                }

                let status = unsafe { IOHIDManagerOpen(manager, 0) };
                if status != 0 {
                    tracing::warn!("IOHIDManagerOpen failed (status={status})");
                    unsafe { CFRelease(manager) };
                    return;
                }

                tracing::debug!("HID device watcher active");

                // Block on the run loop — callbacks fire on this thread.
                unsafe { CFRunLoopRun() };
            });

        if let Err(e) = res {
            tracing::warn!("failed to spawn hid-watcher thread: {e}");
        }
    }

    // ── Power assertion (prevent sleep during active HID operations) ──

    /// RAII guard that holds a macOS IOPMAssertion to prevent system idle sleep
    /// while the device is being configured (connect, feature discovery, divert).
    /// Automatically released on drop.
    pub struct PowerAssertion {
        assertion_id: u32,
    }

    impl PowerAssertion {
        /// Create a power assertion that prevents idle sleep.
        /// Returns None if the assertion couldn't be created (non-fatal).
        pub fn prevent_idle_sleep(reason: &str) -> Option<Self> {
            unsafe extern "C" {
                fn IOPMAssertionCreateWithName(
                    assertion_type: *const c_void,
                    level: u32,
                    name: *const c_void,
                    assertion_id: *mut u32,
                ) -> i32;
            }

            const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;

            let reason_c = std::ffi::CString::new(reason).ok()?;

            let cf_type = unsafe {
                CFStringCreateWithCString(
                    std::ptr::null(),
                    c"PreventUserIdleSystemSleep".as_ptr().cast(),
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
}

#[cfg(not(target_os = "macos"))]
mod imp {
    /// No-op on non-macOS.
    pub fn spawn_wake_watcher(_tx: tokio::sync::mpsc::Sender<()>) {}
    pub fn spawn_hid_watcher(_tx: tokio::sync::mpsc::Sender<()>) {}

    pub struct PowerAssertion;
    impl PowerAssertion {
        pub fn prevent_idle_sleep(_reason: &str) -> Option<Self> {
            Some(Self)
        }
    }
}

pub use imp::PowerAssertion;
pub use imp::spawn_hid_watcher;
pub use imp::spawn_wake_watcher;
