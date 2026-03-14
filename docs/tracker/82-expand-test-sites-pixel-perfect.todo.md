# Issue 82: Expand test sites and achieve pixel-perfect generation

## Problem

We currently verify pixel-perfect generation on only 2 sites (DTC and kids-horror-stories-ru). We need broader coverage to be confident rustkyll is a true drop-in Jekyll replacement.

## Goal

1. Find 10+ additional real Jekyll sites (diverse: blogs, docs, portfolios, organizations)
2. Build each with both Jekyll and rustkyll
3. Achieve pixel-perfect Playwright screenshot match on ALL sites (0% diff, only timestamps excepted)
4. Fix any rendering differences found
5. Document results

## Approach

1. Clone sites into websites/
2. Build with Jekyll, build with rustkyll
3. Run structural comparison (file tree, page count — must be exact)
4. Run Playwright visual comparison (must be 0% pixel diff)
5. For every difference: investigate, fix, or create follow-up issue
6. Update benchmark results with all 3 comparisons (speed, structural, visual)

## Dependencies

- Issue 73 (kramdown compatibility) should be done first — fixes the known rendering gaps
- Issue 72 (visual comparison infrastructure) done

## Acceptance criteria

- At least 10 new real Jekyll sites tested
- Page counts match Jekyll exactly on every site
- Playwright visual comparison: 0% pixel diff on all tested pages (only dynamic timestamps excepted)
- Every difference investigated with documented root cause
- Results documented in docs/benchmark/results.md (speed + structural + visual for all sites)
- No regressions on existing sites
