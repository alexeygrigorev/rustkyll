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
