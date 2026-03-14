# Issue 46: Benchmark rustkyll vs Jekyll

## Problem

We need concrete performance comparisons between rustkyll and Jekyll across all test sites to quantify the speedup.

## Requirements

- Install Jekyll (Ruby) if not already available
- For each site we use for testing (DTC site, alexeygrigorev repos, DataTalksClub repos, complex external sites), run both Jekyll and rustkyll builds
- Measure wall-clock time for each build
- Record results in a comparison table (site name, page count, Jekyll time, rustkyll time, speedup factor)
- Include the results in the README or a separate benchmark document
- Run each build multiple times to get stable numbers

## Dependencies

- Issue #45 (README, for including results)
