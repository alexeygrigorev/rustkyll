# Issue 123: Fix Google Fonts CSS rendering on alexeygrigorev.github.io

Homepage 1.61% pixel diff from Google Fonts loading difference.

## Acceptance criteria
- alexeygrigorev.github.io homepage achieves 0% pixel diff
- Google Fonts referenced correctly in HTML output

## Log

### [SWE] 2026-03-16

**Root cause analysis:**

The 1.61% pixel diff was NOT caused by Google Fonts CSS or any HTML rendering difference. The alexeygrigorev.github.io site uses system fonts (`-apple-system, BlinkMacSystemFont, "Segoe UI"...`) and Font Awesome from CDN -- no Google Fonts at all.

The actual root causes were:
1. **Randomized contribution graph** (primary, ~1.3% of diff): The site's `tabs.js` uses `Math.random()` to generate a contribution graph on every page load. Since Playwright visits Jekyll and rustkyll in separate page loads, the random graph was always different.
2. **Sub-pixel font rendering noise** (secondary, ~0.3%): Non-deterministic Chromium font anti-aliasing between separate browser contexts.

The only HTML difference between Jekyll and rustkyll output was a single extra blank line before `{{ content }}` insertion (whitespace-only, zero visual impact).

**Fix applied:**

Added `seedMathRandom()` function to `playwright/tests/visual-compare.spec.ts` that injects a deterministic PRNG (mulberry32, seed=42) via `page.addInitScript()` before visiting any page. This replaces `Math.random()` with a deterministic function that produces the same sequence on every page load. Both the Jekyll and rustkyll screenshots now use identical random values, eliminating the contribution graph diff entirely.

The `addInitScript` runs on every `page.goto()` call, so the seed resets for each page visit, ensuring both Jekyll and rustkyll get the same random sequence.

**Result:** alexeygrigorev.github.io homepage achieves exactly 0.00% pixel diff (0 differing pixels out of 1,580,800).

**Test added:** `test_alexeygrigorev_site_builds_and_has_correct_content` in `tests/integration_page_counts.rs` -- verifies the site builds correctly, produces 8 HTML pages, has Font Awesome CDN link, main.css reference, rendered content (no raw Liquid), and all expected output files.

**Build results:** 1488 tests pass, 0 fail, 43 ignored. Clippy clean, fmt clean.

**Files modified:**
- `playwright/tests/visual-compare.spec.ts` -- added `seedMathRandom()` and call it before each page visit
- `tests/integration_page_counts.rs` -- added alexeygrigorev site build verification test
- `docs/tracker/123-fix-google-fonts-css.in-progress.md` -- this file
