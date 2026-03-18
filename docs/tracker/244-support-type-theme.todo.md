# Issue 244: Support Type Jekyll theme

## Problem

Type is a popular Jekyll theme (~1k GitHub stars) focused on typography and clean writing. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/rohanchandra/type-theme
- **Stars:** ~1,000
- **Use case:** Personal blogs, writing-focused sites
- **Notable features:** Typography-focused, Google Fonts integration, social links, Disqus comments, Google Analytics, tags, customizable colors, share buttons

## Tasks

1. Clone the Type theme into `websites/type-theme/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Type theme site cloned into `websites/type-theme/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
