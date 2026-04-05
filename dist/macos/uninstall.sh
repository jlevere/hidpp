#!/usr/bin/env bash
set -euo pipefail

BIN_DIR="${HOME}/.local/bin"
PLIST_DST="${HOME}/Library/LaunchAgents/com.hidpp.daemon.plist"

echo "hidpp uninstaller for macOS"
echo ""

# Unload the agent.
if [ -f "${PLIST_DST}" ]; then
    launchctl unload "${PLIST_DST}" 2>/dev/null || true
    rm -f "${PLIST_DST}"
    echo "  plist removed"
fi

# Remove binaries.
rm -f "${BIN_DIR}/hidpp" "${BIN_DIR}/hidppd"
echo "  binaries removed from ${BIN_DIR}/"

echo ""
echo "done. config left in place at ~/.config/hidpp/"
echo "  to fully remove: rm -rf ~/.config/hidpp"
