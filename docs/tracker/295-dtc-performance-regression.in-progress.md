# Issue 295: DTC build performance regression — target < 1s

## Problem

DTC (datatalksclub.github.io, 790 pages) build time regressed from ~1.0s to ~1.7s. The target is < 1s.

large-blog-3000 (3001 pages) also regressed from 1.09s to 2.55s (2.3x slower).

## Likely culprits (added since last fast benchmark)

Several new per-page processing steps were added:
- `escape_quotes_in_text_nodes()` — O(n) scan of every page content + layout content (4 call sites)
- `normalize_block_whitespace()` — O(n) scan of page content + layout content (3 call sites)
- `normalize_newlines_in_html_tags()` — O(n) scan in postprocess()
- `convert_kramdown_underscore_runs()` — preprocessing every markdown input
- `convert_display_math_blocks()` — in postprocess()
- `autolink_bare_urls()` — preprocessing (only for CommonMarkGhPages sites, should not affect DTC)

## Approach

1. Profile the build to identify which phase is slow (config, collections, pages, layouts, generation)
2. Measure each new function's contribution with targeted benchmarks
3. Optimize hot paths:
   - Short-circuit functions when input has no relevant characters (e.g., skip `escape_quotes_in_text_nodes` if no `"` in text nodes)
   - Skip `convert_kramdown_underscore_runs` if no `____` pattern in input
   - Skip `normalize_newlines_in_html_tags` if no newlines inside tags
   - Consider lazy/on-demand processing instead of eager per-page transforms
4. Re-benchmark after each optimization

## Acceptance Criteria

- [ ] DTC builds in < 1.0s (release mode)
- [ ] large-blog-3000 builds in < 1.5s (release mode)
- [ ] No DOM regressions
- [ ] `cargo test` passes
