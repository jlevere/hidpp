#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="/Applications/HID++.app"
PLIST_SRC="${SCRIPT_DIR}/com.hidpp.daemon.plist"
PLIST_DST="${HOME}/Library/LaunchAgents/com.hidpp.daemon.plist"

echo "hidpp installer for macOS"
echo ""

# Unload existing agent if running.
if [ -f "${PLIST_DST}" ]; then
    launchctl unload "${PLIST_DST}" 2>/dev/null || true
fi

# Install the .app bundle.
if [ -d "${SCRIPT_DIR}/HID++.app" ]; then
    # From DMG / release tarball.
    cp -R "${SCRIPT_DIR}/HID++.app" /Applications/
    echo "  app      -> ${APP_DIR}/"
elif [ -f "${SCRIPT_DIR}/hidppd" ]; then
    # From bare binaries — assemble the .app.
    mkdir -p "${APP_DIR}/Contents/MacOS"
    mkdir -p "${APP_DIR}/Contents/Resources"
    cp "${SCRIPT_DIR}/hidppd" "${APP_DIR}/Contents/MacOS/hidppd"
    cp "${SCRIPT_DIR}/hidpp" "${APP_DIR}/Contents/MacOS/hidpp" 2>/dev/null || true
    chmod +x "${APP_DIR}/Contents/MacOS/hidppd"
    if [ -f "${SCRIPT_DIR}/Info.plist" ]; then
        cp "${SCRIPT_DIR}/Info.plist" "${APP_DIR}/Contents/Info.plist"
    fi
    echo 'APPL????' > "${APP_DIR}/Contents/PkgInfo"
    echo "  app      -> ${APP_DIR}/ (assembled)"
else
    echo "error: no hidppd binary or HID++.app found in ${SCRIPT_DIR}"
    exit 1
fi

# Set the binary path for the launchd plist.
HIDPPD_PATH="${APP_DIR}/Contents/MacOS/hidppd"

# Install launchd plist (expand variables).
mkdir -p "$(dirname "${PLIST_DST}")"
sed -e "s|\${HIDPPD_PATH}|${HIDPPD_PATH}|g" -e "s|\${HOME}|${HOME}|g" "${PLIST_SRC}" > "${PLIST_DST}"
echo "  plist    -> ${PLIST_DST}"

# Create default config if none exists.
CONFIG_DIR="${HOME}/.config/hidpp"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    mkdir -p "${CONFIG_DIR}"
    "${HIDPPD_PATH}" sample-config > "${CONFIG_DIR}/config.toml"
    echo "  config   -> ${CONFIG_DIR}/config.toml (sample)"
else
    echo "  config   -> ${CONFIG_DIR}/config.toml (existing, kept)"
fi

# Ensure log directory exists.
mkdir -p "${HOME}/Library/Logs"

# Load the agent.
launchctl load "${PLIST_DST}"

echo ""
echo "done. hidppd is running."
echo ""
echo "  Grant Accessibility permission to HID++.app:"
echo "    System Settings → Privacy & Security → Accessibility → add HID++.app"
echo ""
echo "  logs:   tail -f ~/Library/Logs/hidppd.log"
echo "  config: ${CONFIG_DIR}/config.toml"
echo "  stop:   launchctl unload ${PLIST_DST}"
