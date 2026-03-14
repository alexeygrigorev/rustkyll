# Issue 64: Fix CI/CD - add DTC site checkout to pipeline

## Problem

The CI/CD pipeline is failing because the DTC website (datatalksclub.github.io) is not checked out as part of the pipeline. Tests or builds that reference this site fail in CI.

## Goal

Add a step to the CI pipeline that checks out the DTC site into the websites/ directory so that integration tests and builds that reference it can run.

## Approach

Add a checkout step in .github/workflows/ci.yml that clones datatalksclub.github.io (shallow, depth 1) into websites/DataTalksClub/datatalksclub.github.io/.

Also consider checking out other test sites needed by integration tests (kids-horror-stories-ru, etc.).

## Dependencies

None

## Acceptance criteria

- CI pipeline passes (no failures due to missing site data)
- DTC site is available in websites/ during CI runs
- Shallow clone to minimize CI time
- Only clone sites actually needed by non-ignored tests
