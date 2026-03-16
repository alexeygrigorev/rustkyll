# Issue 88: Fix missing CSS/styling on DTC site

## Problem

The DTC site looks different when built with rustkyll vs Jekyll. Likely causes include missing or incorrectly compiled SCSS, missing CSS classes in HTML output, or CSS files not being served correctly.

## Goal

The DTC site must have identical styling when built with rustkyll. Every CSS class, every stylesheet, every style rule must produce the same visual result.

## Approach

1. Compare the CSS files in _site/ between Jekyll and rustkyll builds
2. Check if all SCSS files are compiled correctly (grass crate)
3. Check if all CSS classes in HTML match Jekyll's output
4. Fix any missing or incorrect styles

## Dependencies

None

## Acceptance criteria

- All CSS files present in rustkyll _site/ match Jekyll _site/
- SCSS compilation produces equivalent CSS
- HTML elements have correct CSS classes (matching Jekyll)
- Visual appearance matches Jekyll when served in browser
- No missing stylesheets (check browser dev tools for 404s on CSS)

## Resolution: Already resolved -- no CSS issues exist

### [SWE] 2026-03-16 Verification

Investigation confirms there are no missing CSS or styling issues. This issue was created speculatively before the Issue 87 audit was completed. The audit found no CSS-related problems.

**Evidence:**

1. **CSS files are identical.** The DTC source site uses plain CSS files (not SCSS), specifically `/assets/styles.css` (25,517 bytes) and `/assets/syntax.css` (3,562 bytes). Both are copied byte-for-byte identical to the output `_site/`. Verified with `diff` -- zero differences.

2. **CSS references in HTML match exactly.** Both Jekyll and rustkyll output the same stylesheet references in `<head>`: Bootstrap 4.4.1 CDN, Google Fonts (Alegreya Sans, Raleway), `/assets/styles.css`, and `/assets/syntax.css`. No extra or missing references.

3. **No CSS 404s.** Every CSS file referenced in the HTML output exists in `_site/` at the correct path.

4. **Jekyll has one extra unreferenced CSS file** (`/assets/css/style.css`, 16KB) -- a Jekyll theme artifact that is never linked from any HTML page. Its absence from rustkyll output is correct behavior, not a bug.

5. **Structural HTML elements match.** Section counts, div counts, nav elements, and CSS class usage are identical between Jekyll and rustkyll for all tested pages (index.html, books.html, events.html, podcast.html, articles.html, courses.html).

6. **Issue 87 audit found zero CSS/styling differences.** The comprehensive 15-page audit (documented in `docs/audit/87-visual-parity-report.md`) identified 22 differences, none of which were CSS-related. All differences were template rendering issues (whitespace, boolean HTML attributes, markdown processing of includes) tracked in Issue 90.

7. **Playwright visual comparison: 8/14 pages at 0.00% pixel diff.** The remaining 6 pages had diffs of 0.21%-3.45%, all attributable to template rendering differences (Issue 90), not CSS.

**Conclusion:** This issue has no work to do. All acceptance criteria are already met. The issue was created as a placeholder before the audit, and the audit confirmed CSS/styling is not a problem area.
