# Issue 237: Support Lanyon Jekyll theme

## Problem

Lanyon is a popular Jekyll theme (~3.2k GitHub stars), a companion to Hyde with a toggle sidebar. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/poole/lanyon
- **Stars:** ~3,200
- **Use case:** Personal blogs, simple sites
- **Notable features:** Toggle sidebar, based on Poole (like Hyde), clean responsive layout, multiple color schemes

## Tasks

1. Clone the Lanyon theme into `websites/lanyon/`
2. Build with both Jekyll and rustkyll
3. Run DOM comparison and record match rate
4. Identify and fix any theme-specific rendering issues

## Acceptance Criteria

- [ ] Lanyon site cloned into `websites/lanyon/`
- [ ] Jekyll build succeeds and produces reference output
- [ ] rustkyll build succeeds without errors
- [ ] DOM comparison run and match rate recorded
- [ ] Any theme-specific Liquid filters or layout patterns identified

## Dependencies

- None (research/benchmark task)
