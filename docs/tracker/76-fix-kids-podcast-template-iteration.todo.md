# Issue 76: Fix kids podcast.xml Liquid template iteration (1/1343 items)

## Problem

Discovered in issue #63 feed/sitemap validation: the kids-horror-stories-ru podcast.xml (a Liquid template that iterates over `site.stories` collection) only produces 1 `<item>` instead of 1343. Jekyll produces all 1343 items correctly.

The podcast.xml template uses a `for` loop over `site.stories` to generate `<item>` elements. The loop is not expanding correctly in rustkyll's Liquid engine.

Additionally, the output contains raw Liquid tags, indicating incomplete template rendering.

## Evidence

From `docs/comparison/feed-sitemap-results.md`:
- Rustkyll: 1 item
- Jekyll: 1343 items
- Raw Liquid tags present in output

## Acceptance Criteria

- [ ] Build kids-horror-stories-ru; `podcast.xml` contains all collection items (within 5% of Jekyll's 1343)
- [ ] No raw Liquid tags in podcast.xml output
- [ ] `test_kids_podcast_validation` from `tests/integration_feed_sitemap.rs` passes
- [ ] `test_kids_podcast_vs_jekyll` from `tests/integration_feed_sitemap.rs` passes

## Dependencies

- Issue #63 (feed/sitemap validation tests) -- provides the tests that verify this
