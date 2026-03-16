# Issue 141: Fix heading ID generation (double-dash and ampersand handling)

## Problem

Kramdown generates heading IDs differently from rustkyll in two ways:

1. **Double-dash preservation**: When a heading contains special characters that map to dashes (e.g., `DevOps / Site Reliability Engineer`), kramdown preserves double dashes (`devops--site-reliability-engineer`) while rustkyll collapses them to single dash (`devops-site-reliability-engineer`). 12 instances across 3 files.

2. **Ampersand in IDs**: Kramdown keeps `--` for `&` in headings (e.g., `free--free-to-audit-courses`) while rustkyll converts `&` to `amp` in the ID (e.g., `free-amp-free-to-audit-courses`). 7 instances across 2 files.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Heading IDs match kramdown output for special characters
- Double dashes preserved where kramdown preserves them
- Ampersand handling matches kramdown behavior
- No regressions

## Log

### [SWE] 2026-03-16
- Root cause: `slugify()` in `src/kramdown.rs` did not match kramdown's `generate_id` algorithm
  - Old code collapsed consecutive dashes (`!slug.ends_with('-')` guard)
  - Old code stripped all non-alphanumeric chars including `/` and `&` without preserving the dash from adjacent spaces
- Fix: Rewrote `slugify()` to match kramdown's exact algorithm:
  1. Strip leading non-alpha characters
  2. Keep only `[a-zA-Z0-9 -]`, remove everything else
  3. Replace spaces with hyphens (no collapsing)
  4. Downcase
- Updated 1 existing test (`test_slugify_hyphens`) to match kramdown behavior
- Added 4 new tests: `test_slugify_slash_preserves_double_dash`, `test_slugify_ampersand_preserves_double_dash`, `test_slugify_leading_non_alpha_stripped`, `test_slugify_trailing_chars_preserved`
- Build: 1238 passed, 2 failed (pre-existing failures in `ol start` tests, unrelated), clippy clean, fmt clean
- Files modified: `src/kramdown.rs`
