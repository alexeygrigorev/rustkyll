# Issue 46: Benchmark rustkyll vs Jekyll

## Problem

We need concrete performance comparisons between rustkyll and Jekyll across all test sites to quantify the speedup.

## Requirements

- Jekyll (Ruby) is already installed at `/home/alexey/.rvm/gems/ruby-3.3.7/bin/jekyll`
- For each site in `websites/` that rustkyll can build successfully, run both Jekyll and rustkyll builds
- Measure wall-clock time for each build
- Record results in a comparison table (site name, page count, Jekyll time, rustkyll time, speedup factor)
- Write results to `docs/benchmark/results.md`
- Do not use bold or italic formatting in the benchmark writeup
- Run each build 3 times minimum to get stable numbers (report median)
- Create a benchmark script (`scripts/benchmark.sh`) that can be re-run to regenerate results

## Scope

This is a scripting and documentation issue, not a code change issue. The deliverables are:

1. A benchmark script that automates the comparison
2. A results document with the data

## Sites to Benchmark

From `websites/` directory. Only benchmark sites where both Jekyll and rustkyll can complete a build. If one tool fails on a site, note it in the results but do not include it in the speedup comparison.

### Primary sites (must include):
- `datatalksclub.github.io/` (the main reference site, from `websites/DataTalksClub/datatalksclub.github.io/`)
- `alexeygrigorev/kids-horror-stories-ru` (large site, 1300+ posts)

### Secondary sites (include as many as build with both tools):
All other sites from `websites/` -- see `docs/complex-site-results.md` and `docs/cross-site-results.md` for the full list and known build status.

## Dependencies

None. This issue can be worked on independently.

## Acceptance Criteria

- [ ] `scripts/benchmark.sh` exists and is executable
- [ ] Running `scripts/benchmark.sh` produces timing results for both Jekyll and rustkyll across test sites
- [ ] The script runs each build at least 3 times and reports the median wall-clock time
- [ ] The script handles build failures gracefully (records "FAIL" instead of crashing)
- [ ] Results are written to `docs/benchmark/results.md`
- [ ] The results document contains a table with columns: site name, approximate page count, Jekyll median time, rustkyll median time, speedup factor (Nx)
- [ ] The results document does not use bold or italic formatting
- [ ] At least the DTC main site and kids-horror-stories-ru are included in the results
- [ ] The speedup factor is calculated as Jekyll time / rustkyll time
- [ ] The script cleans up build output between runs (so caches do not skew results)
- [ ] No changes to rustkyll source code (src/) are made in this issue

## Test Scenarios

This issue does not require `cargo test` tests since it is a benchmarking/documentation task, not a code change. Verification is done by running the script and inspecting results.

### Manual verification

1. Run `scripts/benchmark.sh` and confirm it completes without errors
2. Inspect `docs/benchmark/results.md` and confirm the table is present and well-formed
3. Verify the page counts are reasonable (DTC site should show ~779 pages, kids-horror-stories should show ~1344 pages)
4. Verify timing numbers are plausible (rustkyll should be faster than Jekyll for all sites)
5. Verify the script uses `./scripts/cargo-safe` (not raw `cargo`) to build rustkyll if it needs to compile it
6. Verify the script removes previous build output (e.g., `_site/`) before each timed run

### Edge cases to handle

- A site fails to build with Jekyll but succeeds with rustkyll (or vice versa): mark as FAIL for that tool, exclude from speedup comparison
- A site takes too long with Jekyll (over 5 minutes): allow a timeout and note it
- The `websites/` directory might not exist on a fresh clone: the script should print a clear error message
