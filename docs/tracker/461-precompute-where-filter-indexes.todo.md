# Issue 461: Pre-compute where filter indexes for collection lookups

## Problem

DTC podcast pages call the `where` Liquid filter 15+ times per page
against 428-item arrays (site.people, site.podcasts). Each call is O(n)
linear scan. With 194 podcast pages, this is ~2,900 linear scans.

## Approach

Build hash indexes during `CachedSiteContext` construction:
- For each collection array, create `HashMap<(field_name, value), Vec<&Item>>`
- When `where` filter is called, check if an index exists and use O(1) lookup
- Fall back to linear scan for non-indexed collections

## Expected Impact

Podcast pages are ~55% of DTC generation time. The `where` filter is
the dominant cost per page. Pre-computing indexes could reduce this
by 20-30%, bringing DTC from ~1.0s toward ~0.7s.

## Acceptance Criteria

- [ ] DTC build time < 0.75s (median of 3 runs)
- [ ] DTC DOM stays at 790/790
- [ ] No regression on any other site
- [ ] Only accept if measurable >20% improvement

## Files

- `src/template/filters/where_exp.rs` — the where filter implementation
- `src/template/engine.rs` — CachedSiteContext construction
