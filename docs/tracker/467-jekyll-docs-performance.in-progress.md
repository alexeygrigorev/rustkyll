# Issue 467: jekyll-docs build only 2.5x faster than Jekyll

## Problem

jekyll-docs builds in 1.2s vs 3.0s Jekyll (2.5x). Target is 10x.
Bottleneck: Collections 0.7s + Generation 0.6s for 131 pages.

## Scope

Investigate and optimize. Target: < 0.3s (10x).

## Baseline

Current: 1.2s. Jekyll: 3.0s. Target: < 0.3s.

## Progress

- Re-measured current baseline on `2026-04-03`: rustkyll was actually about `1.47s`,
  Jekyll about `3.12s`, so the issue description was stale.
- Added a cached fast path for large-array `where` lookups in
  `src/template/filters/where_filter.rs`. This helped a little but was not the
  main bottleneck.
- The real hot spot was duplicate markdown work during collection loading:
  markdown documents that contain non-highlight Liquid were being converted to
  `html_content` up front, then rendered again with Liquid during page generation.
- Current partial fix:
  - collection loading leaves `html_content` empty for markdown files with
    non-highlight Liquid
  - collection generation falls back to
    `render_markdown_content_with_cached_site()` when such items have no layout
- Current measured result on `jekyll-docs/docs`:
  - rustkyll: `0.96s`
  - Jekyll: `3.12s`
  - speedup: about `3.25x`
  - collections phase dropped from about `0.64s` to about `0.02s`
- Validation:
  - `cargo test -p rustkyll` passes
  - `scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` stays at `596/790`
  - `scripts/recount-all-dom.sh --site jekyll-docs/docs` reports `22/125`

## Notes

- The stored jekyll-docs DOM baseline appears stale. The current code before and
  after the optimization reports `22/125`, not the `48` currently recorded in
  `docs/dom-baselines.json`.
- This issue is not done yet: the new performance is materially better, but
  still far from the `< 0.3s` target.
