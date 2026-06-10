#!/usr/bin/env bash
# Runs the mounted AppImage inside the container. AppImages need FUSE to
# self-mount; in Docker we extract them instead, then run AppRun directly.
set -euo pipefail

APPIMAGE=$(find /mnt -maxdepth 1 -iname '*.AppImage' | head -1)
if [[ -z "$APPIMAGE" ]]; then
  echo "✗ No .AppImage found in /mnt — mount the build dir with -v"
  exit 1
fi

echo "→ Found: $APPIMAGE"
cp "$APPIMAGE" /app/app.AppImage
chmod +x /app/app.AppImage

echo "→ Extracting (no FUSE in container)…"
cd /app
./app.AppImage --appimage-extract >/dev/null 2>&1

echo "→ GL renderer:"
glxinfo 2>/dev/null | grep -i "renderer string" || echo "  (software GL)"

echo "→ Launching Mainbound against WebKitGTK… (watch memory below)"
echo "─────────────────────────────────────────────────────────────"

# Background memory monitor for the WebKit process
( while true; do
    sleep 3
    ps -eo rss,comm 2>/dev/null | grep -i webkit | awk '{printf "  WebKit RSS: %.1f MB\n", $1/1024}'
  done ) &
MONITOR=$!

# Run it; clean up the monitor on exit
trap "kill $MONITOR 2>/dev/null || true" EXIT
exec /app/squashfs-root/AppRun
