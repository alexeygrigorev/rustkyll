# Issue 242: Support Basically Basic Jekyll theme

## Problem

Basically Basic is a Jekyll theme (~1k GitHub stars) by the creator of Minimal Mistakes and So Simple. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/mmistakes/jekyll-theme-basically-basic
- **Stars:** ~1,000
- **Use case:** Personal blogs, simple sites
- **Notable features:** By mmistakes (same author as minimal-mistakes), clean minimal design, skin support, search (Algolia/Lunr), breadcrumbs, resume/CV layout, responsive

## Tasks

1. Clone the Basically Basic theme demo site into `websites/basically-basic/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Basically Basic demo site cloned into `websites/basically-basic/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
