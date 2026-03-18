# Issue 238: Support TeXt Jekyll theme

## Problem

TeXt is a popular Jekyll theme (~3k GitHub stars) with rich features for content-heavy sites. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/kitian616/jekyll-TeXt-theme
- **Stars:** ~3,000
- **Use case:** Personal blogs, documentation, content-heavy sites
- **Notable features:** Skins and color schemes, table of contents, tags/categories, search, charts (Chart.js), mermaid diagrams, math (MathJax), multiple layout options, i18n support

## Tasks

1. Clone the TeXt theme demo site into `websites/text-theme/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] TeXt demo site cloned into `websites/text-theme/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
