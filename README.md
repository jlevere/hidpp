# HID++

Open-source replacement for Logitech Options+ — configure Logitech mice, keyboards, and trackballs from the browser or a native macOS menu bar app.

## Install (macOS)

1. Download `hidpp-x.x.x-macos-arm64.dmg` from [Releases](https://github.com/jlevere/hidpp/releases)
2. Open the DMG, drag **HID++** to Applications
3. Open **HID++** from Applications
4. Grant Accessibility permission when prompted (System Settings → Privacy & Security → Accessibility)
5. Click the mouse icon in the menu bar → **Start at Login**

That's it. Battery %, DPI, and gesture actions show in the menu bar.

## Web UI

Configure any Logitech HID++ device from Chrome or Edge — no install needed:

**[Open Web Configurator →](https://jlevere.github.io/hidpp/)**

Supports DPI, scroll mode, button remaps, Easy-Switch hosts. 170+ devices in browse/demo mode.

## Config

Edit `~/.config/hidpp/config.toml` (created on first launch):

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

Or use the web UI's config editor and click **Apply to App** to push changes.

## CLI

For scripting and one-shot configuration:

```sh
hidpp info                    # device info + all settings
hidpp get battery             # battery percentage
hidpp set dpi 1600            # set DPI
hidpp export                  # export config as TOML
```

## Supported devices

Any Logitech device speaking HID++ 2.0 over BLE or USB receiver (Bolt / Unifying / Lightspeed). Tested on MX Master 3S. Device database includes 170+ devices.

## Building from source

Requires [Nix](https://nixos.org/) with flakes:

```sh
nix build .#dmg               # macOS DMG with .app bundle
nix build .#app               # just the .app
nix build .#cli               # CLI binary
nix flake check               # build + clippy + tests
```

Or with cargo (needs system hidapi):

```sh
cargo build --workspace --exclude hidpp-web
cargo test --workspace --exclude hidpp-web
```

## Platform support

| | macOS | Linux | Windows |
|---|---|---|---|
| Menu bar app | ✓ | planned | — |
| CLI | ✓ | ✓ | ✓ |
| Web UI | ✓ | ✓ | ✓ |

Linux: install `udev/99-hidpp.rules` for non-root HID access.

## Project structure

```
crates/
  hidpp/            — HID++ 2.0 protocol (pure Rust, no I/O)
  hidpp-transport/  — HID I/O via hidapi
  hidpp-device/     — device session + feature discovery
  hidpp-cli/        — CLI tool
  hidpp-daemon/     — menu bar app
  hidpp-web/        — WASM module for WebHID
web/                — web UI (TypeScript + Vite)
```

## License

MIT OR Apache-2.0
