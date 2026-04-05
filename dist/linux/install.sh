#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
SYSTEMD_DIR="${HOME}/.config/systemd/user"
UDEV_RULES="/etc/udev/rules.d/99-hidpp.rules"

echo "hidpp installer for Linux"
echo ""

# Copy binaries.
mkdir -p "${BIN_DIR}"
cp "${SCRIPT_DIR}/hidpp" "${BIN_DIR}/hidpp"
cp "${SCRIPT_DIR}/hidppd" "${BIN_DIR}/hidppd"
chmod +x "${BIN_DIR}/hidpp" "${BIN_DIR}/hidppd"
echo "  binaries  -> ${BIN_DIR}/"

# Install systemd unit.
mkdir -p "${SYSTEMD_DIR}"
cp "${SCRIPT_DIR}/hidppd.service" "${SYSTEMD_DIR}/hidppd.service"
echo "  systemd   -> ${SYSTEMD_DIR}/hidppd.service"

# Install udev rules (requires root).
if [ -f "${SCRIPT_DIR}/99-hidpp.rules" ]; then
    if [ "$(id -u)" -eq 0 ]; then
        cp "${SCRIPT_DIR}/99-hidpp.rules" "${UDEV_RULES}"
        udevadm control --reload-rules 2>/dev/null || true
        echo "  udev      -> ${UDEV_RULES}"
    else
        echo "  udev      -> skipped (run with sudo, or:"
        echo "               sudo cp ${SCRIPT_DIR}/99-hidpp.rules ${UDEV_RULES}"
        echo "               sudo udevadm control --reload-rules)"
    fi
fi

# Create default config if none exists.
CONFIG_DIR="${HOME}/.config/hidpp"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    mkdir -p "${CONFIG_DIR}"
    "${BIN_DIR}/hidppd" sample-config > "${CONFIG_DIR}/config.toml"
    echo "  config    -> ${CONFIG_DIR}/config.toml (sample)"
else
    echo "  config    -> ${CONFIG_DIR}/config.toml (existing, kept)"
fi

# Enable and start the service.
systemctl --user daemon-reload
systemctl --user enable hidppd.service
systemctl --user start hidppd.service

echo ""
echo "done. hidppd is running."
echo "  logs:   journalctl --user -u hidppd -f"
echo "  config: ${CONFIG_DIR}/config.toml"
echo "  stop:   systemctl --user stop hidppd"
