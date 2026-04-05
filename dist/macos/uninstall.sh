#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/Applications/HID++.app"
PLIST_DST="${HOME}/Library/LaunchAgents/com.hidpp.daemon.plist"
BIN_DIR="${HOME}/.local/bin"

echo "hidpp uninstaller for macOS"
echo ""

# Unload the agent.
if [ -f "${PLIST_DST}" ]; then
    launchctl unload "${PLIST_DST}" 2>/dev/null || true
    rm -f "${PLIST_DST}"
    echo "  plist removed"
fi

# Remove .app bundle.
if [ -d "${APP_DIR}" ]; then
    rm -rf "${APP_DIR}"
    echo "  app removed from ${APP_DIR}"
fi

# Remove legacy bare binaries if present.
rm -f "${BIN_DIR}/hidpp" "${BIN_DIR}/hidppd" 2>/dev/null

echo ""
echo "done. config left in place at ~/.config/hidpp/"
echo "  to fully remove: rm -rf ~/.config/hidpp"
