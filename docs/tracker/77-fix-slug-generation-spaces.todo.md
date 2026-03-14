# Issue 77: Fix slug generation producing URLs with spaces

## Problem

Discovered in issue #63 sitemap comparison: two sitemap URLs contain spaces instead of hyphens in their slugs:

1. `https://datatalks.club/podcast/production-ml-search-vector-search-embeddings-hybrid search.html` -- should be `hybrid-search`
2. `https://datatalks.club/people/ aashishnair.html` -- leading space, should be `aashishnair`

Jekyll produces the correct slugs for both of these pages.

## Evidence

From `docs/comparison/feed-sitemap-results.md` DTC sitemap comparison:
- 8 rustkyll-only URLs, 2 of which are duplicates of Jekyll URLs but with malformed slugs

## Acceptance Criteria

- [ ] No generated URLs contain spaces (neither leading/trailing nor internal)
- [ ] The two specific pages produce correct slugs matching Jekyll output
- [ ] Sitemap URLs for these pages match Jekyll's sitemap URLs exactly

## Dependencies

- Issue #63 (feed/sitemap validation tests) -- provides the comparison tests that detect this
