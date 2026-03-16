#!/usr/bin/env bash
# batch-visual-compare.sh -- Run visual comparison on multiple sites.
#
# Usage:
#   ./scripts/batch-visual-compare.sh [site1 site2 ...]
#
# If no sites specified, runs on all new sites from issue 82.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLAYWRIGHT_DIR="$PROJECT_DIR/playwright"

# Default sites to test (issue 82 new sites)
DEFAULT_SITES=(
  just-the-docs
  mojombo-blog
  cayman-theme
  slate-theme
  leap-day-theme
  midnight-theme
  hacker-theme
  architect-theme
  time-machine-theme
  merlot-theme
  dinky-theme
)

if [[ $# -gt 0 ]]; then
  SITES=("$@")
else
  SITES=("${DEFAULT_SITES[@]}")
fi

RESULTS_FILE="/tmp/visual-compare-results.md"
echo "# Visual Comparison Results" > "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "| Site | Pages Tested | Pass | Fail | Max Diff % |" >> "$RESULTS_FILE"
echo "|------|-------------|------|------|------------|" >> "$RESULTS_FILE"

OVERALL_PASS=0
OVERALL_FAIL=0

for site in "${SITES[@]}"; do
  echo ""
  echo "============================================"
  echo "=== Visual comparing: $site"
  echo "============================================"

  JEKYLL_DIR="/tmp/jekyll-$site"
  RUSTKYLL_DIR="/tmp/rustkyll-$site"

  if [[ ! -d "$JEKYLL_DIR" ]] || [[ ! -d "$RUSTKYLL_DIR" ]]; then
    echo "SKIP: Missing output directories for $site"
    echo "| $site | - | - | SKIP | N/A |" >> "$RESULTS_FILE"
    continue
  fi

  # Find available ports
  JEKYLL_PORT=$((4100 + RANDOM % 100))
  RUSTKYLL_PORT=$((4200 + RANDOM % 100))

  # Start HTTP servers
  python3 -m http.server "$JEKYLL_PORT" --directory "$JEKYLL_DIR" --bind 127.0.0.1 &>/dev/null &
  JEKYLL_PID=$!
  python3 -m http.server "$RUSTKYLL_PORT" --directory "$RUSTKYLL_DIR" --bind 127.0.0.1 &>/dev/null &
  RUSTKYLL_PID=$!

  # Wait for servers (accept any HTTP response, not just 200,
  # since sites without index.html return 404 but still work)
  for i in $(seq 1 10); do
    J_CODE=$(curl -so /dev/null -w '%{http_code}' --max-time 2 "http://localhost:$JEKYLL_PORT/" 2>/dev/null || echo "000")
    R_CODE=$(curl -so /dev/null -w '%{http_code}' --max-time 2 "http://localhost:$RUSTKYLL_PORT/" 2>/dev/null || echo "000")
    if [[ "$J_CODE" != "000" ]] && [[ "$R_CODE" != "000" ]]; then
      break
    fi
    sleep 0.5
  done

  # Create screenshot dir
  SCREENSHOT_DIR="$PLAYWRIGHT_DIR/screenshots/$site"
  mkdir -p "$SCREENSHOT_DIR"

  # Run Playwright
  TEST_EXIT=0
  TEST_OUTPUT=$(cd "$PLAYWRIGHT_DIR" && \
    JEKYLL_URL="http://localhost:$JEKYLL_PORT" \
    RUSTKYLL_URL="http://localhost:$RUSTKYLL_PORT" \
    SCREENSHOT_DIR="$SCREENSHOT_DIR" \
    DIFF_THRESHOLD="0.0" \
    SITE_NAME="$site" \
    npx playwright test 2>&1) || TEST_EXIT=$?

  # Parse results
  PASS_COUNT=$(echo "$TEST_OUTPUT" | grep -c "passed" || echo "0")
  FAIL_COUNT=$(echo "$TEST_OUTPUT" | grep -c "failed" || echo "0")

  # Extract diff percentages
  MAX_DIFF="0.00"
  while IFS= read -r line; do
    if [[ "$line" =~ diff=([0-9]+\.[0-9]+)% ]]; then
      DIFF_VAL="${BASH_REMATCH[1]}"
      if (( $(echo "$DIFF_VAL > $MAX_DIFF" | bc -l) )); then
        MAX_DIFF="$DIFF_VAL"
      fi
    fi
  done <<< "$TEST_OUTPUT"

  # Count actual pass/fail from test output
  TESTS_PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+ passed' | head -1 | grep -oP '\d+' || echo "0")
  TESTS_FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+ failed' | head -1 | grep -oP '\d+' || echo "0")
  TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))

  if [[ "$TEST_EXIT" -eq 0 ]]; then
    STATUS="PASS"
    OVERALL_PASS=$((OVERALL_PASS + 1))
  else
    STATUS="FAIL"
    OVERALL_FAIL=$((OVERALL_FAIL + 1))
  fi

  echo "  Result: $STATUS (${TESTS_PASSED}/${TOTAL_TESTS} passed, max diff: ${MAX_DIFF}%)"
  echo "| $site | $TOTAL_TESTS | $TESTS_PASSED | $TESTS_FAILED | ${MAX_DIFF}% |" >> "$RESULTS_FILE"

  # Print detailed output for failures
  if [[ "$TEST_EXIT" -ne 0 ]]; then
    echo "$TEST_OUTPUT" | grep -E "diff=|FAIL|Error|failed" | head -20
  fi

  # Clean up servers
  kill "$JEKYLL_PID" 2>/dev/null || true
  kill "$RUSTKYLL_PID" 2>/dev/null || true
  wait "$JEKYLL_PID" 2>/dev/null || true
  wait "$RUSTKYLL_PID" 2>/dev/null || true
done

echo "" >> "$RESULTS_FILE"
echo "**Overall: $OVERALL_PASS passed, $OVERALL_FAIL failed out of ${#SITES[@]} sites**" >> "$RESULTS_FILE"

echo ""
echo "============================================"
echo "=== OVERALL RESULTS"
echo "============================================"
echo "Sites passed: $OVERALL_PASS"
echo "Sites failed: $OVERALL_FAIL"
echo "Total sites: ${#SITES[@]}"
echo ""
echo "Detailed results: $RESULTS_FILE"
cat "$RESULTS_FILE"
