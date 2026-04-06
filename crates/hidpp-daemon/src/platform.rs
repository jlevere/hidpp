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
