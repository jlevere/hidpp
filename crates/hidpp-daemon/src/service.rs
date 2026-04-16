use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if the login item is currently registered.
pub fn is_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::plist_path().exists()
    }
    #[cfg(target_os = "linux")]
    {
        linux::service_path().exists()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

/// Register the app to start on login (writes plist/unit, does NOT launch).
pub fn register_login_item() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;

    #[cfg(target_os = "macos")]
    macos::register(&exe)?;

    #[cfg(target_os = "linux")]
    linux::register(&exe)?;

    #[cfg(target_os = "windows")]
    {
        let _ = exe;
        anyhow::bail!("not yet supported on Windows");
    }

    Ok(())
}

/// Unregister the login item.
pub fn uninstall() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    macos::uninstall()?;

    #[cfg(target_os = "linux")]
    linux::uninstall()?;

    #[cfg(target_os = "windows")]
    {
        anyhow::bail!("not yet supported on Windows");
    }

    Ok(())
}

// ── macOS ──────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    const PLIST_LABEL: &str = "com.jlevere.hidpp";

    pub(crate) fn plist_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Path::new(&home)
            .join("Library/LaunchAgents")
            .join(format!("{PLIST_LABEL}.plist"))
    }

    fn generate_plist(exe_path: &Path) -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let log_path = Path::new(&home).join("Library/Logs/hidppd.log");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{PLIST_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>5</integer>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>"#,
            exe = exe_path.display(),
            log = log_path.display(),
        )
    }

    /// Write the launchd plist without loading it.
    pub fn register(exe: &Path) -> anyhow::Result<()> {
        let plist = plist_path();

        if let Some(parent) = plist.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&plist, generate_plist(exe))?;
        Ok(())
    }

    pub fn uninstall() -> anyhow::Result<()> {
        let plist = plist_path();
        if plist.exists() {
            let _ = Command::new("launchctl")
                .args(["unload", &plist.to_string_lossy()])
                .output();
            std::fs::remove_file(&plist)?;
        }
        Ok(())
    }
}

// ── Linux ──────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub(crate) fn service_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Path::new(&home)
            .join(".config/systemd/user")
            .join("hidppd.service")
    }

    fn generate_unit(exe_path: &Path) -> String {
        format!(
            r#"[Unit]
Description=HID++ device daemon
Documentation=https://github.com/jlevere/hidpp
After=graphical-session.target

[Service]
Type=simple
ExecStart={exe}
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=default.target"#,
            exe = exe_path.display(),
        )
    }

    pub fn register(exe: &Path) -> anyhow::Result<()> {
        let unit = service_path();
        if let Some(parent) = unit.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&unit, generate_unit(exe))?;
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
        let _ = Command::new("systemctl")
            .args(["--user", "enable", "hidppd.service"])
            .output();
        Ok(())
    }

    pub fn uninstall() -> anyhow::Result<()> {
        let unit = service_path();
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "hidppd.service"])
            .output();
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "hidppd.service"])
            .output();
        if unit.exists() {
            std::fs::remove_file(&unit)?;
            let _ = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .output();
        }
        Ok(())
    }
}
