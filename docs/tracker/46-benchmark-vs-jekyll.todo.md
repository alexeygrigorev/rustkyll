# Issue 46: Benchmark rustkyll vs Jekyll

## Problem

We need concrete performance comparisons between rustkyll and Jekyll across all test sites to quantify the speedup.

## Requirements

- Install Jekyll (Ruby) if not already available
- For each site we use for testing (DTC site, alexeygrigorev repos, DataTalksClub repos, complex external sites), run both Jekyll and rustkyll builds
- Measure wall-clock time for each build
- Record results in a comparison table (site name, page count, Jekyll time, rustkyll time, speedup factor)
- Write results to docs/benchmark/ folder (not in README directly)
- Do not use bold or italic formatting in the benchmark writeup
- Run each build multiple times to get stable numbers

See these docs for the list of sites:

docs/complex-site-results.md
docs/cross-site-results.md


## Dependencies

None.
