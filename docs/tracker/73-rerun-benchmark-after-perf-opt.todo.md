# Issue 73: Re-run benchmark after performance optimizations

## Problem

docs/benchmark/results.md shows DTC at 5.925s but issue #57 brought it down to 1.05s. The benchmark file was not re-run after the Arc-backed KString and slim site context optimizations.

The README correctly shows 1.05s but the detailed benchmark results are stale.

## Goal

Re-run the full benchmark (scripts/benchmark.sh) and update docs/benchmark/results.md with current numbers.

## Acceptance criteria

- Full benchmark re-run with current code
- docs/benchmark/results.md updated with actual timings
- DTC site shows ~1s (not 5.9s)
- kids-horror-stories-ru shows ~0.3-0.4s
- README benchmark table matches the results file
- No code changes to src/
