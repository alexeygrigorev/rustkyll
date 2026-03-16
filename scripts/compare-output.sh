#!/usr/bin/env bash
# compare-output.sh -- Structural comparison between Jekyll and rustkyll output.
#
# Usage:
#   ./scripts/compare-output.sh --site DataTalksClub/datatalksclub.github.io
#   ./scripts/compare-output.sh --site alexeygrigorev/kids-horror-stories-ru
#   ./scripts/compare-output.sh --jekyll-dir /path/to/jekyll/output --rustkyll-dir /path/to/rustkyll/output
#   ./scripts/compare-output.sh --validate-only --site DataTalksClub/datatalksclub.github.io
#
# Modes:
#   Default (--site or --jekyll-dir/--rustkyll-dir): Full Jekyll vs rustkyll comparison
#   --validate-only: Build with rustkyll only and validate output (no Jekyll required)
#
# This script compares:
# 1. File tree: lists of generated HTML files
# 2. DOM tree comparison: full normalized DOM tree diff via dom_compare.py
# 3. Reports differences and exits nonzero if thresholds exceeded.
#
# Thresholds:
# - File count difference: 5% tolerance
# - DOM comparison: fail if any common file has DOM differences
# - Missing files: fail if >5% of files are missing from either side

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

SITE=""
JEKYLL_DIR=""
RUSTKYLL_DIR=""
THRESHOLD_PCT=5
VALIDATE_ONLY=0
MIN_HTML_FILES=100

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
            THRESHOLD_PCT="$2"
            shift 2
            ;;
        --validate-only)
            VALIDATE_ONLY=1
            shift
            ;;
        --min-files)
            MIN_HTML_FILES="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# --- Helper functions (defined before use) ---

validate_output() {
    local dir="$1"
    local label="${2:-output}"
    local html_count=0
    local empty_count=0
    local raw_liquid_count=0
    local empty_files=""
    local liquid_files=""

    while IFS= read -r -d '' file; do
        html_count=$((html_count + 1))
        local size
        size=$(stat -c%s "$file" 2>/dev/null || echo "0")
        if [[ "$size" -lt 100 ]]; then
            empty_count=$((empty_count + 1))
            local rel="${file#$dir/}"
            empty_files="${empty_files}    ${rel} (${size} bytes)\n"
        fi
        # Check for raw Liquid tags, excluding ${{ (GitHub Actions syntax in code blocks)
        if grep -qP '(?<!\$)\{%|(?<!\$)\{\{' "$file" 2>/dev/null; then
            raw_liquid_count=$((raw_liquid_count + 1))
            local rel="${file#$dir/}"
            liquid_files="${liquid_files}    ${rel}\n"
        fi
    done < <(find "$dir" -name "*.html" -print0)

    echo "  HTML files: $html_count"
    echo "  Empty files (<100 bytes): $empty_count"
    echo "  Files with raw Liquid tags: $raw_liquid_count"

    local failed=0
    if [[ "$empty_count" -gt 0 ]]; then
        echo "  WARNING: $empty_count empty HTML files found in $label"
        if [[ "$empty_count" -le 10 ]]; then
            echo -e "$empty_files" | head -10
        fi
    fi
    if [[ "$raw_liquid_count" -gt 0 ]]; then
        echo "  FAIL: $raw_liquid_count files with raw Liquid tags in $label"
        if [[ "$raw_liquid_count" -le 10 ]]; then
            echo -e "$liquid_files" | head -10
        fi
        failed=1
    fi
    return $failed
}

# --- Validate-only mode: build with rustkyll and validate output (no Jekyll) ---

if [[ "$VALIDATE_ONLY" -eq 1 ]]; then
    if [[ -z "$SITE" ]]; then
        echo "Usage: $0 --validate-only --site <site-path>"
        echo "  e.g.: $0 --validate-only --site DataTalksClub/datatalksclub.github.io"
        exit 1
    fi

    SITE_DIR="$PROJECT_DIR/websites/$SITE"
    if [[ ! -d "$SITE_DIR" ]]; then
        echo "ERROR: Site directory not found: $SITE_DIR"
        exit 1
    fi

    RUSTKYLL_DIR="/tmp/compare-rustkyll-$(echo "$SITE" | tr '/' '-')"

    # Build with rustkyll
    echo "=== Building with rustkyll (validate-only mode) ==="
    "$PROJECT_DIR/target/release/rustkyll" build \
        --source "$SITE_DIR" \
        --destination "$RUSTKYLL_DIR" 2>&1 | tail -5
    echo ""

    # Validate output
    echo "=== Rustkyll Output Validation ==="
    VALIDATE_FAIL=0
    validate_output "$RUSTKYLL_DIR" "rustkyll" || VALIDATE_FAIL=1
    echo ""

    # Count HTML files
    HTML_COUNT=$(find "$RUSTKYLL_DIR" -name "*.html" | wc -l)
    echo "  Total HTML files: $HTML_COUNT"
    echo "  Minimum required: $MIN_HTML_FILES"
    echo ""

    # Summary
    echo "=== Summary ==="
    FAILED=0

    if [[ "$VALIDATE_FAIL" -eq 1 ]]; then
        echo "FAIL: Rustkyll output validation failed (raw Liquid tags found)"
        FAILED=1
    else
        echo "PASS: Rustkyll output validation (no raw Liquid tags)"
    fi

    if [[ "$HTML_COUNT" -lt "$MIN_HTML_FILES" ]]; then
        echo "FAIL: HTML file count ($HTML_COUNT) is below minimum ($MIN_HTML_FILES)"
        FAILED=1
    else
        echo "PASS: HTML file count ($HTML_COUNT) meets minimum ($MIN_HTML_FILES)"
    fi

    exit $FAILED
fi

# --- Site mode: build both Jekyll and rustkyll ---

if [[ -n "$SITE" ]]; then
    SITE_DIR="$PROJECT_DIR/websites/$SITE"
    if [[ ! -d "$SITE_DIR" ]]; then
        echo "ERROR: Site directory not found: $SITE_DIR"
        exit 1
    fi

    JEKYLL_DIR="/tmp/compare-jekyll-$(echo "$SITE" | tr '/' '-')"
    RUSTKYLL_DIR="/tmp/compare-rustkyll-$(echo "$SITE" | tr '/' '-')"

    # Build with rustkyll
    echo "=== Building with rustkyll ==="
    "$PROJECT_DIR/target/release/rustkyll" build \
        --source "$SITE_DIR" \
        --destination "$RUSTKYLL_DIR" 2>&1 | tail -5
    echo ""

    # Build with Jekyll if output doesn't already exist
    if [[ ! -d "$JEKYLL_DIR" ]] || [[ -z "$(ls -A "$JEKYLL_DIR" 2>/dev/null)" ]]; then
        echo "=== Building with Jekyll ==="
        echo "Jekyll output not found at $JEKYLL_DIR, building now..."
        (
            cd "$SITE_DIR"
            bundle exec jekyll build --destination "$JEKYLL_DIR" 2>&1 | tail -5
        )
        echo ""
    else
        echo "=== Using existing Jekyll output at $JEKYLL_DIR ==="
        echo ""
    fi
fi

if [[ -z "$JEKYLL_DIR" ]] || [[ -z "$RUSTKYLL_DIR" ]]; then
    echo "Usage: $0 --site <site-path> OR --jekyll-dir <dir> --rustkyll-dir <dir>"
    echo "       $0 --validate-only --site <site-path>"
    exit 1
fi

# --- Main comparison ---

echo "=== Structural Comparison ==="
echo "Jekyll output:   $JEKYLL_DIR"
echo "Rustkyll output: $RUSTKYLL_DIR"
echo ""

# Validate rustkyll output
echo "--- Rustkyll Output Validation ---"
VALIDATE_FAIL=0
validate_output "$RUSTKYLL_DIR" "rustkyll" || VALIDATE_FAIL=1
echo ""

# 1. Compare file trees
echo "--- File Tree Comparison ---"

JEKYLL_FILES=$(find "$JEKYLL_DIR" -name "*.html" -printf '%P\n' | sort)
RUSTKYLL_FILES=$(find "$RUSTKYLL_DIR" -name "*.html" -printf '%P\n' | sort)

JEKYLL_COUNT=$(echo "$JEKYLL_FILES" | wc -l)
RUSTKYLL_COUNT=$(echo "$RUSTKYLL_FILES" | wc -l)

echo "Jekyll HTML files:   $JEKYLL_COUNT"
echo "Rustkyll HTML files: $RUSTKYLL_COUNT"

DIFF_COUNT=$((JEKYLL_COUNT > RUSTKYLL_COUNT ? JEKYLL_COUNT - RUSTKYLL_COUNT : RUSTKYLL_COUNT - JEKYLL_COUNT))
THRESHOLD=$((JEKYLL_COUNT * THRESHOLD_PCT / 100))

if [[ "$DIFF_COUNT" -gt "$THRESHOLD" ]]; then
    echo "FAIL: File count difference ($DIFF_COUNT) exceeds ${THRESHOLD_PCT}% threshold ($THRESHOLD)"
    FILE_FAIL=1
else
    echo "OK: File count within ${THRESHOLD_PCT}% tolerance"
    FILE_FAIL=0
fi

# Files only in Jekyll
ONLY_JEKYLL=$(comm -23 <(echo "$JEKYLL_FILES") <(echo "$RUSTKYLL_FILES"))
ONLY_JEKYLL_COUNT=$(echo "$ONLY_JEKYLL" | grep -c . || true)

# Files only in rustkyll
ONLY_RUSTKYLL=$(comm -13 <(echo "$JEKYLL_FILES") <(echo "$RUSTKYLL_FILES"))
ONLY_RUSTKYLL_COUNT=$(echo "$ONLY_RUSTKYLL" | grep -c . || true)

echo "Files only in Jekyll:   $ONLY_JEKYLL_COUNT"
echo "Files only in rustkyll: $ONLY_RUSTKYLL_COUNT"

if [[ "$ONLY_JEKYLL_COUNT" -gt 0 ]] && [[ "$ONLY_JEKYLL_COUNT" -le 20 ]]; then
    echo "  Missing from rustkyll:"
    echo "$ONLY_JEKYLL" | head -20 | sed 's/^/    /'
fi

echo ""

# 2. DOM tree comparison using dom_compare.py
echo "--- DOM Tree Comparison ---"

DOM_COMPARE_FAIL=0
uv run python "$SCRIPT_DIR/dom_compare.py" \
    --jekyll-dir "$JEKYLL_DIR" \
    --rustkyll-dir "$RUSTKYLL_DIR" || DOM_COMPARE_FAIL=1

echo ""

# 3. Summary
echo "=== Summary ==="
FAILED=0

if [[ "$VALIDATE_FAIL" -eq 1 ]]; then
    echo "FAIL: Rustkyll output validation failed (raw Liquid tags found)"
    FAILED=1
else
    echo "PASS: Rustkyll output validation"
fi

if [[ "$FILE_FAIL" -eq 1 ]]; then
    echo "FAIL: File count difference exceeds threshold"
    FAILED=1
else
    echo "PASS: File count within tolerance"
fi

MISSING_THRESHOLD=$((JEKYLL_COUNT * THRESHOLD_PCT / 100))
if [[ "$ONLY_JEKYLL_COUNT" -gt "$MISSING_THRESHOLD" ]]; then
    echo "FAIL: Too many files missing from rustkyll output ($ONLY_JEKYLL_COUNT > $MISSING_THRESHOLD)"
    FAILED=1
else
    echo "PASS: Missing files within tolerance"
fi

if [[ "$DOM_COMPARE_FAIL" -eq 1 ]]; then
    echo "FAIL: DOM tree comparison found differences"
    FAILED=1
else
    echo "PASS: DOM tree comparison (all common files match)"
fi

exit $FAILED
