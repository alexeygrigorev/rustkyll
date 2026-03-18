# Issue 236: Support Chirpy Jekyll theme

## Problem

Chirpy (jekyll-theme-chirpy) is a very popular Jekyll theme (~7.5k GitHub stars) for tech blogs and documentation. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/cotes2020/jekyll-theme-chirpy
- **Stars:** ~7,500
- **Use case:** Tech blogs, documentation sites
- **Notable features:** Dark/light mode toggle, table of contents sidebar, categories and tags with dedicated pages, search, SEO optimized, PWA support, mermaid diagrams

## Tasks

1. Clone the Chirpy theme demo site into `websites/chirpy/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Chirpy demo site cloned into `websites/chirpy/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
