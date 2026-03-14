#!/usr/bin/env bash
# visual-compare.sh -- Visual screenshot comparison between Jekyll and rustkyll output.
#
# Usage:
#   ./scripts/visual-compare.sh --site DataTalksClub/datatalksclub.github.io
#   ./scripts/visual-compare.sh --site alexeygrigorev/kids-horror-stories-ru
#   ./scripts/visual-compare.sh --jekyll-dir /path/to/jekyll/_site --rustkyll-dir /path/to/rustkyll/_site
#
# Options:
#   --site <org/repo>         Build both Jekyll and rustkyll for the given site under websites/
#   --jekyll-dir <dir>        Path to pre-built Jekyll output
#   --rustkyll-dir <dir>      Path to pre-built rustkyll output
#   --threshold <0.0-1.0>     Max pixel diff ratio (default: 0.05 = 5%)
#   --jekyll-port <port>      Port for Jekyll HTTP server (default: 4100)
#   --rustkyll-port <port>    Port for rustkyll HTTP server (default: 4101)
#   --skip-build              Skip building, assume output dirs already exist
#
# Prerequisites:
#   - Node.js and npm
#   - Python 3 (for http.server)
#   - Playwright browsers installed (run: cd playwright && npm install && npx playwright install chromium)
#   - rustkyll binary built (cargo build --release)
#   - Jekyll installed (for --site mode without --skip-build)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLAYWRIGHT_DIR="$PROJECT_DIR/playwright"

SITE=""
JEKYLL_DIR=""
RUSTKYLL_DIR=""
THRESHOLD="0.05"
JEKYLL_PORT=4100
RUSTKYLL_PORT=4101
SKIP_BUILD=false

# PIDs to clean up
JEKYLL_SERVER_PID=""
RUSTKYLL_SERVER_PID=""

cleanup() {
    echo ""
    echo "Cleaning up..."
    if [[ -n "$JEKYLL_SERVER_PID" ]]; then
        kill "$JEKYLL_SERVER_PID" 2>/dev/null || true
        wait "$JEKYLL_SERVER_PID" 2>/dev/null || true
        echo "  Stopped Jekyll server (PID $JEKYLL_SERVER_PID)"
    fi
    if [[ -n "$RUSTKYLL_SERVER_PID" ]]; then
        kill "$RUSTKYLL_SERVER_PID" 2>/dev/null || true
        wait "$RUSTKYLL_SERVER_PID" 2>/dev/null || true
        echo "  Stopped rustkyll server (PID $RUSTKYLL_SERVER_PID)"
    fi
}

trap cleanup EXIT

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --site)
            SITE="$2"
            shift 2
            ;;
        --jekyll-dir)
            JEKYLL_DIR="$2"
            shift 2
            ;;
        --rustkyll-dir)
            RUSTKYLL_DIR="$2"
            shift 2
            ;;
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --jekyll-port)
            JEKYLL_PORT="$2"
            shift 2
            ;;
        --rustkyll-port)
            RUSTKYLL_PORT="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        -h|--help)
            head -25 "$0" | tail -22
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Determine site name for test filtering
SITE_NAME=""

if [[ -n "$SITE" ]]; then
    SITE_DIR="$PROJECT_DIR/websites/$SITE"
    if [[ ! -d "$SITE_DIR" ]]; then
        echo "ERROR: Site directory not found: $SITE_DIR"
        exit 1
    fi

    SITE_NAME="$SITE"
    SAFE_SITE="$(echo "$SITE" | tr '/' '-')"
    JEKYLL_DIR="${JEKYLL_DIR:-/tmp/visual-compare-jekyll-$SAFE_SITE}"
    RUSTKYLL_DIR="${RUSTKYLL_DIR:-/tmp/visual-compare-rustkyll-$SAFE_SITE}"

    if [[ "$SKIP_BUILD" == "false" ]]; then
        # Build with rustkyll
        echo "=== Building with rustkyll ==="
        RUSTKYLL_BIN="$PROJECT_DIR/target/release/rustkyll"
        if [[ ! -x "$RUSTKYLL_BIN" ]]; then
            echo "Building rustkyll..."
            "$SCRIPT_DIR/cargo-safe" build --release
        fi
        rm -rf "$RUSTKYLL_DIR"
        "$RUSTKYLL_BIN" build --source "$SITE_DIR" --destination "$RUSTKYLL_DIR"
        echo "  Rustkyll output: $RUSTKYLL_DIR"

        # Build with Jekyll (or check for pre-built output)
        if [[ ! -d "$JEKYLL_DIR" ]] || [[ -z "$(ls -A "$JEKYLL_DIR" 2>/dev/null)" ]]; then
            echo ""
            echo "=== Building with Jekyll ==="
            if command -v bundle &>/dev/null; then
                (cd "$SITE_DIR" && bundle exec jekyll build --destination "$JEKYLL_DIR" 2>&1 | tail -5)
                echo "  Jekyll output: $JEKYLL_DIR"
            else
                echo "ERROR: Jekyll (bundle) not found. Either install Jekyll or provide --jekyll-dir."
                echo "  To build manually: cd $SITE_DIR && bundle exec jekyll build --destination $JEKYLL_DIR"
                exit 1
            fi
        else
            echo "Using existing Jekyll output: $JEKYLL_DIR"
        fi
    fi
elif [[ -n "$JEKYLL_DIR" ]] && [[ -n "$RUSTKYLL_DIR" ]]; then
    SITE_NAME="custom"
else
    echo "Usage: $0 --site <org/repo> OR --jekyll-dir <dir> --rustkyll-dir <dir>"
    exit 1
fi

# Validate directories
if [[ ! -d "$JEKYLL_DIR" ]]; then
    echo "ERROR: Jekyll output directory not found: $JEKYLL_DIR"
    exit 1
fi
if [[ ! -d "$RUSTKYLL_DIR" ]]; then
    echo "ERROR: Rustkyll output directory not found: $RUSTKYLL_DIR"
    exit 1
fi

echo ""
echo "=== Starting HTTP servers ==="

# Start Jekyll server
python3 -m http.server "$JEKYLL_PORT" --directory "$JEKYLL_DIR" --bind 127.0.0.1 &>/dev/null &
JEKYLL_SERVER_PID=$!
echo "  Jekyll server: http://localhost:$JEKYLL_PORT (PID $JEKYLL_SERVER_PID)"

# Start rustkyll server
python3 -m http.server "$RUSTKYLL_PORT" --directory "$RUSTKYLL_DIR" --bind 127.0.0.1 &>/dev/null &
RUSTKYLL_SERVER_PID=$!
echo "  Rustkyll server: http://localhost:$RUSTKYLL_PORT (PID $RUSTKYLL_SERVER_PID)"

# Wait for servers to be ready
echo "  Waiting for servers..."
for i in $(seq 1 30); do
    if curl -sf "http://localhost:$JEKYLL_PORT/" >/dev/null 2>&1 && \
       curl -sf "http://localhost:$RUSTKYLL_PORT/" >/dev/null 2>&1; then
        echo "  Both servers are ready."
        break
    fi
    if [[ "$i" -eq 30 ]]; then
        echo "ERROR: Servers failed to start within 30 seconds."
        exit 1
    fi
    sleep 1
done

# Verify servers return HTML (write to temp file to avoid pipefail/SIGPIPE issues)
verify_html() {
    local url="$1"
    local label="$2"
    local tmpfile
    tmpfile=$(mktemp)
    curl -sf --max-time 10 "$url" > "$tmpfile" 2>/dev/null || true
    if ! grep -qi '<html' "$tmpfile"; then
        echo "ERROR: $label server did not return valid HTML at $url"
        rm -f "$tmpfile"
        exit 1
    fi
    rm -f "$tmpfile"
}
verify_html "http://localhost:$JEKYLL_PORT/" "Jekyll"
verify_html "http://localhost:$RUSTKYLL_PORT/" "Rustkyll"
echo "  Both servers returning valid HTML."

# Set up Playwright if needed
echo ""
echo "=== Setting up Playwright ==="
if [[ ! -d "$PLAYWRIGHT_DIR/node_modules" ]]; then
    echo "  Installing dependencies..."
    (cd "$PLAYWRIGHT_DIR" && npm install)
fi

# Check if browsers are installed
if ! (cd "$PLAYWRIGHT_DIR" && npx playwright install --dry-run chromium 2>&1 | grep -q "already installed") 2>/dev/null; then
    echo "  Installing Chromium browser..."
    (cd "$PLAYWRIGHT_DIR" && npx playwright install chromium)
fi

# Create screenshot output directory
SCREENSHOT_DIR="$PLAYWRIGHT_DIR/screenshots/$SITE_NAME"
mkdir -p "$SCREENSHOT_DIR"

echo ""
echo "=== Running visual comparison tests ==="
echo "  Jekyll URL:    http://localhost:$JEKYLL_PORT"
echo "  Rustkyll URL:  http://localhost:$RUSTKYLL_PORT"
echo "  Site:          $SITE_NAME"
echo "  Threshold:     $(echo "$THRESHOLD * 100" | bc)%"
echo "  Screenshots:   $SCREENSHOT_DIR"
echo ""

# Run Playwright tests
TEST_EXIT=0
(cd "$PLAYWRIGHT_DIR" && \
    JEKYLL_URL="http://localhost:$JEKYLL_PORT" \
    RUSTKYLL_URL="http://localhost:$RUSTKYLL_PORT" \
    SCREENSHOT_DIR="$SCREENSHOT_DIR" \
    DIFF_THRESHOLD="$THRESHOLD" \
    SITE_NAME="$SITE_NAME" \
    npx playwright test) || TEST_EXIT=$?

echo ""
echo "=== Results ==="

if [[ "$TEST_EXIT" -eq 0 ]]; then
    echo "PASS: All visual comparisons passed (threshold: $(echo "$THRESHOLD * 100" | bc)%)"
    echo "Screenshots saved to: $SCREENSHOT_DIR"
else
    echo "FAIL: Some visual comparisons failed"
    echo "Screenshots and diff images saved to: $SCREENSHOT_DIR"
    echo ""
    echo "Review diff images (*__diff.png) to see pixel differences."
fi

# List generated screenshots
echo ""
echo "Generated files:"
ls -la "$SCREENSHOT_DIR"/*.png 2>/dev/null | while read -r line; do
    echo "  $line"
done

exit "$TEST_EXIT"
