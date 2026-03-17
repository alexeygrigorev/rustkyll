#!/usr/bin/env bash
# Recount DOM matches for all benchmark sites.
# Builds both Jekyll and rustkyll, runs dom_compare.py, outputs fresh stats.
#
# Usage:
#   ./scripts/recount-all-dom.sh                    # all sites
#   ./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io  # one site
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WEBSITES_DIR="$PROJECT_DIR/websites"
RUSTKYLL="$PROJECT_DIR/target/release/rustkyll"
OUTPUT_FILE="$PROJECT_DIR/docs/dom-recount-results.md"
DETAILS_DIR="$PROJECT_DIR/docs/comparison/dom-details"
SINGLE_SITE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --site) SINGLE_SITE="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Build rustkyll if needed
if [[ ! -x "$RUSTKYLL" ]]; then
    echo "Building rustkyll..."
    cd "$PROJECT_DIR" && ./scripts/cargo-safe build --release
fi

# Collect all sites with _config.yml
declare -a SITES=()

if [[ -n "$SINGLE_SITE" ]]; then
    SITES+=("$SINGLE_SITE")
else
    for org_dir in "$WEBSITES_DIR"/*/; do
        org=$(basename "$org_dir")
        if [[ -f "$org_dir/_config.yml" ]]; then
            SITES+=("$org")
        else
            for repo_dir in "$org_dir"/*/; do
                repo=$(basename "$repo_dir")
                if [[ -f "$repo_dir/_config.yml" ]]; then
                    SITES+=("$org/$repo")
                elif [[ -d "$repo_dir" ]]; then
                    for sub_dir in "$repo_dir"/*/; do
                        sub=$(basename "$sub_dir")
                        if [[ -f "$sub_dir/_config.yml" ]]; then
                            SITES+=("$org/$repo/$sub")
                        fi
                    done
                fi
            done
        fi
    done
fi

echo "Found ${#SITES[@]} sites"
echo ""

mkdir -p "$DETAILS_DIR"

# Results arrays
declare -a RESULTS=()
declare -A DIFF_CATEGORIES=()
TIMESTAMP=$(date -u '+%Y-%m-%d %H:%M UTC')

for site in "${SITES[@]}"; do
    site_dir="$WEBSITES_DIR/$site"
    echo "=== $site ==="

    if [[ ! -f "$site_dir/_config.yml" ]]; then
        echo "  SKIP: no _config.yml"
        RESULTS+=("$site|SKIP|no _config.yml|||")
        continue
    fi
    src_dir="$site_dir"

    # Build with Jekyll
    jekyll_site="$src_dir/_site_jekyll_recount"
    rm -rf "$jekyll_site"

    echo "  Building with Jekyll..."
    jekyll_ok=0
    if [[ -f "$src_dir/Gemfile.lock" ]]; then
        cd "$src_dir"
        if timeout 120 bundle exec jekyll build --destination "$jekyll_site" --quiet 2>/dev/null; then
            jekyll_ok=1
        fi
        cd "$PROJECT_DIR"
    else
        if timeout 120 jekyll build --source "$src_dir" --destination "$jekyll_site" --quiet 2>/dev/null; then
            jekyll_ok=1
        fi
    fi

    if [[ "$jekyll_ok" -eq 0 ]]; then
        echo "  Jekyll FAILED"
        rm -rf "$jekyll_site"
        RESULTS+=("$site|JEKYLL_FAIL||||")
        continue
    fi

    # Build with rustkyll
    rustkyll_site="$src_dir/_site_rustkyll_recount"
    rm -rf "$rustkyll_site"

    echo "  Building with rustkyll..."
    if ! timeout 120 "$RUSTKYLL" build --source "$src_dir" --destination "$rustkyll_site" 2>/dev/null; then
        echo "  rustkyll FAILED"
        rm -rf "$jekyll_site" "$rustkyll_site"
        RESULTS+=("$site|RUSTKYLL_FAIL||||")
        continue
    fi

    # Count HTML files
    jekyll_count=$(find "$jekyll_site" -name "*.html" | wc -l)
    rustkyll_count=$(find "$rustkyll_site" -name "*.html" | wc -l)

    # Count common HTML files
    jekyll_files=$(cd "$jekyll_site" && find . -name "*.html" | sort)
    rustkyll_files=$(cd "$rustkyll_site" && find . -name "*.html" | sort)
    common_count=$(comm -12 <(echo "$jekyll_files") <(echo "$rustkyll_files") | grep -c '.' || true)

    # Count Liquid leaks in rustkyll output
    liquid_leaks=0
    while IFS= read -r f; do
        [[ -z "$f" ]] && continue
        if grep -qE '\{%|\{\{' "$rustkyll_site/$f" 2>/dev/null; then
            liquid_leaks=$((liquid_leaks + 1))
        fi
    done <<< "$(cd "$rustkyll_site" && find . -name '*.html')"

    if [[ "$common_count" -eq 0 ]]; then
        echo "  No common HTML files"
        rm -rf "$jekyll_site" "$rustkyll_site"
        RESULTS+=("$site|0|0|$jekyll_count|$rustkyll_count|$liquid_leaks")
        continue
    fi

    # Run DOM comparison and save full output
    echo "  Running DOM comparison on $common_count common files..."
    site_slug=$(echo "$site" | tr '/' '-')
    detail_file="$DETAILS_DIR/${site_slug}.txt"

    dom_output=$(cd "$PROJECT_DIR" && uv run scripts/dom_compare.py \
        --jekyll-dir "$jekyll_site" \
        --rustkyll-dir "$rustkyll_site" 2>&1) || true

    echo "$dom_output" > "$detail_file"

    # Parse DOM match count from summary line
    dom_match=0
    summary_line=$(echo "$dom_output" | grep "^Summary:" || true)
    if [[ -n "$summary_line" ]]; then
        dom_match=$(echo "$summary_line" | grep -oP '(\d+) files matched' | grep -oP '^\d+' || echo 0)
    fi

    # Extract diff categories (type counts) - only match known diff types
    diff_cats=$(echo "$dom_output" | grep "^  " | grep -oP ': \K(text_differs|attribute_differs|tag_name_differs|missing_element|extra_element|missing_text|extra_text|missing_attribute|extra_attribute|expected_element_got_text|expected_text_got_element|jsonld_value_differs|jsonld_missing_field|jsonld_extra_field|jsonld_extra_item|jsonld_missing_item)' | sort | uniq -c | sort -rn)
    DIFF_CATEGORIES["$site"]="$diff_cats"

    echo "  DOM: $dom_match/$common_count | Files: $rustkyll_count/$jekyll_count | Liquid leaks: $liquid_leaks"

    RESULTS+=("$site|$dom_match|$common_count|$jekyll_count|$rustkyll_count|$liquid_leaks")

    # Clean up build outputs
    rm -rf "$jekyll_site" "$rustkyll_site"
done

# Write results
echo ""
echo "Writing results to $OUTPUT_FILE..."

cat > "$OUTPUT_FILE" << HEADER
# DOM Comparison Results

Generated: $TIMESTAMP

rustkyll version: $("$RUSTKYLL" --version 2>&1 || echo "unknown")

## How to run

\`\`\`bash
# Recount all sites
./scripts/recount-all-dom.sh

# Recount a single site
./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io
\`\`\`

Prerequisites: Jekyll (via Ruby/Bundler), rustkyll (built via \`cargo build --release\`), and \`uv\` (for running dom_compare.py with its beautifulsoup4 dependency).

The script builds both Jekyll and rustkyll for each site, runs DOM comparison via \`scripts/dom_compare.py\`, and writes results here. Per-site diff details are saved in \`docs/comparison/dom-details/\`.

## All Sites

| Site | DOM Match | File Match | Liquid Leaks |
|------|-----------|------------|-------------|
HEADER

total_dom=0
total_common=0
total_sites=0

for r in "${RESULTS[@]}"; do
    IFS='|' read -r site f1 f2 f3 f4 f5 <<< "$r"

    if [[ "$f1" == SKIP* ]] || [[ "$f1" == JEKYLL_FAIL* ]] || [[ "$f1" == RUSTKYLL_FAIL* ]]; then
        echo "| $site | $f1 | - | - |" >> "$OUTPUT_FILE"
        continue
    fi

    dom_match="$f1"
    common_count="$f2"
    jekyll_count="$f3"
    rustkyll_count="$f4"
    liquid_leaks="$f5"

    if [[ "$common_count" -eq 0 ]]; then
        pct="N/A"
    else
        pct=$(awk "BEGIN {printf \"%.0f\", ($dom_match/$common_count)*100}")
    fi

    echo "| $site | $dom_match/$common_count ($pct%) | $rustkyll_count/$jekyll_count | $liquid_leaks |" >> "$OUTPUT_FILE"

    if [[ "$dom_match" =~ ^[0-9]+$ ]] && [[ "$common_count" =~ ^[0-9]+$ ]]; then
        total_dom=$((total_dom + dom_match))
        total_common=$((total_common + common_count))
        total_sites=$((total_sites + 1))
    fi
done

cat >> "$OUTPUT_FILE" << EOF

## Summary

- Sites compared: $total_sites
- Total DOM matches: $total_dom / $total_common

## Diff Categories by Site

EOF

for site in "${!DIFF_CATEGORIES[@]}"; do
    cats="${DIFF_CATEGORIES[$site]}"
    if [[ -z "$cats" ]]; then
        continue
    fi
    echo "### $site" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo '```' >> "$OUTPUT_FILE"
    echo "$cats" >> "$OUTPUT_FILE"
    echo '```' >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"
echo "Per-site full diff output is in \`docs/comparison/dom-details/\`." >> "$OUTPUT_FILE"

echo ""
echo "Done! Results in $OUTPUT_FILE"
echo "Per-site details in $DETAILS_DIR/"
