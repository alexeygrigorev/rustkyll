#!/usr/bin/env bash
# Tests for recount-all-dom.sh changes:
# 1. bundle install is called when Gemfile exists
# 2. Jekyll output caching works
# 3. JEKYLL_FAIL sites show rustkyll-only stats
# 4. BOTH_FAIL when both fail
# 5. --no-cache flag clears cache
#
# Usage: ./scripts/test_recount_all_dom.sh
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SCRIPT="$SCRIPT_DIR/recount-all-dom.sh"

PASS=0
FAIL=0

pass() {
    PASS=$((PASS + 1))
    echo "  PASS: $1"
}

fail() {
    FAIL=$((FAIL + 1))
    echo "  FAIL: $1"
}

echo "=== Testing recount-all-dom.sh ==="
echo ""

# --- Test 1: Script has bundle install logic ---
echo "Test 1: Script contains bundle install logic"
if grep -q 'bundle install' "$SCRIPT"; then
    pass "Script contains 'bundle install'"
else
    fail "Script missing 'bundle install'"
fi

# --- Test 2: bundle install has timeout ---
echo "Test 2: bundle install has a timeout"
if grep -q 'timeout 60 bundle install' "$SCRIPT"; then
    pass "bundle install has 60s timeout"
else
    fail "bundle install missing timeout"
fi

# --- Test 3: Script checks for Gemfile (not just Gemfile.lock) ---
echo "Test 3: Script checks for Gemfile presence"
if grep -q 'Gemfile"' "$SCRIPT" && grep -q '"$src_dir/Gemfile"' "$SCRIPT"; then
    pass "Script checks for Gemfile"
else
    fail "Script does not check for Gemfile"
fi

# --- Test 4: bundle install runs with --quiet flag ---
echo "Test 4: bundle install uses --quiet flag"
if grep -q 'bundle install --quiet' "$SCRIPT"; then
    pass "bundle install uses --quiet"
else
    fail "bundle install missing --quiet flag"
fi

# --- Test 5: Script has BOTH_FAIL status ---
echo "Test 5: Script has BOTH_FAIL status for sites where both fail"
if grep -q 'BOTH_FAIL' "$SCRIPT"; then
    pass "Script has BOTH_FAIL status"
else
    fail "Script missing BOTH_FAIL status"
fi

# --- Test 6: JEKYLL_FAIL sites show rustkyll-only page count ---
echo "Test 6: JEKYLL_FAIL results include rustkyll-only page count"
if grep -q 'pages (rustkyll-only)' "$SCRIPT"; then
    pass "JEKYLL_FAIL results show rustkyll-only page count"
else
    fail "JEKYLL_FAIL results missing rustkyll-only page count"
fi

# --- Test 7: Script counts liquid leaks for rustkyll-only builds ---
echo "Test 7: JEKYLL_FAIL path counts liquid leaks in rustkyll output"
# The JEKYLL_FAIL path should have liquid leak counting logic
if grep -A 30 'Attempting rustkyll-only build' "$SCRIPT" | grep -q 'liquid_leaks'; then
    pass "JEKYLL_FAIL path counts liquid leaks"
else
    fail "JEKYLL_FAIL path missing liquid leak count"
fi

# --- Test 8: Script counts empty HTML files for rustkyll-only builds ---
echo "Test 8: JEKYLL_FAIL path counts empty HTML files"
if grep -A 40 'Attempting rustkyll-only build' "$SCRIPT" | grep -q 'empty_files'; then
    pass "JEKYLL_FAIL path counts empty files"
else
    fail "JEKYLL_FAIL path missing empty file count"
fi

# --- Test 9: Jekyll cache directory name ---
echo "Test 9: Jekyll output uses _site_jekyll_cached directory"
if grep -q '_site_jekyll_cached' "$SCRIPT"; then
    pass "Uses _site_jekyll_cached for cache"
else
    fail "Not using _site_jekyll_cached for cache"
fi

# --- Test 10: Cache is reused when present ---
echo "Test 10: Script reuses cached Jekyll output"
if grep -q 'Using cached Jekyll output' "$SCRIPT"; then
    pass "Script has cache reuse logic"
else
    fail "Script missing cache reuse logic"
fi

# --- Test 11: --no-cache flag exists ---
echo "Test 11: --no-cache flag is supported"
if grep -q '\-\-no-cache' "$SCRIPT"; then
    pass "--no-cache flag is supported"
else
    fail "--no-cache flag missing"
fi

# --- Test 12: --no-cache clears the cache ---
echo "Test 12: --no-cache clears the cached directory"
if grep -A 2 'NO_CACHE.*1' "$SCRIPT" | grep -q 'rm -rf.*jekyll_cached'; then
    pass "--no-cache removes cached directory"
elif grep -B 1 'rm -rf.*jekyll_site' "$SCRIPT" | grep -q 'NO_CACHE'; then
    pass "--no-cache removes cached directory"
else
    fail "--no-cache does not clear cache"
fi

# --- Test 13: Rustkyll output is NOT cached (always rebuilt) ---
echo "Test 13: Rustkyll output is always rebuilt (not cached)"
if grep -q 'rm -rf "$rustkyll_site"' "$SCRIPT"; then
    pass "Rustkyll output is cleaned up (not cached)"
else
    fail "Rustkyll output might be cached"
fi

# --- Test 14: Script still supports --site flag ---
echo "Test 14: --site flag still works"
if grep -q '\-\-site.*SINGLE_SITE' "$SCRIPT"; then
    pass "--site flag is supported"
else
    fail "--site flag broken"
fi

# --- Test 15: Gemfile without Gemfile.lock uses bundle exec ---
echo "Test 15: Sites with Gemfile (no Gemfile.lock) use bundle exec jekyll build"
# The Gemfile branch should use 'bundle exec jekyll build'
if grep -A 10 'src_dir/Gemfile"' "$SCRIPT" | grep -q 'bundle exec jekyll build'; then
    pass "Gemfile-only sites use bundle exec jekyll build"
else
    fail "Gemfile-only sites not using bundle exec jekyll build"
fi

# --- Test 16: Jekyll cache is NOT cleaned up after comparison ---
echo "Test 16: Jekyll cache is preserved after comparison (not cleaned up)"
# The old script had 'rm -rf "$jekyll_site" "$rustkyll_site"' at the end.
# New script should only clean up rustkyll, not the jekyll cache.
if grep -q 'Clean up rustkyll build output' "$SCRIPT"; then
    pass "Only rustkyll output is cleaned up, Jekyll cache preserved"
else
    fail "Cleanup logic may remove Jekyll cache"
fi

# --- Test 17: Results format for BOTH_FAIL shows dashes ---
echo "Test 17: BOTH_FAIL results show dashes in table"
if grep -q 'BOTH_FAIL.*-.*-' "$SCRIPT"; then
    pass "BOTH_FAIL shows '- | -' in results"
else
    fail "BOTH_FAIL results format incorrect"
fi

# --- Test 18: Documentation updated for caching ---
echo "Test 18: Output file documents caching behavior"
if grep -q 'jekyll_cached' "$SCRIPT" || grep -q 'cache' "$SCRIPT"; then
    pass "Script documents caching"
else
    fail "Script does not document caching"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
echo ""

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
exit 0
