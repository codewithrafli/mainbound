#!/usr/bin/env bash
# Record the Mainbound demo and export MP4 + GIF for README.
#
#   bun run demo          → record until you press ■ in menu bar, then convert
#   bun run demo 75       → record exactly 75 seconds
#   bun run demo --gif    → skip MP4, only output GIF
#
# Requirements: ffmpeg  (brew install ffmpeg)
set -euo pipefail

DUR=""
GIF_ONLY=false
for arg in "$@"; do
  case "$arg" in
    --gif) GIF_ONLY=true ;;
    [0-9]*) DUR="$arg" ;;
  esac
done

OUT_DIR="docs/assets"
MOV="$OUT_DIR/demo-raw.mov"
MP4="$OUT_DIR/demo.mp4"
GIF="$OUT_DIR/demo.gif"
mkdir -p "$OUT_DIR"

# ── Pre-record checklist ───────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║          Mainbound Demo — Pre-record checklist        ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""
echo "  1. Open Mainbound (bun run tauri dev in another tab)"
echo "  2. Add a workspace with a real git repo"
echo "  3. Have some uncommitted changes ready"
echo "  4. Make sure you're logged in to GitHub"
echo "  5. Window size: resize to ~1200×750 for best crop"
echo "  6. Hide the menu bar / use full screen for cleaner look"
echo ""
echo "  Script outline (≈75s):"
echo "    0:00  Open app — cockpit bar visible"
echo "    0:08  Type: git checkout -b feat/demo"
echo "    0:14  Type: bun run build (let it finish)"
echo "    0:25  Cockpit shows '5 changed' — press ⌘2"
echo "    0:32  Click a file → diff appears"
echo "    0:38  Stage files → Generate with AI → Commit"
echo "    0:50  Push (↑1 chip) → PR #N chip appears"
echo "    0:58  Click PR chip → timeline opens in-app"
echo "    1:05  Merge → toast → done"
echo ""
read -rp "  Ready? Press Enter to start recording… "
echo ""

# ── Record ────────────────────────────────────────────────────────────────
if [[ -n "$DUR" ]]; then
  echo "→ Recording for ${DUR}s — switch to Mainbound NOW"
  echo "  (screencapture will ask you to select a window/area)"
  screencapture -v -V "$DUR" "$MOV"
else
  echo "→ Recording — switch to Mainbound NOW"
  echo "  Press ■ in the menu bar when done"
  screencapture -v "$MOV"
fi

echo ""
echo "→ Converting…"

# ── Export MP4 (high quality, small file) ─────────────────────────────────
if [[ "$GIF_ONLY" == false ]]; then
  ffmpeg -y -loglevel error \
    -i "$MOV" \
    -vf "scale=1200:-2:flags=lanczos" \
    -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p \
    -movflags +faststart \
    "$MP4"
  echo "  MP4 → $(du -h "$MP4" | cut -f1 | tr -d ' ')  $MP4"
fi

# ── Export GIF (README hero, max 128 colors, bayer dither) ────────────────
ffmpeg -y -loglevel error \
  -i "$MOV" \
  -vf "fps=12,scale=1200:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=bayer" \
  "$GIF"
echo "  GIF → $(du -h "$GIF" | cut -f1 | tr -d ' ')  $GIF"

# Cleanup raw
rm -f "$MOV"

echo ""
echo "✓ Done! Next steps:"
echo "  git add docs/assets/demo.mp4 docs/assets/demo.gif"
echo "  git commit -m 'docs: update demo video'"
echo "  git push"
echo "  → README hero auto-updates"
echo ""
