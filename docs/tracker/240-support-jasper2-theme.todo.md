# Issue 240: Support Jasper2 Jekyll theme

## Problem

Jasper2 is a popular Jekyll theme (~1.8k GitHub stars) that ports the Ghost default theme to Jekyll. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/jekyller/jasper2
- **Stars:** ~1,800
- **Use case:** Blogs, content publishing
- **Notable features:** Ghost (Casper) design port, cover images, author pages, tag pages, responsive, navigation menu

## Tasks

1. Clone the Jasper2 theme into `websites/jasper2/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Jasper2 site cloned into `websites/jasper2/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
