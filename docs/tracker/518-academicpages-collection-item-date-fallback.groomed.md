# Issue 518: Restore date fallback for non-post collection items in template context

## Problem

Issue #474 correctly identified that Jekyll's `DocumentDrop#date` returns nil for
non-post collection items when accessed via `page.date` in the page's own template.
However, the fix went too far: it also removed the date from the iteration context
(`post.date` when looping over `collection.docs`).

In Jekyll, `Document#date` lazily assigns `site.time` for ALL documents. The
`DocumentDrop` (Liquid interface) exposes this date for ALL documents when iterated
in loops like `{% for post in collection.docs %}`. The nil-return behavior only
applies to the special `page` variable in the page's own template context.

### Visible symptoms (academicpages, 7 pages, ~155 diffs)

1. **collection-archive/index.html (10 diffs)**: Portfolio items show excerpt instead
   of date in `archive-single.html` because `post.date` is nil
2. **portfolio/index.html (10 diffs)**: Same issue for portfolio listing
3. **portfolio/portfolio-1/index.html (23 diffs)**: Missing `article:published_time`
   and `og:type=article` meta tags in `<head>`, causing cascading position diffs
4. **portfolio/portfolio-2/index.html (23 diffs)**: Same as portfolio-1
5. **sitemap/index.html (88 diffs)**: All collection items (portfolio, publications,
   talks, teaching) show excerpt instead of date, plus missing page

### Root cause

Two changes from #474 interact:

1. `backfill_default_dates()` in `src/collection.rs` no longer sets
   `front_matter["date"]` for non-post items. This is correct for `page.date` but
   wrong for iteration context.

2. `generator.rs` no longer copies `item.date` into `page_fm["date"]` for non-post
   items. This prevents the SEO template from seeing `page.date` and emitting the
   `article:published_time` meta tag.

The fix needs to restore date visibility in the right contexts while preserving
the #474 fix for the `page.date` nil behavior in page-level templates.

### Key insight from Jekyll source

In Jekyll's Ruby code:
- `Document#date` returns `site.time` fallback for all docs (used in sorts, maps)
- `DocumentDrop#date` returns nil for non-post docs only when the page variable
  is built via `page_payload` (the page's own rendering context)
- When a document is accessed via collection iteration (`site.portfolio`,
  `collection.docs`), it goes through `to_liquid` which includes the date

The simplest correct fix: **revert the #474 restriction**. Set `front_matter["date"]`
for all collection items during backfill. The academic pages SEO template's
`{% if page.date %}` will be true for all items, matching Jekyll.

The original #474 analysis was based on reading Jekyll source code comments that
suggested `page.date` should be nil for non-posts. But the actual Jekyll cached
HTML output proves that `page.date` IS available for portfolio items (the
`article:published_time` meta tag IS present in the Jekyll reference output).

## Affected Sites

- academicpages (primary, 7 pages, ~155 diffs)
- Potentially any site using custom collections with date-conditional templates

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790
- Academicpages baseline: 38/45 (must improve, not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes (all existing tests, plus new tests)
- [ ] DTC DOM match count must not drop below 790/790
- [ ] DTC docs DOM must not regress
- [ ] Academicpages DOM match count improves from 38/45 to at least 42/45
- [ ] Building academicpages: `portfolio/portfolio-1/index.html` contains
      `<meta property="article:published_time"` tag
- [ ] Building academicpages: `collection-archive/index.html` portfolio items
      show `<p class="page__date">` (not `<p class="archive__item-excerpt">`)
- [ ] Building academicpages: `sitemap/index.html` portfolio items show dates
- [ ] The fix does not break the DTC podcast `season_end` computation
      (which relies on `site.podcast | map: "date"`)

## Test Scenarios

### Unit: Date backfill for non-post items

- Create a portfolio-type collection item with no date in frontmatter
- Verify `front_matter["date"]` is set to the build time after backfill
- Verify the date is accessible as `page.date` in the template context

### Unit: Regression - DTC podcast date mapping

- Create a podcast collection with items that have no explicit dates
- Verify `site.podcast | map: "date"` returns build time values (not nil)

### Integration: Academicpages portfolio meta tags

- Build academicpages, check portfolio-1 `<head>` contains `article:published_time`
- Build academicpages, check portfolio-1 `<head>` contains `og:type=article`

### Integration: Academicpages archive-single date display

- Build academicpages, check collection-archive portfolio items have `page__date` class
- Build academicpages, check sitemap portfolio section shows dates

### Integration: DOM comparison

- Run DOM comparison on academicpages, verify 42+ of 45 pages match
- Run DOM comparison on DTC, verify 790/790 maintained
