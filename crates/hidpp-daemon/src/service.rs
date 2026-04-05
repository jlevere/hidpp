use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if the daemon service is currently installed.
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

/// Install the daemon as a system service.
pub fn install() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    println!("hidppd install");
    println!();

    #[cfg(target_os = "macos")]
    macos::install(&exe)?;

    #[cfg(target_os = "linux")]
    linux::install(&exe)?;

    #[cfg(target_os = "windows")]
    {
        anyhow::bail!("Windows service installation not yet supported. Run hidppd manually.");
    }

    // Create default config if none exists.
    let config_path = crate::config::default_config_path();
    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_path, crate::daemon::SAMPLE_CONFIG)?;
        println!("  config  -> {} (sample)", config_path.display());
    } else {
        println!("  config  -> {} (existing)", config_path.display());
    }

    println!();
    println!("done.");
    Ok(())
}

/// Uninstall the daemon service.
pub fn uninstall() -> anyhow::Result<()> {
    println!("hidppd uninstall");
    println!();

    #[cfg(target_os = "macos")]
    macos::uninstall()?;

    #[cfg(target_os = "linux")]
    linux::uninstall()?;

    #[cfg(target_os = "windows")]
    {
        anyhow::bail!("Windows service uninstallation not yet supported.");
    }

    println!();
    println!("done. config left at ~/.config/hidpp/");
    Ok(())
}

// ── macOS ──────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    const PLIST_LABEL: &str = "com.hidpp.daemon";

    pub(crate) fn plist_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Path::new(&home)
            .join("Library/LaunchAgents")
            .join(format!("{PLIST_LABEL}.plist"))
    }

    fn generate_plist(exe_path: &Path, log_path: &Path) -> String {
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
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
    <key>ProcessType</key>
    <string>Background</string>
    <key>ThrottleInterval</key>
    <integer>5</integer>
</dict>
</plist>"#,
            exe = exe_path.display(),
            log = log_path.display(),
        )
    }

    pub fn install(exe: &Path) -> anyhow::Result<()> {
        let home = std::env::var("HOME")?;
        let log_path = Path::new(&home).join("Library/Logs/hidppd.log");
        let plist = plist_path();

        // Unload existing if present.
        if plist.exists() {
            let _ = Command::new("launchctl")
                .args(["unload", &plist.to_string_lossy()])
                .output();
        }

        // Ensure directories exist.
        if let Some(parent) = plist.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(log_path.parent().unwrap_or(Path::new("/tmp")))?;

        // Write the plist with the actual binary path baked in.
        let content = generate_plist(exe, &log_path);
        std::fs::write(&plist, content)?;
        println!("  plist   -> {}", plist.display());

        // Load.
        let output = Command::new("launchctl")
            .args(["load", &plist.to_string_lossy()])
            .output()?;

        if output.status.success() {
            println!("  service -> loaded");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("  service -> load failed: {stderr}");
        }

        println!();
        println!("  logs: tail -f {}", log_path.display());

        // Remind about Accessibility.
        println!();
        if exe.to_string_lossy().contains(".app/") {
            println!("  Grant Accessibility to the .app bundle:");
        } else {
            println!("  Grant Accessibility to hidppd:");
        }
        println!("    System Settings -> Privacy & Security -> Accessibility");

        Ok(())
    }

    pub fn uninstall() -> anyhow::Result<()> {
        let plist = plist_path();

        if plist.exists() {
            let _ = Command::new("launchctl")
                .args(["unload", &plist.to_string_lossy()])
                .output();
            std::fs::remove_file(&plist)?;
            println!("  plist   -> removed");
        } else {
            println!("  plist   -> not installed");
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
Description=HID++ 2.0 device daemon
Documentation=https://github.com/jlevere/logi-re
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

    pub fn install(exe: &Path) -> anyhow::Result<()> {
        let unit = service_path();

        // Stop existing.
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "hidppd.service"])
            .output();

        // Write unit file.
        if let Some(parent) = unit.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&unit, generate_unit(exe))?;
        println!("  unit    -> {}", unit.display());

        // Reload, enable, start.
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
        let _ = Command::new("systemctl")
            .args(["--user", "enable", "hidppd.service"])
            .output();
        let output = Command::new("systemctl")
            .args(["--user", "start", "hidppd.service"])
            .output()?;

        if output.status.success() {
            println!("  service -> started");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("  service -> start failed: {stderr}");
        }

        println!();
        println!("  logs: journalctl --user -u hidppd -f");

        // Remind about udev rules.
        println!();
        println!("  For HID access without root, install udev rules:");
        println!("    sudo cp udev/99-hidpp.rules /etc/udev/rules.d/");
        println!("    sudo udevadm control --reload-rules");

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
            println!("  unit    -> removed");
        } else {
            println!("  unit    -> not installed");
        }

        Ok(())
    }
}
