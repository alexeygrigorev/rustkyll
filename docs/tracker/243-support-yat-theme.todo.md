# Issue 243: Support Yat Jekyll theme

## Problem

Yat (Yet Another Theme) is a popular Jekyll theme (~1k GitHub stars) with a modern, feature-rich design. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/jeffreytse/jekyll-theme-yat
- **Stars:** ~1,000
- **Use case:** Blogs, portfolios
- **Notable features:** Banner with animated background, dark mode, table of contents, tags/categories, math (MathJax), mermaid diagrams, search, translations/i18n

## Tasks

1. Clone the Yat theme demo site into `websites/yat/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Yat demo site cloned into `websites/yat/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
