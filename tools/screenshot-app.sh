#!/usr/bin/env bash
#
# screenshot-app.sh — launch a Toile app/example/editor, let it render, and take
# a screenshot of its window. The windowed counterpart to `toile-harness` (which
# is fully headless): use this to visually verify the egui editor UI and the full
# render pipeline (post-processing / lighting) that only run in a real window.
#
# Usage:
#   tools/screenshot-app.sh cargo run --example breakout -p toile-app
#   tools/screenshot-app.sh cargo run -p toile-cli -- editor
#   OUT=shots/editor.png WAIT=6 tools/screenshot-app.sh cargo run -p toile-cli -- editor
#
# Env:
#   OUT   output PNG path           (default: harness-out/app-<epoch>.png)
#   WAIT  seconds to wait for the window before capturing (default: 5)
#
# macOS notes: `screencapture` needs Screen Recording permission for the
# terminal app; the per-window crop additionally needs Accessibility permission
# (System Settings > Privacy & Security). Without Accessibility it falls back to
# capturing the full main display.
set -uo pipefail

if [[ $# -eq 0 ]]; then
  echo "usage: $0 <command to launch the app...>" >&2
  exit 2
fi

OUT="${OUT:-harness-out/app-$(date +%s).png}"
WAIT="${WAIT:-5}"
mkdir -p "$(dirname "$OUT")"

if [[ "$(uname)" != "Darwin" ]]; then
  echo "error: this helper currently supports macOS only (uses screencapture)." >&2
  echo "On Linux use the headless 'toile-harness' instead, or grim/scrot." >&2
  exit 1
fi

echo "launching: $*"
"$@" &
APP_PID=$!

cleanup() { kill "$APP_PID" 2>/dev/null; wait "$APP_PID" 2>/dev/null; }
trap cleanup EXIT

sleep "$WAIT"

if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "error: app exited before capture (build error or early crash). See output above." >&2
  exit 1
fi

# Try to capture just the frontmost window's region (needs Accessibility).
BOUNDS="$(osascript -e 'tell application "System Events" to tell (first process whose frontmost is true) to get {position, size} of front window' 2>/dev/null || true)"

if [[ -n "$BOUNDS" ]]; then
  # BOUNDS looks like: "x, y, w, h"
  IFS=', ' read -r X Y W H <<< "$BOUNDS"
  if [[ -n "${X:-}" && -n "${Y:-}" && -n "${W:-}" && -n "${H:-}" ]]; then
    screencapture -x -R "${X},${Y},${W},${H}" "$OUT"
  else
    screencapture -x "$OUT"
  fi
else
  echo "note: could not read front-window bounds (grant Accessibility for a cropped shot); capturing full display." >&2
  screencapture -x "$OUT"
fi

echo "saved $OUT"
