# Issue 86: Fix benchmark site regressions (beautiful-jekyll, jekyll-docs, homebrew-site)

## Problem

Issue #73 benchmark rerun revealed 3 site regressions compared to the previous benchmark run:

1. **beautiful-jekyll** -- previously built with rustkyll (0.022s), now FAIL. Likely caused by kramdown compatibility changes in issue #84.
2. **jekyll-docs/docs** -- previously built with rustkyll (0.060s), now FAIL. Likely caused by kramdown compatibility changes in issue #84.
3. **homebrew-site** -- previously built with Jekyll (1.212s), now Jekyll FAIL. This may be a Jekyll environment issue (missing gems after system updates) rather than a rustkyll bug, but should be investigated.

The dual-success site count dropped from 16 to 14 (net: lost 3, gained muan-blog).

## Goal

Investigate and fix the 3 regressions so that all 3 sites build successfully again with both tools where they did before. Restore the dual-success count to at least 16.

## Scope

- Diagnose root cause for each regression
- Fix rustkyll build failures for beautiful-jekyll and jekyll-docs/docs
- Investigate homebrew-site Jekyll failure (may be environment, not rustkyll)
- Update docs/benchmark/results.md if fixes change the numbers

## Dependencies

- Issue #73 (rerun-benchmark-after-perf-opt) -- must be DONE first
- Issue #84 (kramdown-compatibility) -- DONE (likely caused the regressions)

## Acceptance Criteria

- [ ] beautiful-jekyll builds successfully with rustkyll
- [ ] jekyll-docs/docs builds successfully with rustkyll
- [ ] homebrew-site root cause documented (fix if rustkyll issue, document if Jekyll environment issue)
- [ ] `./scripts/cargo-safe test` passes
- [ ] No other sites regress as a result of fixes
