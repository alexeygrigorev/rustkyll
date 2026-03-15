# Issue 110: Fix CI — all tests must run, no skipping

## Priority

HIGH — tests exist for a reason. Skipping them hides bugs.

## Problem

The CI integration job skips 7+ test categories with `--skip` flags:
```
--skip structural_comparison --skip vs_jekyll --skip kids_ --skip page_count --skip _notes_exist --skip _stories_exist --skip build_time
```

This defeats the purpose of having tests. Tests must always run in CI. If a test needs a site, clone it. If it needs Jekyll, install it. If it's too slow, optimize it — don't skip it.

## Goal

Remove ALL `--skip` flags from the CI integration test command. Every test must run and pass.

## What needs to happen

1. **Clone all required sites** in CI: DTC site, kids-horror-stories-ru, and any benchmark sites needed by page_count tests
2. **Install Jekyll** in CI: `gem install bundler jekyll` + `bundle install` for sites that need it — this enables vs_jekyll tests and structural_comparison tests
3. **Fix build_time tests**: Either raise thresholds for CI runners or make them adaptive (detect CI and use appropriate threshold)
4. **Remove all --skip flags** from the cargo test command

## Acceptance criteria

- CI cargo test command has ZERO `--skip` flags
- ALL ignored integration tests run and pass
- DTC site, kids-horror-stories-ru cloned in CI
- Jekyll installed in CI for comparison tests
- Structural comparison tests run against real Jekyll output
- Build time tests pass on CI hardware
- No test failures
