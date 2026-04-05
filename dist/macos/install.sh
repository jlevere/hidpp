#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
PLIST_SRC="${SCRIPT_DIR}/com.hidpp.daemon.plist"
PLIST_DST="${HOME}/Library/LaunchAgents/com.hidpp.daemon.plist"

echo "hidpp installer for macOS"
echo ""

# Unload existing agent if running.
if [ -f "${PLIST_DST}" ]; then
    launchctl unload "${PLIST_DST}" 2>/dev/null || true
fi

# Copy binaries.
mkdir -p "${BIN_DIR}"
cp "${SCRIPT_DIR}/hidpp" "${BIN_DIR}/hidpp"
cp "${SCRIPT_DIR}/hidppd" "${BIN_DIR}/hidppd"
chmod +x "${BIN_DIR}/hidpp" "${BIN_DIR}/hidppd"
echo "  binaries -> ${BIN_DIR}/"

# Install launchd plist (expand HOME — macOS doesn't do this natively).
mkdir -p "$(dirname "${PLIST_DST}")"
sed "s|\${HOME}|${HOME}|g" "${PLIST_SRC}" > "${PLIST_DST}"
echo "  plist    -> ${PLIST_DST}"

# Create default config if none exists.
CONFIG_DIR="${HOME}/.config/hidpp"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    mkdir -p "${CONFIG_DIR}"
    "${BIN_DIR}/hidppd" sample-config > "${CONFIG_DIR}/config.toml"
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
echo "  logs:   tail -f ~/Library/Logs/hidppd.log"
echo "  config: ${CONFIG_DIR}/config.toml"
echo "  stop:   launchctl unload ${PLIST_DST}"
