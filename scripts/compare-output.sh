#!/usr/bin/env bash
# compare-output.sh -- Structural comparison between Jekyll and rustkyll output.
#
# Usage:
#   ./scripts/compare-output.sh --site DataTalksClub/datatalksclub.github.io
#   ./scripts/compare-output.sh --site alexeygrigorev/kids-horror-stories-ru
#   ./scripts/compare-output.sh --jekyll-dir /path/to/jekyll/output --rustkyll-dir /path/to/rustkyll/output
#
# This script compares:
# 1. File tree: lists of generated HTML files
# 2. Structural elements: title, h1-h6, links, images per page
# 3. Reports differences and exits nonzero if thresholds exceeded.
#
# Thresholds:
# - File count difference: 5% tolerance
# - Per-page structural diff: fail if >50% of sampled files differ
# - Missing files: fail if >5% of files are missing from either side

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

SITE=""
JEKYLL_DIR=""
RUSTKYLL_DIR=""
THRESHOLD_PCT=5

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
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# --- Helper functions (defined before use) ---

extract_structural_elements() {
    local file="$1"
    # Extract title, headings, links, and images using grep/sed
    # This is a lightweight structural extraction without requiring Python/Node
    # Note: each grep pipeline uses "|| true" to avoid pipefail exit on no-match
    {
        # Title
        (grep -oiP '<title[^>]*>\K[^<]+' "$file" 2>/dev/null || true) | head -1 | sed 's/^/TITLE: /'

        # Headings h1-h6
        (grep -oiP '<h[1-6][^>]*>\K[^<]+' "$file" 2>/dev/null || true) | sed 's/^/HEADING: /'

        # Links (href values)
        (grep -oiP 'href="\K[^"]+' "$file" 2>/dev/null || true) | sort -u | sed 's/^/LINK: /'

        # Images (src values)
        (grep -oiP '<img[^>]*src="\K[^"]+' "$file" 2>/dev/null || true) | sort -u | sed 's/^/IMG: /'
    } | sort
}

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

# 2. Compare structural elements for common files
echo "--- Structural Element Comparison ---"

COMMON_FILES=$(comm -12 <(echo "$JEKYLL_FILES") <(echo "$RUSTKYLL_FILES"))
COMMON_COUNT=$(echo "$COMMON_FILES" | grep -c . || true)
echo "Common files to compare: $COMMON_COUNT"

STRUCT_DIFFS=0
SAMPLE_DIFFS=""

# Compare a sample of files (up to 50)
SAMPLE_COUNT=0
while IFS= read -r rel_path; do
    [[ -z "$rel_path" ]] && continue
    SAMPLE_COUNT=$((SAMPLE_COUNT + 1))
    [[ "$SAMPLE_COUNT" -gt 50 ]] && break

    JEKYLL_FILE="$JEKYLL_DIR/$rel_path"
    RUSTKYLL_FILE="$RUSTKYLL_DIR/$rel_path"

    JEKYLL_STRUCT=$(extract_structural_elements "$JEKYLL_FILE")
    RUSTKYLL_STRUCT=$(extract_structural_elements "$RUSTKYLL_FILE")

    if [[ "$JEKYLL_STRUCT" != "$RUSTKYLL_STRUCT" ]]; then
        STRUCT_DIFFS=$((STRUCT_DIFFS + 1))
        if [[ "$STRUCT_DIFFS" -le 5 ]]; then
            SAMPLE_DIFFS="${SAMPLE_DIFFS}\n--- $rel_path ---\n"
            SAMPLE_DIFFS="${SAMPLE_DIFFS}$(diff <(echo "$JEKYLL_STRUCT") <(echo "$RUSTKYLL_STRUCT") | head -20 || true)\n"
        fi
    fi
done <<< "$COMMON_FILES"

echo "Files compared:    $SAMPLE_COUNT"
echo "Files with diffs:  $STRUCT_DIFFS"

if [[ "$STRUCT_DIFFS" -gt 0 ]] && [[ -n "$SAMPLE_DIFFS" ]]; then
    echo ""
    echo "Sample structural differences:"
    echo -e "$SAMPLE_DIFFS"
fi

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

if [[ "$STRUCT_DIFFS" -gt "$((SAMPLE_COUNT / 2))" ]]; then
    echo "FAIL: More than half of sampled files have structural differences ($STRUCT_DIFFS/$SAMPLE_COUNT)"
    FAILED=1
else
    echo "PASS: Structural differences within tolerance ($STRUCT_DIFFS/$SAMPLE_COUNT)"
fi

exit $FAILED
