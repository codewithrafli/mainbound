#!/usr/bin/env bash
# Test the Linux AppImage against real WebKitGTK, GUI forwarded to your Mac.
#
#   ./scripts/linux-test/test.sh path/to/Mainbound_x.y.z_amd64.AppImage
#
# Requirements (one-time):
#   brew install --cask docker xquartz
#   Then in XQuartz → Settings → Security: enable "Allow connections from
#   network clients", and log out/in once.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
APPIMAGE="${1:-}"

if [[ -z "$APPIMAGE" || ! -f "$APPIMAGE" ]]; then
  echo "Usage: $0 <path-to-AppImage>"
  echo "  Download the .AppImage from GitHub Releases first."
  exit 1
fi

# 1. XQuartz check
if ! pgrep -x Xquartz >/dev/null 2>&1; then
  echo "→ Starting XQuartz…"
  open -a XQuartz
  sleep 3
fi

# 2. Allow local X11 connections from Docker
IP=$(ifconfig en0 2>/dev/null | grep 'inet ' | awk '{print $2}' | head -1)
if [[ -z "$IP" ]]; then
  IP=$(ifconfig en1 2>/dev/null | grep 'inet ' | awk '{print $2}' | head -1)
fi
echo "→ Host IP for X11: $IP"
xhost + "$IP" >/dev/null 2>&1 || xhost +localhost >/dev/null 2>&1 || true

# 3. Build the runtime image (cached after first run)
echo "→ Building Linux test image (first run ~2-3 min)…"
docker build -t mainbound-linux-test "$HERE"

# 4. Run the AppImage, forwarding the display to XQuartz
APPDIR="$(cd "$(dirname "$APPIMAGE")" && pwd)"
echo "→ Running… (close the window or Ctrl+C to stop)"
docker run --rm -it \
  -e DISPLAY="${IP}:0" \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  -v "$APPDIR":/mnt:ro \
  --shm-size=512m \
  mainbound-linux-test

xhost - "$IP" >/dev/null 2>&1 || true
