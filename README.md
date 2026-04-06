# HID++

Open-source replacement for Logitech Options+ — configure Logitech mice, keyboards, and trackballs from Rust, the browser, or a native menu bar app.

## What it does

- **Web UI** — connect to any Logitech HID++ device from Chrome/Edge via WebHID. Configure DPI, scroll mode, button remaps, Easy-Switch hosts. Supports 170+ devices in demo/browse mode.
- **Menu bar app** (macOS) — background daemon with gesture support. Hold a button + swipe to trigger keystrokes. Shows battery %, DPI, and last action in the menu bar.
- **CLI** — one-shot commands for scripting: `hidpp info`, `hidpp get dpi`, `hidpp set dpi 1600`, `hidpp export`.
- **Rust library** — full HID++ 2.0 protocol implementation with 15 features, 70+ unit tests.

## Quick start

### Web UI (no install)

Visit the [web configurator](https://jlevere.github.io/logi-re/) in Chrome or Edge. Click Connect, pick your device.

### macOS app

```sh
# Build with Nix
nix build .#dmg
open result/HID++.dmg
# Drag to Applications, then:
/Applications/HID++.app/Contents/MacOS/hidppd install
```

Or from source:
```sh
nix develop
cargo run -p hidpp-daemon
```

### CLI

```sh
cargo run -p hidpp-cli -- info
cargo run -p hidpp-cli -- get battery
cargo run -p hidpp-cli -- set dpi 1600
```

## Daemon config

Edit `~/.config/hidpp/config.toml`:

```toml
[buttons]
83 = "alt+left"       # Back → browser back
86 = "alt+right"      # Forward → browser forward

[gestures.195]        # Gesture button (thumb)
up = "ctrl+up"        # Swipe up → Mission Control
down = "ctrl+down"    # Swipe down → App Exposé
left = "ctrl+left"    # Swipe left → prev desktop
right = "ctrl+right"  # Swipe right → next desktop
tap = "playpause"     # Quick tap → play/pause
```

Generate a sample: `hidppd sample-config`

## Project structure

```
crates/
  hidpp/            — HID++ 2.0 protocol codec (pure Rust, no I/O)
  hidpp-transport/  — HID I/O via hidapi (macOS/Linux/Windows)
  hidpp-device/     — device session, feature discovery, typed API
  hidpp-cli/        — CLI tool (hidpp)
  hidpp-daemon/     — menu bar app with gesture support (hidppd)
  hidpp-web/        — WASM module for WebHID browser access
web/                — web UI (TypeScript + Vite)
```

## Supported devices

Any Logitech device speaking HID++ 2.0 over BLE or USB receiver (Bolt/Unifying/Lightspeed). Tested on MX Master 3S. The device database includes 170+ devices.

## Building

Requires [Nix](https://nixos.org/) with flakes enabled:

```sh
nix develop                          # dev shell with all tools
nix build .#app                      # macOS .app bundle
nix build .#dmg                      # macOS DMG
nix build .#daemon                   # daemon binary only
nix build .#cli                      # CLI binary only
nix flake check                      # build + clippy + tests
```

Or with plain cargo (needs hidapi system library):
```sh
cargo build --workspace --exclude hidpp-web
cargo test --workspace --exclude hidpp-web
```

## Cross-platform

| Platform | CLI | Daemon | Web UI |
|----------|-----|--------|--------|
| macOS    | ✓   | ✓ (menu bar app) | ✓ |
| Linux    | ✓   | ✓ (systemd service) | ✓ |
| Windows  | ✓   | ✓ (headless) | ✓ |

Linux users: install udev rules for non-root HID access:
```sh
sudo cp udev/99-hidpp.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
```

## License

MIT OR Apache-2.0
