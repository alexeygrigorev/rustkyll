# Issue 298: mlwiki.org head element regression (0/639 -- stale Jekyll cache)

## Problem

mlwiki.org shows 0/639 DOM matches. The DOM diff report shows every single page failing with the same pattern: extra `<link>` and `<script>` elements in `<head>`.

## Root Cause Analysis

**This is NOT a rustkyll code bug. It is a stale Jekyll cache.**

The mlwiki.org site layout (`_layouts/default.html`) was updated in commit `26b287e` ("Improve design, fix MediaWiki artifacts, translate Russian pages", 2026-03-15) to add:
- `<link rel="canonical" ...>`
- Google Fonts preconnect links (3 link elements)
- highlight.js CSS and JS (1 link + 1 script)

However, the Jekyll cached output (`_site_jekyll_cached/`) was generated from the **previous** version of the layout (commit `0fb125c`) which did NOT have these elements. The cache was never regenerated after the layout change.

**Rustkyll is producing correct output** -- it renders the current layout template faithfully. The comparison is failing because it's comparing against stale Jekyll output.

### Evidence

Jekyll cached `<head>` (from old layout):
```
<meta charset>, <meta viewport>, <title>, <meta description>
<link rel="stylesheet" href="/assets/css/style.css">
<script> MathJax config </script>
<script id="MathJax-script" ...>
```

Rustkyll `<head>` (from current layout):
```
<meta charset>, <meta viewport>, <title>, <meta description>
<link rel="canonical" ...>           <-- added in 26b287e
<link preconnect fonts.googleapis>   <-- added in 26b287e
<link preconnect fonts.gstatic>      <-- added in 26b287e
<link fonts.googleapis.com/css2>     <-- added in 26b287e
<link rel="stylesheet" href="/assets/css/style.css">
<link highlightjs CSS>               <-- added in 26b287e
<script highlightjs JS>              <-- added in 26b287e
<script> MathJax config </script>
<script id="MathJax-script" ...>
```

The layout template at `websites/alexeygrigorev/mlwiki.org/_layouts/default.html` matches what rustkyll produces, confirming rustkyll is correct.

## Fix Required

Regenerate the Jekyll cached output for mlwiki.org from the current source, then re-run the DOM comparison.

## Dependencies

- Requires Jekyll to be installed (or access to a Jekyll build environment)
- No rustkyll code changes needed

## Scope

This issue is purely test infrastructure: regenerate `_site_jekyll_cached/` and update `docs/comparison/dom-details/alexeygrigorev-mlwiki.org.txt`.

After regeneration, there will likely be some remaining DOM diffs from actual content rendering differences (body content, not head elements). Those should be triaged separately -- the head element diffs affecting all 639 pages will be resolved.

## Acceptance Criteria

- [ ] Jekyll cached output for mlwiki.org is regenerated from the current source (current commit of the mlwiki.org submodule/website directory)
- [ ] The regenerated Jekyll output matches the current `_layouts/default.html` template (contains canonical link, fonts, highlight.js)
- [ ] DOM comparison is re-run after regeneration
- [ ] The head-element-ordering diffs (canonical, fonts, highlight.js) are gone from all 639 pages
- [ ] The new DOM match count is documented (expected: significant improvement from 0/639, likely not 639/639 due to body content diffs)
- [ ] `docs/comparison/dom-details/alexeygrigorev-mlwiki.org.txt` is updated with the new comparison results
- [ ] No regressions on other sites' DOM comparisons
- [ ] If body content diffs remain, they are triaged and documented (categories + count)
- [ ] `cargo test` still passes (no code changes expected, but verify)

## Test Scenarios

### Verification: Jekyll cache freshness
- Build mlwiki.org with Jekyll, inspect `_site/404.html` `<head>` section
- Confirm it contains canonical link, Google Fonts links, highlight.js link/script
- Confirm it matches rustkyll output for the same page

### Verification: DOM comparison
- Run `./scripts/compare-output.sh --site alexeygrigorev/mlwiki.org` (or equivalent)
- Verify head element diffs are eliminated
- Count remaining body-only diffs and document them

### Regression check
- Run DOM comparison on at least 2 other sites to confirm no impact
- Run `cargo test` to confirm no code regressions

## Notes

The previous score of 236/639 was likely from an earlier state where the layout was closer to the cached version. The regression to 0/639 was caused by the layout being updated in the source repo while the Jekyll cache remained stale -- NOT by any rustkyll code change.
