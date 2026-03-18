# Issue 241: Support Hydeout Jekyll theme

## Problem

Hydeout is a popular Jekyll theme (~1k GitHub stars), an updated version of Hyde with additional features. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/fongandrew/hydeout
- **Stars:** ~1,000
- **Use case:** Personal blogs
- **Notable features:** Updated Hyde with pagination, tags/categories support, customizable sidebar, related posts, SEO tags

## Tasks

1. Clone the Hydeout theme into `websites/hydeout/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Hydeout site cloned into `websites/hydeout/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
