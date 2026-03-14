# Issue 73: Re-run benchmark after performance optimizations

## Problem

docs/benchmark/results.md shows DTC at 5.925s but issue #57 brought it down to 1.05s. The benchmark file was not re-run after the Arc-backed KString and slim site context optimizations.

The README correctly shows 1.05s but the detailed benchmark results are stale.

## Goal

Re-run the full benchmark (scripts/benchmark.sh) and update docs/benchmark/results.md with current numbers.

## Scope

This is not just a timing re-run. It's a full quality validation: speed + correctness + visual match.

## Acceptance criteria

### Speed benchmark
- Full benchmark re-run with current code (scripts/benchmark.sh)
- docs/benchmark/results.md updated with actual timings
- DTC site shows ~1s (not 5.9s)
- kids-horror-stories-ru shows ~0.3-0.4s
- README benchmark table matches the results file

### Structural equivalence (for sites where both tools succeed)
- Run scripts/compare-output.sh on every site where both Jekyll and rustkyll succeed
- File tree comparison: same HTML files generated (within 5% tolerance)
- Structural elements (title, headings, links, images) match for sampled pages
- No raw Liquid tags in any rustkyll output
- No empty HTML files
- Results documented per site

### Playwright visual comparison (for DTC and kids sites)
- Run scripts/visual-compare.sh against real Jekyll output (NOT self-comparison)
- Sites served over HTTP with full CSS/images/fonts/JS
- At least 5 DTC pages and 3 kids pages compared
- Target: pixel-perfect match (<1% diff per page)
- Every difference >0% investigated with documented root cause
- Diff images saved for review
- Results documented with pass/fail per page

### Documentation
- Updated docs/benchmark/results.md with speed + quality summary
- Updated docs/comparison/structural-results.md with latest structural comparison
- New docs/comparison/visual-results.md with Playwright comparison results
- No code changes to src/
