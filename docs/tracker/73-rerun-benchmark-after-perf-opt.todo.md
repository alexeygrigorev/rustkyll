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

### Structural equivalence (ALL sites where both tools succeed)
- Run scripts/compare-output.sh on EVERY site where both Jekyll and rustkyll succeed (not just DTC and kids)
- For EACH site: file tree comparison, structural element comparison, raw Liquid check, empty file check
- Results added to docs/benchmark/results.md as new columns or a companion table
- Per-site structural match percentage documented

### Playwright visual comparison (ALL sites where both tools succeed)
- Run scripts/visual-compare.sh on EVERY site where both Jekyll and rustkyll succeed
- Sites served over HTTP with full CSS/images/fonts/JS (NOT self-comparison, NOT raw HTML)
- At minimum: homepage + one content page per site
- Target: pixel-perfect match (<1% diff per page)
- Every difference >0% investigated with documented root cause
- Diff images saved for review
- Per-site visual match results documented

### Documentation — all in docs/benchmark/results.md
The benchmark results file must contain ALL three comparisons for every site:
1. Speed (Jekyll time vs rustkyll time)
2. Structural equivalence (file count match %, structural element match %)
3. Visual match (Playwright pixel diff % for sampled pages)

This is ONE table (or linked companion tables) — not scattered across separate files. The reader should see speed, accuracy, and visual quality for every site in one place.

- No code changes to src/
