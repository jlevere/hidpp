#!/usr/bin/env bash
set -euo pipefail

BIN_DIR="${HOME}/.local/bin"
SYSTEMD_DIR="${HOME}/.config/systemd/user"

echo "hidpp uninstaller for Linux"
echo ""

# Stop and disable the service.
systemctl --user stop hidppd.service 2>/dev/null || true
systemctl --user disable hidppd.service 2>/dev/null || true
rm -f "${SYSTEMD_DIR}/hidppd.service"
systemctl --user daemon-reload
echo "  systemd removed"

# Remove binaries.
rm -f "${BIN_DIR}/hidpp" "${BIN_DIR}/hidppd"
echo "  binaries removed from ${BIN_DIR}/"

echo ""
echo "done. config left in place at ~/.config/hidpp/"
echo "  to fully remove: rm -rf ~/.config/hidpp"
echo ""
echo "note: udev rules at /etc/udev/rules.d/99-hidpp.rules must be"
echo "  removed manually with sudo if installed."
