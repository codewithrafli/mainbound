#!/usr/bin/env bash
# Record a Mainbound demo and convert it to the README gif.
#   bun run demo            → record until you press the stop button, then convert
#   bun run demo 20         → record 20 seconds
set -euo pipefail

DUR="${1:-}"
OUT_DIR="docs/assets"
MOV="$OUT_DIR/demo.mov"
GIF="$OUT_DIR/demo.gif"
mkdir -p "$OUT_DIR"

echo "→ recording screen$( [[ -n "$DUR" ]] && echo " for ${DUR}s" )…"
echo "  (select the Mainbound window; stop via the menu-bar ■ button)"
if [[ -n "$DUR" ]]; then
  screencapture -v -V "$DUR" "$MOV"
else
  screencapture -v "$MOV"
fi

echo "→ converting to gif (1200px, 12fps)…"
ffmpeg -y -loglevel error -i "$MOV" \
  -vf "fps=12,scale=1200:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer" \
  "$GIF"

echo "✓ $(du -h "$GIF" | cut -f1 | tr -d ' ') → $GIF"
echo "  commit & push, and the README hero updates."
