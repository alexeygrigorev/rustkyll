# Issue 149: Fix podcast page sorting to match Jekyll

## Priority

CRITICAL — user-reported. Podcast episodes appear in wrong order on the podcast listing page.

## Problem

The podcast page (/podcast.html) shows episodes in a different order than Jekyll. This is visible to users and breaks the site experience.

## Goal

Podcast episodes must appear in the exact same order as Jekyll on the podcast listing page. Investigate how Jekyll sorts them (by date? by filename? by episode number?) and match exactly.

## Approach

1. Build DTC site with both Jekyll and rustkyll
2. Compare the podcast.html page — extract episode order from both
3. Identify the sorting difference
4. Fix the sort to match Jekyll
5. Verify with DOM comparison and Playwright

## Acceptance criteria

- Podcast episodes appear in same order as Jekyll on /podcast.html
- DOM comparison shows 0 diffs for podcast episode ordering
- Playwright pixel diff for /podcast.html is 0%
- No regressions on other pages
- Test: write a failing test that checks episode order, fix, test passes

## Log

### [SWE] 2026-03-16

**Root causes found:** Two bugs causing podcast page to differ from Jekyll:

1. **Numeric sort in Liquid `sort` filter**: The `| sort | reverse` chain on season numbers
   did string comparison ("9" > "23") instead of numeric comparison (23 > 9). This caused
   seasons to appear in order 9, 8, 7, ..., 3, 23, 22, ... instead of 23, 22, 21, ..., 1.
   Fixed `nil_safe_compare()` in `src/template/filters/sort.rs` to try `parse::<f64>()`
   before falling back to string comparison, matching Jekyll/Ruby's `<=>` operator.

2. **Document ID preserving spaces**: Jekyll's `document.id` preserves spaces from the
   original filename (e.g. `_podcast/hybrid search.md` -> id `/podcast/hybrid search`),
   while rustkyll was deriving `id` from the permalink URL which has hyphens. Added a
   new `id` field to `CollectionItem` computed from the raw filename stem, and updated
   `generator.rs` to use `item.id` instead of deriving from `item.url`.

**Tests added:**
- `test_collection_item_id_preserves_spaces_in_filename` - verifies non-post collection id preserves spaces
- `test_post_item_id_uses_date_path` - verifies post id uses /YYYY/MM/DD/slug format
- `test_sort_integers_numerically` - verifies sort filter compares numbers numerically
- `test_sort_scalars_numerically` - verifies bare numeric scalar sorting

**Verification:** Built DTC site with rustkyll and compared all 193 podcast episodes against
Jekyll output. Result: PERFECT MATCH (identical season order, episode order, and URLs).

**Build:** 1491 tests pass, 0 fail, clippy clean, fmt clean.

**Files modified:**
- `src/template/filters/sort.rs` - numeric comparison in nil_safe_compare
- `src/collection.rs` - added `id` field to CollectionItem, compute from raw filename
- `src/generator.rs` - use `item.id` instead of deriving from url
- `src/feed.rs` - added id field to test helpers
- `src/jsonld.rs` - added id field to test helpers
- `src/pagination.rs` - added id field to test helpers
- `src/sitemap.rs` - added id field to test helpers
- `tests/integration_performance.rs` - added id field to test helpers
