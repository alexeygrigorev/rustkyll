# Issue 468: large-blog-3000 build only 3.7x faster than Jekyll

## Problem

large-blog-3000 builds in 1.2s vs 4.4s Jekyll (3.7x). Target is 10x.
Bottleneck: template rendering at scale (3001 pages).

## Scope

Investigate and optimize. The where-filter indexing (#461) may have
already helped. Re-benchmark after #461. Target: < 0.44s (10x).

## Baseline

Current: 1.2s. Jekyll: 4.4s. Target: < 0.44s.

## Progress

- Re-measured on `2026-04-03`.
- Current benchmark:
  - rustkyll: `0.960s`
  - Jekyll: `4.429s`
  - speedup: `4.61x`
- Direct release build phase split:
  - Collections: `0.108s`
  - Context: `0.041s`
  - Generation: `0.817s`
- Current shape of the site:
  - 3000 post pages + 1 index page
  - one posts collection
  - no static files
  - simple default/post layout chain
- Large-blog templates are simple and do not use `where`; the main repeated
  work is per-post layout rendering plus loops over `site.categories[cat]` and
  `site.tags[tag]`.
- Recent jekyll-docs performance work already helps here indirectly:
  rustkyll is faster than the stale `1.2s` issue baseline without any
  large-blog-specific code change.

## Notes

- The issue text is stale but the issue is still active.
- The current target gap is no longer collection loading; it is mostly
  generation-time per-page overhead.
