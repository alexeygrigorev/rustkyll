# Issue 77: Fix slug generation producing URLs with spaces

**STATUS: Already fixed by issue #70. No further work required.**

## Problem

Discovered in issue #63 sitemap comparison: two sitemap URLs contain spaces instead of hyphens in their slugs:

1. `https://datatalks.club/podcast/production-ml-search-vector-search-embeddings-hybrid search.html` -- should be `hybrid-search`
2. `https://datatalks.club/people/ aashishnair.html` -- leading space, should be `aashishnair`

Jekyll produces the correct slugs for both of these pages.

## Evidence

From `docs/comparison/feed-sitemap-results.md` DTC sitemap comparison:
- 8 rustkyll-only URLs, 2 of which are duplicates of Jekyll URLs but with malformed slugs

## Resolution: Already Fixed in Issue #70

Issue #70 (Fix missing pages in DTC site build) identified these exact two slug problems as root cause A and implemented `sanitize_slug()` in `src/collection.rs` (line 331). The function:

- Trims leading and trailing whitespace
- Replaces internal spaces with hyphens
- Collapses multiple consecutive hyphens into a single hyphen

This was verified by the PM in issue #70's acceptance review:
- `people/aashishnair.html`: 7677 bytes, contains "Aashish Nair" 8 times
- `podcast/production-ml-search-vector-search-embeddings-hybrid-search.html`: 191180 bytes
- No generated URLs or output filenames contain spaces
- DTC structural comparison: 0 missing, 0 extra files

Six unit tests cover slug sanitization:
- `test_sanitize_slug_leading_space` -- verifies ` aashishnair` becomes `aashishnair`
- `test_sanitize_slug_internal_space` -- verifies `hybrid search` becomes `hybrid-search`
- `test_sanitize_slug_trailing_space`
- `test_sanitize_slug_normal_unchanged`
- `test_sanitize_slug_multiple_consecutive_spaces`
- `test_sanitize_slug_space_and_hyphen_collapsed`

## Acceptance Criteria

All criteria are already met by issue #70. Verification only -- no code changes needed.

- [x] `sanitize_slug()` exists in `src/collection.rs` and is applied to all slug generation paths
- [x] No generated URLs contain spaces (neither leading/trailing nor internal)
- [x] The two specific pages produce correct slugs matching Jekyll output:
  - `people/aashishnair.html` (not `people/ aashishnair.html`)
  - `podcast/production-ml-search-vector-search-embeddings-hybrid-search.html` (not `hybrid search`)
- [x] Sitemap URLs for these pages match Jekyll's sitemap URLs exactly
- [x] Unit tests exist covering leading spaces, trailing spaces, internal spaces, and consecutive spaces
- [x] DTC structural comparison shows 0 missing / 0 extra files

## Test Scenarios

All already implemented and passing as part of issue #70.

### Unit: Slug sanitization (in src/collection.rs, 6 tests)
- Leading space in filename produces slug without leading space
- Internal space in filename produces slug with hyphen
- Trailing space in filename produces slug without trailing space
- Normal filename unchanged
- Multiple consecutive spaces collapsed to single hyphen
- Mixed spaces and hyphens collapsed

### Integration: DTC site file parity (#[ignore] -- requires full site)
- DTC build produces 787 HTML files matching Jekyll
- Both specific pages exist with correct filenames

## Dependencies

- Issue #63 (feed/sitemap validation tests) -- provides the comparison tests that detected this
- Issue #70 (fix missing pages in DTC site build) -- **already implemented the fix**

## Log

### [PM Grooming] 2026-03-14

Investigated whether this issue requires any work:

1. Read issue #70 (done/70-fix-missing-pages-dtc.done.md) -- it explicitly identifies these two slug problems as "Root Cause A: Slug generation does not sanitize spaces" and fixes them.
2. Verified `sanitize_slug()` exists in `src/collection.rs` at line 331, correctly trimming whitespace and replacing spaces with hyphens.
3. Verified 6 unit tests exist covering all slug sanitization edge cases, including the two exact filenames from this issue.
4. Issue #70's PM acceptance review confirmed both pages generate correctly and no URLs contain spaces.

**Conclusion:** This issue is a duplicate of work already completed in issue #70. All acceptance criteria are already met. Moving directly to groomed status so it can be closed as already-done.

No criteria descoped -- all original acceptance criteria are verified as met by the existing implementation.
