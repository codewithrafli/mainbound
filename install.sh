#!/usr/bin/env bash
# Mainbound installer — from shell to main.
#   curl -fsSL https://raw.githubusercontent.com/codewithrafli/mainbound/main/install.sh | bash
#
# Downloads via curl, so macOS never applies the browser quarantine
# attribute — no "damaged app" Gatekeeper drama.
set -euo pipefail

REPO="codewithrafli/mainbound"
APP="/Applications/Mainbound.app"

case "$(uname -m)" in
  arm64) ARCH="aarch64" ;;
  x86_64) ARCH="x64" ;;
  *) echo "error: unsupported architecture $(uname -m)" >&2; exit 1 ;;
esac

echo "→ Mainbound installer (${ARCH})"

URL=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep -o "https://[^\"]*Mainbound_${ARCH}\.app\.tar\.gz" | head -1)

if [[ -z "$URL" ]]; then
  echo "error: no ${ARCH} build found in the latest release" >&2
  echo "       check https://github.com/${REPO}/releases" >&2
  exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "→ downloading $(basename "$URL")"
curl -fL --progress-bar "$URL" -o "$TMP/mainbound.tar.gz"

echo "→ installing to /Applications"
osascript -e 'quit app "Mainbound"' >/dev/null 2>&1 || true
rm -rf "$APP"
tar -xzf "$TMP/mainbound.tar.gz" -C /Applications

# belt and braces — harmless if the attribute isn't present
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

echo "✓ installed $(defaults read "$APP/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "") — launching"
open "$APP"
