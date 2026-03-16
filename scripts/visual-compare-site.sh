#!/usr/bin/env bash
# visual-compare-site.sh -- Run visual comparison for a single site.
#
# Copies missing CSS/assets from Jekyll output to rustkyll output before
# comparing, so we focus on HTML rendering differences, not SCSS compilation.
#
# Usage:
#   ./scripts/visual-compare-site.sh <site-name> [--no-copy-css]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLAYWRIGHT_DIR="$PROJECT_DIR/playwright"

SITE="$1"
COPY_CSS=true
if [[ "${2:-}" == "--no-copy-css" ]]; then
  COPY_CSS=false
fi

JEKYLL_DIR="/tmp/jekyll-$SITE"
RUSTKYLL_DIR="/tmp/rustkyll-$SITE"

if [[ ! -d "$JEKYLL_DIR" ]]; then
  echo "ERROR: Jekyll output not found: $JEKYLL_DIR"
  exit 1
fi
if [[ ! -d "$RUSTKYLL_DIR" ]]; then
  echo "ERROR: Rustkyll output not found: $RUSTKYLL_DIR"
  exit 1
fi

# Copy missing CSS/assets from Jekyll to rustkyll (focus on HTML diffs)
if [[ "$COPY_CSS" == "true" ]]; then
  echo "Copying missing assets from Jekyll to rustkyll..."
  # Copy CSS files
  find "$JEKYLL_DIR" -name "*.css" -o -name "*.css.map" | while read -r f; do
    rel="${f#$JEKYLL_DIR/}"
    target="$RUSTKYLL_DIR/$rel"
    if [[ ! -f "$target" ]]; then
      mkdir -p "$(dirname "$target")"
      cp "$f" "$target"
      echo "  Copied: $rel"
    fi
  done
  # Copy JS files
  find "$JEKYLL_DIR" -name "*.js" | while read -r f; do
    rel="${f#$JEKYLL_DIR/}"
    target="$RUSTKYLL_DIR/$rel"
    if [[ ! -f "$target" ]]; then
      mkdir -p "$(dirname "$target")"
      cp "$f" "$target"
      echo "  Copied: $rel"
    fi
  done
fi

# Find available ports
JEKYLL_PORT=$((4100 + (RANDOM % 100)))
RUSTKYLL_PORT=$((4200 + (RANDOM % 100)))

# Cleanup function
cleanup() {
  kill "$JPID" 2>/dev/null || true
  kill "$RPID" 2>/dev/null || true
  wait "$JPID" 2>/dev/null || true
  wait "$RPID" 2>/dev/null || true
}
trap cleanup EXIT

# Start HTTP servers
uv run python -m http.server "$JEKYLL_PORT" --directory "$JEKYLL_DIR" --bind 127.0.0.1 &>/dev/null &
JPID=$!
uv run python -m http.server "$RUSTKYLL_PORT" --directory "$RUSTKYLL_DIR" --bind 127.0.0.1 &>/dev/null &
RPID=$!

# Wait for servers (accept any HTTP response, not just 200,
# since sites without index.html return 404 but still work)
for i in $(seq 1 15); do
  J_CODE=$(curl -so /dev/null -w '%{http_code}' --max-time 2 "http://localhost:$JEKYLL_PORT/" 2>/dev/null || echo "000")
  R_CODE=$(curl -so /dev/null -w '%{http_code}' --max-time 2 "http://localhost:$RUSTKYLL_PORT/" 2>/dev/null || echo "000")
  if [[ "$J_CODE" != "000" ]] && [[ "$R_CODE" != "000" ]]; then
    break
  fi
  sleep 0.5
done

# Create screenshot dir
SCREENSHOT_DIR="$PLAYWRIGHT_DIR/screenshots/$SITE"
mkdir -p "$SCREENSHOT_DIR"

# Run Playwright
echo ""
echo "Running visual comparison for $SITE..."
echo "  Jekyll:   http://localhost:$JEKYLL_PORT"
echo "  Rustkyll: http://localhost:$RUSTKYLL_PORT"
echo ""

cd "$PLAYWRIGHT_DIR"
JEKYLL_URL="http://localhost:$JEKYLL_PORT" \
RUSTKYLL_URL="http://localhost:$RUSTKYLL_PORT" \
SCREENSHOT_DIR="$SCREENSHOT_DIR" \
DIFF_THRESHOLD="0.0" \
SITE_NAME="$SITE" \
npx playwright test 2>&1

echo ""
echo "Screenshots: $SCREENSHOT_DIR"
