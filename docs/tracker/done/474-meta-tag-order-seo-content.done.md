# Issue 474: Spurious page.date backfill for non-post collections (academicpages)

## Problem

Rustkyll's `backfill_default_dates()` in `src/collection.rs` assigns `site.time` as the
default `page.date` for ALL collection items (portfolio, talks, teaching, publications,
etc.). Jekyll only exposes `page.date` in the Liquid template context for post documents.
For other collections, `page.date` remains nil if not set in front matter or filename,
even though `Document#date` lazily assigns `site.time` internally.

This causes two visible problems in academicpages (and any site using custom collections
with `{% if page.date %}` conditionals):

1. **Extra `article:published_time` meta tag** -- the `_includes/seo.html` template has
   `{% if page.date %}` guards around `<meta property="article:published_time">`. Because
   rustkyll backfills a date, this tag appears when it should not, shifting all subsequent
   `<head>` children and causing cascading DOM diffs.

2. **Extra `og:type=article` tag** -- the same template outputs
   `<meta property="og:type" content="article">` inside `{% if page.date %}`. Portfolio
   and other non-post pages should not have this tag.

## Root Cause

Two locations were responsible:

1. `src/collection.rs`: `backfill_default_dates()` wrote both `item.date` AND
   `item.front_matter["date"]` for all collections. Jekyll only exposes the date
   via `page.date` (front matter) for posts.

2. `src/generator.rs`: The per-page render loop copied `item.date` back into
   `page_fm["date"]` unconditionally, which re-injected the backfilled date
   for non-post items.

## Fix

1. Added `set_frontmatter: bool` parameter to `backfill_default_dates()`.
   For posts (`set_frontmatter=true`): sets both `item.date` and `front_matter["date"]`.
   For other collections (`set_frontmatter=false`): sets only `item.date`.

2. In `generator.rs`: only copy `item.date` into `page_fm` for posts.

3. In `main.rs`: pass `is_posts` flag to `backfill_default_dates`.

This preserves:
- `site.podcast | map: "date"` still returns build_time for undated items (via `item.date`)
- `page.date` is nil for non-post items without explicit dates (matching Jekyll)

## Acceptance Criteria

- [x] `cargo build` compiles without errors
- [x] `cargo clippy -- -D warnings` is clean
- [x] `cargo fmt` produces no changes
- [x] `backfill_default_dates` only sets front_matter["date"] for the `posts` collection
- [x] For a collection item without an explicit date (e.g. `_portfolio/portfolio-1.md`),
      `page.date` is nil/absent in the template context
- [x] For a `_posts` item without an explicit date, `page.date` is still backfilled
      with the build timestamp (existing behavior preserved)
- [x] Building academicpages produces no `article:published_time` meta tag in
      `portfolio/portfolio-1/index.html`
- [x] Building academicpages produces no `og:type=article` meta tag in
      `portfolio/portfolio-1/index.html`
- [x] DTC DOM match count remains at 790/790 (no regression)
- [x] DTC docs DOM match count remains at 57/57 (no regression)
- [x] `cargo test` passes (all existing tests, plus new tests)

## Log

### [SWE] 2026-03-29

- Investigated Jekyll's actual behavior using Ruby scripts against real Jekyll:
  - `Document#date` lazily assigns `site.time` for ALL docs
  - `DocumentDrop#date` (used for `page.date` in Liquid) returns nil for non-post docs
  - `collection | map: "date"` accesses `item.date` via `collection_item_to_liquid_slim`, not via `page` context
- Initial approach (restrict backfill to posts only) broke DTC DOM (596/790) because
  podcast `season_end` computation relies on `site.podcast | map: "date"` seeing dates
- Revised approach: backfill `item.date` for ALL collections (for map/sort), but only
  set `front_matter["date"]` for posts (for `page.date`)
- Also fixed generator.rs line 1764 which unconditionally copied `item.date` into
  `page_fm`, re-injecting the backfilled date for non-post items
- TDD: wrote 3 new unit tests for the set_frontmatter flag behavior
- All 3238 tests pass (3095 lib + 143 integration), 0 failed
- Clippy clean, fmt clean
- DOM results: DTC 790/790, DTC docs 57/57, academicpages improved from ~27/45 to 29/45
  (portfolio-1 and portfolio-2 now match Jekyll perfectly)
- Files modified: src/main.rs, src/collection.rs, src/generator.rs
