# HID++

Configure Logitech devices without Logitech Options+. Open source, runs on macOS, Linux, and Windows.

Supports 170+ Logitech devices over Bluetooth LE and USB receivers (Bolt, Unifying, Lightspeed).

## macOS

Download the DMG from [Releases](https://github.com/jlevere/hidpp/releases), drag HID++ to Applications, then:

```
xattr -cr /Applications/HID++.app
```

Open the app. Grant Accessibility when prompted. The mouse icon appears in the menu bar showing battery percentage and device status.

To configure button actions and gestures, edit `~/.config/hidpp/config.toml`:

```toml
[buttons]
83 = "alt+left"
86 = "alt+right"

[gestures.195]
up = "ctrl+up"
down = "ctrl+down"
left = "ctrl+left"
right = "ctrl+right"
tap = "playpause"
```

## Web

Configure DPI, scroll mode, button remaps, and Easy-Switch hosts from the browser. No install required.

https://jlevere.github.io/hidpp/

Works in Chrome and Edge (requires WebHID).

## CLI

```
hidpp info
hidpp get battery
hidpp set dpi 1600
hidpp export
```

## Building

With Nix:

```
nix build .#dmg
nix build .#app
nix build .#cli
nix flake check
```

With cargo:

```
cargo build --workspace --exclude hidpp-web
cargo test --workspace --exclude hidpp-web
```

Linux users need `libudev-dev`, `libxkbcommon-dev`, `libglib2.0-dev`, `libgtk-3-dev`, `libxdo-dev`. Install `udev/99-hidpp.rules` for non-root HID access.

## License

MIT or Apache-2.0.
