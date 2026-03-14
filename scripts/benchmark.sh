#!/usr/bin/env bash
# Benchmark script: compares rustkyll vs Jekyll build times across all test sites.
#
# Usage: scripts/benchmark.sh [--runs N] [--timeout SECS] [--site SITE_PATH]
#
# Options:
#   --runs N         Number of runs per tool per site (default: 3)
#   --timeout SECS   Max seconds per build before killing (default: 300)
#   --site PATH      Only benchmark a specific site directory (relative to websites/)
#
# Output: docs/benchmark/results.md
#
# Prerequisites:
#   - rustkyll must be built (the script will build it if needed)
#   - Jekyll must be installed (checks standard path)
#   - Test sites must exist in websites/

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WEBSITES_DIR="$PROJECT_DIR/websites"
RESULTS_FILE="$PROJECT_DIR/docs/benchmark/results.md"
RUSTKYLL="$PROJECT_DIR/target/release/rustkyll"
JEKYLL_BIN="/home/alexey/.rvm/gems/ruby-3.3.7/bin/jekyll"

NUM_RUNS=3
TIMEOUT_SECS=300
FILTER_SITE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --runs)    NUM_RUNS="$2"; shift 2 ;;
        --timeout) TIMEOUT_SECS="$2"; shift 2 ;;
        --site)    FILTER_SITE="$2"; shift 2 ;;
        *)         echo "Unknown option: $1"; exit 1 ;;
    esac
done

# --- Validation ---

if [ ! -d "$WEBSITES_DIR" ]; then
    echo "ERROR: websites/ directory not found at $WEBSITES_DIR"
    echo "Clone test sites into websites/ before running this benchmark."
    exit 1
fi

if [ ! -x "$JEKYLL_BIN" ] && ! command -v jekyll &>/dev/null; then
    echo "ERROR: Jekyll not found at $JEKYLL_BIN and not in PATH."
    echo "Install Jekyll (Ruby) to run this benchmark."
    exit 1
fi

if command -v jekyll &>/dev/null; then
    JEKYLL_BIN="$(command -v jekyll)"
fi

# Build rustkyll in release mode if needed
if [ ! -x "$RUSTKYLL" ]; then
    echo "Building rustkyll in release mode..."
    "$SCRIPT_DIR/cargo-safe" build --release
fi

# --- Helper functions ---

# Get median of an array of numbers (passed as arguments)
median() {
    local sorted
    sorted=($(printf '%s\n' "$@" | sort -g))
    local n=${#sorted[@]}
    local mid=$((n / 2))
    echo "${sorted[$mid]}"
}

# Time a single build. Prints elapsed seconds (float) or "FAIL" on failure.
# Usage: time_build TOOL SITE_DIR
time_build() {
    local tool="$1"
    local site_dir="$2"
    local exit_code=0

    # Clean previous output
    rm -rf "$site_dir/_site" 2>/dev/null || true

    local start end elapsed
    start=$(date +%s.%N)

    if [ "$tool" = "rustkyll" ]; then
        (cd "$site_dir" && timeout "$TIMEOUT_SECS" "$RUSTKYLL" build) >/dev/null 2>&1 || exit_code=$?
    elif [ "$tool" = "jekyll" ]; then
        # Try bundle exec first, fall back to direct jekyll
        if [ -f "$site_dir/Gemfile.lock" ]; then
            (cd "$site_dir" && timeout "$TIMEOUT_SECS" bundle exec jekyll build) >/dev/null 2>&1 || exit_code=$?
        elif [ -f "$site_dir/Gemfile" ]; then
            # Install gems first, then build
            (cd "$site_dir" && bundle install --quiet 2>/dev/null && timeout "$TIMEOUT_SECS" bundle exec jekyll build) >/dev/null 2>&1 || exit_code=$?
        else
            (cd "$site_dir" && timeout "$TIMEOUT_SECS" "$JEKYLL_BIN" build) >/dev/null 2>&1 || exit_code=$?
        fi
    fi

    end=$(date +%s.%N)

    if [ "$exit_code" -eq 0 ]; then
        elapsed=$(echo "$end - $start" | bc)
        printf "%.3f" "$elapsed"
    elif [ "$exit_code" -eq 124 ]; then
        echo "TIMEOUT"
    else
        echo "FAIL"
    fi
}

# Count pages in _site directory (HTML files)
count_pages() {
    local site_dir="$1"
    if [ -d "$site_dir/_site" ]; then
        find "$site_dir/_site" -name '*.html' | wc -l
    else
        echo "0"
    fi
}

# Run benchmark for one site with one tool, NUM_RUNS times. Prints median time.
benchmark_tool() {
    local tool="$1"
    local site_dir="$2"
    local times=()
    local result

    for ((i = 1; i <= NUM_RUNS; i++)); do
        result=$(time_build "$tool" "$site_dir")
        if [ "$result" = "FAIL" ] || [ "$result" = "TIMEOUT" ]; then
            echo "$result"
            return
        fi
        times+=("$result")
    done

    median "${times[@]}"
}

# --- Discover sites ---

discover_sites() {
    local sites=()

    # Sites at websites/ORG/REPO level (alexeygrigorev/*, DataTalksClub/*)
    for org_dir in "$WEBSITES_DIR"/alexeygrigorev "$WEBSITES_DIR"/DataTalksClub; do
        if [ -d "$org_dir" ]; then
            for site_dir in "$org_dir"/*/; do
                if [ -f "$site_dir/_config.yml" ]; then
                    sites+=("$site_dir")
                fi
            done
        fi
    done

    # Sites at websites/SITE level (top-level repos)
    for site_dir in "$WEBSITES_DIR"/*/; do
        local name
        name=$(basename "$site_dir")
        # Skip org-level directories
        if [ "$name" = "alexeygrigorev" ] || [ "$name" = "DataTalksClub" ]; then
            continue
        fi
        if [ -f "$site_dir/_config.yml" ]; then
            sites+=("$site_dir")
        fi
        # Check for subdirectory sites (e.g., jekyll-docs/docs/)
        for sub_dir in "$site_dir"/*/; do
            if [ -f "$sub_dir/_config.yml" ] && [ ! -f "$site_dir/_config.yml" ]; then
                sites+=("$sub_dir")
            fi
        done
    done

    printf '%s\n' "${sites[@]}"
}

# Get a display name for a site directory
site_display_name() {
    local site_dir="$1"
    local rel_path
    rel_path="${site_dir#$WEBSITES_DIR/}"
    rel_path="${rel_path%/}"
    # Remove any double slashes
    rel_path="${rel_path//\/\//\/}"
    echo "$rel_path"
}

# --- Main ---

echo "=== Rustkyll vs Jekyll Benchmark ==="
echo "Runs per tool: $NUM_RUNS"
echo "Timeout: ${TIMEOUT_SECS}s"
echo ""

# Collect sites
declare -a ALL_SITES
if [ -n "$FILTER_SITE" ]; then
    site_path="$WEBSITES_DIR/$FILTER_SITE"
    if [ -d "$site_path" ] && [ -f "$site_path/_config.yml" ]; then
        ALL_SITES=("$site_path")
    else
        echo "ERROR: Site not found or has no _config.yml: $site_path"
        exit 1
    fi
else
    mapfile -t ALL_SITES < <(discover_sites)
fi

echo "Found ${#ALL_SITES[@]} sites to benchmark."
echo ""

# Results arrays
declare -a RESULT_NAMES RESULT_PAGES RESULT_JEKYLL RESULT_RUSTKYLL

for site_dir in "${ALL_SITES[@]}"; do
    name=$(site_display_name "$site_dir")
    echo "--- Benchmarking: $name ---"

    # Run rustkyll first (to count pages)
    echo "  rustkyll..."
    rustkyll_time=$(benchmark_tool "rustkyll" "$site_dir")
    if [ "$rustkyll_time" != "FAIL" ] && [ "$rustkyll_time" != "TIMEOUT" ]; then
        pages=$(count_pages "$site_dir")
    else
        pages="?"
    fi

    echo "  rustkyll: ${rustkyll_time}s"

    # Run Jekyll
    echo "  Jekyll..."
    jekyll_time=$(benchmark_tool "jekyll" "$site_dir")
    echo "  Jekyll:   ${jekyll_time}s"

    # If rustkyll failed but Jekyll succeeded, get page count from Jekyll
    if [ "$pages" = "?" ] && [ "$jekyll_time" != "FAIL" ] && [ "$jekyll_time" != "TIMEOUT" ]; then
        # Rebuild once to count pages
        time_build "jekyll" "$site_dir" >/dev/null 2>&1 || true
        pages=$(count_pages "$site_dir")
    fi

    RESULT_NAMES+=("$name")
    RESULT_PAGES+=("$pages")
    RESULT_JEKYLL+=("$jekyll_time")
    RESULT_RUSTKYLL+=("$rustkyll_time")

    # Clean up after benchmarking
    rm -rf "$site_dir/_site" 2>/dev/null || true

    echo ""
done

# --- Generate results document ---

mkdir -p "$(dirname "$RESULTS_FILE")"

{
    echo "# Benchmark: rustkyll vs Jekyll"
    echo ""
    echo "Generated: $(date -u '+%Y-%m-%d %H:%M UTC')"
    echo ""
    echo "Configuration: $NUM_RUNS runs per tool, median wall-clock time reported."
    echo "Timeout: ${TIMEOUT_SECS}s per build."
    echo ""
    echo "rustkyll version: $($RUSTKYLL --version 2>&1 || echo 'unknown')"
    echo "Jekyll version: $($JEKYLL_BIN --version 2>&1 || echo 'unknown')"
    echo ""
    echo "## Results"
    echo ""
    echo "| Site | Pages | Jekyll (s) | rustkyll (s) | Speedup |"
    echo "|------|-------|------------|-------------|---------|"

    for ((i = 0; i < ${#RESULT_NAMES[@]}; i++)); do
        name="${RESULT_NAMES[$i]}"
        pages="${RESULT_PAGES[$i]}"
        jt="${RESULT_JEKYLL[$i]}"
        rt="${RESULT_RUSTKYLL[$i]}"

        # Calculate speedup
        if [ "$jt" != "FAIL" ] && [ "$jt" != "TIMEOUT" ] && \
           [ "$rt" != "FAIL" ] && [ "$rt" != "TIMEOUT" ]; then
            # Avoid division by zero
            is_zero=$(echo "$rt == 0" | bc)
            if [ "$is_zero" -eq 1 ]; then
                speedup="N/A"
            else
                speedup=$(echo "scale=2; $jt / $rt" | bc)
                speedup="${speedup}x"
            fi
        else
            speedup="N/A"
        fi

        echo "| $name | $pages | $jt | $rt | $speedup |"
    done

    echo ""
    echo "## Notes"
    echo ""
    echo "- FAIL means the tool could not build the site (template error, missing plugin, etc.)"
    echo "- TIMEOUT means the build exceeded ${TIMEOUT_SECS}s and was killed"
    echo "- Speedup = Jekyll time / rustkyll time (higher is better for rustkyll)"
    echo "- Page count is the number of HTML files generated in _site/"
    echo "- Each build starts from a clean _site/ directory (no caching)"
    echo "- Jekyll builds use bundle exec when a Gemfile is present"
    echo "- rustkyll is pre-compiled in release mode"

} > "$RESULTS_FILE"

echo "=== Results written to $RESULTS_FILE ==="
echo ""
cat "$RESULTS_FILE"
