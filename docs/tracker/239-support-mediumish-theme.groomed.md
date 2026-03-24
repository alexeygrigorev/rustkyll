# Issue 239: Support Mediumish Jekyll theme

## Problem

Mediumish is a popular Jekyll theme (~2k GitHub stars) that provides a Medium.com-like blogging experience. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/wowthemesnet/mediumish-theme-jekyll
- **Stars:** ~2,000
- **Use case:** Blogs, content publishing
- **Notable features:** Medium-like design, featured posts, author boxes, lazy loading images, related posts, categories, SEO optimized, newsletter integration

## Tasks

1. Clone the Mediumish theme demo site into `websites/mediumish/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Mediumish demo site cloned into `websites/mediumish/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
