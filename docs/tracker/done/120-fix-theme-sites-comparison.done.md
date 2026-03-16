# Issue 120: Fix comparison for theme-based sites (missing HTML root)

## Problem

Several sites produce HTML without proper `<html>` root element or with minimal/invalid structure. The visual comparison scripts skip these sites, but they should still be compared.

Affected sites:
- DataTalksClub/docs (just-the-docs theme)
- Other theme sites with no valid homepage

## Goal

Make DOM and visual comparison work for ALL sites, even those with minimal/invalid HTML:
1. DOM comparison (dom_compare.py) should work with HTML fragments — BeautifulSoup handles this
2. Playwright should serve and screenshot even without `<html>` — browsers render fragments
3. The comparison scripts should not skip sites just because the HTML is "invalid"

## Approach

1. Check why visual-compare.sh skips these sites
2. Remove the "valid HTML" check or make it more lenient
3. Run DOM comparison on these sites
4. Run Playwright comparison (browser will render whatever HTML exists)

## Acceptance criteria

- DataTalksClub/docs compared (DOM + visual) against Jekyll
- No sites skipped due to "invalid HTML" — compare everything
- Results documented

## Log

### [SWE] 2026-03-16

**Root cause:** Three issues prevented comparison of theme sites with minimal/invalid HTML:

1. `visual-compare.sh` had a `verify_html()` function (lines 191-205) that grepped the HTTP response for `<html` tag and exited with error if not found. Theme sites producing HTML fragments (no `<html>` root) would fail this check.

2. All three comparison shell scripts (`visual-compare.sh`, `batch-visual-compare.sh`, `visual-compare-site.sh`) used `curl -sf` for server readiness checks. The `-f` flag makes curl return non-zero on HTTP 404. Sites without an `index.html` at root would return 404, causing the readiness loop to time out or skip the site.

3. `dom_compare.py` already handled fragments correctly (falls back to soup root if no `<html>` tag found). No changes needed there.

**Fixes applied:**

1. **Removed `verify_html()` check** from `visual-compare.sh` -- replaced with a comment explaining why we intentionally do not check for valid HTML structure. Browsers and BeautifulSoup both handle fragments fine.

2. **Fixed server readiness checks** in all three scripts -- changed from `curl -sf` (fails on 404) to `curl -so /dev/null -w '%{http_code}'` which accepts any HTTP response code. A "000" code means the server is not yet listening; any other code (200, 404, etc.) means it is ready.

**Tests added:** 8 new Python tests in `TestHTMLFragments` class:
- `test_fragment_identical` -- identical fragments produce 0 diffs
- `test_fragment_with_differences` -- text diffs detected in fragments
- `test_fragment_multiple_roots` -- multiple root-level elements work
- `test_fragment_vs_fragment_missing_element` -- missing elements detected
- `test_bare_text_fragment` -- bare text (no tags) handled
- `test_fragment_with_doctype_only` -- doctype without `<html>` works
- `test_fragment_file_comparison` -- file-level comparison works with fragments
- `test_fragment_directory_comparison` -- directory-level comparison works with fragments

**Test results:** All 38 Python tests pass (30 existing + 8 new). All Rust tests pass. Clippy clean, fmt clean.

**Files modified:**
- `scripts/visual-compare.sh` -- removed verify_html, fixed server readiness check
- `scripts/batch-visual-compare.sh` -- fixed server readiness check
- `scripts/visual-compare-site.sh` -- fixed server readiness check
- `scripts/test_dom_compare.py` -- added 8 HTML fragment tests
