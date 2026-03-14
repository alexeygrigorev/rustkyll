# Issue 68: CI integration test — clone DTC site and run extensive test suite

## Problem

The CI currently has no integration tests against the real DTC site. The structural comparison, visual comparison, and performance tests only run locally.

## Goal

Add a CI job that:
1. Clones the DTC website (datatalksclub.github.io) in CI
2. Builds it with rustkyll
3. Runs the extensive test suite: structural comparison, output validation, performance checks

## Approach

1. Add a new GitHub Actions job (or extend ci.yml) that:
   - Clones datatalksclub.github.io (shallow, depth 1) into websites/
   - Optionally clones kids-horror-stories-ru too
   - Builds rustkyll in release mode
   - Runs `cargo test -- --ignored` to execute the large-site integration tests
   - Runs `scripts/compare-output.sh --site DataTalksClub/datatalksclub.github.io`
   - Reports pass/fail
2. This job should run on PRs and pushes to main (not just tags)
3. It will be slower than the unit test job, so it should be a separate job

## Dependencies

- Issue 67 (fix CI basics) should be done first
- Issue 61 (structural comparison) done
- Issue 62 (Playwright comparison) done

## Acceptance criteria

- CI job clones DTC site and builds it with rustkyll
- Build succeeds (no errors, no timeout)
- Structural comparison script runs and passes
- Integration tests with #[ignore] run and pass
- Page count matches expected (within 5%)
- No raw Liquid tags in output
- Job runs on push to main and PRs
- Job is separate from the fast unit test job
- Total CI time for this job is under 10 minutes
